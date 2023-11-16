use std::error::Error;

use ethers::{
    providers::Middleware,
    types::{H160, U256},
};

use crate::{
    contract::{IErc20, IUniswapRouter},
    middleware::EthProvider,
};

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
        .value(amount)
        .send()
        .await?;
    Ok(last_token_erc20.balance_of(public_key).await? - current_balance)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::address_book::AddressBook;
    use ethers::types::H160;

    async fn setup() -> (AddressBook, EthProvider, Vec<H160>, H160, String) {
        let provider = EthProvider::new_ganache().await;
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
        (book, provider, path, public_key, private_key)
    }

    #[tokio::test]
    async fn test_simulate_using_router() {
        let (book, provider, path, public_key, private_key) = setup().await;
        let amount_in = U256::exp10(17);
        let result = simulate_using_router(
            &provider,
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
}
