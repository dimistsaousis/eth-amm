use ethers::abi::{ParamType, Token};
use ethers::prelude::abigen;
use ethers::providers::Middleware;
use ethers::types::{Bytes, H160, U256};
use indicatif::ProgressBar;
use std::fmt;
use std::sync::{Arc, Mutex};

use crate::concurrent::run_concurrent;

abigen!(
    IGetUniswapV2PairsBatchRequest,
        "src/contract/abi/GetUniswapV2PairsBatchRequestABI.json";
);

#[derive(Debug)]
pub struct PairsAddressesBatchError {
    factory: H160,
    start: U256,
    end: U256,
    message: String,
}

impl fmt::Display for PairsAddressesBatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Error fetching pairs for factory {} between start {} and end {}: {}",
            self.factory, self.start, self.end, self.message
        )
    }
}

async fn get_pair_addresses_batch<M: Middleware>(
    factory: H160,
    start: U256,
    end: U256,
    middleware: Arc<M>,
    progress_bar: Option<Arc<Mutex<ProgressBar>>>,
) -> Result<Vec<H160>, PairsAddressesBatchError> {
    let mut pairs = vec![];
    let constructor_args = Token::Tuple(vec![
        Token::Uint(start),
        Token::Uint(end),
        Token::Address(factory),
    ]);

    let deployer =
        IGetUniswapV2PairsBatchRequest::deploy(middleware, constructor_args).map_err(|err| {
            PairsAddressesBatchError {
                factory,
                start,
                end,
                message: format!("Failed to deploy contract: {}", err),
            }
        })?;
    let return_data: Bytes = deployer
        .call_raw()
        .await
        .map_err(|err| PairsAddressesBatchError {
            factory,
            start,
            end,
            message: format!("Failed to call contract: {}", err),
        })?;

    let return_data_tokens = ethers::abi::decode(
        &[ParamType::Array(Box::new(ParamType::Address))],
        &return_data,
    )
    .map_err(|err| PairsAddressesBatchError {
        factory,
        start,
        end,
        message: format!("Failed to decode return data: {}", err),
    })?;

    for token_array in return_data_tokens {
        if let Some(arr) = token_array.into_array() {
            for token in arr {
                if let Some(addr) = token.into_address() {
                    if !addr.is_zero() {
                        pairs.push(addr);
                    }
                }
            }
        }
    }

    if let Some(pb) = progress_bar {
        pb.lock().unwrap().inc(end.as_u64() - start.as_u64());
    }

    Ok(pairs)
}

pub async fn get_pair_addresses_concurrent<'a, M: Middleware + 'a>(
    factory: H160,
    start: usize,
    end: usize,
    step: usize,
    middleware: Arc<M>,
) -> Result<Vec<H160>, PairsAddressesBatchError> {
    let batch_func = |start_index: usize,
                      end_index: usize,
                      middleware: Arc<M>,
                      pb: Option<Arc<Mutex<ProgressBar>>>| {
        get_pair_addresses_batch(
            factory,
            U256::from(start_index),
            U256::from(end_index),
            middleware.clone(),
            pb,
        )
    };
    run_concurrent(start, end, step, middleware, batch_func).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::providers::{Http, Provider};
    use std::str::FromStr;

    struct SetupResult(H160, Arc<Provider<Http>>);

    fn setup() -> SetupResult {
        // Create and return the necessary test
        dotenv::dotenv().ok();
        let factory: H160 = H160::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f").unwrap();
        let rpc_endpoint = std::env::var("NETWORK_RPC").unwrap();
        let middleware = Arc::new(Provider::<Http>::try_from(rpc_endpoint).unwrap());

        SetupResult(factory, middleware)
    }

    #[tokio::test]
    async fn test_get_pair_addresses_batch_success() {
        let SetupResult(factory, middleware) = setup();
        let result =
            get_pair_addresses_batch(factory, U256::from(0), U256::from(10), middleware, None)
                .await
                .unwrap();
        assert_eq!(result.len(), 10);
    }

    #[tokio::test]
    async fn test_get_pair_addresses_batch_failure() {
        let SetupResult(factory, middleware) = setup();
        let result = get_pair_addresses_batch(
            factory,
            U256::from(10_000_000),
            U256::from(10_000_010),
            middleware,
            None,
        )
        .await;

        assert!(matches!(
            result,
            Err(PairsAddressesBatchError {
                factory: _,
                start: _,
                end: _,
                message: _
            })
        ),);
    }

    #[tokio::test]
    async fn test_get_pair_addresses_concurrent_success() {
        let SetupResult(factory, middleware) = setup();
        let result = get_pair_addresses_concurrent(factory, 0, 10, 1, middleware)
            .await
            .unwrap();
        assert_eq!(result.len(), 10);
    }

    #[tokio::test]
    async fn test_get_pair_addresses_concurrent_failure() {
        let SetupResult(factory, middleware) = setup();
        let result =
            get_pair_addresses_concurrent(factory, 10_000_000, 10_000_010, 1, middleware).await;
        assert!(matches!(
            result,
            Err(PairsAddressesBatchError {
                factory: _,
                start: _,
                end: _,
                message: _
            })
        ),);
    }
}
