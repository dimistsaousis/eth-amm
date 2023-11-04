use ethers::abi::Token;
use ethers::types::H160;

use crate::amm::UniswapV2Pool;

fn get_pool_from_tokens(tokens: Vec<Token>, address: H160, fee: u32) -> Option<UniswapV2Pool> {
    Some(UniswapV2Pool::new(
        address,
        tokens[0].to_owned().into_address()?,
        tokens[1].to_owned().into_uint()?.as_u32() as u8,
        tokens[2].to_owned().into_address()?,
        tokens[3].to_owned().into_uint()?.as_u32() as u8,
        tokens[4].to_owned().into_uint()?.as_u128(),
        tokens[5].to_owned().into_uint()?.as_u128(),
        fee,
    ))
}
