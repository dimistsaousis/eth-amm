use std::sync::Arc;

use ethers::providers::{Http, Middleware, Provider};

pub struct EthProvider {
    pub provider: Arc<Provider<Http>>,
}

impl EthProvider {
    pub fn new() -> EthProvider {
        let rpc_endpoint = std::env::var("NETWORK_RPC").expect("Could not load env `NETWORK_RPC`");
        let provider = Arc::new(Provider::<Http>::try_from(rpc_endpoint).unwrap());
        EthProvider { provider }
    }

    pub async fn get_block_number(&self) -> u64 {
        self.provider
            .get_block_number()
            .await
            .expect("Could not get block number from provider.")
            .as_u64()
    }
}
