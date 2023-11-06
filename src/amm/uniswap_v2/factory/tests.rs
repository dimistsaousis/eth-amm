use super::*;
use ethers::providers::{Http, Provider};
use std::str::FromStr;

struct SetupResult(UniswapV2Factory, Arc<Provider<Http>>);

fn setup() -> SetupResult {
    // Create and return the necessary test
    dotenv::dotenv().ok();
    let address: H160 = H160::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f").unwrap();
    let factory: UniswapV2Factory = UniswapV2Factory { address };
    let rpc_endpoint = std::env::var("NETWORK_RPC").unwrap();
    let middleware = Arc::new(Provider::<Http>::try_from(rpc_endpoint).unwrap());

    SetupResult(factory, middleware)
}

#[tokio::test]
async fn test_get_pair_addresses_from_factory_concurrent_success() {
    let SetupResult(factory, middleware) = setup();
    let result = factory
        .get_pair_addresses_from_factory(0, 10, 1, middleware)
        .await;
    assert_eq!(result.len(), 10);
}

#[tokio::test]
#[should_panic]
async fn test_get_pair_addresses_from_factory_concurrent_failure() {
    let SetupResult(factory, middleware) = setup();
    factory
        .get_pair_addresses_from_factory(10_000_000, 10_000_010, 1, middleware)
        .await;
}

#[tokio::test]
async fn test_get_pair_addresses_from_logs_success() {
    let SetupResult(factory, middleware) = setup();
    let result = factory
        .get_pair_addresses_from_logs_concurrent(10008355, 10009355, 100, middleware)
        .await;
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn test_all_pairs_length() {
    let SetupResult(factory, middleware) = setup();
    let result = factory.all_pairs_length(middleware).await;
    assert!(result > 279_174, "Result: {}", result);
}
