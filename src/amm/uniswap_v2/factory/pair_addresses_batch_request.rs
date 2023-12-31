use std::sync::{Arc, Mutex};

use ethers::{
    abi::{ParamType, Token},
    providers::Middleware,
    types::{Bytes, H160, U256},
};
use indicatif::ProgressBar;

use crate::{
    concurrent::{run_concurrent, BatchError},
    contract::GetUniswapV2PairsBatchRequest,
};

use super::UniswapV2Factory;

impl UniswapV2Factory {
    async fn get_pair_addresses_from_factory_batch<M: Middleware>(
        &self,
        start: usize,
        end: usize,
        middleware: Arc<M>,
        progress_bar: Option<Arc<Mutex<ProgressBar>>>,
    ) -> Result<Vec<H160>, BatchError> {
        let mut pairs = vec![];
        let constructor_args = Token::Tuple(vec![
            Token::Uint(U256::from(start)),
            Token::Uint(U256::from(end)),
            Token::Address(self.address),
        ]);

        let deployer = GetUniswapV2PairsBatchRequest::deploy(middleware, constructor_args)
            .map_err(|_| BatchError::new(start, end))?;
        let return_data: Bytes = deployer
            .call_raw()
            .await
            .map_err(|_| BatchError::new(start, end))?;

        let return_data_tokens = ethers::abi::decode(
            &[ParamType::Array(Box::new(ParamType::Address))],
            &return_data,
        )
        .map_err(|_| BatchError::new(start, end))?;

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

    pub async fn get_pair_addresses_from_factory<'a, M: Middleware + 'a>(
        &self,
        start: usize,
        end: usize,
        step: usize,
        middleware: Arc<M>,
    ) -> Vec<H160> {
        let batch_func =
            |start: usize, end: usize, middleware: Arc<M>, pb: Option<Arc<Mutex<ProgressBar>>>| {
                self.get_pair_addresses_from_factory_batch(start, end, middleware.clone(), pb)
            };
        println!(
            "Getting pair addresses from Uniswap v2 factory {:?}, from {} to {} with step {}",
            self.address, start, end, step
        );
        run_concurrent(start, end, step, middleware, batch_func).await
    }
}
