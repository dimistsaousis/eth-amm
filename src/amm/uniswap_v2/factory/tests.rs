use super::*;
use ethers::providers::{Http, Provider};
use std::str::FromStr;

struct SetupResult(UniswapV2Factory, Arc<Provider<Http>>);

fn setup() -> SetupResult {
    // Create and return the necessary test
    dotenv::dotenv().ok();
    let address: H160 = H160::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f").unwrap();
    let factory: UniswapV2Factory = UniswapV2Factory { address, fee: 300 };
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

#[tokio::test]
async fn test_get_pair_address() {
    let SetupResult(factory, middleware) = setup();
    let weth = H160::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap();
    let usdc = H160::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap();
    let a1 = factory
        .get_pair_address(middleware.clone(), weth, usdc)
        .await;
    let a2 = factory
        .get_pair_address(middleware.clone(), usdc, weth)
        .await;
    assert_eq!(a1, a2);
    assert_eq!(
        a1,
        H160::from_str("0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc").unwrap()
    );
}
