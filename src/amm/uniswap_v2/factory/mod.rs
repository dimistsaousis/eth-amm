pub mod contracts;
pub mod pair_created_event;
use self::contracts::IUniswapV2Factory;
use ethers::{providers::Middleware, types::H160};
use std::sync::Arc;

pub struct UniswapV2Factory {
    pub address: H160,
}

impl UniswapV2Factory {
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
}
