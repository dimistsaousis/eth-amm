use eth_amm::{
    address_book::AddressBook,
    amm::uniswap_v2::{factory::UniswapV2Factory, pool::UniswapV2Pool},
    checkpoint::Checkpoint,
    middleware::EthProvider,
    pair_paths::{find_pool_paths, get_token_to_pool_map},
    simulator::{write_simulations_to_csv, Simulation},
};
use ethers::types::U256;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let provider = EthProvider::new().await;
    let book = AddressBook::new();
    let factory: UniswapV2Factory = UniswapV2Factory::new(book.mainnet.uniswap_v2.factory, 300);
    let mut pools_checkpoint = Checkpoint::<Vec<UniswapV2Pool>>::get(&provider, factory, 100).await;
    pools_checkpoint.sync(&provider).await;
    // pools_checkpoint
    //     .sync_eth_value(&provider, book.mainnet.erc20["weth"], U256::exp10(18))
    //     .await;
    let pools_before = pools_checkpoint.data.len();
    let target_pools: Vec<UniswapV2Pool> = pools_checkpoint
        .data
        .into_iter()
        .filter(|p| p.eth_value > U256::exp10(19))
        .collect();
    println!(
        "From {} pools, filtered and got {}.",
        pools_before,
        target_pools.len()
    );
    let token_to_pool_map = get_token_to_pool_map(target_pools);
    let weth = book.mainnet.erc20["weth"];
    let paths = find_pool_paths(weth, token_to_pool_map, 4);
    println!("Found {} possible paths", paths.len());
    let mut simulations = vec![];
    for path in paths {
        simulations.push(Simulation::new(weth, path, U256::exp10(14)));
    }
    write_simulations_to_csv(simulations, "simulations.csv");
}
