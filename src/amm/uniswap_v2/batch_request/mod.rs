use ethers::abi::Token;
use ethers::types::H160;

use crate::amm::UniswapV2Pool;

fn get_pool_from_tokens(tokens: Vec<Token>, address: H160, fee: u32) -> UniswapV2Pool {
    UniswapV2Pool::new(
        address,
        tokens[0]
            .to_owned()
            .into_address()
            .expect("Could not parse token A for UniswapV2 pool"),
        tokens[1]
            .to_owned()
            .into_uint()
            .expect("Could not parse token A decimals for UniswapV2 pool")
            .as_u32() as u8,
        tokens[2]
            .to_owned()
            .into_address()
            .expect("Could not parse token B for UniswapV2 pool"),
        tokens[3]
            .to_owned()
            .into_uint()
            .expect("Could not parse token B decimals for UniswapV2 pool")
            .as_u32() as u8,
        tokens[4]
            .to_owned()
            .into_uint()
            .expect("Could not parse reserve 0 for UniswapV2 pool")
            .as_u128(),
        tokens[5]
            .to_owned()
            .into_uint()
            .expect("Could not parse reserve 1 for UniswapV2 pool")
            .as_u128(),
        fee,
    )
}
