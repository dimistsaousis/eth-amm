use std::sync::Arc;

use ethers::providers::{Http, Middleware, Provider, Ws};

pub struct EthProvider {
    pub http: Arc<Provider<Http>>,
    pub wss: Arc<Provider<Ws>>,
}

impl EthProvider {
    pub async fn new() -> EthProvider {
        let rpc_endpoint = std::env::var("NETWORK_RPC").expect("Could not load env `NETWORK_RPC`");
        let http = Arc::new(Provider::<Http>::try_from(rpc_endpoint).unwrap());

        let wss_endpoint = std::env::var("NETWORK_WSS").expect("Could not load env `NETWORK_WSS`");
        let wss = Arc::new(
            Provider::<Ws>::connect(wss_endpoint)
                .await
                .expect("Could not connect to web socket."),
        );
        EthProvider { http, wss }
    }

    pub async fn get_block_number(&self) -> u64 {
        self.http
            .get_block_number()
            .await
            .expect("Could not get block number from provider.")
            .as_u64()
    }
}
