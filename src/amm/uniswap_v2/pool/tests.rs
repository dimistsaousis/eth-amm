use crate::tests::fixtures;

use super::*;
use maplit::hashset;

#[tokio::test]
async fn test_get_reserves() {
    let fixture = fixtures::setup().await;
    let pool = fixture.weth_usdc_uniswap_v2_pool.clone();
    let http = fixture.alchemy_provider.http.clone();
    let (r0, r1) = pool.get_reserves(http).await;
    assert_ne!(r0, 0);
    assert_ne!(r1, 0);
}

#[tokio::test]
async fn test_sync_reserves() {
    let fixture = fixtures::setup().await;
    let mut pool = fixture.weth_usdc_uniswap_v2_pool.clone();
    let http = fixture.alchemy_provider.http.clone();
    let (r0, r1) = pool.get_reserves(http.clone()).await;
    pool.sync_reserves(http).await;
    assert_eq!(r0, pool.reserve_0);
    assert_eq!(r1, pool.reserve_1);
}

#[tokio::test]
async fn test_price() {
    let fixture = fixtures::setup().await;
    let pool = fixture.weth_usdc_uniswap_v2_pool.clone();
    let price = pool.price(pool.token_b);
    assert!(price < 2500.0);
    assert!(price > 1000.0);
}

#[tokio::test]
async fn test_simulate_swap() {
    let fixture = fixtures::setup().await;
    let pool = fixture.weth_usdc_uniswap_v2_pool.clone();
    let amount_in = U256::from(1000)
        .checked_mul(U256::from(10).pow(U256::from(pool.token_b_decimals)))
        .unwrap()
        .checked_div(U256::from(997))
        .unwrap();
    let amount_out = pool.simulate_swap(&pool.token_b, amount_in).as_u128();
    let price = pool.price(pool.token_b);
    let expected_amount_no_slippage: f64 = price * 10f64.powi(pool.token_a_decimals as i32);
    let diff = (amount_out as f64 / expected_amount_no_slippage - 1f64).abs();
    assert!(diff < 0.1 / 100f64, "{}", diff);
}

#[tokio::test]
async fn test_get_token_decimals() {
    let fixture = fixtures::setup().await;
    let mut pool = fixture.weth_usdc_uniswap_v2_pool.clone();
    let http = fixture.alchemy_provider.http.clone();
    let (t0, t1) = pool.get_token_decimals(http).await;
    assert_eq!(t0, 6);
    assert_eq!(t1, 18);
}

#[tokio::test]
async fn test_get_uniswap_v2_pool_data_concurrent() {
    let fixture = fixtures::setup().await;
    let pool = fixture.weth_usdc_uniswap_v2_pool.clone();
    let http = fixture.alchemy_provider.http.clone();
    let addresses = vec![pool.address];
    let pools = get_uniswap_v2_pool_data_concurrent(&addresses, http, 300, 1).await;
    let new_pool = pools.into_iter().next().unwrap();
    assert_eq!(pool.address, new_pool.address);
    assert_eq!(pool.token_a, new_pool.token_a);
    assert_eq!(pool.token_b, new_pool.token_b);
    assert_eq!(pool.token_a_decimals, new_pool.token_a_decimals);
    assert_eq!(pool.token_b_decimals, new_pool.token_b_decimals);
}

#[tokio::test]
async fn test_factory() {
    let fixture = fixtures::setup().await;
    let pool = fixture.weth_usdc_uniswap_v2_pool.clone();
    let http = fixture.alchemy_provider.http.clone();
    assert_eq!(
        pool.factory(http).await,
        fixture.book.mainnet.uniswap_v2.factory
    );
}

#[tokio::test]
async fn test_sync_events() {
    let fixture = fixtures::setup().await;
    let mut pool = fixture.weth_usdc_uniswap_v2_pool.clone();
    let http = fixture.alchemy_provider.http.clone();
    pool.sync_reserves(http.clone()).await;
    let last_block = fixture.alchemy_provider.get_block_number().await;
    let events = UniswapV2Pool::get_sync_events_from_logs_concurrent(
        (last_block - 100) as usize,
        last_block as usize,
        10,
        hashset![pool.address],
        http,
    )
    .await;
    assert!(events.contains_key(&pool.address));
    let event = &events[&pool.address];
    assert_eq!(event.reserve_0, pool.reserve_0);
    assert_eq!(event.reserve_1, pool.reserve_1);
}

#[tokio::test]
async fn test_sync_pools_from_logs() {
    let fixture = fixtures::setup().await;
    let mut pool = fixture.weth_usdc_uniswap_v2_pool.clone();
    let http = fixture.alchemy_provider.http.clone();
    pool.reserve_0 = 0;
    pool.reserve_1 = 0;
    let mut pools = vec![pool];
    let last_block = fixture.alchemy_provider.get_block_number().await;
    assert_eq!(&0, &pools[0].reserve_0);
    assert_eq!(&0, &pools[0].reserve_1);
    UniswapV2Pool::sync_pools_from_logs(
        (last_block - 100) as usize,
        last_block as usize,
        10,
        &mut pools,
        http.clone(),
    )
    .await;
    let (r0, r1) = pools[0].get_reserves(http).await;
    assert_eq!(&r0, &pools[0].reserve_0);
    assert_eq!(&r1, &pools[0].reserve_1);
}
