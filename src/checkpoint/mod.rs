use ethers::types::H160;
use serde::{Deserialize, Serialize};
use std::fs;

use crate::{amm::uniswap_v2::factory::UniswapV2Factory, middleware::EthProvider};

#[derive(Serialize, Deserialize)]
pub struct Checkpoint<T> {
    pub last_block: u64,
    pub data: Vec<T>,
}

impl<T: for<'a> Deserialize<'a> + Serialize> Checkpoint<T> {
    pub fn new(last_block: u64, data: Vec<T>) -> Self {
        Checkpoint { last_block, data }
    }

    fn path(loc: &str) -> String {
        format!("src/checkpoint/data/{}", loc)
    }

    pub fn load_data(loc: &str) -> Option<Self> {
        match fs::read_to_string(Self::path(loc))
            .map_err(|e| e.to_string())
            .and_then(|data| serde_json::from_str(&data).map_err(|e| e.to_string()))
        {
            Ok(checkpoint) => checkpoint,
            Err(_) => None,
        }
    }

    pub fn save_data(&self, loc: &str) {
        let serialized = serde_json::to_string(self).unwrap();
        fs::write(Self::path(loc), serialized).unwrap();
    }
}

impl Checkpoint<H160> {
    pub async fn sync_uniswap_v2_pair_addresses(
        provider: EthProvider,
        factory: UniswapV2Factory,
    ) -> Checkpoint<H160> {
        let checkpoint = Checkpoint::<H160>::load_data("uniswap_v2_pair_addresses");
        let step = 100;
        let mut checkpoint = match checkpoint {
            None => {
                let block_number = provider.get_block_number().await;
                let pairs = factory
                    .get_pair_addresses_from_factory(
                        0,
                        factory.all_pairs_length(provider.http.clone()).await,
                        step,
                        provider.http.clone(),
                    )
                    .await;
                let checkpoint = Checkpoint::<H160>::new(block_number, pairs);
                checkpoint.save_data("uniswap_v2_pair_addresses");
                checkpoint
            }
            Some(checkpoint) => checkpoint,
        };
        let block_number = provider.get_block_number().await;
        let new_pairs = factory
            .get_pair_addresses_from_logs_concurrent(
                checkpoint.last_block,
                block_number,
                step,
                provider.http.clone(),
            )
            .await;
        checkpoint.data.extend(new_pairs);
        checkpoint.last_block = block_number;
        checkpoint
    }
}
