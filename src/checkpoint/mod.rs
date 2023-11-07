use ethers::types::H160;
use serde::{Deserialize, Serialize};
use std::fs;

use crate::{
    amm::{
        uniswap_v2::{
            factory::UniswapV2Factory,
            pool::pool_data_batch_request::get_uniswap_v2_pool_data_concurrent,
        },
        UniswapV2Pool,
    },
    middleware::EthProvider,
};

#[derive(Serialize, Deserialize)]
pub struct Checkpoint<T> {
    pub last_block: u64,
    pub data: Vec<T>,
    pub id: String,
}

impl<T: for<'a> Deserialize<'a> + Serialize> Checkpoint<T> {
    pub fn new(last_block: u64, data: Vec<T>, id: &str) -> Self {
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

impl Checkpoint<H160> {
    pub async fn sync_uniswap_v2_pair_addresses(
        provider: &EthProvider,
        factory: UniswapV2Factory,
        step: usize,
    ) -> Checkpoint<H160> {
        let id = format!("uniswap_v2_pair_addresses.{:?}", factory.address);
        let current_block = provider.get_block_number().await;
        match Self::load_data(&id) {
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
        }
    }
}

impl Checkpoint<UniswapV2Pool> {
    pub async fn sync_uniswap_v2_pools(
        provider: &EthProvider,
        factory: UniswapV2Factory,
        step: usize,
    ) -> Checkpoint<UniswapV2Pool> {
        let id = format!("uniswap_v2_pools.{:?}", factory.address);
        let current_block = provider.get_block_number().await;
        match Self::load_data(&id) {
            // Get all pairs from factory if no checkpoint
            None => {
                let pairs =
                    Checkpoint::<H160>::sync_uniswap_v2_pair_addresses(provider, factory, step)
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
                    step,
                )
                .await;
                checkpoint.data.extend(new_pools);
                checkpoint.last_block = current_block;
                checkpoint
            }
        }
    }
}
