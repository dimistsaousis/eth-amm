use super::UniswapV2Pool;
use crate::{
    concurrent::{run_concurrent, BatchError},
    contract::GetUniswapV2PoolDataBatchRequest,
};
use ethers::{
    abi::{ParamType, Token},
    providers::Middleware,
    types::{Bytes, H160, U256},
};
use indicatif::ProgressBar;
use std::sync::{Arc, Mutex};

pub async fn get_amm_data_batch_request<M: Middleware>(
    addresses: &[H160],
    middleware: Arc<M>,
    fee: u32,
    progress_bar: Option<Arc<Mutex<ProgressBar>>>,
    start: usize,
    end: usize,
) -> Result<Vec<UniswapV2Pool>, BatchError> {
    let token_addresses = addresses
        .into_iter()
        .map(|&address| Token::Address(address))
        .collect();
    let constructor_args = Token::Tuple(vec![Token::Array(token_addresses)]);
    let deployer = GetUniswapV2PoolDataBatchRequest::deploy(middleware.clone(), constructor_args)
        .map_err(|_| BatchError::new(start, end))?;

    let return_data: Bytes = deployer
        .call_raw()
        .await
        .map_err(|_| BatchError::new(start, end))?;
    let return_data_tokens = ethers::abi::decode(
        &[ParamType::Array(Box::new(ParamType::Tuple(vec![
            ParamType::Address,   // token a
            ParamType::Uint(8),   // token a decimals
            ParamType::Address,   // token b
            ParamType::Uint(8),   // token b decimals
            ParamType::Uint(112), // reserve 0
            ParamType::Uint(112), // reserve 1
        ])))],
        &return_data,
    )
    .map_err(|_| BatchError::new(start, end))?;
    let pool_tokens = return_data_tokens
        .into_iter()
        .next()
        .unwrap()
        .into_array()
        .unwrap();
    let mut pools = vec![];
    pool_tokens
        .into_iter()
        .enumerate()
        .for_each(|(idx, token)| {
            if let Some(tup) = token.into_tuple() {
                let pool = UniswapV2Pool {
                    address: addresses[idx],
                    token_a: tup[0].to_owned().into_address().unwrap(),
                    token_a_decimals: tup[1].to_owned().into_uint().unwrap().as_u128() as u8,
                    token_b: tup[2].to_owned().into_address().unwrap(),
                    token_b_decimals: tup[3].to_owned().into_uint().unwrap().as_u128() as u8,
                    reserve_0: tup[4].to_owned().into_uint().unwrap().as_u128(),
                    reserve_1: tup[5].to_owned().into_uint().unwrap().as_u128(),
                    fee,
                    eth_value: U256::zero(),
                };
                pools.push(pool);
            }
        });
    if let Some(pb) = progress_bar {
        pb.lock().unwrap().inc(addresses.len() as u64);
    }
    Ok(pools)
}

pub async fn get_uniswap_v2_pool_data_concurrent<M: Middleware>(
    addresses: &Vec<H160>,
    middleware: Arc<M>,
    fee: u32,
    step: usize,
) -> Vec<UniswapV2Pool> {
    let batch_func =
        |start: usize, end: usize, middleware: Arc<M>, pb: Option<Arc<Mutex<ProgressBar>>>| {
            get_amm_data_batch_request(
                &addresses[start..end],
                middleware.clone(),
                fee,
                pb,
                start,
                end,
            )
        };
    println!("Getting amm data for {} pairs", addresses.len());
    run_concurrent(0, addresses.len(), step, middleware, batch_func).await
}
