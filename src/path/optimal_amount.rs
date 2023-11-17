use super::simulator::simulate_swap_using_pools;
use crate::amm::uniswap_v2::pool::UniswapV2Pool;
use ethers::types::{H160, U256};
use std::collections::HashMap;

fn find_local_maximum<F>(mut low: f64, mut high: f64, epsilon: f64, mut f: F) -> (f64, usize)
where
    F: FnMut(f64) -> f64,
{
    let mut step: usize = 0;

    while high - low > epsilon {
        let mid1 = low + (high - low) / 3.0;
        let mid2 = high - (high - low) / 3.0;

        if f(mid1) < f(mid2) {
            low = mid1;
        } else {
            high = mid2;
        }
        step += 1;
    }

    ((low + high) / 2.0, step)
}

pub fn find_optimal_amount_in(
    path: &Vec<H160>,
    pool_map: &HashMap<(&H160, &H160), &UniswapV2Pool>,
    epsilon: f64,
) -> U256 {
    let f = |amount_in: f64| {
        let amount_out = simulate_swap_using_pools(U256::from(amount_in as u128), path, pool_map);
        amount_out.as_u128() as f64 - amount_in
    };
    let (amount, _) = find_local_maximum(0.0, 10f64.powf(20.0), epsilon, f);
    U256::from(amount as u128)
}

pub fn find_optimal_amount_in_and_out(
    path: &Vec<H160>,
    pool_map: &HashMap<(&H160, &H160), &UniswapV2Pool>,
    epsilon: f64,
) -> (U256, U256) {
    let amount_in = find_optimal_amount_in(path, pool_map, epsilon);
    let amount_out = simulate_swap_using_pools(amount_in, path, pool_map);
    (amount_in, amount_out)
}

#[cfg(test)]
mod tests {
    use crate::tests::fixtures;

    use super::*;

    #[tokio::test]
    async fn test_find_optimal_amount_in() {
        let fixture = fixtures::Fixtures::new().await;
        let amount_in = find_optimal_amount_in(
            &fixture.weth_link_matic_weth_path,
            &fixture.pools.token_to_pool_map(),
            10f64.powf(4.0),
        );
        assert_ne!(amount_in, U256::zero());
        assert!(amount_in < U256::exp10(13));
    }
}
