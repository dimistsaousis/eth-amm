use crate::amm::uniswap_v2::pool::{contracts::SyncFilter, UniswapV2Pool};
use crate::concurrent::{run_concurrent_hash, BatchError};
use ethers::prelude::EthEvent;
use ethers::providers::{Provider, Ws};
use ethers::types::{H160, U256};
use ethers::{
    abi::RawLog,
    providers::Middleware,
    types::{BlockNumber, Filter, ValueOrArray, H256, U64},
};
use futures::StreamExt;
use indicatif::ProgressBar;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub const SYNC_EVENT_SIGNATURE: H256 = H256([
    28, 65, 30, 154, 150, 224, 113, 36, 28, 47, 33, 247, 114, 107, 23, 174, 137, 227, 202, 180,
    199, 139, 229, 14, 6, 43, 3, 169, 255, 251, 186, 209,
]);

impl UniswapV2Pool {
    async fn get_sync_events_from_logs<'a, M: Middleware + 'a>(
        start: usize,
        end: usize,
        addresses: HashSet<H160>,
        middleware: Arc<M>,
        progress_bar: Option<Arc<Mutex<ProgressBar>>>,
    ) -> Result<HashMap<H160, SyncFilter>, BatchError> {
        let logs = middleware
            .get_logs(
                &Filter::new()
                    .topic0(ValueOrArray::Value(SYNC_EVENT_SIGNATURE))
                    .from_block(BlockNumber::Number(U64([start as u64])))
                    .to_block(BlockNumber::Number(U64([end as u64]))),
            )
            .await
            .map_err(|_| BatchError::new(start, end))?;

        let mut sync_events = HashMap::new();
        let mut last_event: HashMap<H160, (U64, U256)> = HashMap::new();
        for log in logs {
            let address = log.address;
            if addresses.contains(&address) {
                if let (Some(block_number), Some(index)) = (log.block_number, log.log_index) {
                    let default_value = (U64::from(0), U256::from(0));
                    let (current_block_number, current_index) =
                        last_event.get(&address).unwrap_or(&default_value);
                    if (&block_number, &index) >= (current_block_number, current_index) {
                        let sync_event: SyncFilter = SyncFilter::decode_log(&RawLog::from(log))
                            .map_err(|_| BatchError::new(start, end))?;
                        last_event.insert(address, (*current_block_number, *current_index));
                        sync_events.insert(address, sync_event);
                    }
                }
            }
        }
        if let Some(pb) = progress_bar {
            pb.lock().unwrap().inc(end as u64 - start as u64);
        }
        Ok(sync_events)
    }

    pub async fn get_sync_events_from_logs_concurrent<'a, M: Middleware + 'a>(
        start: usize,
        end: usize,
        step: usize,
        addresses: HashSet<H160>,
        middleware: Arc<M>,
    ) -> HashMap<H160, SyncFilter> {
        let batch_func = |start: usize,
                          end: usize,
                          middleware: Arc<M>,
                          pb: Option<Arc<Mutex<ProgressBar>>>| {
            Self::get_sync_events_from_logs(start, end, addresses.clone(), middleware.clone(), pb)
        };
        println!(
            "Getting sync events from logs for Uniswap v2 factory, from block {} to {} with step {}",
            start, end, step
        );
        run_concurrent_hash(start, end, step, middleware, batch_func).await
    }

    pub async fn sync_pools_from_logs<'a, M: Middleware + 'a>(
        start: usize,
        end: usize,
        step: usize,
        pools: &mut Vec<Self>,
        middleware: Arc<M>,
    ) -> &mut Vec<Self> {
        let mut pools_map = HashMap::new();
        let addresses = pools
            .into_iter()
            .map(|p| {
                let address = p.address;
                pools_map.insert(address, p);
                address
            })
            .collect();
        let sync_events =
            Self::get_sync_events_from_logs_concurrent(start, end, step, addresses, middleware)
                .await;
        for (address, event) in sync_events {
            pools_map.get_mut(&address).unwrap().reserve_0 = event.reserve_0;
            pools_map.get_mut(&address).unwrap().reserve_1 = event.reserve_1;
        }
        pools
    }

    pub async fn subscribe_sync_event<F>(wss: Arc<Provider<Ws>>, func: F)
    where
        F: Fn(H160, SyncFilter) -> (),
    {
        let filter = Filter::new().topic0(ValueOrArray::Value(SYNC_EVENT_SIGNATURE));
        let mut stream = wss
            .subscribe_logs(&filter)
            .await
            .expect("Could not subscribe to sync event");
        while let Some(log) = stream.next().await {
            let sync_event: SyncFilter = SyncFilter::decode_log(&RawLog::from(log.clone()))
                .expect("Failed to decode SyncFilter data");
            func(log.address, sync_event);
        }
    }
}
