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
    use crate::tests::fixtures::{self, Fixtures};
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_get_weth_value_in_pool_concurrent() {
        let fixture = fixtures::Fixtures::new().await;
        let http = fixture.alchemy_provider.http.clone();
        let weth_threshold = U256::from(10).pow(U256::from(18));
        let pool_addresses = vec![fixture.book.mainnet.uniswap_v2.pairs["weth"]["usdc"]];
        let factory_addresses = vec![fixture.book.mainnet.uniswap_v2.factory];
        let weth_address = fixture.book.mainnet.erc20["weth"];
        let weth_values = get_weth_value_in_pool_concurrent(
            &pool_addresses,
            &factory_addresses,
            weth_address.clone(),
            weth_threshold,
            5,
            http,
        )
        .await;
        let pool = &fixture.weth_usdc_uniswap_v2_pool;
        let weth_usdt_value = weth_values[&pool.address].as_u128() as f64;
        let weth_reserve = pool.get_reserve_for_token(&weth_address) as f64;
        Fixtures::assert_almost_equal(weth_usdt_value, weth_reserve * 2.0, 0.0005);
    }
}
