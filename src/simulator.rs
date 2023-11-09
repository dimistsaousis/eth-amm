use crate::contract::{SimulatorV1, SwapParams};
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

pub async fn simulate_swap<M: Middleware>(
    middleware: Arc<M>,
    token_in: H160,
    token_out: H160,
) -> U256 {
    let params = vec![SwapParams {
        protocol: 0,
        pool: H160::zero(),
        token_in: token_in,
        token_out: token_out,
        fee: 300,
        amount: U256::exp10(18),
    }];

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
    use crate::middleware::EthProvider;
    use std::str::FromStr;

    struct SetupResult(EthProvider, H160, H160);

    async fn setup() -> SetupResult {
        // Create and return the necessary test
        dotenv::dotenv().ok();
        let provider = EthProvider::new().await;
        let weth = H160::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap();
        let usdc = H160::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap();

        SetupResult(provider, weth, usdc)
    }

    #[tokio::test]
    async fn test_simulate_swap() {
        let SetupResult(provider, weth, usdc) = setup().await;
        let res = simulate_swap(provider.http, weth, usdc).await;
        println!("{:?}", res);
        assert!(false);
    }
}
