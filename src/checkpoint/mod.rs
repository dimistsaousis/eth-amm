use ethers::types::H160;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
pub struct Checkpoint {
    pub last_block: u64,
    pub data: Vec<H160>,
}

impl Checkpoint {
    pub fn new(last_block: u64, data: Vec<H160>) -> Self {
        Checkpoint { last_block, data }
    }

    pub fn load_data(loc: &str) -> Option<Self> {
        match fs::read_to_string(loc)
            .map_err(|e| e.to_string())
            .and_then(|data| serde_json::from_str(&data).map_err(|e| e.to_string()))
        {
            Ok(checkpoint) => checkpoint,
            Err(_) => None,
        }
    }

    pub fn save_data(&self, loc: &str) {
        let serialized = serde_json::to_string(self).unwrap();
        fs::write(loc, serialized).unwrap();
    }
}
