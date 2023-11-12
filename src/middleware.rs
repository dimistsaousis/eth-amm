use std::{str::FromStr, sync::Arc};

use ethers::{
    abi::Address,
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider, Ws},
    signers::LocalWallet,
    types::{H160, U256},
};

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

    pub async fn get_balance(&self, address: H160) -> U256 {
        self.http.get_balance(address, None).await.unwrap()
    }

    pub async fn get_block_number(&self) -> u64 {
        self.http
            .get_block_number()
            .await
            .expect("Could not get block number from provider.")
            .as_u64()
    }

    pub async fn get_chain_id(&self) -> u64 {
        self.http
            .get_chainid()
            .await
            .expect("Could not get chain id")
            .as_u64()
    }

    pub fn get_signer_middleware(
        &self,
        private_key: &str,
    ) -> Arc<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>> {
        let wallet = private_key
            .parse::<LocalWallet>()
            .expect("Could not parse private key.");
        Arc::new(SignerMiddleware::new(self.http.clone(), wallet))
    }
}
