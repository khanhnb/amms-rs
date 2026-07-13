use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

use alloy::{
    eips::BlockId, network::Network, primitives::Address, providers::Provider, sol,
    sol_types::SolValue,
};
use error::{AMMError, BatchContractError};
use futures::{stream::FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};

pub mod amm;
pub mod balancer;
pub mod cleo_v2;
pub mod consts;
pub mod erc_4626;
pub mod error;
pub mod factory;
pub mod float;
pub mod moe_v2_2;
pub mod uniswap_v2;
pub mod uniswap_v3;

sol! {
    #[sol(rpc)]
    GetTokenDecimalsBatchRequest,
    "src/amms/abi/GetTokenDecimalsBatchRequest.json",
}

sol!(
#[derive(Debug, PartialEq, Eq)]
#[sol(rpc)]
contract IERC20 {
    function decimals() external view returns (uint8);
    function symbol() external view returns (string memory);
});

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Token {
    pub address: Address,
    #[serde(default)]
    pub decimals: u8,
    #[serde(default)]
    pub symbol: String,
    // TODO: add optional tax
}

impl Token {
    pub async fn new<N, P>(address: Address, provider: P) -> Result<Self, AMMError>
    where
        N: Network,
        P: Provider<N> + Clone,
    {
        let ierc20 = IERC20::new(address, provider);
        let decimals = ierc20.decimals().call().await?;
        let symbol = ierc20.symbol().call().await?;

        Ok(Self {
            address,
            decimals,
            symbol,
        })
    }

    // Used by v2-like amms
    pub const fn new_with_decimals_and_symbol(
        address: Address,
        decimals: u8,
        symbol: String,
    ) -> Self {
        Self {
            address,
            decimals,
            symbol,
        }
    }

    pub const fn address(&self) -> &Address {
        &self.address
    }

    pub const fn decimals(&self) -> u8 {
        self.decimals
    }
}

impl From<Address> for Token {
    fn from(address: Address) -> Self {
        Self {
            address,
            decimals: 0,
            symbol: String::new(),
        }
    }
}

impl Hash for Token {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.address.hash(state);
    }
}

/// Fetches the decimal precision for a list of ERC-20 tokens.
///
/// # Returns
/// A map of token addresses to their decimal precision.
/// Used by v3-like amms.
pub async fn get_token_decimals<N, P>(
    tokens: Vec<Address>,
    provider: P,
) -> Result<HashMap<Address, (u8, String)>, BatchContractError>
where
    N: Network,
    P: Provider<N> + Clone + Clone,
{
    let step = 128;

    let mut futures = FuturesUnordered::new();
    tokens.chunks(step).for_each(|group| {
        let provider = provider.clone();
        let deployer =
            GetTokenDecimalsBatchRequest::deploy_builder(provider.clone(), group.to_vec());
        futures.push(async move {
            let res = deployer.call_raw().block(BlockId::latest()).await?;
            let return_data = <Vec<(u32, String)> as SolValue>::abi_decode(&res)?;
            Ok::<(Vec<Address>, Vec<(u32, String)>), BatchContractError>((
                group.to_vec(),
                return_data,
            ))
        });
    });

    let mut token_decimals = HashMap::new();
    while let Some(res) = futures.next().await {
        let (group, return_data) = res?;
        for (pool_data, pool_address) in return_data.iter().zip(group.iter()) {
            token_decimals.insert(*pool_address, (pool_data.0 as u8, pool_data.1.clone()));
        }
    }
    Ok(token_decimals)
}
