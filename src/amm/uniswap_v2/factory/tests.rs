use crate::tests::fixtures;

#[tokio::test]
async fn test_get_pair_addresses_from_factory_concurrent_success() {
    let fixture = fixtures::Fixtures::new().await;
    let result = fixture
        .uniswap_v2_factory
        .get_pair_addresses_from_factory(0, 10, 1, fixture.alchemy_provider.http.clone())
        .await;
    assert_eq!(result.len(), 10);
}

#[tokio::test]
async fn test_get_pair_addresses_from_factory_concurrent_failure() {
    let fixture = fixtures::Fixtures::new().await;
    let result = fixture
        .uniswap_v2_factory
        .get_pair_addresses_from_factory(
            10_000_000,
            10_000_010,
            1,
            fixture.alchemy_provider.http.clone(),
        )
        .await;
    assert_eq!(result.len(), 0);
}

#[tokio::test]
async fn test_get_pair_addresses_from_logs_success() {
    let fixture = fixtures::Fixtures::new().await;
    let result = fixture
        .uniswap_v2_factory
        .get_pair_addresses_from_logs_concurrent(
            10008355,
            10009355,
            100,
            fixture.alchemy_provider.http.clone(),
        )
        .await;
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn test_all_pairs_length() {
    let fixture = fixtures::Fixtures::new().await;
    let result = fixture
        .uniswap_v2_factory
        .all_pairs_length(fixture.alchemy_provider.http.clone())
        .await;
    assert!(result > 279_174, "Result: {}", result);
}

#[tokio::test]
async fn test_get_pair_address() {
    let fixture = fixtures::Fixtures::new().await;
    let a1 = fixture
        .uniswap_v2_factory
        .get_pair_address(
            fixture.alchemy_provider.http.clone(),
            &fixture.book.mainnet.erc20["weth"],
            &fixture.book.mainnet.erc20["usdc"],
        )
        .await;
    let a2 = fixture
        .uniswap_v2_factory
        .get_pair_address(
            fixture.alchemy_provider.http.clone(),
            &fixture.book.mainnet.erc20["usdc"],
            &fixture.book.mainnet.erc20["weth"],
        )
        .await;
    assert_eq!(a1, a2);
    assert_eq!(a1, fixture.book.mainnet.uniswap_v2.pairs["weth"]["usdc"]);
}
