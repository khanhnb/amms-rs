use crate::{
    amms::{amm::AMM, error::AMMError, factory::Factory},
    init_progress,
    sync::checkpoint::discovery_amms_from_checkpoint,
};
use alloy::providers::{Network, Provider};
use indicatif::MultiProgress;
use std::{fs::File, sync::Arc};

pub mod checkpoint;

pub async fn sync_amms<N, P>(
    factories: Vec<Factory>,
    provider: Arc<P>,
    checkpoint_folder: &str,
) -> Result<(Vec<AMM>, u64), AMMError>
where
    N: Network,
    P: Provider<N> + 'static,
{
    tracing::info!(?factories, "Syncing AMMs");

    let current_block: u64 = provider.get_block_number().await?;
    let mut aggregated_amms = vec![];
    let multi_progress = MultiProgress::new();
    let mut futures = vec![];

    for factory in factories {
        let provider = provider.clone();
        let amm_checkpoint_path = format!("{}/{}.json", checkpoint_folder, factory.address());
        let discovery_pb = multi_progress.add(init_progress!(
            0,
            &format!("Discovery AMM {}", factory.address())
        ));
        let sync_pb = multi_progress.add(init_progress!(
            0,
            &format!("Sync AMM {}", factory.address())
        ));
        futures.push(tokio::spawn(async move {
            let mut amms = vec![];
            if File::open(&amm_checkpoint_path).is_err() {
                checkpoint::construct_checkpoint(
                    &factory,
                    &amms,
                    factory.creation_block(),
                    &amm_checkpoint_path,
                )?;
            }
            amms = discovery_amms_from_checkpoint(
                &amm_checkpoint_path,
                provider.clone(),
                Some(&discovery_pb),
            )
            .await?;

            // save discovered amms
            checkpoint::construct_checkpoint(&factory, &amms, current_block, &amm_checkpoint_path)?;

            // apply discovery filters

            //sync data
            amms = factory
                .sync(amms, current_block.into(), provider, Some(&sync_pb))
                .await?;

            // apply sync filters

            checkpoint::construct_checkpoint(&factory, &amms, current_block, &amm_checkpoint_path)?;
            Ok::<Vec<AMM>, AMMError>(amms)
        }));
    }

    for future in futures {
        match future.await {
            Ok(future_result) => match future_result {
                Ok(amms) => {
                    aggregated_amms.extend(amms);
                }
                Err(e) => {
                    tracing::error!("sync amms from checkpoint error {:?}", e);
                }
            },
            Err(e) => {
                tracing::error!("Future error {:?}", e);
            }
        }
    }

    Ok((aggregated_amms, current_block))
}
