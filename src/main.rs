use eth_amm::{address_book::AddressBook, middleware::EthProvider, simulator::Simulation};
use ethers::types::{Address, U256};
use eyre::Result;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let provider = EthProvider::new_ganache().await.clone();
    let book = AddressBook::new();
    let public_address = Address::from_str("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")?;
    let private_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

    let path = vec![
        book.mainnet.erc20["weth"],
        book.mainnet.erc20["usdt"],
        book.mainnet.erc20["usd_old"],
        book.mainnet.erc20["weth"],
    ];

    let simu = Simulation::new_from_erc20_path(
        provider.clone(),
        book.mainnet.uniswap_v2.factory,
        path,
        U256::exp10(4),
    )
    .await;

    println!(
        "Best amount is {:?} with amount out {:?} and profit of {:?}",
        simu.amount_in,
        simu.amount_out,
        simu.profit()
    );

    let amount_out = simu
        .swap_using_router(
            book.mainnet.uniswap_v2.router,
            provider.clone(),
            public_address,
            private_key,
        )
        .await;
    println!("Swap yielded: {:?}", amount_out);
    Ok(())
}
