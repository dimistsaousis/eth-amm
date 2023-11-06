pub mod contracts;
use std::sync::Arc;

use ethers::{providers::Middleware, types::H160};
use serde::{Deserialize, Serialize};

use self::contracts::IUniswapV2Pair;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UniswapV2Pool {
    pub address: H160,
    pub token_a: H160,
    pub token_a_decimals: u8,
    pub token_b: H160,
    pub token_b_decimals: u8,
    pub reserve_0: u128,
    pub reserve_1: u128,
    pub fee: u32,
}

impl UniswapV2Pool {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        address: H160,
        token_a: H160,
        token_a_decimals: u8,
        token_b: H160,
        token_b_decimals: u8,
        reserve_0: u128,
        reserve_1: u128,
        fee: u32,
    ) -> UniswapV2Pool {
        UniswapV2Pool {
            address,
            token_a,
            token_a_decimals,
            token_b,
            token_b_decimals,
            reserve_0,
            reserve_1,
            fee,
        }
    }
    pub async fn get_reserves<M: Middleware>(&self, middleware: Arc<M>) -> (u128, u128) {
        let v2_pair = IUniswapV2Pair::new(self.address, middleware);
        let (reserve_0, reserve_1, _) = v2_pair
            .get_reserves()
            .call()
            .await
            .expect("Could not get reserves");
        (reserve_0, reserve_1)
    }

    pub async fn sync_reserves<M: Middleware>(&mut self, middleware: Arc<M>) {
        (self.reserve_0, self.reserve_1) = self.get_reserves(middleware).await
    }
}
