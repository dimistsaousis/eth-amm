use crate::amm::uniswap_v2::factory::contracts::PairCreatedFilter;
use crate::amm::uniswap_v2::factory::UniswapV2Factory;
use crate::concurrent::run_concurrent;
use ethers::abi::RawLog;
use ethers::prelude::EthEvent;
use ethers::providers::{Provider, Ws};
use ethers::{
    providers::Middleware,
    types::{BlockNumber, Filter, ValueOrArray, H160, H256, U64},
};
use futures::StreamExt;
use indicatif::ProgressBar;
use std::sync::{Arc, Mutex};

pub const PAIR_CREATED_EVENT_SIGNATURE: H256 = H256([
    13, 54, 72, 189, 15, 107, 168, 1, 52, 163, 59, 169, 39, 90, 197, 133, 217, 211, 21, 240, 173,
    131, 85, 205, 222, 253, 227, 26, 250, 40, 208, 233,
]);

impl UniswapV2Factory {
    async fn get_pair_addresses_from_logs<'a, M: Middleware + 'a>(
        &self,
        start: usize,
        end: usize,
        middleware: Arc<M>,
        progress_bar: Option<Arc<Mutex<ProgressBar>>>,
    ) -> Vec<H160> {
        let logs = middleware
            .get_logs(
                &Filter::new()
                    .topic0(ValueOrArray::Value(PAIR_CREATED_EVENT_SIGNATURE))
                    .address(self.address)
                    .from_block(BlockNumber::Number(U64([start as u64])))
                    .to_block(BlockNumber::Number(U64([end as u64]))),
            )
            .await
            .expect("Failed to decode pair created events");

        let mut addresses = vec![];
        for log in logs {
            let pair_created_event: PairCreatedFilter =
                PairCreatedFilter::decode_log(&RawLog::from(log)).expect("Failed to decode data");
            addresses.push(pair_created_event.pair);
        }
        if let Some(pb) = progress_bar {
            pb.lock().unwrap().inc(end as u64 - start as u64);
        }
        addresses
    }

    pub async fn get_pair_addresses_from_logs_concurrent<'a, M: Middleware + 'a>(
        &self,
        start: usize,
        end: usize,
        step: usize,
        middleware: Arc<M>,
    ) -> Vec<H160> {
        let batch_func =
            |start: usize, end: usize, middleware: Arc<M>, pb: Option<Arc<Mutex<ProgressBar>>>| {
                self.get_pair_addresses_from_logs(start, end, middleware.clone(), pb)
            };
        println!(
            "Getting pair addresses from logs for Uniswap v2 factory {:?}, from block {} to {} with step {}",
            self.address, start, end, step
        );
        run_concurrent(start, end, step, middleware, batch_func).await
    }

    pub async fn subscribe_pair_created_event<F>(wss: Arc<Provider<Ws>>, func: F)
    where
        F: Fn(H160, PairCreatedFilter) -> (),
    {
        let filter = Filter::new().topic0(ValueOrArray::Value(PAIR_CREATED_EVENT_SIGNATURE));
        let mut stream = wss
            .subscribe_logs(&filter)
            .await
            .expect("Could not subscribe to new pairs stream");
        while let Some(log) = stream.next().await {
            let pair_created_event: PairCreatedFilter =
                PairCreatedFilter::decode_log(&RawLog::from(log.clone()))
                    .expect("Failed to decode PairCreatedFilter data");
            func(log.address, pair_created_event);
        }
    }
}
