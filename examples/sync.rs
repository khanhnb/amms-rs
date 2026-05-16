use alloy::{
    primitives::{address, aliases::U24, U256},
    rpc::client::ClientBuilder,
    transports::layers::{RetryBackoffLayer, ThrottleLayer},
};
use alloy_provider::ProviderBuilder;
use amms::{
    amms::{
        amm::AMMType,
        cleo_v2::CleoV2Factory,
        moe_v2_2::{tree_uint24::TreeUint24, MoeV22Factory},
        uniswap_v2::UniswapV2Factory,
        uniswap_v3::UniswapV3Factory,
    },
    state_space::{
        filters::{
            whitelist::{PoolWhitelistFilter, TokenWhitelistFilter},
            AMMFilter,
        },
        StateSpaceBuilder,
    },
};
use std::{ops::Shl, sync::Arc};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();

    // let mut tree = TreeUint24::default();
    // tree.add(8374774);
    // assert!(tree.contains(8374774));

    let rpc_endpoint = std::env::var("ETHEREUM_PROVIDER")?;
    let client = ClientBuilder::default()
        .layer(ThrottleLayer::new(15))
        .layer(RetryBackoffLayer::new(5, 200, 330))
        .http(rpc_endpoint.parse()?);

    let sync_provider = Arc::new(ProviderBuilder::new().connect_client(client));

    let factories = vec![
        // // Moe v2
        // UniswapV2Factory::new(
        //     address!("0x5bef015ca9424a7c07b68490616a4c1f094bedec"),
        //     300,
        //     29969727,
        // )
        // .into(),
        // cleo v2
        // CleoV2Factory::new(
        //     address!("0xAAA16c016BF556fcD620328f0759252E29b1AB57"),
        //     300,
        //     34705175,
        //     AMMType::CleoV2,
        // )
        // .into(),
        // // Agni - v3
        // UniswapV3Factory::new(
        //     address!("0x25780dc8Fc3cfBD75F33bFDAB65e969b603b2035"),
        //     110692,
        //     // 60_000_000,
        //     10_000,
        // )
        // .into(),
        // // cleo - v3
        // UniswapV3Factory::new(
        //     address!("0xAAA32926fcE6bE95ea2c51cB4Fcb60836D320C42"),
        //     34705175,
        //     10_000,
        // )
        // .into(),
        MoeV22Factory::new(
            address!("0xa6630671775c4EA2743840F9A5016dCf2A104054"),
            300,
            61742960,
            AMMType::MoeV22,
        )
        .into(),
    ];

    let filters: Vec<Box<dyn AMMFilter>> = vec![
        // Box::new(PoolWhitelistFilter::new(vec![address!(
        //     "88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"
        // )])),
        // Box::new(TokenWhitelistFilter::new(vec![address!(
        //     "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
        // )])),
    ];

    let checkpoint_folder = "mantle";

    let _state_space_manager = StateSpaceBuilder::new(sync_provider.clone())
        .with_filters(filters)
        .with_factories(factories)
        .sync_from_checkpoint(checkpoint_folder, true)
        .await?;

    println!(
        "Synced {} pools",
        _state_space_manager.state.read().await.state.len()
    );

    Ok(())
}
