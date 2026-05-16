pub mod tree_uint24;
use super::{
    amm::{AutomatedMarketMaker, AMM},
    consts::{
        MPFR_T_PRECISION, U256_0X100, U256_0X10000, U256_0X100000000,
        U256_0XFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF,
        U256_0XFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF, U256_1, U256_128, U256_16,
        U256_191, U256_192, U256_2, U256_255, U256_32, U256_4, U256_64, U256_8,
    },
    error::AMMError,
    factory::{AutomatedMarketMakerFactory, DiscoverySync},
    float::q64_to_float,
    Token,
};

use crate::{
    amms::{
        amm::AMMType,
        consts::MAX_CODE_SIZE,
        moe_v2_2::{
            tree_uint24::TreeUint24,
            IGetMoeV22PoolBinDataBatchRequest::IGetMoeV22PoolBinDataBatchRequestInstance,
            IGetMoeV22PoolDataBatchRequest::IGetMoeV22PoolDataBatchRequestInstance,
            ILBFactory::ILBFactoryInstance,
        },
        uniswap_v2::UniswapV2Error,
    },
    finish_progress, update_progress,
};
use alloy::{
    eips::BlockId,
    network::Network,
    primitives::{aliases::U24, Address, Bytes, B256, U256},
    providers::Provider,
    rpc::types::Log,
    sol,
    sol_types::{SolEvent, SolValue},
};
use futures::{future::BoxFuture, stream::FuturesUnordered, StreamExt};
use indicatif::ProgressBar;
use itertools::Itertools;
use rug::Float;
use serde::{Deserialize, Serialize};
use std::{
    cmp::min,
    collections::{hash_map::Entry, HashMap, HashSet},
    future::Future,
    hash::Hash,
};
use thiserror::Error;
use tracing::{info, warn};

sol! {
#[allow(missing_docs)]
#[derive(Debug)]
#[sol(rpc)]
interface ILBFactory {
    event LBPairCreated(
        address indexed tokenX, address indexed tokenY, uint256 indexed binStep, address lbPair, uint256 pid
    );
    function getNumberOfLBPairs() external view returns (uint256);
    function getLBPairAtIndex(uint256 id) external returns (address);
}

#[derive(Debug, PartialEq, Eq)]
#[sol(rpc)]
interface ILBPair {
    event Sync(uint112 reserve0, uint112 reserve1);
    function getTokenX() external view returns (address tokenX);
    function getTokenY() external view returns (address tokenY);
    function getBinStep() external view returns (uint16 binStep);
    function getReserves() external view returns (uint128 reserveX, uint128 reserveY);
    function getActiveId() external view returns (uint24 activeId);
    function getBin(uint24 id) external view returns (uint128 binReserveX, uint128 binReserveY);
    function getNextNonEmptyBin(bool swapForY, uint24 id) external view returns (uint24 nextId);
}}

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    IGetMoeV22PairsBatchRequest,
    "src/amms/abi/GetMoeV22PairsBatchRequest.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    IGetMoeV22PoolDataBatchRequest,
    "src/amms/abi/GetMoeV22PoolDataBatchRequest.json"
);

sol!(
    #[derive(Debug)]
    #[allow(missing_docs)]
    #[sol(rpc)]
    IGetMoeV22PoolBinDataBatchRequest,
    "src/amms/abi/GetMoeV22PoolBinDataBatchRequest.json"
);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MoeV22Pool {
    pub address: Address,
    pub token_a: Token,
    pub token_b: Token,

    #[serde(skip_serializing, skip_deserializing, default)]
    pub min_bin_id: U24,

    #[serde(skip_serializing, skip_deserializing, default)]
    pub max_bin_id: U24,

    #[serde(skip_serializing, skip_deserializing, default)]
    pub bins: HashMap<u32, U256>,

    #[serde(skip_serializing, skip_deserializing, default)]
    pub bin_bitmap: TreeUint24,

    pub amm_type: AMMType,
}

#[derive(Debug, Clone, Default)]
pub struct BinData {
    pub id: U24,
    pub reserve_x: u128,
    pub reserve_y: u128,
}

impl AutomatedMarketMaker for MoeV22Pool {
    fn address(&self) -> Address {
        self.address
    }

    fn sync_events(&self) -> Vec<B256> {
        vec![]
    }

    fn sync(&mut self, log: &Log) -> Result<(), AMMError> {
        todo!()
        // let sync_event = IUniswapV2Pair::Sync::decode_log(&log.inner)?;
        //
        // let (reserve_0, reserve_1) = (
        //     sync_event.reserve0.to::<u128>(),
        //     sync_event.reserve1.to::<u128>(),
        // );
        //
        // info!(
        //     target = "amm::moe_v2_2::sync",
        //     address = ?self.address,
        //     reserve_0, reserve_1, "Sync"
        // );
        //
        // self.reserve_0 = reserve_0;
        // self.reserve_1 = reserve_1;
        // Ok(())
    }

    fn simulate_swap(
        &self,
        base_token: Address,
        _quote_token: Address,
        amount_in: U256,
    ) -> Result<U256, AMMError> {
        todo!()
    }

    fn simulate_swap_mut(
        &mut self,
        base_token: Address,
        _quote_token: Address,
        amount_in: U256,
    ) -> Result<U256, AMMError> {
        todo!()
    }

    fn tokens(&self) -> Vec<Address> {
        vec![self.token_a.address, self.token_b.address]
    }

    fn calculate_price(&self, base_token: Address, _quote_token: Address) -> Result<f64, AMMError> {
        let price = self.calculate_price_64_x_64(base_token)?;
        q64_to_float(price)
    }

    async fn init<N, P>(mut self, block_number: BlockId, provider: P) -> Result<Self, AMMError>
    where
        N: Network,
        P: Provider<N> + Clone,
    {
        todo!();
        let deployer = IGetMoeV22PoolDataBatchRequestInstance::deploy_builder(
            provider.clone(),
            vec![self.address()],
        );

        let res = deployer.call_raw().block(block_number).await?;

        let pool_data =
            <Vec<(Address, Address, u128, u128, u32, u32)> as SolValue>::abi_decode(&res)?[0];

        if pool_data.0.is_zero() {
            todo!("Return error");
        }

        self.token_a = Token::new_with_decimals(pool_data.0, pool_data.4 as u8);
        self.token_b = Token::new_with_decimals(pool_data.1, pool_data.5 as u8);

        // TODO: populate fee?

        Ok(self)
    }

    fn token0(&self) -> Token {
        self.token_a.clone()
    }

    fn token1(&self) -> Token {
        self.token_b.clone()
    }

    fn amm_type(&self) -> super::amm::AMMType {
        self.amm_type
    }
}

pub fn u128_to_float(num: u128) -> Result<Float, AMMError> {
    let value_string = num.to_string();
    let parsed_value = Float::parse_radix(value_string, 10)?;
    Ok(Float::with_val(MPFR_T_PRECISION, parsed_value))
}

impl MoeV22Pool {
    // Create a new, unsynced UniswapV2 pool
    // TODO: update the init function to derive the fee
    pub fn new(address: Address, fee: usize) -> Self {
        Self {
            address,
            ..Default::default()
        }
    }

    /// Calculates the amount received for a given `amount_in` `reserve_in` and `reserve_out`.
    pub fn get_amount_out(&self, amount_in: U256, reserve_in: U256, reserve_out: U256) -> U256 {
        todo!()
        // if amount_in.is_zero() || reserve_in.is_zero() || reserve_out.is_zero() {
        //     return U256::ZERO;
        // }
        // let fee = U256_100000 - U256::from(self.fee);
        // let amount_in = amount_in * fee;
        // let numerator = amount_in * reserve_out;
        // let denominator = reserve_in * U256_100000 + amount_in;
        //
        // numerator / denominator
    }

    /// Calculates the price of the base token in terms of the quote token.
    ///
    /// Returned as a Q64 fixed point number.
    pub fn calculate_price_64_x_64(&self, base_token: Address) -> Result<u128, AMMError> {
        todo!()
    }

    pub fn swap_calldata(
        &self,
        amount_0_out: U256,
        amount_1_out: U256,
        to: Address,
        calldata: Vec<u8>,
    ) -> Result<Bytes, AMMError> {
        todo!()
        // Ok(IUniswapV2Pair::swapCall {
        //     amount0Out: amount_0_out,
        //     amount1Out: amount_1_out,
        //     to,
        //     data: calldata.into(),
        // }
        // .abi_encode()
        // .into())
    }
}

pub fn div_uu(x: U256, y: U256) -> Result<u128, AMMError> {
    if !y.is_zero() {
        let mut answer;

        if x <= U256_0XFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF {
            answer = (x << U256_64) / y;
        } else {
            let mut msb = U256_192;
            let mut xc = x >> U256_192;

            if xc >= U256_0X100000000 {
                xc >>= U256_32;
                msb += U256_32;
            }

            if xc >= U256_0X10000 {
                xc >>= U256_16;
                msb += U256_16;
            }

            if xc >= U256_0X100 {
                xc >>= U256_8;
                msb += U256_8;
            }

            if xc >= U256_16 {
                xc >>= U256_4;
                msb += U256_4;
            }

            if xc >= U256_4 {
                xc >>= U256_2;
                msb += U256_2;
            }

            if xc >= U256_2 {
                msb += U256_1;
            }

            answer = (x << (U256_255 - msb)) / (((y - U256_1) >> (msb - U256_191)) + U256_1);
        }

        if answer > U256_0XFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF {
            return Ok(0);
        }

        let hi = answer * (y >> U256_128);
        let mut lo = answer * (y & U256_0XFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF);

        let mut xh = x >> U256_192;
        let mut xl = x << U256_64;

        if xl < lo {
            xh -= U256_1;
        }

        xl = xl.overflowing_sub(lo).0;
        lo = hi << U256_128;

        if xl < lo {
            xh -= U256_1;
        }

        xl = xl.overflowing_sub(lo).0;

        if xh != hi >> U256_128 {
            return Err(UniswapV2Error::RoundingError.into());
        }

        answer += xl / y;

        if answer > U256_0XFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF {
            return Ok(0_u128);
        }

        Ok(answer.to::<u128>())
    } else {
        Err(UniswapV2Error::DivisionByZero.into())
    }
}

#[derive(Error, Debug)]
pub enum MoeV22Error {
    #[error("Get bin range error")]
    GetBinRangeError,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct MoeV22Factory {
    pub address: Address,
    pub fee: usize,
    pub creation_block: u64,
    pub amm_type: AMMType,
}

impl MoeV22Factory {
    pub fn new(address: Address, fee: usize, creation_block: u64, amm_type: AMMType) -> Self {
        Self {
            address,
            creation_block,
            fee,
            amm_type,
        }
    }

    pub async fn get_all_pairs<N, P>(
        factory_address: Address,
        block_number: BlockId,
        provider: P,
        pb: Option<&ProgressBar>,
    ) -> Result<Vec<Address>, AMMError>
    where
        N: Network,
        P: Provider<N> + Clone,
    {
        let factory = ILBFactoryInstance::new(factory_address, provider.clone());
        let mut cur_progress = 0;
        let pairs_length = factory
            .getNumberOfLBPairs()
            .call()
            .block(block_number)
            .await?
            .to::<usize>();

        pb.iter().for_each(|f| f.set_length(pairs_length as u64));

        let step = 766;
        let mut futures_unordered = FuturesUnordered::new();
        for i in (0..pairs_length).step_by(step) {
            // Note that the batch contract handles if the step is greater than the pairs length
            // So we can pass the step in as is without checking for this condition
            let deployer = IGetMoeV22PairsBatchRequest::deploy_builder(
                provider.clone(),
                U256::from(i),
                U256::from(step),
                factory_address,
            );

            futures_unordered.push(async move {
                let res = deployer.call_raw().block(block_number).await?;
                let return_data = <Vec<Address> as SolValue>::abi_decode(&res)?;

                Ok::<Vec<Address>, AMMError>(return_data)
            });
        }

        let mut pairs = Vec::new();
        while let Some(res) = futures_unordered.next().await {
            let tokens = res?;
            cur_progress += tokens.len() as u64;
            pb.iter().for_each(|f| f.set_position(cur_progress));
            for token in tokens {
                if !token.is_zero() {
                    pairs.push(token);
                }
            }
        }

        pb.iter().for_each(|f| {
            finish_progress!(f);
        });

        Ok(pairs)
    }

    pub async fn sync_all_pools<N, P>(
        mut amms: Vec<AMM>,
        block_number: BlockId,
        provider: P,
        pb: Option<&ProgressBar>,
    ) -> Result<Vec<AMM>, AMMError>
    where
        N: Network,
        P: Provider<N> + Clone,
    {
        MoeV22Factory::sync_pools_data(&mut amms, block_number, provider.clone(), pb).await?;
        MoeV22Factory::sync_bin_ranges(&mut amms, block_number, provider.clone(), pb).await?;
        MoeV22Factory::sync_bins(&mut amms, block_number, provider, pb).await?;
        MoeV22Factory::create_bin_bitmap(&mut amms)?;
        // TODO: remove this check
        // MoeV22Factory::check_bin_ranges(&amms)?;

        // filter out pools with zero liquidity
        let zero_liquidity_pools = amms
            .iter()
            .map(|amm| {
                let AMM::MoeV22Pool(pool) = amm else {
                    unreachable!()
                };
                pool
            })
            .filter(|pool| pool.bins.is_empty())
            .map(|pool| pool.address())
            .collect::<HashSet<_>>();

        let pools = amms
            .into_iter()
            .filter(|amm| {
                !amm.tokens().iter().any(|t| t.is_zero())
                    && !zero_liquidity_pools.contains(&amm.address())
            })
            .collect::<Vec<_>>();
        Ok(pools)
    }

    async fn sync_bins<N, P>(
        pools: &mut [AMM],
        block_number: BlockId,
        provider: P,
        pb: Option<&ProgressBar>,
    ) -> Result<(), AMMError>
    where
        N: Network,
        P: Provider<N> + Clone,
    {
        pb.iter().for_each(|f| {
            f.reset();
            f.set_message("Sync bins");
            f.set_length(pools.len() as u64);
        });
        let mut futures_unordered: FuturesUnordered<BoxFuture<'_, _>> = FuturesUnordered::new();
        let mut remaining_code_size = MAX_CODE_SIZE - 64;
        let mut group_range = 0_i32;
        let mut group = vec![];
        for pool in pools.iter() {
            let AMM::MoeV22Pool(pool) = pool else {
                unreachable!()
            };
            if pool.min_bin_id == U24::ZERO && pool.max_bin_id == U24::MAX {
                continue;
            }
            remaining_code_size -= 64;
            let mut start_bin_id = pool.min_bin_id;
            while start_bin_id <= pool.max_bin_id {
                let mut end_bin_id = start_bin_id
                    + U24::from((remaining_code_size - group_range * 64) / 64)
                    - U24::ONE;
                end_bin_id = min(end_bin_id, pool.max_bin_id);
                group.push(GetMoeV22PoolBinDataBatchRequest::PoolInfo {
                    pool: pool.address(),
                    minId: start_bin_id,
                    maxId: end_bin_id,
                });
                group_range += (end_bin_id - start_bin_id + U24::ONE).to::<i32>();
                start_bin_id = end_bin_id + U24::ONE;

                // cannot add one more bin range
                // flush the group
                if remaining_code_size < (group_range + 1) * 64 {
                    // reset
                    remaining_code_size = MAX_CODE_SIZE - 64 * 2;
                    let provider = provider.clone();
                    let pool_info = group.iter().map(|info| info.pool).collect::<Vec<_>>();
                    let calldata = std::mem::take(&mut group);
                    group_range = 0;
                    futures_unordered.push(Box::pin(async move {
                        Ok::<(Vec<Address>, Bytes), AMMError>((
                            pool_info,
                            IGetMoeV22PoolBinDataBatchRequestInstance::deploy_builder(
                                provider, calldata,
                            )
                            .call_raw()
                            .block(block_number)
                            .await?,
                        ))
                    }));
                }
            }
        }

        if !group.is_empty() {
            let provider = provider.clone();
            let pool_info = group.iter().map(|info| info.pool).collect::<Vec<_>>();
            let calldata = std::mem::take(&mut group);
            futures_unordered.push(Box::pin(async move {
                Ok::<(Vec<Address>, Bytes), AMMError>((
                    pool_info,
                    IGetMoeV22PoolBinDataBatchRequestInstance::deploy_builder(provider, calldata)
                        .call_raw()
                        .block(block_number)
                        .await?,
                ))
            }));
        }

        let mut pool_set = pools
            .iter_mut()
            .map(|pool| (pool.address(), pool))
            .collect::<HashMap<Address, &mut AMM>>();

        let mut unique_pools = HashSet::new();

        while let Some(res) = futures_unordered.next().await {
            let (pool_info, return_data) = res?;
            let return_data = <Vec<Vec<(U24, U256)>> as SolValue>::abi_decode(&return_data)?;
            for (bin_data, pool_info) in return_data.iter().zip(pool_info.iter()) {
                for bin in bin_data.iter() {
                    let id = bin.0;
                    let AMM::MoeV22Pool(pool) = pool_set.get_mut(pool_info).unwrap() else {
                        unreachable!()
                    };
                    pool.bins.insert(id.to::<u32>(), bin.1);
                }
            }
            unique_pools.insert(pool_info);
            pb.iter().for_each(|f| {
                update_progress!(f, unique_pools.len() as u64);
            });
        }
        pb.iter().for_each(|f| {
            finish_progress!(f);
        });
        Ok(())
    }

    async fn sync_bin_ranges<N, P>(
        pools: &mut [AMM],
        block_number: BlockId,
        provider: P,
        pb: Option<&ProgressBar>,
    ) -> Result<(), AMMError>
    where
        N: Network,
        P: Provider<N> + Clone,
    {
        let mut cur_progress = 0;
        pb.iter().for_each(|f| {
            f.reset();
            f.set_message("Sync bin ranges");
            f.set_length(pools.len() as u64);
        });

        let mut futures_unordered = FuturesUnordered::new();
        for pool in pools.iter() {
            let provider = provider.clone();
            let pool_addr = pool.address();
            futures_unordered.push(async move {
                let lb_pair = ILBPair::new(pool_addr, provider.clone());
                let min_bin_id = lb_pair.getNextNonEmptyBin(false, U24::ZERO);

                let max_bin_id = lb_pair.getNextNonEmptyBin(true, U24::MAX);

                let multicall = provider
                    .multicall()
                    .add(min_bin_id)
                    .add(max_bin_id)
                    .block(block_number);
                let (min_bin_id, max_bin_id) = match multicall.aggregate3_value().await {
                    Ok(res) => res,
                    Err(_) => {
                        return Err(AMMError::MoeV22Error(MoeV22Error::GetBinRangeError));
                    }
                };

                match (min_bin_id, max_bin_id) {
                    (Ok(min_bin_id), Ok(max_bin_id)) => Ok((pool_addr, min_bin_id, max_bin_id)),
                    _ => Err(AMMError::MoeV22Error(MoeV22Error::GetBinRangeError)),
                }
            });
        }

        let mut pool_set = pools
            .iter_mut()
            .map(|pool| (pool.address(), pool))
            .collect::<HashMap<Address, &mut AMM>>();

        while let Some(res) = futures_unordered.next().await {
            let (pool_address, min_bin_id, max_bin_id) = res?;
            let AMM::MoeV22Pool(pool) = pool_set.get_mut(&pool_address).unwrap() else {
                unreachable!()
            };
            pool.min_bin_id = min_bin_id;
            pool.max_bin_id = max_bin_id;
            cur_progress += 1;
            pb.iter().for_each(|f| {
                update_progress!(f, cur_progress);
            });
        }

        pb.iter().for_each(|f| {
            finish_progress!(f);
        });

        Ok(())
    }

    fn create_bin_bitmap(pools: &mut [AMM]) -> Result<(), AMMError> {
        for pool in pools.iter_mut() {
            let AMM::MoeV22Pool(pool) = pool else {
                unreachable!()
            };
            pool.bin_bitmap = pool
                .bins
                .iter()
                .fold(TreeUint24::default(), |mut acc, bin| {
                    if !acc.add(*bin.0) {
                        warn!("bin id {} already exists", *bin.0);
                    }
                    acc
                });
        }
        Ok(())
    }

    fn check_bin_ranges(pools: &[AMM]) -> Result<(), AMMError> {
        for pool in pools.iter() {
            let AMM::MoeV22Pool(pool) = pool else {
                unreachable!()
            };
            for bin in pool.bins.iter() {
                assert!(pool.bin_bitmap.contains(*bin.0))
            }
        }
        Ok(())
    }

    async fn sync_pools_data<N, P>(
        pools: &mut [AMM],
        block_number: BlockId,
        provider: P,
        pb: Option<&ProgressBar>,
    ) -> Result<(), AMMError>
    where
        N: Network,
        P: Provider<N> + Clone,
    {
        let mut cur_progress = 0;
        pb.iter().for_each(|f| f.set_length(pools.len() as u64));
        let step = 120;
        let pairs = pools
            .iter()
            .chunks(step)
            .into_iter()
            .map(|chunk| chunk.map(|amm| amm.address()).collect())
            .collect::<Vec<Vec<Address>>>();

        let mut futures_unordered = FuturesUnordered::new();
        for group in pairs {
            let deployer = IGetMoeV22PoolDataBatchRequestInstance::deploy_builder(
                provider.clone(),
                group.clone(),
            );

            futures_unordered.push(async move {
                let res = deployer.call_raw().block(block_number).await?;

                let return_data =
                    <Vec<(Address, Address, u128, u128, u32, u32)> as SolValue>::abi_decode(&res)?;

                Ok::<(Vec<Address>, Vec<(Address, Address, u128, u128, u32, u32)>), AMMError>((
                    group,
                    return_data,
                ))
            });
        }

        let mut pools = pools
            .iter_mut()
            .map(|amm| (amm.address(), amm))
            .collect::<HashMap<_, _>>();

        while let Some(res) = futures_unordered.next().await {
            let (group, return_data) = res?;
            cur_progress += return_data.len() as u64;
            pb.iter().for_each(|f| {
                update_progress!(f, cur_progress);
            });
            for (pool_data, pool_address) in return_data.iter().zip(group.iter()) {
                // If the pool token A is not zero, signaling that the pool data was polulated

                if pool_data.0.is_zero() {
                    continue;
                }

                let amm = pools.get_mut(pool_address).unwrap();

                let AMM::MoeV22Pool(pool) = amm else {
                    // TODO:: We should never receive a non MoeV22Pool AMM here, we can handle this more gracefully in the future
                    panic!("Unexpected pool type")
                };

                pool.token_a = Token::new_with_decimals(pool_data.0, pool_data.4 as u8);
                pool.token_b = Token::new_with_decimals(pool_data.1, pool_data.5 as u8);
            }
        }

        pb.iter().for_each(|f| {
            finish_progress!(f);
        });

        Ok(())
    }
}

impl AutomatedMarketMakerFactory for MoeV22Factory {
    type PoolVariant = MoeV22Pool;

    fn address(&self) -> Address {
        self.address
    }

    fn pool_creation_event(&self) -> B256 {
        ILBFactory::LBPairCreated::SIGNATURE_HASH
    }

    fn create_pool(&self, log: Log) -> Result<AMM, AMMError> {
        let event = ILBFactory::LBPairCreated::decode_log(&log.inner)?;
        Ok(AMM::MoeV22Pool(MoeV22Pool {
            address: event.lbPair,
            token_a: event.tokenX.into(),
            token_b: event.tokenY.into(),
            amm_type: self.amm_type,
            ..Default::default()
        }))
    }

    fn creation_block(&self) -> u64 {
        self.creation_block
    }
}

impl DiscoverySync for MoeV22Factory {
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
        info!(
            target = "amms::moe_v2_2::discover",
            address = ?self.address,
            "Discovering all pools"
        );

        let provider = provider.clone();
        async move {
            let pairs =
                MoeV22Factory::get_all_pairs(self.address, to_block, provider.clone(), pb).await?;

            Ok(pairs
                .into_iter()
                .map(|pair| {
                    AMM::MoeV22Pool(MoeV22Pool {
                        address: pair,
                        token_a: Address::default().into(),
                        token_b: Address::default().into(),
                        amm_type: self.amm_type,
                        ..Default::default()
                    })
                })
                .collect())
        }
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
        info!(
            target = "amms::moe_v2_2::sync",
            address = ?self.address,
            "Syncing all pools"
        );

        MoeV22Factory::sync_all_pools(amms, to_block, provider, pb)
    }
}

pub fn decode_packed_reserves(encoded: U256) -> (u128, u128) {
    let reserve_x: u128 = (encoded & U256::from(u128::MAX)).to::<u128>();
    let reserve_y: u128 = (encoded >> 128_u8).to::<u128>();
    (reserve_x, reserve_y)
}

pub fn encode_packed_reserves(reserve_x: u128, reserve_y: u128) -> U256 {
    let encoded_x = U256::from(reserve_x);
    let encoded_y = U256::from(reserve_y);
    encoded_x | (encoded_y << 128_u8)
}
