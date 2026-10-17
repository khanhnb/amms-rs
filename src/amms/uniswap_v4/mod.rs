//! Uniswap V4 pools.
//!
//! All V4 pools live in one singleton, the PoolManager, and are identified by a `PoolId`
//! (`keccak256(abi.encode(PoolKey))`) instead of an address. The bot keys pools by address
//! everywhere, so each pool gets a synthetic address: the low 20 bytes of its `PoolId`
//! (`UniswapV4Pool::address`). The real storage is in the PoolManager (`pools[id]` mapping, see
//! `UniswapV4Pool::state_slot`).
//!
//! As for Algebra, only discovery and static info are kept here: the bot simulates swaps with the
//! PoolManager bytecode in revm and keeps its storage fresh from block traces, so the V4 tick
//! math is not mirrored (`simulate_swap` is not supported).
//!
//! Native ETH: a currency of `address(0)` is exposed as the wrapped native token (WETH) so the
//! rest of the bot only sees ERC20s; `native0` / `native1` keep the real currency, and the Arb /
//! LilRouter contracts unwrap / wrap around the hop.
//!
//! Hooks: pools with a hook are skipped at discovery (a hook can change the swap result, take
//! fees or revert). Only `hooks == address(0)` pools are supported.
//!
//! PoolManager event:
//! `Initialize(bytes32 indexed id, address indexed currency0, address indexed currency1,
//!             uint24 fee, int24 tickSpacing, address hooks, uint160 sqrtPriceX96, int24 tick)`

use super::{
    amm::{AMMType, AutomatedMarketMaker, FlashType, SwapType, AMM},
    error::AMMError,
    factory::{AutomatedMarketMakerFactory, DiscoverySync},
    get_token_decimals, Token,
};
use crate::finish_progress;
use alloy::{
    eips::BlockId,
    network::Network,
    primitives::{keccak256, Address, B256, U256},
    providers::Provider,
    rpc::types::{Filter, FilterSet, Log},
    sol,
    sol_types::{SolEvent, SolValue},
};
use futures::{stream::FuturesUnordered, StreamExt};
use indicatif::ProgressBar;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, future::Future};
use thiserror::Error;
use tracing::info;

sol! {
    #[allow(missing_docs)]
    #[derive(Debug)]
    contract IPoolManager {
        event Initialize(
            bytes32 indexed id,
            address indexed currency0,
            address indexed currency1,
            uint24 fee,
            int24 tickSpacing,
            address hooks,
            uint160 sqrtPriceX96,
            int24 tick
        );
    }
}

/// Storage slot of the `pools` mapping in the PoolManager (`StateLibrary.POOLS_SLOT`).
pub const POOLS_SLOT: u64 = 6;
/// Offsets inside a `Pool.State` struct (`StateLibrary`).
pub const SLOT0_OFFSET: u64 = 0;
pub const FEE_GROWTH_GLOBAL0_OFFSET: u64 = 1;
pub const FEE_GROWTH_GLOBAL1_OFFSET: u64 = 2;
pub const LIQUIDITY_OFFSET: u64 = 3;
pub const TICKS_OFFSET: u64 = 4;
pub const TICK_BITMAP_OFFSET: u64 = 5;
/// Fee value meaning "dynamic fee set by the hook" (`LPFeeLibrary.DYNAMIC_FEE_FLAG`).
pub const DYNAMIC_FEE_FLAG: u32 = 0x80_0000;

#[derive(Error, Debug)]
pub enum UniswapV4Error {
    #[error("Uniswap V4 pools are simulated in revm only, off-chain swap math is not implemented")]
    SimulationNotSupported,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UniswapV4Pool {
    /// Synthetic address: low 20 bytes of `pool_id`.
    pub address: Address,
    pub pool_id: B256,
    /// The PoolManager holding the pool.
    pub pool_manager: Address,
    /// currency0 / currency1, with native ETH replaced by the wrapped native token.
    pub token_a: Token,
    pub token_b: Token,
    /// true when currency0 / currency1 is native ETH (`address(0)`).
    pub native0: bool,
    pub native1: bool,
    /// LP fee in hundredths of a bip (uint24).
    pub fee: u32,
    pub tick_spacing: i32,
    pub hooks: Address,
    pub amm_type: AMMType,
    pub swap_type: SwapType,
    pub flash_type: FlashType,
}

impl UniswapV4Pool {
    /// Synthetic pool address for a `PoolId`.
    pub fn address_from_id(id: B256) -> Address {
        Address::from_word(id)
    }

    /// Base storage slot of `pools[id]` in the PoolManager: `keccak256(abi.encode(id, POOLS_SLOT))`.
    pub fn state_slot(&self) -> U256 {
        state_slot(self.pool_id)
    }

    /// Currency0 / currency1 as the PoolManager sees them (`address(0)` for native ETH).
    pub fn currency0(&self) -> Address {
        if self.native0 { Address::ZERO } else { self.token_a.address }
    }

    pub fn currency1(&self) -> Address {
        if self.native1 { Address::ZERO } else { self.token_b.address }
    }

    /// `PoolId` of a PoolKey: `keccak256(abi.encode(currency0, currency1, fee, tickSpacing, hooks))`.
    pub fn compute_id(
        currency0: Address,
        currency1: Address,
        fee: u32,
        tick_spacing: i32,
        hooks: Address,
    ) -> B256 {
        // abi.encode of a static struct: 5 words
        let fee = U256::from(fee);
        let tick_spacing = alloy::primitives::I256::try_from(tick_spacing).unwrap_or_default();
        keccak256((currency0, currency1, fee, tick_spacing, hooks).abi_encode())
    }
}

/// Base storage slot of `pools[id]` in the PoolManager.
pub fn state_slot(id: B256) -> U256 {
    keccak256((id, U256::from(POOLS_SLOT)).abi_encode()).into()
}

impl AutomatedMarketMaker for UniswapV4Pool {
    fn address(&self) -> Address {
        self.address
    }

    fn sync_events(&self) -> Vec<B256> {
        // state comes from block traces of the PoolManager in the bot, not from events
        vec![]
    }

    fn sync(&mut self, _log: &Log) -> Result<(), AMMError> {
        Ok(())
    }

    fn tokens(&self) -> Vec<Address> {
        vec![self.token_a.address, self.token_b.address]
    }

    fn token0(&self) -> Token {
        self.token_a.clone()
    }

    fn token1(&self) -> Token {
        self.token_b.clone()
    }

    fn amm_type(&self) -> AMMType {
        self.amm_type
    }

    fn swap_type(&self) -> SwapType {
        self.swap_type
    }

    fn flash_type(&self) -> FlashType {
        self.flash_type
    }

    fn calculate_price(&self, _base: Address, _quote: Address) -> Result<f64, AMMError> {
        Err(UniswapV4Error::SimulationNotSupported.into())
    }

    fn simulate_swap(
        &self,
        _base: Address,
        _quote: Address,
        _amount_in: U256,
    ) -> Result<U256, AMMError> {
        Err(UniswapV4Error::SimulationNotSupported.into())
    }

    fn simulate_swap_mut(
        &mut self,
        _base: Address,
        _quote: Address,
        _amount_in: U256,
    ) -> Result<U256, AMMError> {
        Err(UniswapV4Error::SimulationNotSupported.into())
    }

    async fn init<N, P>(mut self, _block_number: BlockId, provider: P) -> Result<Self, AMMError>
    where
        N: Network,
        P: Provider<N> + Clone,
    {
        // the PoolKey is fully known from the Initialize event, only token info is missing
        self.token_a = Token::new(self.token_a.address, provider.clone()).await?;
        self.token_b = Token::new(self.token_b.address, provider).await?;
        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct UniswapV4Factory {
    /// The PoolManager (emits `Initialize`).
    pub address: Address,
    pub creation_block: u64,
    pub sync_step: u64,
    /// Wrapped native token that stands for native ETH (`address(0)`) currencies.
    pub wrapped_native: Address,
    pub amm_type: AMMType,
    pub swap_type: SwapType,
    pub flash_type: FlashType,
}

impl UniswapV4Factory {
    pub fn new(
        address: Address,
        creation_block: u64,
        sync_step: u64,
        wrapped_native: Address,
        amm_type: AMMType,
        swap_type: SwapType,
        flash_type: FlashType,
    ) -> Self {
        Self { address, creation_block, sync_step, wrapped_native, amm_type, swap_type, flash_type }
    }

    /// Pool from an `Initialize` log, or `None` for pools the bot cannot trade (hooks, dynamic
    /// fee, or both currencies mapping to the same token, e.g. ETH / WETH).
    pub fn pool_from_log(&self, log: &Log) -> Result<Option<UniswapV4Pool>, AMMError> {
        let event = IPoolManager::Initialize::decode_log(&log.inner)?;
        let fee: u32 = event.fee.to();
        if event.hooks != Address::ZERO || fee & DYNAMIC_FEE_FLAG != 0 {
            return Ok(None);
        }
        let native0 = event.currency0 == Address::ZERO;
        let native1 = event.currency1 == Address::ZERO;
        let wrap = |c: Address| if c == Address::ZERO { self.wrapped_native } else { c };
        let (token0, token1) = (wrap(event.currency0), wrap(event.currency1));
        if token0 == token1 {
            return Ok(None);
        }
        let id = event.id;
        Ok(Some(UniswapV4Pool {
            address: UniswapV4Pool::address_from_id(id),
            pool_id: id,
            pool_manager: self.address,
            token_a: token0.into(),
            token_b: token1.into(),
            native0,
            native1,
            fee,
            tick_spacing: event.tickSpacing.as_i32(),
            hooks: event.hooks,
            amm_type: self.amm_type,
            swap_type: self.swap_type,
            flash_type: self.flash_type,
        }))
    }

    pub async fn get_all_pools<N, P>(
        &self,
        block_number: BlockId,
        provider: P,
        pb: Option<&ProgressBar>,
    ) -> Result<Vec<AMM>, AMMError>
    where
        N: Network,
        P: Provider<N> + Clone,
    {
        let disc_filter = Filter::new()
            .event_signature(FilterSet::from(vec![self.pool_creation_event()]))
            .address(vec![self.address]);
        let to = block_number.as_u64().unwrap_or_default();
        let step = self.sync_step.max(1);
        pb.iter().for_each(|f| f.set_length(to.saturating_sub(self.creation_block) + 1));

        let mut futures = FuturesUnordered::new();
        let mut from = self.creation_block;
        while from < to {
            let end = (from + step).min(to);
            let filter = disc_filter.clone().from_block(from).to_block(end);
            let provider = provider.clone();
            futures.push(async move { provider.get_logs(&filter).await });
            from = end + 1;
        }

        let (mut pools, mut skipped) = (vec![], 0usize);
        let mut progress = 0;
        while let Some(res) = futures.next().await {
            progress += step;
            pb.iter().for_each(|f| f.set_position(progress));
            for log in res? {
                match self.pool_from_log(&log)? {
                    Some(pool) => pools.push(AMM::UniswapV4Pool(pool)),
                    None => skipped += 1,
                }
            }
        }
        pb.iter().for_each(|f| {
            finish_progress!(f);
        });
        info!(
            target = "amms::uniswap_v4::discover",
            pools = pools.len(),
            skipped,
            "Discovered V4 pools (hooked / dynamic fee pools skipped)"
        );
        Ok(pools)
    }

    /// Fill token decimals/symbols; drop pools whose tokens do not answer `decimals()`.
    pub async fn sync_all_pools<N, P>(
        mut pools: Vec<AMM>,
        _block_number: BlockId,
        provider: P,
        _pb: Option<&ProgressBar>,
    ) -> Result<Vec<AMM>, AMMError>
    where
        N: Network,
        P: Provider<N> + Clone,
    {
        let tokens: HashSet<Address> = pools.iter().flat_map(|p| p.tokens()).collect();
        let info = get_token_decimals(tokens.into_iter().collect(), provider).await?;
        for pool in pools.iter_mut() {
            let AMM::UniswapV4Pool(p) = pool else { continue };
            if let Some((d, s)) = info.get(&p.token_a.address) {
                p.token_a.decimals = *d;
                p.token_a.symbol = s.clone();
            }
            if let Some((d, s)) = info.get(&p.token_b.address) {
                p.token_b.decimals = *d;
                p.token_b.symbol = s.clone();
            }
        }
        pools.retain(|pool| match pool {
            AMM::UniswapV4Pool(p) => p.token_a.decimals > 0 && p.token_b.decimals > 0,
            _ => true,
        });
        Ok(pools)
    }
}

impl AutomatedMarketMakerFactory for UniswapV4Factory {
    type PoolVariant = UniswapV4Pool;

    fn address(&self) -> Address {
        self.address
    }

    fn pool_creation_event(&self) -> B256 {
        IPoolManager::Initialize::SIGNATURE_HASH
    }

    fn create_pool(&self, log: Log) -> Result<AMM, AMMError> {
        // callers that cannot skip get the pool even if unsupported; discovery uses
        // `pool_from_log` and drops hooked / dynamic-fee pools
        let event = IPoolManager::Initialize::decode_log(&log.inner)?;
        match self.pool_from_log(&log)? {
            Some(pool) => Ok(AMM::UniswapV4Pool(pool)),
            None => Ok(AMM::UniswapV4Pool(UniswapV4Pool {
                address: UniswapV4Pool::address_from_id(event.id),
                pool_id: event.id,
                pool_manager: self.address,
                token_a: event.currency0.into(),
                token_b: event.currency1.into(),
                native0: event.currency0 == Address::ZERO,
                native1: event.currency1 == Address::ZERO,
                fee: event.fee.to(),
                tick_spacing: event.tickSpacing.as_i32(),
                hooks: event.hooks,
                amm_type: AMMType::NotSupported,
                swap_type: SwapType::NotSupported,
                flash_type: self.flash_type,
            })),
        }
    }

    fn creation_block(&self) -> u64 {
        self.creation_block
    }
}

impl DiscoverySync for UniswapV4Factory {
    fn discover<N, P>(
        &self,
        to_block: BlockId,
        provider: P,
        pb: Option<&ProgressBar>,
    ) -> impl Future<Output = Result<Vec<AMM>, AMMError>>
    where
        N: Network,
        P: Provider<N> + Clone,
    {
        info!(target = "amms::uniswap_v4::discover", address = ?self.address, "Discovering all pools");
        self.get_all_pools(to_block, provider, pb)
    }

    fn sync<N, P>(
        &self,
        amms: Vec<AMM>,
        to_block: BlockId,
        provider: P,
        pb: Option<&ProgressBar>,
    ) -> impl Future<Output = Result<Vec<AMM>, AMMError>>
    where
        N: Network,
        P: Provider<N> + Clone,
    {
        info!(target = "amms::uniswap_v4::sync", address = ?self.address, "Syncing all pools");
        UniswapV4Factory::sync_all_pools(amms, to_block, provider, pb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{address, aliases::I24, aliases::U24, b256, LogData};

    const WETH: Address = address!("4200000000000000000000000000000000000006");
    const USDC: Address = address!("79a02482a880bce3f13e09da970dc34db4cd24d1");
    const PM: Address = address!("b1860d529182ac3bc1f51fa2abd56662b7d13f33");

    fn factory() -> UniswapV4Factory {
        UniswapV4Factory::new(
            PM,
            0,
            100,
            WETH,
            AMMType::UniswapV4,
            SwapType::V4,
            FlashType::Normal,
        )
    }

    fn init_log(c0: Address, c1: Address, fee: u32, ts: i32, hooks: Address) -> Log {
        let id = UniswapV4Pool::compute_id(c0, c1, fee, ts, hooks);
        let t = |a: Address| B256::left_padding_from(a.as_slice());
        let data = IPoolManager::Initialize {
            id,
            currency0: c0,
            currency1: c1,
            fee: U24::from(fee),
            tickSpacing: I24::try_from(ts).unwrap(),
            hooks,
            sqrtPriceX96: alloy::primitives::aliases::U160::from(1u64) << 96,
            tick: I24::ZERO,
        }
        .encode_data();
        Log {
            inner: alloy::primitives::Log {
                address: PM,
                data: LogData::new_unchecked(
                    vec![IPoolManager::Initialize::SIGNATURE_HASH, id, t(c0), t(c1)],
                    data.into(),
                ),
            },
            ..Default::default()
        }
    }

    #[test]
    fn initialize_topic() {
        assert_eq!(
            IPoolManager::Initialize::SIGNATURE_HASH,
            b256!("dd466e674ea557f56295e2d0218a125ea4b4f0f6f3307b95f85e6110838d6438")
        );
    }

    #[test]
    fn state_slot_matches_state_library() {
        // keccak256(abi.encode(id, 6)) as in StateLibrary._getPoolStateSlot
        let id = b256!("1111111111111111111111111111111111111111111111111111111111111111");
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(id.as_slice());
        buf[63] = 6;
        assert_eq!(state_slot(id), U256::from_be_bytes(keccak256(buf).0));
    }

    #[test]
    fn native_eth_pool_maps_to_weth() {
        let f = factory();
        let log = init_log(Address::ZERO, USDC, 500, 10, Address::ZERO);
        let p = f.pool_from_log(&log).unwrap().expect("supported pool");
        assert!(p.native0 && !p.native1);
        assert_eq!(p.token_a.address, WETH);
        assert_eq!(p.token_b.address, USDC);
        assert_eq!(p.currency0(), Address::ZERO);
        assert_eq!(p.fee, 500);
        assert_eq!(p.tick_spacing, 10);
        assert_eq!(p.address, Address::from_word(p.pool_id));
        assert_eq!(
            p.pool_id,
            UniswapV4Pool::compute_id(Address::ZERO, USDC, 500, 10, Address::ZERO)
        );
    }

    #[test]
    fn hooked_dynamic_fee_and_eth_weth_pools_are_skipped() {
        let f = factory();
        let hook = address!("00000000000000000000000000000000000000ff");
        assert!(f.pool_from_log(&init_log(Address::ZERO, USDC, 500, 10, hook)).unwrap().is_none());
        assert!(f
            .pool_from_log(&init_log(Address::ZERO, USDC, DYNAMIC_FEE_FLAG, 10, Address::ZERO))
            .unwrap()
            .is_none());
        // ETH / WETH would become WETH / WETH
        assert!(f
            .pool_from_log(&init_log(Address::ZERO, WETH, 100, 1, Address::ZERO))
            .unwrap()
            .is_none());
    }

    #[test]
    fn pool_id_matches_world_chain_initialize() {
        // World Chain PoolManager Initialize log (block ~36,450,000): fee 20000, tickSpacing 200
        let id = UniswapV4Pool::compute_id(
            address!("1ce198b7ee377c9e7a8b6d027f25987476e220e2"),
            USDC,
            20000,
            200,
            Address::ZERO,
        );
        assert_eq!(id, b256!("bdc2d59ae33f62bf24d372dff7bf5a9ee917ef2b883e80207e138cafaeb7dad8"));
    }

    #[test]
    fn negative_tick_spacing_id() {
        // tickSpacing is int24: the id must use its sign-extended word
        let a = UniswapV4Pool::compute_id(WETH, USDC, 3000, 60, Address::ZERO);
        let b = UniswapV4Pool::compute_id(WETH, USDC, 3000, -60, Address::ZERO);
        assert_ne!(a, b);
    }
}
