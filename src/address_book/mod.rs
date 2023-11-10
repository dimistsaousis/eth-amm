use ethers::types::H160;
use serde::Deserialize;
use serde_yaml;
use std::{collections::HashMap, fs};

#[derive(Debug, Deserialize)]
pub struct AddressBook {
    pub mainnet: Network,
}

#[derive(Debug, Deserialize)]
pub struct Network {
    pub erc20: HashMap<String, H160>,
    pub uniswap_v2: UniswapV2,
}

#[derive(Debug, Deserialize)]
pub struct UniswapV2 {
    pub factory: H160,
    pub pairs: HashMap<String, HashMap<String, H160>>,
}

impl UniswapV2 {
    fn add_inverse_pairs(&mut self) {
        let mut inverse_pairs = HashMap::new();

        for (key, sub_pairs) in &self.pairs {
            for (sub_key, address) in sub_pairs {
                inverse_pairs
                    .entry(sub_key.clone())
                    .or_insert_with(HashMap::new)
                    .insert(key.clone(), address.clone());
            }
        }

        for (key, sub_pairs) in inverse_pairs {
            self.pairs
                .entry(key)
                .and_modify(|e| e.extend(sub_pairs.clone()))
                .or_insert(sub_pairs);
        }
    }
}

impl AddressBook {
    pub fn new() -> Self {
        let data: String = fs::read_to_string("src/address_book/address_book.yaml")
            .expect("Could not read address_book.yaml");
        let mut address_book: AddressBook = serde_yaml::from_str(&data).unwrap();
        address_book.mainnet.uniswap_v2.add_inverse_pairs();
        address_book
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[tokio::test]
    async fn test_new_address_book() {
        let book = AddressBook::new();
        assert_eq!(
            book.mainnet.erc20["weth"],
            H160::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap()
        );
        assert_eq!(
            book.mainnet.uniswap_v2.factory,
            H160::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f").unwrap()
        );
        assert_eq!(
            book.mainnet.uniswap_v2.pairs["weth"]["usdc"],
            book.mainnet.uniswap_v2.pairs["usdc"]["weth"]
        );
    }
}
