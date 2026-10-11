//! Algebra Integral pools (Blast).
//!
//! Only discovery and static info (tokens, decimals) are kept here. The bot simulates swaps with
//! the real pool bytecode in revm and keeps the pool storage fresh from block traces, so this
//! module does not mirror Algebra's tick tree or dynamic fee (plugin) math: `simulate_swap` is
//! not supported.
//!
//! Factory event: `Pool(address indexed token0, address indexed token1, address pool)`.
//! Swap: `swap(address,bool,int256,uint160,bytes)` (same selector as Uniswap V3), callback
//! `algebraSwapCallback(int256,int256,bytes)` (same calldata layout as Uniswap V3).

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
    primitives::{Address, B256, U256},
    providers::Provider,
    rpc::types::{Filter, FilterSet, Log},
    sol,
    sol_types::SolEvent,
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
    contract IAlgebraFactory {
        event Pool(address indexed token0, address indexed token1, address pool);
    }

    #[allow(missing_docs)]
    #[derive(Debug)]
    #[sol(rpc)]
    contract IAlgebraPool {
        function token0() external view returns (address);
        function token1() external view returns (address);
    }
}

#[derive(Error, Debug)]
pub enum AlgebraError {
    #[error("Algebra pools are simulated in revm only, off-chain swap math is not implemented")]
    SimulationNotSupported,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlgebraPool {
    pub address: Address,
    pub token_a: Token,
    pub token_b: Token,
    pub amm_type: AMMType,
    pub swap_type: SwapType,
    pub flash_type: FlashType,
}

impl AutomatedMarketMaker for AlgebraPool {
    fn address(&self) -> Address {
        self.address
    }

    fn sync_events(&self) -> Vec<B256> {
        // state comes from block traces in the bot, not from events
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
        Err(AlgebraError::SimulationNotSupported.into())
    }

    fn simulate_swap(&self, _base: Address, _quote: Address, _amount_in: U256) -> Result<U256, AMMError> {
        Err(AlgebraError::SimulationNotSupported.into())
    }

    fn simulate_swap_mut(
        &mut self,
        _base: Address,
        _quote: Address,
        _amount_in: U256,
    ) -> Result<U256, AMMError> {
        Err(AlgebraError::SimulationNotSupported.into())
    }

    async fn init<N, P>(mut self, block_number: BlockId, provider: P) -> Result<Self, AMMError>
    where
        N: Network,
        P: Provider<N> + Clone,
    {
        let pool = IAlgebraPool::new(self.address, provider.clone());
        let t0 = pool.token0().block(block_number).call().await?;
        let t1 = pool.token1().block(block_number).call().await?;
        self.token_a = Token::new(t0, provider.clone()).await?;
        self.token_b = Token::new(t1, provider).await?;
        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct AlgebraFactory {
    pub address: Address,
    pub creation_block: u64,
    pub sync_step: u64,
    pub amm_type: AMMType,
    pub swap_type: SwapType,
    pub flash_type: FlashType,
}

impl AlgebraFactory {
    pub fn new(
        address: Address,
        creation_block: u64,
        sync_step: u64,
        amm_type: AMMType,
        swap_type: SwapType,
        flash_type: FlashType,
    ) -> Self {
        Self { address, creation_block, sync_step, amm_type, swap_type, flash_type }
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

        let mut pools = vec![];
        let mut progress = 0;
        while let Some(res) = futures.next().await {
            progress += step;
            pb.iter().for_each(|f| f.set_position(progress));
            for log in res? {
                pools.push(self.create_pool(log)?);
            }
        }
        pb.iter().for_each(|f| {
            finish_progress!(f);
        });
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
            let AMM::AlgebraPool(p) = pool else { continue };
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
            AMM::AlgebraPool(p) => p.token_a.decimals > 0 && p.token_b.decimals > 0,
            _ => true,
        });
        Ok(pools)
    }
}

impl AutomatedMarketMakerFactory for AlgebraFactory {
    type PoolVariant = AlgebraPool;

    fn address(&self) -> Address {
        self.address
    }

    fn pool_creation_event(&self) -> B256 {
        IAlgebraFactory::Pool::SIGNATURE_HASH
    }

    fn create_pool(&self, log: Log) -> Result<AMM, AMMError> {
        let event = IAlgebraFactory::Pool::decode_log(&log.inner)?;
        Ok(AMM::AlgebraPool(AlgebraPool {
            address: event.pool,
            token_a: event.token0.into(),
            token_b: event.token1.into(),
            amm_type: self.amm_type,
            swap_type: self.swap_type,
            flash_type: self.flash_type,
        }))
    }

    fn creation_block(&self) -> u64 {
        self.creation_block
    }
}

impl DiscoverySync for AlgebraFactory {
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
        info!(target = "amms::algebra::discover", address = ?self.address, "Discovering all pools");
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
        info!(target = "amms::algebra::sync", address = ?self.address, "Syncing all pools");
        AlgebraFactory::sync_all_pools(amms, to_block, provider, pb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{address, b256, LogData};

    #[test]
    fn pool_event_topic() {
        assert_eq!(
            IAlgebraFactory::Pool::SIGNATURE_HASH,
            b256!("91ccaa7a278130b65168c3a0c8d3bcae84cf5e43704342bd3ec0b59e59c036db")
        );
    }

    #[test]
    fn create_pool_from_log() {
        // Blast factory 0x7a44… created pool 0x4c57… (USDB / 0xb1a5…) at block 5311608
        let f = AlgebraFactory::new(
            address!("7a44cd060afc1b6f4c80a2b9b37f4473e74e25df"),
            1586095,
            10000,
            AMMType::AlgebraIntegral,
            SwapType::Algebra,
            FlashType::Normal,
        );
        let mut word = [0u8; 32];
        word[12..].copy_from_slice(address!("4c577f4873061003d6c83eaac20e24397ff5b89b").as_slice());
        let t = |a: Address| B256::left_padding_from(a.as_slice());
        let log = Log {
            inner: alloy::primitives::Log {
                address: f.address,
                data: LogData::new_unchecked(
                    vec![
                        IAlgebraFactory::Pool::SIGNATURE_HASH,
                        t(address!("4300000000000000000000000000000000000003")),
                        t(address!("b1a5700fa2358173fe465e6ea4ff52e36e88e2ad")),
                    ],
                    word.to_vec().into(),
                ),
            },
            ..Default::default()
        };
        let AMM::AlgebraPool(p) = f.create_pool(log).unwrap() else { panic!("not an Algebra pool") };
        assert_eq!(p.address, address!("4c577f4873061003d6c83eaac20e24397ff5b89b"));
        assert_eq!(p.token_a.address, address!("4300000000000000000000000000000000000003"));
        assert_eq!(p.token_b.address, address!("b1a5700fa2358173fe465e6ea4ff52e36e88e2ad"));
        assert_eq!(p.amm_type, AMMType::AlgebraIntegral);
        assert_eq!(p.swap_type, SwapType::Algebra);
    }
}
