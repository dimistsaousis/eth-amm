use super::*;
use crate::{address_book::AddressBook, eth_provider::EthProvider};
use maplit::hashset;
struct SetupResult(UniswapV2Pool, EthProvider, AddressBook);

async fn setup() -> SetupResult {
    dotenv::dotenv().ok();
    let book = AddressBook::new();
    let provider = EthProvider::new_alchemy().await;
    let pool = UniswapV2Pool::from_address(
        provider.http.clone(),
        book.mainnet.uniswap_v2.pairs["weth"]["usdc"],
        300,
    )
    .await;
    SetupResult(pool, provider, book)
}

#[tokio::test]
async fn test_get_reserves() {
    let SetupResult(pool, provider, _) = setup().await;
    let (r0, r1) = pool.get_reserves(provider.http).await;
    assert_ne!(r0, 0);
    assert_ne!(r1, 0);
}

#[tokio::test]
async fn test_sync_reserves() {
    let SetupResult(mut pool, provider, _) = setup().await;
    let (r0, r1) = pool.get_reserves(provider.http.clone()).await;
    pool.sync_reserves(provider.http.clone()).await;
    assert_eq!(r0, pool.reserve_0);
    assert_eq!(r1, pool.reserve_1);
}

#[tokio::test]
async fn test_price() {
    let SetupResult(pool, _, _) = setup().await;
    let price = pool.price(pool.token_b);
    assert!(price < 2500.0);
    assert!(price > 1000.0);
}

#[tokio::test]
async fn test_simulate_swap() {
    let SetupResult(pool, _, _) = setup().await;
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
    let SetupResult(mut pool, provider, _) = setup().await;
    let (t0, t1) = pool.get_token_decimals(provider.http).await;
    assert_eq!(t0, 6);
    assert_eq!(t1, 18);
}

#[tokio::test]
async fn test_get_uniswap_v2_pool_data_concurrent() {
    let SetupResult(pool, provider, _) = setup().await;
    let addresses = vec![pool.address];
    let pools = get_uniswap_v2_pool_data_concurrent(&addresses, provider.http, 300, 1).await;
    let new_pool = pools.into_iter().next().unwrap();
    assert_eq!(pool.address, new_pool.address);
    assert_eq!(pool.token_a, new_pool.token_a);
    assert_eq!(pool.token_b, new_pool.token_b);
    assert_eq!(pool.token_a_decimals, new_pool.token_a_decimals);
    assert_eq!(pool.token_b_decimals, new_pool.token_b_decimals);
}

#[tokio::test]
async fn test_factory() {
    let SetupResult(pool, provider, book) = setup().await;
    assert_eq!(
        pool.factory(provider.http).await,
        book.mainnet.uniswap_v2.factory
    );
}

#[tokio::test]
async fn test_sync_events() {
    let SetupResult(mut pool, provider, _) = setup().await;
    pool.sync_reserves(provider.http.clone()).await;
    let last_block = provider.get_block_number().await;
    let events = UniswapV2Pool::get_sync_events_from_logs_concurrent(
        (last_block - 100) as usize,
        last_block as usize,
        10,
        hashset![pool.address],
        provider.http,
    )
    .await;
    assert!(events.contains_key(&pool.address));
    let event = &events[&pool.address];
    assert_eq!(event.reserve_0, pool.reserve_0);
    assert_eq!(event.reserve_1, pool.reserve_1);
}

#[tokio::test]
async fn test_sync_pools_from_logs() {
    let SetupResult(mut pool, provider, _) = setup().await;
    pool.reserve_0 = 0;
    pool.reserve_1 = 0;
    let mut pools = vec![pool];
    let last_block = provider.get_block_number().await;
    assert_eq!(&0, &pools[0].reserve_0);
    assert_eq!(&0, &pools[0].reserve_1);
    UniswapV2Pool::sync_pools_from_logs(
        (last_block - 100) as usize,
        last_block as usize,
        10,
        &mut pools,
        provider.http.clone(),
    )
    .await;
    let (r0, r1) = pools[0].get_reserves(provider.http).await;
    assert_eq!(&r0, &pools[0].reserve_0);
    assert_eq!(&r1, &pools[0].reserve_1);
}
