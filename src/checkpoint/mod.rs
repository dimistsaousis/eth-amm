use ethers::types::H160;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
pub enum CheckpointData {
    H160Data(H160),
}

#[derive(Serialize, Deserialize)]
pub struct Checkpoint {
    last_block: u64,
    data: CheckpointData,
}

impl Checkpoint {
    pub fn new(last_block: u64, data: CheckpointData) -> Self {
        Checkpoint { last_block, data }
    }

    pub fn load_data(loc: &str) -> Self {
        let data = fs::read_to_string(loc).unwrap();
        let checkpoint: Checkpoint = serde_json::from_str(&data).unwrap();
        checkpoint
    }

    pub fn save_data(&self, loc: &str) {
        let serialized = serde_json::to_string(self).unwrap();
        fs::write(loc, serialized).unwrap();
    }
}
