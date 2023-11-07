use std::str::FromStr;

use eth_amm::{
    amm::{uniswap_v2::factory::UniswapV2Factory, UniswapV2Pool},
    checkpoint::Checkpoint,
    middleware::EthProvider,
};
use ethers::types::H160;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let provider = EthProvider::new().await;
    let factory_address = H160::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f").unwrap();
    let factory: UniswapV2Factory = UniswapV2Factory::new(factory_address, 300);
    let checkpoint =
        Checkpoint::<UniswapV2Pool>::sync_uniswap_v2_pools(&provider, factory, 100).await;
    checkpoint.save_data();
}
