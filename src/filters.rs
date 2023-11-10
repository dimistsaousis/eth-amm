use ethers::types::{H160, U256};
use itertools::Itertools;
use std::collections::HashMap;

pub fn filter_pools_for_eth_value(
    pools: Vec<H160>,
    eth_value_in_pools: &HashMap<H160, U256>,
    value: U256,
) -> Vec<H160> {
    pools
        .into_iter()
        .filter(|p| eth_value_in_pools.get(&p).unwrap_or(&U256::zero()) > &value)
        .collect_vec()
}
