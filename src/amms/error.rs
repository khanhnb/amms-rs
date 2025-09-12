use super::{
    balancer::BalancerError, erc_4626::ERC4626VaultError, uniswap_v2::UniswapV2Error,
    uniswap_v3::UniswapV3Error,
};
use alloy::{primitives::FixedBytes, transports::TransportErrorKind};
use std::time::SystemTimeError;
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum AMMError {
    #[error(transparent)]
    TransportError(#[from] alloy::transports::RpcError<TransportErrorKind>),
    #[error(transparent)]
    ContractError(#[from] alloy::contract::Error),
    #[error(transparent)]
    ABIError(#[from] alloy::dyn_abi::Error),
    #[error(transparent)]
    SolTypesError(#[from] alloy::sol_types::Error),
    #[error(transparent)]
    UniswapV2Error(#[from] UniswapV2Error),
    #[error(transparent)]
    UniswapV3Error(#[from] UniswapV3Error),
    #[error(transparent)]
    BalancerError(#[from] BalancerError),
    #[error(transparent)]
    ERC4626VaultError(#[from] ERC4626VaultError),
    #[error(transparent)]
    BatchContractError(#[from] BatchContractError),
    #[error(transparent)]
    ParseFloatError(#[from] rug::float::ParseFloatError),
    #[error("Unrecognized Event Signature {0}")]
    UnrecognizedEventSignature(FixedBytes<32>),
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
    #[error(transparent)]
    CheckpointError(#[from] CheckpointError),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::error::Error),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum BatchContractError {
    #[error(transparent)]
    ContractError(#[from] alloy::contract::Error),
    #[error(transparent)]
    DynABIError(#[from] alloy::dyn_abi::Error),
}

#[derive(Error, Debug)]
pub enum CheckpointError {
    #[error(transparent)]
    SystemTimeError(#[from] SystemTimeError),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::error::Error),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error("Error Populating AMM")]
    ErrorPopulatingAMM,
    #[error("Join error")]
    JoinError(#[from] JoinError),
}
