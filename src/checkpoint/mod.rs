use ethers::types::{H160, U256};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, str::FromStr};

use crate::{
    amm::{
        uniswap_v2::{
            factory::UniswapV2Factory,
            pool::{pool_data_batch_request::get_uniswap_v2_pool_data_concurrent, UniswapV2Pool},
        },
        weth_value::get_weth_value_in_pool_concurrent,
    },
    middleware::EthProvider,
};

#[derive(Serialize, Deserialize)]
pub struct Checkpoint<T> {
    pub last_block: u64,
    pub data: T,
    pub id: String,
}

impl<T: for<'a> Deserialize<'a> + Serialize> Checkpoint<T> {
    pub fn new(last_block: u64, data: T, id: &str) -> Self {
        Checkpoint {
            last_block,
            data,
            id: id.to_string(),
        }
    }

    fn path(id: &str) -> String {
        format!("src/checkpoint/data/{}", id)
    }

    pub fn load_data(id: &str) -> Option<Self> {
        match fs::read_to_string(Self::path(&id))
            .map_err(|e| e.to_string())
            .and_then(|data| serde_json::from_str(&data).map_err(|e| e.to_string()))
        {
            Ok(checkpoint) => checkpoint,
            Err(_) => None,
        }
    }

    pub fn save_data(&self) {
        let serialized = serde_json::to_string(self).unwrap();
        fs::write(Self::path(&self.id), serialized).unwrap();
    }
}

impl Checkpoint<Vec<H160>> {
    pub async fn sync_uniswap_v2_pair_addresses(
        provider: &EthProvider,
        factory: UniswapV2Factory,
        step: usize,
    ) -> Self {
        let id = format!("uniswap_v2_pair_addresses.{:?}", factory.address);
        let current_block = provider.get_block_number().await;
        let checkpoint = match Self::load_data(&id) {
            // Get all pairs from factory if no checkpoint
            None => {
                let pairs = factory
                    .get_pair_addresses_from_factory(
                        0,
                        factory.all_pairs_length(provider.http.clone()).await as usize,
                        step,
                        provider.http.clone(),
                    )
                    .await;
                let checkpoint = Self::new(current_block, pairs, &id);
                checkpoint
            }

            // Continue from last synced block and get the rest from the logs
            Some(mut checkpoint) => {
                let new_pairs = factory
                    .get_pair_addresses_from_logs_concurrent(
                        checkpoint.last_block as usize,
                        current_block as usize,
                        step,
                        provider.http.clone(),
                    )
                    .await;
                checkpoint.data.extend(new_pairs);
                checkpoint.last_block = current_block;
                checkpoint
            }
        };
        checkpoint.save_data();
        checkpoint
    }
}

impl Checkpoint<Vec<UniswapV2Pool>> {
    pub async fn sync_uniswap_v2_pools(
        provider: &EthProvider,
        factory: UniswapV2Factory,
        step: usize,
    ) -> Self {
        let id = format!("uniswap_v2_pools.{:?}", factory.address);
        let current_block = provider.get_block_number().await;
        let checkpoint = match Self::load_data(&id) {
            // Get all pairs from factory if no checkpoint
            None => {
                let pairs = Checkpoint::<Vec<H160>>::sync_uniswap_v2_pair_addresses(
                    provider, factory, step,
                )
                .await;
                let pools = get_uniswap_v2_pool_data_concurrent(
                    &pairs.data,
                    provider.http.clone(),
                    300,
                    step,
                )
                .await;
                Self::new(current_block, pools, &id)
            }

            // Continue from last synced block and get the rest from the logs
            Some(mut checkpoint) => {
                UniswapV2Pool::sync_pools_from_logs(
                    (checkpoint.last_block + 1) as usize,
                    current_block as usize,
                    100,
                    &mut checkpoint.data,
                    provider.http.clone(),
                )
                .await;
                let new_pairs = factory
                    .get_pair_addresses_from_logs_concurrent(
                        checkpoint.last_block as usize,
                        current_block as usize,
                        step,
                        provider.http.clone(),
                    )
                    .await;
                let new_pools = get_uniswap_v2_pool_data_concurrent(
                    &new_pairs,
                    provider.http.clone(),
                    300,
                    new_pairs.len().div_ceil(10).max(step),
                )
                .await;
                checkpoint.data.extend(new_pools);
                checkpoint.last_block = current_block;
                checkpoint
            }
        };
        checkpoint.save_data();
        checkpoint
    }
}

impl Checkpoint<HashMap<H160, U256>> {
    pub async fn sync_uniswap_v2_pools_eth_value(
        provider: &EthProvider,
        factory: UniswapV2Factory,
        step: usize,
    ) -> Checkpoint<HashMap<H160, U256>> {
        let id = format!("uniswap_v2_pools_eth_value.{:?}", factory.address);
        let current_block = provider.get_block_number().await;
        let factory_addresses = vec![factory.address];
        let weth = H160::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap();
        let checkpoint = match Self::load_data(&id) {
            // Get all pairs from factory if no checkpoint
            None => {
                let pairs = Checkpoint::<Vec<H160>>::sync_uniswap_v2_pair_addresses(
                    provider, factory, step,
                )
                .await;
                let weth_values = get_weth_value_in_pool_concurrent(
                    &pairs.data,
                    &factory_addresses,
                    weth,
                    U256::exp10(18),
                    100,
                    provider.http.clone(),
                )
                .await;
                Self::new(current_block, weth_values, &id)
            }

            // Continue from last synced block and get the rest from the logs
            Some(mut checkpoint) => {
                let new_pairs = factory
                    .get_pair_addresses_from_logs_concurrent(
                        checkpoint.last_block as usize,
                        current_block as usize,
                        step,
                        provider.http.clone(),
                    )
                    .await;
                let weth_values = get_weth_value_in_pool_concurrent(
                    &new_pairs,
                    &factory_addresses,
                    weth,
                    U256::exp10(18),
                    100,
                    provider.http.clone(),
                )
                .await;
                checkpoint.data.extend(weth_values);
                checkpoint.last_block = current_block;
                checkpoint
            }
        };
        checkpoint.save_data();
        checkpoint
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use ethers::types::H160;
    use itertools::Itertools;
    use rand::{seq::SliceRandom, thread_rng};

    use crate::{
        amm::uniswap_v2::{factory::UniswapV2Factory, pool::UniswapV2Pool},
        checkpoint::Checkpoint,
        middleware::EthProvider,
    };

    #[tokio::test]
    async fn test_checkpoint_sync_pools_from_logs() {
        dotenv::dotenv().ok();
        let provider = EthProvider::new().await;
        let factory_address = H160::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f").unwrap();
        let factory: UniswapV2Factory = UniswapV2Factory::new(factory_address, 300);
        let pools =
            Checkpoint::<Vec<UniswapV2Pool>>::sync_uniswap_v2_pools(&provider, factory, 100).await;
        // Randomly choose 100 elements
        let mut rng = thread_rng();
        let random_pools: Vec<_> = pools
            .data
            .into_iter()
            .filter(|p| p.token_a_decimals > 0 && p.token_b_decimals > 0 && p.reserve_0 > 0)
            .collect_vec()
            .choose_multiple(&mut rng, 1000)
            .cloned()
            .collect();

        assert_eq!(random_pools.len(), 1000);
        // Use cloned data in async calls
        let futures = random_pools
            .clone()
            .into_iter()
            .map(|p| {
                let http_client = provider.http.clone();
                async move { p.get_reserves(http_client).await }
            })
            .collect::<Vec<_>>();

        let results = futures::future::join_all(futures).await;

        for idx in 0..results.len() {
            assert_eq!(
                random_pools[idx].reserve_0, results[idx].0,
                "{:?}",
                random_pools[idx].address
            );
            assert_eq!(random_pools[idx].reserve_1, results[idx].1);
        }
    }
}
