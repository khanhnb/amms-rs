use super::{
    balancer::BalancerPool, erc_4626::ERC4626Vault, error::AMMError, uniswap_v2::UniswapV2Pool,
    uniswap_v3::UniswapV3Pool,
};
use crate::amms::{Token, cleo_v2::CleoV2Pool, moe_v2_2::MoeV22Pool};
use alloy::{
    eips::BlockId,
    network::Network,
    primitives::{Address, B256, U256},
    providers::Provider,
    rpc::types::Log,
};
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum AMMType {
    UniswapV2 = 0,
    UniswapV3 = 1,
    CleoV2 = 2,
    CleoV3 = 3,
    MoeV2 = 4,
    PancakeV3 = 5,
    Balancer = 6,
    ERC4626Vault = 7,
    MoeV22 = 8,
    AgniV3 = 9,
    FusionXV2 = 10,
    FusionXV3 = 11,
    ButterV3 = 12,
    #[default]
    NotSupported = 100,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SwapType {
    V2 = 0,
    V3 = 1,
    MoeV22 = 2,
    #[default]
    NotSupported = 100,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum FlashType {
    Normal = 0,
    AAVE = 1,
    #[default]
    NotSupported = 100,
}

#[allow(async_fn_in_trait)]
pub trait AutomatedMarketMaker {
    /// Address of the AMM
    fn address(&self) -> Address;

    /// Event signatures that indicate when the AMM should be synced
    fn sync_events(&self) -> Vec<B256>;

    /// Syncs the AMM state
    fn sync(&mut self, log: &Log) -> Result<(), AMMError>;

    /// Returns a list of token addresses used in the AMM
    fn tokens(&self) -> Vec<Address>;

    fn token0(&self) -> Token;

    fn token1(&self) -> Token;

    fn amm_type(&self) -> AMMType;

    fn swap_type(&self) -> SwapType;

    fn flash_type(&self) -> FlashType;

    /// Calculates the price of `base_token` in terms of `quote_token`
    fn calculate_price(&self, base_token: Address, quote_token: Address) -> Result<f64, AMMError>;

    /// Simulate a swap
    /// Returns the amount_out in `quote token` for a given `amount_in` of `base_token`
    fn simulate_swap(
        &self,
        base_token: Address,
        quote_token: Address,
        amount_in: U256,
    ) -> Result<U256, AMMError>;

    /// Simulate a swap, mutating the AMM state
    /// Returns the amount_out in `quote token` for a given `amount_in` of `base_token`
    fn simulate_swap_mut(
        &mut self,
        base_token: Address,
        quote_token: Address,
        amount_in: U256,
    ) -> Result<U256, AMMError>;

    // Initializes an empty pool and syncs state up to `block_number`
    async fn init<N, P>(self, block_number: BlockId, provider: P) -> Result<Self, AMMError>
    where
        Self: Sized,
        N: Network,
        P: Provider<N> + Clone;
}

macro_rules! amm {
    ($($pool_type:ident),+ $(,)?) => {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum AMM {
            $($pool_type($pool_type),)+
        }

        impl AutomatedMarketMaker for AMM {
            fn address(&self) -> Address{
                match self {
                    $(AMM::$pool_type(pool) => pool.address(),)+
                }
            }

            fn sync_events(&self) -> Vec<B256> {
                match self {
                    $(AMM::$pool_type(pool) => pool.sync_events(),)+
                }
            }

            fn sync(&mut self, log: &Log) -> Result<(), AMMError> {
                match self {
                    $(AMM::$pool_type(pool) => pool.sync(log),)+
                }
            }

            fn simulate_swap(&self, base_token: Address, quote_token: Address,amount_in: U256) -> Result<U256, AMMError> {
                match self {
                    $(AMM::$pool_type(pool) => pool.simulate_swap(base_token, quote_token, amount_in),)+
                }
            }

            fn simulate_swap_mut(&mut self, base_token: Address, quote_token: Address, amount_in: U256) -> Result<U256, AMMError> {
                match self {
                    $(AMM::$pool_type(pool) => pool.simulate_swap_mut(base_token, quote_token, amount_in),)+
                }
            }

            fn tokens(&self) -> Vec<Address> {
                match self {
                    $(AMM::$pool_type(pool) => pool.tokens(),)+
                }
            }

            fn token0(&self) -> Token {
                match self {
                    $(AMM::$pool_type(pool) => pool.token0(),)+
                }
            }

            fn token1(&self) -> Token {
                match self {
                    $(AMM::$pool_type(pool) => pool.token1(),)+
                }
            }

            fn amm_type(&self) -> AMMType {
                match self {
                    $(AMM::$pool_type(pool) => pool.amm_type(),)+
                }
            }

            fn calculate_price(&self, base_token: Address, quote_token: Address) -> Result<f64, AMMError> {
                match self {
                    $(AMM::$pool_type(pool) => pool.calculate_price(base_token, quote_token),)+
                }
            }

            async fn init<N, P>(self, block_number: BlockId, provider: P) -> Result<Self, AMMError>
            where
                Self: Sized,
                N: Network,
                P: Provider<N> + Clone,
            {
                match self {
                    $(AMM::$pool_type(pool) => pool.init(block_number, provider).await.map(AMM::$pool_type),)+
                }
            }

            fn flash_type(&self) -> FlashType {
                match self {
                    $(AMM::$pool_type(pool) => pool.flash_type(),)+
                }
            }

            fn swap_type(&self) -> SwapType {
                match self {
                    $(AMM::$pool_type(pool) => pool.swap_type(),)+
                }
            }
        }


        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum Variant {
            $($pool_type,)+
        }

        impl AMM {
            pub fn variant(&self) -> Variant {
                match self {
                    $(AMM::$pool_type(_) => Variant::$pool_type,)+
                }
            }
        }

        impl Hash for AMM {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.address().hash(state);
            }
        }

        impl PartialEq for AMM {
            fn eq(&self, other: &Self) -> bool {
                self.address() == other.address()
            }
        }

        impl Eq for AMM {}

        $(
            impl From<$pool_type> for AMM {
                fn from(amm: $pool_type) -> Self {
                    AMM::$pool_type(amm)
                }
            }
        )+
    };
}

amm!(
    UniswapV2Pool,
    UniswapV3Pool,
    ERC4626Vault,
    BalancerPool,
    CleoV2Pool,
    MoeV22Pool
);
