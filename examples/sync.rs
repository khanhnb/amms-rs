use alloy::{
    primitives::{address, aliases::U24, Address, U256},
    rpc::client::ClientBuilder,
    transports::layers::{RetryBackoffLayer, ThrottleLayer},
};
use alloy_provider::ProviderBuilder;
use amms::{
    amms::{
        amm::{AMMType, AutomatedMarketMaker, FlashType, SwapType}, cleo_v2::CleoV2Factory, moe_v2_2::{MoeV22Factory, tree_uint24::TreeUint24}, uniswap_v2::UniswapV2Factory, uniswap_v3::UniswapV3Factory,
    }, state_space::{
        StateSpaceBuilder, filters::{
            AMMFilter, checkpoint_filter::CheckpointFilter, whitelist::{PoolWhitelistFilter, TokenWhitelistFilter},
        },
    },
};
use std::{
    fs::{read_to_string, File},
    ops::Shl,
    sync::Arc,
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();

    // let mut tree = TreeUint24::default();
    // tree.add(8374774);
    // assert!(tree.contains(8374774));

    let rpc_endpoint = std::env::var("ETHEREUM_PROVIDER")?;
    let client = ClientBuilder::default()
        .layer(ThrottleLayer::new(10))
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
        //     AMMType::AgniV3,
        // )
        // .into(),
        // // cleo - v3
        // UniswapV3Factory::new(
        //     address!("0xAAA32926fcE6bE95ea2c51cB4Fcb60836D320C42"),
        //     34705175,
        //     10_000,
        //     AMMType::CleoV3
        // )
        // .into(),
        // MoeV22Factory::new(
        //     address!("0xa6630671775c4EA2743840F9A5016dCf2A104054"),
        //     61742960,
        //     AMMType::MoeV22,
        //     SwapType::MoeV22,
        //     FlashType::AAVE,
        // )
        // .into(),
        // FusionX V3
        // UniswapV3Factory::new(
        //     address!("0x530d2766D1988CC1c000C8b7d00334c14B69AD71"),
        //     2876,
        //     10_000,
        //     AMMType::FusionXV3,
        // )
        // .into(),

        // ButterSwap V3
        UniswapV3Factory::new(
            address!("0xEECa0a86431A7B42ca2Ee5F479832c3D4a4c2644"),
            22966090,
            10_000,
            AMMType::ButterV3,
            SwapType::V3,
            FlashType::Normal,
        )
        .into(),
    ];

    let mut filters: Vec<Box<dyn AMMFilter>> = vec![
        // Box::new(PoolWhitelistFilter::new(vec![address!(
        //     "88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"
        // )])),
        // Box::new(TokenWhitelistFilter::new(vec![address!(
        //     "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
        // )])),
    ];
    // if File::open("filtered.json").is_ok() {
    //     let filtered_pools: Vec<Address> =
    //         serde_json::from_str(read_to_string("filtered.json")?.as_str())?;
    //     filters.push(Box::new(CheckpointFilter::new(filtered_pools)));
    // }

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

    for amm in _state_space_manager.state.read().await.state.values() {
        println!("pair: {}-{}", amm.token0().symbol, amm.token1().symbol);
    }

    Ok(())
}
