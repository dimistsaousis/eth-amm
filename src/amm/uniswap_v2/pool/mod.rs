pub mod contracts;
pub mod sync_event;
use crate::arithmetic::{div_uu, q64_to_f64, U128_0X10000000000000000};

use self::contracts::IUniswapV2Pair;
use ethers::{
    providers::Middleware,
    types::{H160, U256},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

    pub fn calculate_price_64_x_64(&self, base_token: H160) -> u128 {
        let decimal_shift = self.token_a_decimals as i8 - self.token_b_decimals as i8;

        let (r_0, r_1) = if decimal_shift < 0 {
            (
                U256::from(self.reserve_0)
                    * U256::from(10u128.pow(decimal_shift.unsigned_abs() as u32)),
                U256::from(self.reserve_1),
            )
        } else {
            (
                U256::from(self.reserve_0),
                U256::from(self.reserve_1) * U256::from(10u128.pow(decimal_shift as u32)),
            )
        };

        if base_token == self.token_a {
            if r_0.is_zero() {
                U128_0X10000000000000000
            } else {
                div_uu(r_1, r_0)
            }
        } else if r_1.is_zero() {
            U128_0X10000000000000000
        } else {
            div_uu(r_0, r_1)
        }
    }

    pub fn price(&self, base_token: H160) -> f64 {
        q64_to_f64(self.calculate_price_64_x_64(base_token))
    }
}
