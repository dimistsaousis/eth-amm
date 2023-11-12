use std::sync::Arc;

use ethers::{
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
    async fn new(rpc_endpoint: &str, wss_endpoint: &str) -> EthProvider {
        let http = Arc::new(Provider::<Http>::try_from(rpc_endpoint).unwrap());
        let wss = Arc::new(
            Provider::<Ws>::connect(wss_endpoint)
                .await
                .expect("Could not connect to web socket."),
        );
        EthProvider { http, wss }
    }

    pub async fn new_alchemy() -> EthProvider {
        let rpc_endpoint = std::env::var("ALCHEMY_RPC").expect("Could not load env `ALCHEMY_RPC`");
        let wss_endpoint = std::env::var("ALCHEMY_WSS").expect("Could not load env `ALCHEMY_WSS`");
        Self::new(&rpc_endpoint, &wss_endpoint).await
    }

    pub async fn new_ganache() -> EthProvider {
        let rpc_endpoint = "http://localhost:8545";
        let wss_endpoint = "wss://localhost:8545";
        Self::new(&rpc_endpoint, &wss_endpoint).await
    }

    pub fn clone(self) -> Arc<EthProvider> {
        let provider = Arc::new(self);
        provider.clone()
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
