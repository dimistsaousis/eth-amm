use super::*;
use crate::{address_book::AddressBook, eth_provider::EthProvider};

struct SetupResult(UniswapV2Factory, EthProvider, AddressBook);

async fn setup() -> SetupResult {
    // Create and return the necessary test
    dotenv::dotenv().ok();
    let book = AddressBook::new();
    let provider = EthProvider::new_alchemy().await;
    let factory = UniswapV2Factory::new(book.mainnet.uniswap_v2.factory, 300);
    SetupResult(factory, provider, book)
}

#[tokio::test]
async fn test_get_pair_addresses_from_factory_concurrent_success() {
    let SetupResult(factory, provider, _) = setup().await;
    let result = factory
        .get_pair_addresses_from_factory(0, 10, 1, provider.http)
        .await;
    assert_eq!(result.len(), 10);
}

#[tokio::test]
async fn test_get_pair_addresses_from_factory_concurrent_failure() {
    let SetupResult(factory, provider, _) = setup().await;
    let result = factory
        .get_pair_addresses_from_factory(10_000_000, 10_000_010, 1, provider.http)
        .await;
    assert_eq!(result.len(), 0);
}

#[tokio::test]
async fn test_get_pair_addresses_from_logs_success() {
    let SetupResult(factory, provider, _) = setup().await;
    let result = factory
        .get_pair_addresses_from_logs_concurrent(10008355, 10009355, 100, provider.http)
        .await;
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn test_all_pairs_length() {
    let SetupResult(factory, provider, _) = setup().await;
    let result = factory.all_pairs_length(provider.http).await;
    assert!(result > 279_174, "Result: {}", result);
}

#[tokio::test]
async fn test_get_pair_address() {
    let SetupResult(factory, provider, book) = setup().await;
    let a1 = factory
        .get_pair_address(
            provider.http.clone(),
            &book.mainnet.erc20["weth"],
            &book.mainnet.erc20["usdc"],
        )
        .await;
    let a2 = factory
        .get_pair_address(
            provider.http.clone(),
            &book.mainnet.erc20["usdc"],
            &book.mainnet.erc20["weth"],
        )
        .await;
    assert_eq!(a1, a2);
    assert_eq!(a1, book.mainnet.uniswap_v2.pairs["weth"]["usdc"]);
}
