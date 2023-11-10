use ethers::types::{H160, U256};
use serde::{Deserialize, Serialize};
use std::{fs, str::FromStr};

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
    async fn create(
        provider: &EthProvider,
        factory: UniswapV2Factory,
        step: usize,
        id: &str,
        current_block: u64,
    ) -> Self {
        let pairs = factory
            .get_pair_addresses_from_factory(
                0,
                factory.all_pairs_length(provider.http.clone()).await as usize,
                step,
                provider.http.clone(),
            )
            .await;
        Self::new(current_block, pairs, &id)
    }
    async fn update(
        mut self,
        provider: &EthProvider,
        factory: UniswapV2Factory,
        step: usize,
        current_block: u64,
    ) -> Self {
        let new_pairs = factory
            .get_pair_addresses_from_logs_concurrent(
                self.last_block as usize,
                current_block as usize,
                step,
                provider.http.clone(),
            )
            .await;
        self.data.extend(new_pairs);
        self.last_block = current_block;
        self
    }
    pub async fn sync_uniswap_v2_pair_addresses(
        provider: &EthProvider,
        factory: UniswapV2Factory,
        step: usize,
    ) -> Self {
        let id = format!("uniswap_v2_pair_addresses.{:?}", factory.address);
        let current_block = provider.get_block_number().await;
        let checkpoint = match Self::load_data(&id) {
            None => Self::create(provider, factory, step, &id, current_block).await,
            Some(c) => c.update(provider, factory, step, current_block).await,
        };
        checkpoint.save_data();
        checkpoint
    }
}

impl Checkpoint<Vec<UniswapV2Pool>> {
    fn id(factory_address: &H160) -> String {
        format!("uniswap_v2_pools.{:?}", factory_address)
    }

    fn get_factory_address_from_id(id: &String) -> H160 {
        let address = id.strip_prefix("uniswap_v2_pools.").unwrap();
        H160::from_str(address).unwrap()
    }

    pub fn factory_address(&self) -> H160 {
        Self::get_factory_address_from_id(&self.id)
    }

    async fn create(
        provider: &EthProvider,
        factory: UniswapV2Factory,
        id: &str,
        step: usize,
        current_block: u64,
    ) -> Self {
        let pairs =
            Checkpoint::<Vec<H160>>::sync_uniswap_v2_pair_addresses(provider, factory, step).await;
        let pools =
            get_uniswap_v2_pool_data_concurrent(&pairs.data, provider.http.clone(), 300, step)
                .await;
        Self::new(current_block, pools, id)
    }

    async fn update(
        &mut self,
        provider: &EthProvider,
        factory: UniswapV2Factory,
        step: usize,
        current_block: u64,
    ) {
        if current_block <= self.last_block {
            return;
        }

        UniswapV2Pool::sync_pools_from_logs(
            (self.last_block + 1) as usize,
            current_block as usize,
            100,
            &mut self.data,
            provider.http.clone(),
        )
        .await;
        let new_pairs = factory
            .get_pair_addresses_from_logs_concurrent(
                self.last_block as usize,
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
        self.data.extend(new_pools);
        self.last_block = current_block;
    }

    pub async fn get(provider: &EthProvider, factory: UniswapV2Factory, step: usize) -> Self {
        let id = Self::id(&factory.address);
        let current_block = provider.get_block_number().await;
        let checkpoint = match Self::load_data(&id) {
            None => Self::create(provider, factory, &id, step, current_block).await,
            Some(mut c) => {
                c.update(provider, factory, step, current_block).await;
                c
            }
        };
        checkpoint.save_data();
        checkpoint
    }

    pub async fn sync(mut self, provider: &EthProvider) {
        let factory = UniswapV2Factory::new(self.factory_address(), 300);
        let current_block = provider.get_block_number().await;
        self.update(provider, factory, 100, current_block).await;
        self.save_data()
    }

    pub async fn sync_eth_value(
        mut self,
        provider: &EthProvider,
        factory_addresses: &[H160],
        weth: H160,
        weth_threshold: U256,
    ) {
        let pool_addresses: Vec<H160> = self.data.iter().map(|p| p.address).collect();
        let weth_values = get_weth_value_in_pool_concurrent(
            &pool_addresses,
            factory_addresses,
            weth,
            weth_threshold,
            100,
            provider.http.clone(),
        )
        .await;
        for pool in &mut self.data {
            pool.eth_value = *weth_values.get(&pool.address).ok_or(U256::zero()).unwrap();
        }
        self.save_data();
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use rand::{seq::SliceRandom, thread_rng};

    use crate::{
        address_book::AddressBook,
        amm::uniswap_v2::{factory::UniswapV2Factory, pool::UniswapV2Pool},
        checkpoint::Checkpoint,
        middleware::EthProvider,
    };

    #[tokio::test]
    async fn test_checkpoint_sync_pools_from_logs() {
        dotenv::dotenv().ok();
        let provider = EthProvider::new().await;
        let book = AddressBook::new();
        let factory: UniswapV2Factory = UniswapV2Factory::new(book.mainnet.uniswap_v2.factory, 300);
        let pools = Checkpoint::<Vec<UniswapV2Pool>>::get(&provider, factory, 100).await;
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
