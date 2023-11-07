use super::*;
use ethers::providers::{Http, Provider};
use std::str::FromStr;
struct SetupResult(UniswapV2Pool, Arc<Provider<Http>>);

async fn setup() -> SetupResult {
    // Create and return the necessary test
    dotenv::dotenv().ok();
    let weth_usdc_address: H160 =
        H160::from_str("0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc").unwrap();
    let rpc_endpoint = std::env::var("NETWORK_RPC").unwrap();
    let middleware = Arc::new(Provider::<Http>::try_from(rpc_endpoint).unwrap());
    let pool = UniswapV2Pool::from_address(middleware.clone(), weth_usdc_address, 300).await;
    SetupResult(pool, middleware)
}

#[tokio::test]
async fn test_get_reserves() {
    let SetupResult(pool, middleware) = setup().await;
    let (r0, r1) = pool.get_reserves(middleware).await;
    assert_ne!(r0, 0);
    assert_ne!(r1, 0);
}

#[tokio::test]
async fn test_sync_reserves() {
    let SetupResult(mut pool, middleware) = setup().await;
    let (r0, r1) = pool.get_reserves(middleware.clone()).await;
    pool.sync_reserves(middleware.clone()).await;
    assert_eq!(r0, pool.reserve_0);
    assert_eq!(r1, pool.reserve_1);
}

#[tokio::test]
async fn test_price() {
    let SetupResult(pool, _) = setup().await;
    let price = pool.price(pool.token_b);
    assert!(price < 2000.0);
    assert!(price > 1000.0);
}

#[tokio::test]
async fn test_simulate_swap() {
    let SetupResult(pool, _) = setup().await;
    let amount_in = U256::from(1000)
        .checked_mul(U256::from(10).pow(U256::from(pool.token_b_decimals)))
        .unwrap()
        .checked_div(U256::from(997))
        .unwrap();
    let amount_out = pool.simulate_swap(pool.token_b, amount_in).as_u128();
    let price = pool.price(pool.token_b);
    let expected_amount_no_slippage: f64 = price * 10f64.powi(pool.token_a_decimals as i32);
    let diff = (amount_out as f64 / expected_amount_no_slippage - 1f64).abs();
    assert!(diff < 0.1 / 100f64, "{}", diff);
}

#[tokio::test]
async fn test_get_token_decimals() {
    let SetupResult(mut pool, middleware) = setup().await;
    let (t0, t1) = pool.get_token_decimals(middleware).await;
    assert_eq!(t0, 6);
    assert_eq!(t1, 18);
}

#[tokio::test]
async fn test_get_uniswap_v2_pool_data_concurrent() {
    let SetupResult(pool, middleware) = setup().await;
    let addresses = vec![pool.address];
    let pools = get_uniswap_v2_pool_data_concurrent(&addresses, middleware, 300, 1).await;
    let new_pool = pools.into_iter().next().unwrap();
    assert_eq!(pool.address, new_pool.address);
    assert_eq!(pool.token_a, new_pool.token_a);
    assert_eq!(pool.token_b, new_pool.token_b);
    assert_eq!(pool.token_a_decimals, new_pool.token_a_decimals);
    assert_eq!(pool.token_b_decimals, new_pool.token_b_decimals);
}
