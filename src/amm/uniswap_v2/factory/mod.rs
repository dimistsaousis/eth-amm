pub mod contracts;
pub mod events;
mod pair_addresses_batch_request;
use self::contracts::IUniswapV2Factory;
use ethers::{providers::Middleware, types::H160};
use std::sync::Arc;

pub struct UniswapV2Factory {
    pub address: H160,
    pub fee: u64,
}

impl UniswapV2Factory {
    pub fn new(address: H160, fee: u64) -> Self {
        UniswapV2Factory { address, fee }
    }
    pub fn contract<M: Middleware>(&self, middleware: Arc<M>) -> IUniswapV2Factory<M> {
        IUniswapV2Factory::new(self.address, middleware)
    }

    pub async fn all_pairs_length<M: Middleware>(&self, middleware: Arc<M>) -> u64 {
        self.contract(middleware)
            .all_pairs_length()
            .call()
            .await
            .expect(
                format!(
                    "Could not get all pairs length for factory {:?}",
                    self.address
                )
                .as_str(),
            )
            .as_u64()
    }

    pub async fn get_pair_address<M: Middleware>(
        &self,
        middleware: Arc<M>,
        token_a: H160,
        token_b: H160,
    ) -> H160 {
        self.contract(middleware)
            .get_pair(token_a, token_b)
            .call()
            .await
            .expect(
                format!(
                    "Could not get pair address for tokens {:?}, {:?}",
                    token_a, token_b
                )
                .as_str(),
            )
    }
}

#[cfg(test)]
mod tests;
