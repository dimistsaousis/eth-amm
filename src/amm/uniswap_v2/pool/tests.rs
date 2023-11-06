use super::*;
use ethers::providers::{Http, Provider};
struct SetupResult(UniswapV2Pool, Arc<Provider<Http>>);

// fn setup() -> SetupResult {
//     // Create and return the necessary test
//     dotenv::dotenv().ok();
//     let address: H160 = H160::from_str("0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc").unwrap();
//     /
//     let rpc_endpoint = std::env::var("NETWORK_RPC").unwrap();
//     let middleware = Arc::new(Provider::<Http>::try_from(rpc_endpoint).unwrap());

//     SetupResult(factory, middleware)
// }
