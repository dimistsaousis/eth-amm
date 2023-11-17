use std::str::FromStr;

use crate::{
    address_book::AddressBook,
    amm::uniswap_v2::{factory::UniswapV2Factory, pool::UniswapV2Pool},
    checkpoint::Checkpoint,
    eth_provider::EthProvider,
};
use ethers::types::H160;
use itertools::Itertools;
use rand::{seq::SliceRandom, thread_rng};

pub struct Fixtures {
    pub alchemy_provider: EthProvider,
    pub local_provider: EthProvider,
    pub book: AddressBook,
    pub uniswap_v2_factory: UniswapV2Factory,
    pub pools: Checkpoint<Vec<UniswapV2Pool>>,
    pub weth_usdc_uniswap_v2_pool: UniswapV2Pool,
    pub weth_link_matic_weth_path: Vec<H160>,
    pub local_node_account: Account,
}

pub struct Account {
    pub address: H160,
    pub private_key: String,
}

impl Fixtures {
    pub async fn new() -> Fixtures {
        dotenv::dotenv().ok();
        let alchemy_provider = EthProvider::new_alchemy().await;
        let local_provider = EthProvider::new_local().await;
        let book = AddressBook::new();
        let uniswap_v2_factory = UniswapV2Factory::new(book.mainnet.uniswap_v2.factory, 300);
        let pools =
            Checkpoint::<Vec<UniswapV2Pool>>::get(&alchemy_provider, &uniswap_v2_factory, 100)
                .await;
        let weth_usdc_uniswap_v2_pool = UniswapV2Pool::from_address(
            alchemy_provider.http.clone(),
            book.mainnet.uniswap_v2.pairs["weth"]["usdc"],
            300,
        )
        .await;
        let local_node_account = Account {
            address: H160::from_str("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap(),
            private_key: "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
                .to_string(),
        };
        let weth_link_matic_weth_path = vec![
            book.mainnet.erc20["weth"],
            book.mainnet.erc20["link"],
            book.mainnet.erc20["matic"],
            book.mainnet.erc20["weth"],
        ];
        Fixtures {
            alchemy_provider,
            local_provider,
            book,
            uniswap_v2_factory,
            pools,
            weth_usdc_uniswap_v2_pool,
            weth_link_matic_weth_path,
            local_node_account,
        }
    }

    pub fn random_pools(&self, size: usize) -> Vec<&UniswapV2Pool> {
        let mut rng = thread_rng();
        self.pools
            .data
            .iter()
            .filter(|p| p.token_a_decimals > 0 && p.token_b_decimals > 0 && p.reserve_0 > 0)
            .collect_vec()
            .choose_multiple(&mut rng, size)
            .cloned()
            .collect()
    }

    pub fn assert_almost_equal(v1: f64, v2: f64, epsilon: f64) {
        assert!((v1 / v2 - 1f64).abs() < epsilon);
    }
}
