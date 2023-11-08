use std::str::FromStr;

use eth_amm::{
    amm::uniswap_v2::{factory::UniswapV2Factory, pool::UniswapV2Pool},
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
        Checkpoint::<Vec<UniswapV2Pool>>::sync_uniswap_v2_pools(&provider, factory, 100).await;
    let address = H160::from_str("0xeb1eb49bf534a4b43e47ba2ae7351ac966d29a9a").unwrap();
    let pool = UniswapV2Pool::from_address(provider.http.clone(), address, 300).await;
    println!(
        "{}, {}, {}, {}",
        pool.reserve_0, pool.reserve_1, pool.token_a_decimals, pool.token_b_decimals
    );

    let pools: Vec<UniswapV2Pool> = checkpoint
        .data
        .into_iter()
        .filter(|p| p.address == address)
        .collect();
    let pool = &pools[0];
    println!("{}, {}", pool.reserve_0, pool.reserve_1);
}
