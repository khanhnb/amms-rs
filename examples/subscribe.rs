use alloy::{
    primitives::address,
    providers::ProviderBuilder,
    rpc::client::ClientBuilder,
    transports::layers::{RetryBackoffLayer, ThrottleLayer},
};
use alloy_provider::WsConnect;
use amms::{
    amms::{amm::AMMType, uniswap_v2::UniswapV2Factory, uniswap_v3::UniswapV3Factory},
    state_space::StateSpaceBuilder,
};
use futures::StreamExt;
use std::sync::Arc;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();
    let rpc_endpoint = std::env::var("ETHEREUM_PROVIDER")?;
    let client = ClientBuilder::default()
        .layer(ThrottleLayer::new(15))
        .layer(RetryBackoffLayer::new(5, 200, 330))
        .ws(WsConnect::new(rpc_endpoint))
        .await?;

    let sync_provider = Arc::new(ProviderBuilder::new().connect_client(client));

    let factories = vec![
        // Moe v2
        UniswapV2Factory::new(
            address!("0x5bef015ca9424a7c07b68490616a4c1f094bedec"),
            300,
            29969727,
            AMMType::UniswapV2,
        )
        .into(),
        // Agni - v3
        UniswapV3Factory::new(
            address!("0x25780dc8Fc3cfBD75F33bFDAB65e969b603b2035"),
            110692,
            // 60_000_000,
            10_000,
            AMMType::UniswapV3,
        )
        .into(),
        // cleo v2
        UniswapV2Factory::new(
            address!("0xAAA16c016BF556fcD620328f0759252E29b1AB57"),
            300,
            34705175,
            AMMType::CleoV2,
        )
        .into(),
        // cleo - v3
        UniswapV3Factory::new(
            address!("0xAAA32926fcE6bE95ea2c51cB4Fcb60836D320C42"),
            34705175,
            10_000,
            AMMType::CleoV3,
        )
        .into(),
    ];

    let state_space_manager = StateSpaceBuilder::new(sync_provider.clone())
        .with_factories(factories)
        .sync_from_checkpoint("mantle", true)
        .await?;

    /*
    The subscribe method listens for new blocks and fetches
    all logs matching any `sync_events()` specified by the AMM variants in the state space.
    Under the hood, this method applies all state changes to any affected AMMs and returns a Vec of
    addresses, indicating which AMMs have been updated.
    */
    let mut stream = state_space_manager.subscribe().await?;
    while let Some(updated_amms) = stream.next().await {
        if let Ok(amms) = updated_amms {
            if amms.is_empty() {
                continue;
            }
            println!("Updated AMMs: {:?}", amms);
        }
    }

    Ok(())
}
