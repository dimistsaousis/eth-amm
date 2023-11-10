use crate::{
    amm::uniswap_v2::pool::UniswapV2Pool,
    contract::{SimulatorV1, SwapParams},
};
use ethers::{
    abi::{ParamType, Token},
    providers::Middleware,
    types::{Bytes, H160, U256},
};
use std::sync::Arc;

impl SwapParams {
    pub fn to_tokens(&self) -> Token {
        Token::Tuple(vec![
            Token::Uint(U256::from(self.protocol)),
            Token::Address(self.pool),
            Token::Address(self.token_in),
            Token::Address(self.token_out),
            Token::Uint(U256::from(self.fee)),
            Token::Uint(self.amount),
        ])
    }

    pub fn to_constructor_args(params: Vec<Self>) -> Token {
        Token::Array(
            params
                .iter()
                .map(|param| param.to_tokens())
                .collect::<Vec<Token>>(),
        )
    }
}

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

pub fn find_best_amount(
    token: H160,
    path: Vec<UniswapV2Pool>, // Assuming UniswapV2Pool is a struct defined in your context
    epsilon: f64,
) -> (f64, usize) {
    let f = |amount: f64| {
        let amount_out = simulate_swap_offline(token, U256::from(amount as u128), &path);
        amount_out.as_u128() as f64 - amount
    };
    find_local_maximum(0.0, 10f64.powf(20.0), epsilon, f)
}

pub fn simulate_swap_offline(token: H160, amount: U256, path: &Vec<UniswapV2Pool>) -> U256 {
    let mut token = token;
    let mut amount = amount;
    for pool in path {
        amount = pool.simulate_swap(token, amount);
        token = pool.get_token_out(&token);
    }
    amount
}

pub async fn simulate_swap<M: Middleware>(
    middleware: Arc<M>,
    token: H160,
    amount: U256,
    path: &Vec<UniswapV2Pool>,
) -> U256 {
    let mut token_in = token;
    let mut token_out;
    let mut params = vec![];

    for pool in path {
        token_out = pool.get_token_out(&token_in);
        params.push(SwapParams {
            protocol: 0,
            pool: H160::zero(),
            token_in,
            token_out,
            fee: 300,
            amount,
        });
        token_in = token_out;
    }

    let deployer = SimulatorV1::deploy(middleware, SwapParams::to_constructor_args(params))
        .expect("Failed deployment");
    let return_data: Bytes = deployer.call_raw().await.expect("Could not call raw data");
    let return_data_tokens =
        ethers::abi::decode(&[ParamType::Uint(256)], &return_data).expect("Failed decoding");

    if let Some(Token::Uint(v)) = return_data_tokens.into_iter().next() {
        return v;
    }

    U256::zero()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{amm::uniswap_v2::factory::UniswapV2Factory, middleware::EthProvider};
    use std::str::FromStr;

    struct SetupResult(EthProvider, H160, H160, Vec<UniswapV2Pool>);

    async fn setup() -> SetupResult {
        // Create and return the necessary test
        dotenv::dotenv().ok();
        let provider = EthProvider::new().await;
        let weth = H160::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap();
        let usdc = H160::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap();
        let matic = H160::from_str("0x7D1AfA7B718fb893dB30A3aBc0Cfc608AaCfeBB0").unwrap();
        let factory_address = H160::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f").unwrap();
        let factory: UniswapV2Factory = UniswapV2Factory::new(factory_address, 300);
        let weth_usdc = factory
            .get_pair_address(provider.http.clone(), weth, usdc)
            .await;
        let usdc_matic = factory
            .get_pair_address(provider.http.clone(), matic, usdc)
            .await;
        let matic_weth = factory
            .get_pair_address(provider.http.clone(), matic, weth)
            .await;
        let pools: Vec<UniswapV2Pool> = vec![
            UniswapV2Pool::from_address(provider.http.clone(), weth_usdc, 300).await,
            UniswapV2Pool::from_address(provider.http.clone(), usdc_matic, 300).await,
            UniswapV2Pool::from_address(provider.http.clone(), matic_weth, 300).await,
        ];
        println!("{:?} {:?} {:?}", weth_usdc, usdc_matic, matic_weth);
        SetupResult(provider, weth, usdc, pools)
    }

    #[tokio::test]
    async fn test_simulate_swap() {
        let SetupResult(provider, weth, _, pools) = setup().await;
        let pool = pools.into_iter().next().unwrap();
        let res = simulate_swap(provider.http, weth, U256::exp10(18), &vec![pool]).await;
        assert!(res > U256::exp10(6) * U256::from(1000));
        assert!(res < U256::exp10(6) * U256::from(2500));
    }

    #[tokio::test]
    async fn test_compare_simulate_swap_offline_and_online() {
        let SetupResult(provider, weth, _, pools) = setup().await;
        let r0 = simulate_swap(provider.http, weth, U256::exp10(18), &pools).await;
        let r1 = simulate_swap_offline(weth, U256::exp10(18), &pools);
        assert_eq!(r0, r1);
    }
    #[tokio::test]
    async fn test_find_best_amount_binary_search() {
        let SetupResult(_, weth, _, pools) = setup().await;
        let (amount, steps) = find_best_amount(weth, pools, 1_000_000_000f64);
        assert!(amount < 10f64.powf(15.0));
        assert!(steps > 50);
    }
}
