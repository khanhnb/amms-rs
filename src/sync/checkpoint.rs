use crate::amms::{
    amm::{AutomatedMarketMaker, AMM},
    balancer::BalancerFactory,
    cleo_v2::CleoV2Factory,
    error::{AMMError, CheckpointError},
    factory::Factory,
    uniswap_v2::UniswapV2Factory,
    uniswap_v3::UniswapV3Factory,
};
use alloy::{
    primitives::Address,
    providers::{Network, Provider},
};
use indicatif::ProgressBar;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::read_to_string,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub timestamp: usize,
    pub block_number: u64,
    pub factory: Factory,
    pub amms: Vec<AMM>,
}

impl Checkpoint {
    pub fn new(
        timestamp: usize,
        block_number: u64,
        factory: Factory,
        amms: Vec<AMM>,
    ) -> Checkpoint {
        Checkpoint {
            timestamp,
            block_number,
            factory,
            amms,
        }
    }
}

pub fn construct_checkpoint<P>(
    factory: &Factory,
    amms: &[AMM],
    latest_block: u64,
    checkpoint_path: P,
) -> Result<(), CheckpointError>
where
    P: AsRef<Path>,
{
    let checkpoint = Checkpoint::new(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64() as usize,
        latest_block,
        factory.clone(),
        amms.to_vec(),
    );

    std::fs::write(checkpoint_path, serde_json::to_string_pretty(&checkpoint)?)?;

    Ok(())
}

pub async fn discovery_amms_from_checkpoint<N, P, A>(
    path_to_checkpoint: A,
    provider: P,
    pb: Option<&ProgressBar>,
    current_block: u64,
) -> Result<Vec<AMM>, AMMError>
where
    N: Network,
    P: Provider<N> + Clone + 'static,
    A: AsRef<Path>,
{
    let checkpoint: Checkpoint =
        serde_json::from_str(read_to_string(&path_to_checkpoint)?.as_str())?;

    let mut discovered_amms: HashSet<Address> =
        HashSet::from_iter(checkpoint.amms.iter().map(|f| f.address()));

    let mut aggregated_amms = checkpoint.amms;

    // Discover all pools from the since synced block
    for amm in get_new_amms_from_range(
        &checkpoint.factory,
        checkpoint.block_number,
        current_block,
        provider.clone(),
        pb,
    )
    .await?
    {
        if discovered_amms.contains(&amm.address()) {
            continue;
        }
        discovered_amms.insert(amm.address());
        aggregated_amms.push(amm);
    }

    // Update the checkpoint
    construct_checkpoint(
        &checkpoint.factory,
        &aggregated_amms,
        current_block,
        path_to_checkpoint,
    )?;

    Ok(aggregated_amms)
}

pub async fn get_new_amms_from_range<N, P>(
    factory: &Factory,
    from_block: u64,
    to_block: u64,
    provider: P,
    pb: Option<&ProgressBar>,
) -> Result<Vec<AMM>, AMMError>
where
    N: Network,
    P: Provider<N> + Clone + 'static,
{
    let factory: Factory = match factory {
        Factory::UniswapV2Factory(f) => UniswapV2Factory::new(f.address, f.fee, from_block).into(),
        Factory::CleoV2Factory(f) => CleoV2Factory::new(f.address, f.fee, from_block).into(),
        Factory::UniswapV3Factory(f) => {
            UniswapV3Factory::new(f.address, from_block, f.sync_step).into()
        }
        Factory::BalancerFactory(f) => BalancerFactory::new(f.address, from_block).into(),
    };

    let amms = factory.discover(to_block.into(), provider, pb).await?;

    Ok(amms)
}
