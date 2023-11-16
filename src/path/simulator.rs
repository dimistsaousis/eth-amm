use std::{error::Error, vec};

use ethers::{
    abi::{ParamType, Token},
    types::{Bytes, H160, U256},
};

use crate::{
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::{address_book::AddressBook, eth_provider::LOCAL_NODE_TEST_MUTEX};
    use ethers::types::H160;

    async fn setup() -> (
        AddressBook,
        EthProvider,
        EthProvider,
        Vec<H160>,
        H160,
        String,
        U256,
    ) {
        dotenv::dotenv().ok();
        let local_provider = EthProvider::new_local().await;
        let alchemy_provider = EthProvider::new_alchemy().await;
        let book = AddressBook::new();
        let path = vec![
            book.mainnet.erc20["weth"],
            book.mainnet.erc20["link"],
            book.mainnet.erc20["matic"],
            book.mainnet.erc20["weth"],
        ];
        let public_key = H160::from_str("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap();
        let private_key =
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string();
        let amount_in = U256::exp10(17);
        (
            book,
            local_provider,
            alchemy_provider,
            path,
            public_key,
            private_key,
            amount_in,
        )
    }

    #[tokio::test]
    async fn test_simulate_using_router() {
        let (book, local_provider, _, path, public_key, private_key, amount_in) = setup().await;
        let _guard = LOCAL_NODE_TEST_MUTEX.lock().unwrap();
        let result = simulate_using_router(
            &local_provider,
            book.mainnet.uniswap_v2.router,
            amount_in,
            path,
            public_key,
            &private_key,
        )
        .await
        .unwrap();
        assert_ne!(result, U256::zero());
        assert!(result < amount_in);
    }

    #[tokio::test]
    async fn test_simulate_swap_using_simulator_v1() {
        let (_, _, alchemy_provider, path, _, _, amount_in) = setup().await;
        let result = simulate_swap_using_simulator_v1(&alchemy_provider, amount_in, path)
            .await
            .unwrap();
        assert_ne!(result, U256::zero());
        assert!(result < amount_in);
    }

    #[tokio::test]
    async fn test_simulate_compare_router_and_simulator_v1() {
        let (book, local_provider, alchemy_provider, path, public_key, private_key, amount_in) =
            setup().await;
        let _guard = LOCAL_NODE_TEST_MUTEX.lock().unwrap();
        local_provider.reset_local_to_alchemy_fork().await.unwrap();
        let router_result = simulate_using_router(
            &local_provider,
            book.mainnet.uniswap_v2.router,
            amount_in,
            path.clone(),
            public_key,
            &private_key,
        )
        .await
        .unwrap();
        let simulator_v1_result =
            simulate_swap_using_simulator_v1(&alchemy_provider, amount_in, path)
                .await
                .unwrap();
        assert_eq!(router_result, simulator_v1_result);
    }
}
