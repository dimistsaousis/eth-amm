use std::{collections::HashMap, error::Error, vec};

use ethers::{
    abi::{ParamType, Token},
    types::{Bytes, H160, U256},
};

use crate::{
    amm::uniswap_v2::pool::UniswapV2Pool,
    contract::{IErc20, IUniswapRouter, SimulatorV1},
    eth_provider::EthProvider,
};

pub async fn simulate_swap_using_simulator_v1(
    provider: &EthProvider,
    amount_in: U256,
    path: Vec<H160>,
) -> Result<U256, Box<dyn Error>> {
    let mut params = vec![];

    for i in 0..path.len() - 1 {
        let token_in = path[i];
        let token_out = path[i + 1];
        params.push(Token::Tuple(vec![
            Token::Uint(U256::from(0)),
            Token::Address(H160::zero()),
            Token::Address(token_in),
            Token::Address(token_out),
            Token::Uint(U256::from(300)),
            Token::Uint(amount_in),
        ]))
    }
    let deployer = SimulatorV1::deploy(provider.http.clone(), Token::Array(params)).unwrap();
    let return_data: Bytes = deployer.call_raw().await?;
    let return_data_tokens = ethers::abi::decode(&[ParamType::Uint(256)], &return_data)?;
    if let Some(Token::Uint(v)) = return_data_tokens.into_iter().next() {
        return Ok(v);
    }
    Ok(U256::zero())
}

pub async fn simulate_using_router(
    provider: &EthProvider,
    router_address: H160,
    amount_in: U256,
    path: Vec<H160>,
    public_key: H160,
    private_key: &str,
) -> Result<U256, Box<dyn Error>> {
    let last_token = *path.last().ok_or("Path is empty")?;
    let last_token_erc20 = IErc20::new(last_token, provider.http.clone());
    let current_balance = last_token_erc20.balance_of(public_key).await?;
    let router = IUniswapRouter::new(
        router_address,
        provider.get_signer_middleware(private_key).await,
    );
    let amount_out_min = U256::zero();
    let deadline = U256::from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
            + 10,
    );
    router
        .swap_exact_eth_for_tokens(amount_out_min, path, public_key, deadline)
        .value(amount_in)
        .send()
        .await?;
    Ok(last_token_erc20.balance_of(public_key).await? - current_balance)
}

pub fn simulate_swap_using_pools(
    amount_in: U256,
    path: &Vec<H160>,
    pool_map: &HashMap<(&H160, &H160), &UniswapV2Pool>,
) -> U256 {
    let mut amount = amount_in;
    for i in 0..path.len() - 1 {
        let token_in = &path[i];
        let token_out = &path[i + 1];
        let pool = pool_map[&(token_in, token_out)];
        amount = pool.simulate_swap(token_in, amount);
    }
    amount
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::fixtures;
    use serial_test::serial;
    use test_retry::retry;

    #[tokio::test]
    #[serial]
    async fn test_simulate_using_router() {
        let fixture = fixtures::setup().await;
        let amount_in = U256::exp10(17);
        let result = simulate_using_router(
            &fixture.local_provider,
            fixture.book.mainnet.uniswap_v2.router,
            amount_in,
            fixture.weth_link_matic_weth_path.clone(),
            fixture.local_node_account.address,
            &fixture.local_node_account.private_key,
        )
        .await
        .unwrap();
        assert_ne!(result, U256::zero());
        assert!(result < amount_in);
    }

    #[tokio::test]
    async fn test_simulate_swap_using_simulator_v1() {
        let fixture = fixtures::setup().await;
        let amount_in = U256::exp10(17);
        let result = simulate_swap_using_simulator_v1(
            &fixture.alchemy_provider,
            amount_in,
            fixture.weth_link_matic_weth_path.clone(),
        )
        .await
        .unwrap();
        assert_ne!(result, U256::zero());
        assert!(result < amount_in);
    }

    #[tokio::test]
    #[serial]
    #[retry]
    async fn test_simulate_compare_router_and_simulator_v1() {
        let fixture = fixtures::setup().await;
        let amount_in = U256::exp10(17);
        fixture
            .local_provider
            .reset_local_to_alchemy_fork()
            .await
            .unwrap();
        let router_result = simulate_using_router(
            &fixture.local_provider,
            fixture.book.mainnet.uniswap_v2.router,
            amount_in,
            fixture.weth_link_matic_weth_path.clone(),
            fixture.local_node_account.address,
            &fixture.local_node_account.private_key,
        )
        .await
        .unwrap();
        let simulator_v1_result = simulate_swap_using_simulator_v1(
            &fixture.alchemy_provider,
            amount_in,
            fixture.weth_link_matic_weth_path.clone(),
        )
        .await
        .unwrap();
        assert_eq!(router_result, simulator_v1_result);
    }

    #[tokio::test]
    async fn test_simulate_swap_using_pools() {
        let fixture = fixtures::setup().await;
        let amount_in = U256::exp10(17);
        let result = simulate_swap_using_pools(
            amount_in,
            &fixture.weth_link_matic_weth_path,
            &fixture.pools.token_to_pool_map(),
        );
        assert_ne!(result, U256::zero());
        assert!(result < amount_in);
    }

    #[tokio::test]
    #[retry]
    async fn test_simulate_compare_simulator_v1_and_pool() {
        let fixture = fixtures::setup().await;
        let amount_in = U256::exp10(17);
        let simulator_v1_result = simulate_swap_using_simulator_v1(
            &fixture.alchemy_provider,
            amount_in,
            fixture.weth_link_matic_weth_path.clone(),
        )
        .await
        .unwrap();
        let pool_result = simulate_swap_using_pools(
            amount_in,
            &fixture.weth_link_matic_weth_path,
            &fixture.pools.token_to_pool_map(),
        );
        assert_eq!(pool_result, simulator_v1_result);
    }
}
