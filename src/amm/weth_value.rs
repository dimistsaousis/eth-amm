use crate::concurrent::{run_concurrent_hash, BatchError};
use crate::contract::GetWethValueInPoolBatchRequest;
use ethers::abi::{ParamType, Token};
use ethers::types::{Bytes, U256};
use ethers::{providers::Middleware, types::H160};
use indicatif::ProgressBar;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

async fn get_weth_value_in_pool_batch_request<M: Middleware>(
    pool_addresses: &[H160],
    factory_addresses: &[H160],
    weth: H160,
    weth_threshold: U256,
    middleware: Arc<M>,
    progress_bar: Option<Arc<Mutex<ProgressBar>>>,
    start: usize,
    end: usize,
) -> Result<HashMap<H160, U256>, BatchError> {
    let pools = pool_addresses
        .iter()
        .map(|a| Token::Address(*a))
        .collect::<Vec<Token>>();

    let factory_is_uni_v3 = factory_addresses
        .iter()
        .map(|_| Token::Bool(false))
        .collect::<Vec<Token>>();

    let factories = factory_addresses
        .iter()
        .map(|a| Token::Address(*a))
        .collect::<Vec<Token>>();

    let constructor_args = Token::Tuple(vec![
        Token::Array(pools),
        Token::Array(factories),
        Token::Array(factory_is_uni_v3),
        Token::Address(weth),
        Token::Uint(weth_threshold),
    ]);

    let deployer = GetWethValueInPoolBatchRequest::deploy(middleware, constructor_args)
        .map_err(|_| BatchError::new(start, end))?;

    let return_data: Bytes = deployer
        .call_raw()
        .await
        .map_err(|_| BatchError::new(start, end))?;

    let return_data_tokens = ethers::abi::decode(
        &[ParamType::Array(Box::new(ParamType::Uint(256)))],
        &return_data,
    )
    .map_err(|_| BatchError::new(start, end))?;

    let mut weth_values: HashMap<H160, U256> = HashMap::new();

    for token_array in return_data_tokens {
        if let Some(arr) = token_array.into_array() {
            for (idx, token) in arr.into_iter().enumerate() {
                if let Some(weth_value_in_pool) = token.into_uint() {
                    let address = pool_addresses[idx];
                    weth_values.insert(address, weth_value_in_pool);
                }
            }
        }
    }
    if let Some(pb) = progress_bar {
        pb.lock().unwrap().inc(pool_addresses.len() as u64);
    }

    Ok(weth_values)
}
pub async fn get_weth_value_in_pool_concurrent<M: Middleware>(
    pool_addresses: &[H160],
    factory_addresses: &[H160],
    weth: H160,
    weth_threshold: U256,
    step: usize,
    middleware: Arc<M>,
) -> HashMap<H160, U256> {
    let batch_func =
        |start: usize, end: usize, middleware: Arc<M>, pb: Option<Arc<Mutex<ProgressBar>>>| {
            get_weth_value_in_pool_batch_request(
                &pool_addresses[start..end],
                factory_addresses,
                weth,
                weth_threshold,
                middleware.clone(),
                pb,
                start,
                end,
            )
        };
    println!(
        "Getting ETH equivalent values for {} pools with value at least {:?} GWEI",
        pool_addresses.len(),
        weth_threshold
    );
    run_concurrent_hash(0, pool_addresses.len(), step, middleware, batch_func).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        address_book::AddressBook, amm::uniswap_v2::pool::UniswapV2Pool, eth_provider::EthProvider,
    };

    fn almost_equal(v1: f64, v2: f64, epsilon: f64) -> bool {
        (v1 / v2 - 1f64).abs() < epsilon
    }

    #[tokio::test]
    async fn test_get_weth_value_in_pool_concurrent() {
        dotenv::dotenv().ok();
        let provider = EthProvider::new_alchemy().await;
        let book = AddressBook::new();
        let weth_threshold = U256::from(10).pow(U256::from(18));
        let pool_addresses = vec![book.mainnet.uniswap_v2.pairs["weth"]["usdc"]];
        let factory_addresses = vec![book.mainnet.uniswap_v2.factory];
        let weth_values = get_weth_value_in_pool_concurrent(
            &pool_addresses,
            &factory_addresses,
            book.mainnet.erc20["weth"],
            weth_threshold,
            5,
            provider.http.clone(),
        )
        .await;
        let pool = UniswapV2Pool::from_address(
            provider.http.clone(),
            book.mainnet.uniswap_v2.pairs["weth"]["usdc"],
            300,
        )
        .await;
        let weth_usdt_value = weth_values[&book.mainnet.uniswap_v2.pairs["weth"]["usdc"]];
        assert!(almost_equal(
            weth_usdt_value.as_u128() as f64,
            (pool.get_reserve_for_token(book.mainnet.erc20["weth"]) * 2) as f64,
            0.0005
        ));
    }
}
