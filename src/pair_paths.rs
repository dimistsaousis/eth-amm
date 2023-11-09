use std::collections::{HashMap, HashSet};

use ethers::types::H160;

use crate::amm::uniswap_v2::pool::UniswapV2Pool;

pub fn get_pools_by_token_address(pools: Vec<UniswapV2Pool>) -> HashMap<H160, Vec<UniswapV2Pool>> {
    let mut pools_by_token_address: HashMap<H160, Vec<UniswapV2Pool>> = HashMap::new();
    for pool in pools {
        pools_by_token_address
            .entry(pool.token_a)
            .or_insert(vec![])
            .push(pool.clone());
        pools_by_token_address
            .entry(pool.token_b)
            .or_insert(vec![])
            .push(pool);
    }
    pools_by_token_address
}

pub fn get_token_path(start_token: H160, pool_path: Vec<UniswapV2Pool>) -> Vec<H160> {
    let mut current_token = start_token;
    let mut token_path = vec![current_token];
    for pool in pool_path {
        current_token = pool.get_token_out(current_token);
        token_path.push(current_token);
    }
    token_path
}

pub fn get_all_token_paths(
    token: H160,
    exchange_map: HashMap<H160, Vec<UniswapV2Pool>>,
    max_length: usize,
) -> Vec<Vec<H160>> {
    let exchange_paths = get_all_exchange_paths(token, exchange_map, max_length);
    exchange_paths
        .into_iter()
        .map(|path| get_token_path(token, path))
        .collect()
}

pub fn get_all_exchange_paths(
    token: H160,
    exchange_map: HashMap<H160, Vec<UniswapV2Pool>>,
    max_length: usize,
) -> Vec<Vec<UniswapV2Pool>> {
    let mut paths = Vec::new();
    let mut visited = HashSet::new();
    let mut current_path = Vec::new();
    let mut unique_paths = HashSet::new();
    find_paths(
        token,
        token,
        &exchange_map,
        max_length,
        &mut visited,
        &mut current_path,
        &mut paths,
        &mut unique_paths,
    );
    paths
}

fn find_paths(
    start_token: H160,
    current_token: H160,
    exchange_map: &HashMap<H160, Vec<UniswapV2Pool>>,
    max_length: usize,
    visited: &mut HashSet<H160>,
    current_path: &mut Vec<UniswapV2Pool>,
    paths: &mut Vec<Vec<UniswapV2Pool>>,
    unique_paths: &mut HashSet<String>,
) {
    if start_token == current_token && !current_path.is_empty() {
        let mut path_addresses: Vec<_> = current_path
            .iter()
            .map(|pool| pool.address.to_string())
            .collect();
        path_addresses.sort();
        let path_identifier = path_addresses.join(",");
        if unique_paths.insert(path_identifier) {
            paths.push(current_path.clone());
        }
        return;
    }

    if current_path.len() == max_length {
        return;
    }

    if let Some(exchanges) = exchange_map.get(&current_token) {
        for exchange in exchanges {
            if !visited.contains(&exchange.address) {
                visited.insert(exchange.address.clone());
                current_path.push(exchange.clone());
                let mut next_token = exchange.token_a;
                if current_token == next_token {
                    next_token = exchange.token_b;
                }
                find_paths(
                    start_token,
                    next_token,
                    exchange_map,
                    max_length,
                    visited,
                    current_path,
                    paths,
                    unique_paths,
                );
                visited.remove(&exchange.address);
                current_path.pop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{amm::uniswap_v2::factory::UniswapV2Factory, middleware::EthProvider};
    use itertools::Itertools;
    use std::str::FromStr;

    struct SetupResult(H160, HashMap<H160, Vec<UniswapV2Pool>>);

    async fn setup() -> SetupResult {
        // Create and return the necessary test
        dotenv::dotenv().ok();
        let provider = EthProvider::new().await;
        let factory_address = H160::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f").unwrap();
        let factory: UniswapV2Factory = UniswapV2Factory::new(factory_address, 300);

        let tokens: Vec<H160> = vec![
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
            "0x514910771AF9Ca656af840dff83E8264EcF986CA",
            "0x7D1AfA7B718fb893dB30A3aBc0Cfc608AaCfeBB0",
        ]
        .into_iter()
        .map(|a| H160::from_str(a).unwrap())
        .collect();

        let start_token = tokens[0].clone();
        let mut pools = vec![];
        for t in tokens.clone().into_iter().combinations(2) {
            if let [t1, t2] = t.as_slice() {
                let pool_address = factory
                    .get_pair_address(provider.http.clone(), *t1, *t2)
                    .await;
                let pool =
                    UniswapV2Pool::from_address(provider.http.clone(), pool_address, 300).await;
                pools.push(pool)
            }
        }
        let pools_map: HashMap<H160, Vec<UniswapV2Pool>> = get_pools_by_token_address(pools);

        SetupResult(start_token, pools_map)
    }

    #[tokio::test]
    async fn test_get_all_exchanges_paths() {
        let SetupResult(start_token, pools_map) = setup().await;
        let paths = get_all_exchange_paths(start_token, pools_map, 3);
        assert_eq!(paths.len(), 1);
    }

    #[tokio::test]
    async fn test_get_all_token_paths() {
        let SetupResult(start_token, pools_map) = setup().await;
        let paths = get_all_token_paths(start_token, pools_map, 3);
        assert_eq!(paths.len(), 1);
        let path = paths.into_iter().next().unwrap();
        assert_eq!(path.len(), 4);
        assert_eq!(path.as_slice()[0], start_token);
        assert_ne!(path.as_slice()[1], start_token);
        assert_ne!(path.as_slice()[2], start_token);
        assert_ne!(path.as_slice()[2], path.as_slice()[1]);
        assert_eq!(path.last(), Some(&start_token));
    }
}
