use std::{collections::HashMap, str::FromStr};

use eth_amm::{
    amm::uniswap_v2::factory::UniswapV2Factory, checkpoint::Checkpoint, middleware::EthProvider,
};
use ethers::types::{H160, U256};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let provider = EthProvider::new().await;
    let factory_address = H160::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f").unwrap();
    let factory: UniswapV2Factory = UniswapV2Factory::new(factory_address, 300);
    let Checkpoint {
        data: weth_values, ..
    } = Checkpoint::<HashMap<H160, U256>>::sync_uniswap_v2_pools_eth_value(&provider, factory, 100)
        .await;
    let min_value = U256::from(10) * U256::exp10(18);
    let weth_values: HashMap<_, _> = weth_values
        .iter()
        .filter(|(_, &value)| &value > &min_value)
        .collect();
    println!("Got {} pools with value > 1000 ETH", weth_values.len());
}
