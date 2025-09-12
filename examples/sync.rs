use alloy::{
    primitives::address,
    rpc::client::ClientBuilder,
    transports::layers::{RetryBackoffLayer, ThrottleLayer},
};
use alloy_provider::ProviderBuilder;
use amms::{
    amms::{uniswap_v2::UniswapV2Factory, uniswap_v3::UniswapV3Factory},
    state_space::StateSpaceBuilder,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();

    let rpc_endpoint = std::env::var("ETHEREUM_PROVIDER")?;
    let client = ClientBuilder::default()
        .layer(ThrottleLayer::new(20))
        .layer(RetryBackoffLayer::new(5, 200, 330))
        .http(rpc_endpoint.parse()?);

    let sync_provider = Arc::new(ProviderBuilder::new().connect_client(client));

    let factories = vec![
        // Moe v2
        UniswapV2Factory::new(
            address!("0x5bef015ca9424a7c07b68490616a4c1f094bedec"),
            300,
            29969727,
        )
        .into(),
        // Agni - v3
        UniswapV3Factory::new(
            address!("0x25780dc8Fc3cfBD75F33bFDAB65e969b603b2035"),
            110692,
            // 60_000_000,
            10_000,
        )
        .into(),
        // cleo v2
        UniswapV2Factory::new(
            address!("0xAAA16c016BF556fcD620328f0759252E29b1AB57"),
            300,
            34705175,
        )
        .into(),
        // cleo - v3
        UniswapV3Factory::new(
            address!("0xAAA32926fcE6bE95ea2c51cB4Fcb60836D320C42"),
            34705175,
            10_000,
        )
        .into(),
    ];

    let checkpoint_folder = "mantle";

    let _state_space_manager = StateSpaceBuilder::new(sync_provider.clone())
        .with_factories(factories)
        .sync_from_checkpoint(checkpoint_folder)
        .await?;

    Ok(())
}
