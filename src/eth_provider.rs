use std::{error::Error, sync::Arc};

use ethers::{
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider, Ws},
    signers::LocalWallet,
    types::{H160, U256},
};
use lazy_static::lazy_static;
use serde_json::json;
use std::sync::Mutex;

lazy_static! {
    pub static ref LOCAL_NODE_TEST_MUTEX: Mutex<()> = Mutex::new(());
}

pub struct EthProvider {
    pub http: Arc<Provider<Http>>,
    pub http_endpoint: String,
    pub wss_endpoint: String,
}

impl EthProvider {
    async fn new(http_endpoint: String, wss_endpoint: String) -> EthProvider {
        let http = Arc::new(Provider::<Http>::try_from(&http_endpoint).unwrap());
        EthProvider {
            http,
            http_endpoint,
            wss_endpoint,
        }
    }

    pub fn alchemy_rpc() -> String {
        std::env::var("ALCHEMY_RPC").expect("Could not load env `ALCHEMY_RPC`")
    }

    pub fn alchemy_wss() -> String {
        std::env::var("ALCHEMY_WSS").expect("Could not load env `ALCHEMY_WSS`")
    }

    pub async fn new_alchemy() -> EthProvider {
        Self::new(Self::alchemy_rpc(), Self::alchemy_wss()).await
    }

    pub async fn new_local() -> EthProvider {
        let rpc_endpoint = "http://localhost:8545".to_string();
        let wss_endpoint = "wss://localhost:8545".to_string();
        Self::new(rpc_endpoint, wss_endpoint).await
    }

    pub async fn reset_local_to_alchemy_fork(&self) -> Result<(), Box<dyn Error>> {
        let client = reqwest::Client::new();
        let reset_payload = json!({
            "jsonrpc": "2.0",
            "method": "hardhat_reset",
            "params": [{
                "forking": {
                    "jsonRpcUrl": Self::alchemy_rpc(),
                }
            }],
            "id": 1
        });
        client
            .post(self.http.url().to_string())
            .json(&reset_payload)
            .send()
            .await?;
        Ok(())
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

    pub async fn get_wss(&self) -> Arc<Provider<Ws>> {
        Arc::new(
            Provider::<Ws>::connect(&self.wss_endpoint)
                .await
                .expect("Could not connect to web socket."),
        )
    }

    pub async fn get_signer_middleware(
        &self,
        private_key: &str,
    ) -> Arc<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>> {
        let wallet = private_key
            .parse::<LocalWallet>()
            .expect("Could not parse private key.");
        Arc::new(
            SignerMiddleware::new_with_provider_chain(self.http.clone(), wallet)
                .await
                .unwrap(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reset_local_to_alchemy_fork() {
        let _guard = LOCAL_NODE_TEST_MUTEX.lock().unwrap();
        let provider = EthProvider::new_local().await;
        provider.reset_local_to_alchemy_fork().await.unwrap();
    }
}
