use std::str::FromStr;

use eth_amm::{
    amm::{uniswap_v2::factory::UniswapV2Factory, weth_value::get_weth_value_in_pool_concurrent},
    checkpoint::Checkpoint,
    middleware::EthProvider,
};
use ethers::types::{H160, U256};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let provider = EthProvider::new().await;
    let factory_address = H160::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f").unwrap();
    let factory: UniswapV2Factory = UniswapV2Factory::new(factory_address, 300);
    let weth = H160::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap();
    let pool_addresses =
        Checkpoint::<H160>::sync_uniswap_v2_pair_addresses(&provider, factory, 100).await;
    let factory_addresses = vec![factory_address];
    let weth_values = get_weth_value_in_pool_concurrent(
        &pool_addresses.data,
        &factory_addresses,
        weth,
        U256::exp10(18),
        100,
        provider.http.clone(),
    )
    .await;
    println!(
        "Got {} pairs with maximum value {:?} and minimum value {:?}",
        weth_values.len(),
        weth_values.values().max(),
        weth_values.values().min()
    )
}
