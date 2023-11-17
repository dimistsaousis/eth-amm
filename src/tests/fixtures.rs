use crate::{
    address_book::AddressBook,
    amm::uniswap_v2::{factory::UniswapV2Factory, pool::UniswapV2Pool},
    checkpoint::Checkpoint,
    eth_provider::EthProvider,
};
use itertools::Itertools;
use rand::{seq::SliceRandom, thread_rng};
use tokio::sync::OnceCell;

static TEST_FIXTURES: OnceCell<Fixtures> = OnceCell::const_new();

pub async fn setup() -> &'static Fixtures {
    TEST_FIXTURES.get_or_init(Fixtures::new).await
}

pub struct Fixtures {
    pub alchemy_provider: EthProvider,
    pub local_provider: EthProvider,
    pub book: AddressBook,
    pub uniswap_v2_factory: UniswapV2Factory,
    pub pools: Checkpoint<Vec<UniswapV2Pool>>,
}

impl Fixtures {
    async fn new() -> Fixtures {
        dotenv::dotenv().ok();
        let alchemy_provider = EthProvider::new_alchemy().await;
        let local_provider = EthProvider::new_local().await;
        let book = AddressBook::new();
        let uniswap_v2_factory = UniswapV2Factory::new(book.mainnet.uniswap_v2.factory, 300);
        let pools =
            Checkpoint::<Vec<UniswapV2Pool>>::get(&alchemy_provider, &uniswap_v2_factory, 100)
                .await;
        Fixtures {
            alchemy_provider,
            local_provider,
            book,
            uniswap_v2_factory,
            pools,
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
}
