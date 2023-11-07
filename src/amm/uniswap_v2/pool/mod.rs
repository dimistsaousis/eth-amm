pub mod contracts;
pub mod events;
pub mod pool_data_batch_request;
use self::{
    contracts::{IErc20, IUniswapV2Pair},
    pool_data_batch_request::get_uniswap_v2_pool_data_concurrent,
};
use crate::arithmetic::{div_uu, q64_to_f64, U128_0X10000000000000000};
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

    pub fn contract<M: Middleware>(&self, middleware: Arc<M>) -> IUniswapV2Pair<M> {
        IUniswapV2Pair::new(self.address, middleware)
    }

    pub async fn factory<M: Middleware>(&self, middleware: Arc<M>) -> H160 {
        self.contract(middleware)
            .factory()
            .call()
            .await
            .expect(&format!(
                "Could not get factory for pair {:?}",
                self.address
            ))
    }

    pub async fn from_address<M: Middleware>(middleware: Arc<M>, address: H160, fee: u32) -> Self {
        let pool = get_uniswap_v2_pool_data_concurrent(&vec![address], middleware, fee, 1).await;
        pool.into_iter().next().unwrap()
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

    pub fn simulate_swap(&self, token_in: H160, amount_in: U256) -> U256 {
        if self.token_a == token_in {
            self.get_amount_out(
                amount_in,
                U256::from(self.reserve_0),
                U256::from(self.reserve_1),
            )
        } else {
            self.get_amount_out(
                amount_in,
                U256::from(self.reserve_1),
                U256::from(self.reserve_0),
            )
        }
    }

    pub fn simulate_swap_mut(&mut self, token_in: H160, amount_in: U256) -> U256 {
        let amount_out = self.simulate_swap(token_in, amount_in);
        if self.token_a == token_in {
            self.reserve_0 += amount_in.as_u128();
            self.reserve_1 -= amount_out.as_u128();
        } else {
            self.reserve_0 -= amount_out.as_u128();
            self.reserve_1 += amount_in.as_u128();
        }
        amount_out
    }

    pub fn get_amount_out(&self, amount_in: U256, reserve_in: U256, reserve_out: U256) -> U256 {
        if amount_in.is_zero() || reserve_in.is_zero() || reserve_out.is_zero() {
            return U256::zero();
        }
        let fee = (10000 - (self.fee / 10)) / 10; //Fee of 300 => (10,000 - 30) / 10  = 997
        let amount_in_with_fee = amount_in * U256::from(fee);
        let numerator = amount_in_with_fee * reserve_out;
        let denominator = reserve_in * U256::from(1000) + amount_in_with_fee;
        numerator / denominator
    }

    pub fn get_token_out(&self, token_in: H160) -> H160 {
        if self.token_a == token_in {
            self.token_b
        } else {
            self.token_a
        }
    }

    pub async fn get_token_decimals<M: Middleware>(&mut self, middleware: Arc<M>) -> (u8, u8) {
        let token_a_decimals = IErc20::new(self.token_a, middleware.clone())
            .decimals()
            .call()
            .await
            .unwrap();

        let token_b_decimals = IErc20::new(self.token_b, middleware)
            .decimals()
            .call()
            .await
            .unwrap();

        (token_a_decimals, token_b_decimals)
    }

    pub fn get_reserve_for_token(&self, token: H160) -> u128 {
        if self.token_a == token {
            return self.reserve_0;
        } else if self.token_b == token {
            return self.reserve_1;
        }
        0
    }
}

#[cfg(test)]
mod tests;
