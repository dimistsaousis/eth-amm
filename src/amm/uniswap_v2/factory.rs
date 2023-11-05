use ethers::abi::{ParamType, RawLog, Token};
use ethers::prelude::{abigen, EthEvent};
use ethers::providers::Middleware;
use ethers::types::{BlockNumber, Bytes, Filter, ValueOrArray, H160, H256, U256, U64};
use indicatif::ProgressBar;
use std::fmt;
use std::sync::{Arc, Mutex};

use crate::concurrent::run_concurrent;

abigen!(
    IGetUniswapV2PairsBatchRequest,
        "src/contract/abi/GetUniswapV2PairsBatchRequestABI.json";

    IUniswapV2Factory,
    r#"[
        function getPair(address tokenA, address tokenB) external view returns (address pair)
        function allPairs(uint256 index) external view returns (address)
        event PairCreated(address indexed token0, address indexed token1, address pair, uint256)
        function allPairsLength() external view returns (uint256)

    ]"#;
);

#[derive(Debug)]
pub struct PairsAddressesBatchError {
    factory: H160,
    start: u64,
    end: u64,
    message: String,
}

impl fmt::Display for PairsAddressesBatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Error fetching pairs for factory {:?} between start {} and end {}: {}",
            self.factory, self.start, self.end, self.message
        )
    }
}

#[derive(Debug)]
pub struct PairsAddressesLogsError {
    factory: H160,
    start: u64,
    end: u64,
    message: String,
}

impl fmt::Display for PairsAddressesLogsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Error fetching pairs from logs for factory {:?} between blocks {} and {}: {}",
            self.factory, self.start, self.end, self.message
        )
    }
}

pub const PAIR_CREATED_EVENT_SIGNATURE: H256 = H256([
    13, 54, 72, 189, 15, 107, 168, 1, 52, 163, 59, 169, 39, 90, 197, 133, 217, 211, 21, 240, 173,
    131, 85, 205, 222, 253, 227, 26, 250, 40, 208, 233,
]);

async fn get_pair_addresses_from_factory_batch<M: Middleware>(
    factory: H160,
    start: u64,
    end: u64,
    middleware: Arc<M>,
    progress_bar: Option<Arc<Mutex<ProgressBar>>>,
) -> Result<Vec<H160>, PairsAddressesBatchError> {
    let mut pairs = vec![];
    let constructor_args = Token::Tuple(vec![
        Token::Uint(U256::from(start)),
        Token::Uint(U256::from(end)),
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
        pb.lock().unwrap().inc(end as u64 - start as u64);
    }

    Ok(pairs)
}

pub async fn get_pair_addresses_from_factory_concurrent<'a, M: Middleware + 'a>(
    factory: H160,
    start: u64,
    end: u64,
    step: usize,
    middleware: Arc<M>,
) -> Result<Vec<H160>, PairsAddressesBatchError> {
    let batch_func =
        |start: u64, end: u64, middleware: Arc<M>, pb: Option<Arc<Mutex<ProgressBar>>>| {
            get_pair_addresses_from_factory_batch(factory, start, end, middleware.clone(), pb)
        };
    run_concurrent(start, end, step, middleware, batch_func).await
}

async fn get_pair_addresses_from_logs<'a, M: Middleware + 'a>(
    factory: H160,
    start: u64,
    end: u64,
    middleware: Arc<M>,
    progress_bar: Option<Arc<Mutex<ProgressBar>>>,
) -> Result<Vec<H160>, PairsAddressesLogsError> {
    let logs = middleware
        .get_logs(
            &Filter::new()
                .topic0(ValueOrArray::Value(PAIR_CREATED_EVENT_SIGNATURE))
                .address(factory)
                .from_block(BlockNumber::Number(U64([start as u64])))
                .to_block(BlockNumber::Number(U64([end as u64]))),
        )
        .await
        .map_err(|err| PairsAddressesLogsError {
            factory,
            start,
            end,
            message: format!("Failed to decode data: {}", err),
        })?;
    let mut addresses = vec![];
    for log in logs {
        let pair_created_event: PairCreatedFilter =
            PairCreatedFilter::decode_log(&RawLog::from(log)).map_err(|err| {
                PairsAddressesLogsError {
                    factory,
                    start,
                    end,
                    message: format!("Failed to decode data: {}", err),
                }
            })?;
        addresses.push(pair_created_event.pair);
    }

    if let Some(pb) = progress_bar {
        pb.lock().unwrap().inc(end as u64 - start as u64);
    }

    Ok(addresses)
}

pub async fn get_pair_addresses_from_logs_concurrent<'a, M: Middleware + 'a>(
    factory: H160,
    start: u64,
    end: u64,
    step: usize,
    middleware: Arc<M>,
) -> Result<Vec<H160>, PairsAddressesLogsError> {
    let batch_func =
        |start: u64, end: u64, middleware: Arc<M>, pb: Option<Arc<Mutex<ProgressBar>>>| {
            get_pair_addresses_from_logs(factory, start, end, middleware.clone(), pb)
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
    async fn test_get_pair_addresses_from_factory_batch_success() {
        let SetupResult(factory, middleware) = setup();
        let result = get_pair_addresses_from_factory_batch(factory, 0, 10, middleware, None)
            .await
            .unwrap();
        assert_eq!(result.len(), 10);
    }

    #[tokio::test]
    async fn test_get_pair_addresses_from_factory_batch_failure() {
        let SetupResult(factory, middleware) = setup();
        let result = get_pair_addresses_from_factory_batch(
            factory, 10_000_000, 10_000_010, middleware, None,
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
    async fn test_get_pair_addresses_from_factory_concurrent_success() {
        let SetupResult(factory, middleware) = setup();
        let result = get_pair_addresses_from_factory_concurrent(factory, 0, 10, 1, middleware)
            .await
            .unwrap();
        assert_eq!(result.len(), 10);
    }

    #[tokio::test]
    async fn test_get_pair_addresses_from_factory_concurrent_failure() {
        let SetupResult(factory, middleware) = setup();
        let result = get_pair_addresses_from_factory_concurrent(
            factory, 10_000_000, 10_000_010, 1, middleware,
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
    async fn test_get_pair_addresses_from_logs_success() {
        let SetupResult(factory, middleware) = setup();
        let result = get_pair_addresses_from_logs(factory, 10008355, 10009355, middleware, None)
            .await
            .unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_get_pair_addresses_from_logs_concurrent_success() {
        let SetupResult(factory, middleware) = setup();
        let result =
            get_pair_addresses_from_logs_concurrent(factory, 10008355, 10009355, 100, middleware)
                .await
                .unwrap();
        assert_eq!(result.len(), 2);
    }
}
