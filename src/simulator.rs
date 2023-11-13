use crate::{
    amm::uniswap_v2::pool::UniswapV2Pool,
    contract::{IErc20, IUniswapRouter, SimulatorV1, SwapParams},
    middleware::EthProvider,
};
use csv::Writer;
use ethers::{
    abi::{ParamType, Token},
    providers::Middleware,
    types::{Bytes, H160, U256},
};
use eyre::Result;
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
pub struct Simulation {
    pub token: H160,
    pub path: Vec<UniswapV2Pool>,
    pub amount_in: U256,
    pub amount_out: U256,
    pub amount_path: Vec<U256>,
    pub epsilon: U256,
}

pub fn write_simulations_to_csv(simulations: Vec<Simulation>, file_path: &str) {
    let mut wtr = Writer::from_path(file_path).unwrap();
    wtr.write_record(&[
        "token",
        "path",
        "amount_in",
        "amount_out",
        "amount_path",
        "profit",
    ])
    .unwrap();
    for sim in simulations {
        let token = format!("{:?}", sim.token);
        let path = sim
            .path
            .iter()
            .map(|p| format!("{:?}", p.address))
            .collect::<Vec<_>>()
            .join(", ");
        let amount_in = sim.amount_in.to_string();
        let amount_out = sim.amount_out.to_string();
        let amount_path = format!("{:?}", sim.amount_path);
        let profit = sim.profit().to_string();
        wtr.write_record(&[token, path, amount_in, amount_out, amount_path, profit])
            .unwrap();
    }
    wtr.flush().unwrap();
}

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

impl Simulation {
    pub fn new(token: H160, path: Vec<UniswapV2Pool>, epsilon: U256) -> Self {
        let mut simulation = Simulation {
            token,
            path,
            amount_in: U256::zero(),
            amount_out: U256::zero(),
            amount_path: vec![U256::zero()],
            epsilon,
        };
        simulation.get_best_amount();
        simulation
    }

    pub fn profit(&self) -> U256 {
        if self.amount_in < self.amount_out {
            return self.amount_out - self.amount_in;
        }
        U256::zero()
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

    pub async fn swap_using_router(
        &self,
        router_address: H160,
        provider: EthProvider,
        public_key: H160,
        private_key: &str,
    ) -> Result<U256> {
        let erc20_path = self.get_erc20_path();
        let last_token = IErc20::new(erc20_path.last().unwrap().clone(), provider.http.clone());
        let current_balance = last_token.balance_of(public_key).await?;
        let router = IUniswapRouter::new(
            router_address,
            provider.get_signer_middleware(private_key).await,
        );
        let amount_out_min = U256::zero();
        let deadline = U256::from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs()
                + 10,
        );
        router
            .swap_exact_eth_for_tokens(amount_out_min, erc20_path, public_key, deadline)
            .value(self.amount_in)
            .send()
            .await?;
        Ok(last_token.balance_of(public_key).await? - current_balance)
    }

    pub fn get_erc20_path(&self) -> Vec<H160> {
        let mut token = self.token;
        let mut tokens = vec![];
        for pair in &self.path {
            tokens.push(token.clone());
            token = pair.get_token_out(&token);
        }
        tokens
    }

    pub fn simulate_swap_offline(&self, amount: U256) -> (U256, Vec<U256>) {
        let mut token = self.token;
        let mut amount = amount;
        let mut amounts = vec![amount];
        for pool in &self.path {
            amount = pool.simulate_swap(token, amount);
            token = pool.get_token_out(&token);
            amounts.push(amount);
        }
        (amount, amounts)
    }

    pub fn get_best_amount(&mut self) {
        let f = |amount: f64| {
            let (amount_out, _) = self.simulate_swap_offline(U256::from(amount as u128));
            amount_out.as_u128() as f64 - amount
        };
        let (amount, _) =
            Self::find_local_maximum(0.0, 10f64.powf(20.0), self.epsilon.as_u128() as f64, f);
        let amount = U256::from(amount as u128);
        self.amount_in = amount;
        (self.amount_out, self.amount_path) = self.simulate_swap_offline(amount);
    }

    pub async fn simulate_swap<M: Middleware>(&self, middleware: Arc<M>, amount: U256) -> U256 {
        let mut token_in = self.token;
        let mut token_out;
        let mut params = vec![];

        for pool in &self.path {
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

    pub fn reversed(&mut self) {
        self.path.reverse();
        self.get_best_amount();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        address_book::AddressBook, amm::uniswap_v2::factory::UniswapV2Factory,
        middleware::EthProvider,
    };

    struct SetupResult(EthProvider, Simulation, AddressBook);

    async fn setup() -> SetupResult {
        // Create and return the necessary test
        dotenv::dotenv().ok();
        let provider = EthProvider::new_alchemy().await;
        let book = AddressBook::new();
        let factory: UniswapV2Factory = UniswapV2Factory::new(book.mainnet.uniswap_v2.factory, 300);
        let weth_usdc = factory
            .get_pair_address(
                provider.http.clone(),
                book.mainnet.erc20["weth"],
                book.mainnet.erc20["usdc"],
            )
            .await;
        let usdc_matic = factory
            .get_pair_address(
                provider.http.clone(),
                book.mainnet.erc20["matic"],
                book.mainnet.erc20["usdc"],
            )
            .await;
        let matic_weth = factory
            .get_pair_address(
                provider.http.clone(),
                book.mainnet.erc20["matic"],
                book.mainnet.erc20["weth"],
            )
            .await;
        let pools: Vec<UniswapV2Pool> = vec![
            UniswapV2Pool::from_address(provider.http.clone(), weth_usdc, 300).await,
            UniswapV2Pool::from_address(provider.http.clone(), usdc_matic, 300).await,
            UniswapV2Pool::from_address(provider.http.clone(), matic_weth, 300).await,
        ];
        println!("{:?} {:?} {:?}", weth_usdc, usdc_matic, matic_weth);
        let simulation = Simulation::new(book.mainnet.erc20["weth"], pools, U256::exp10(6));
        SetupResult(provider, simulation, book)
    }

    #[tokio::test]
    async fn test_simulate_swap() {
        let SetupResult(provider, simulation, book) = setup().await;
        let sim = Simulation::new(
            book.mainnet.erc20["weth"],
            vec![simulation.path.into_iter().next().unwrap()], // weth-usdc
            simulation.epsilon,
        );
        let res = sim.simulate_swap(provider.http, U256::exp10(18)).await;
        assert!(res > U256::exp10(6) * U256::from(1000));
        assert!(res < U256::exp10(6) * U256::from(2500));
    }

    #[tokio::test]
    async fn test_compare_simulate_swap_offline_and_online() {
        let SetupResult(provider, simulation, _) = setup().await;
        let r0 = simulation
            .simulate_swap(provider.http, U256::exp10(18))
            .await;
        let (r1, _) = simulation.simulate_swap_offline(U256::exp10(18));
        assert_eq!(r0, r1);
    }
    #[tokio::test]
    async fn test_find_best_amount_binary_search() {
        let SetupResult(_, mut simulation, _) = setup().await;
        assert!(simulation.amount_in < U256::exp10(15));
        assert!(simulation.amount_in > U256::exp10(14));
        assert!(simulation.amount_in < simulation.amount_out);
        simulation.reversed();
        assert!(simulation.amount_in > simulation.amount_out);
    }

    #[tokio::test]
    async fn test_simulate_swap_rogue() {
        let SetupResult(provider, _, book) = setup().await;
        let path = vec![
            UniswapV2Pool::from_address(
                provider.http.clone(),
                book.mainnet.uniswap_v2.pairs["weth"]["usdt"],
                300,
            )
            .await,
            UniswapV2Pool::from_address(
                provider.http.clone(),
                book.mainnet.uniswap_v2.pairs["usdt"]["usd_old"],
                300,
            )
            .await,
            UniswapV2Pool::from_address(
                provider.http.clone(),
                book.mainnet.uniswap_v2.pairs["usd_old"]["weth"],
                300,
            )
            .await,
        ];
        let sim = Simulation::new(book.mainnet.erc20["weth"], path, U256::exp10(14));
        let amount_out = sim
            .simulate_swap(provider.http.clone(), sim.amount_in)
            .await;
        assert_eq!(amount_out, sim.amount_out);
    }
}
