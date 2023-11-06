use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
pub struct Checkpoint<T> {
    pub last_block: u64,
    pub data: T,
}

impl<T: for<'a> Deserialize<'a> + Serialize> Checkpoint<T> {
    pub fn new(last_block: u64, data: T) -> Self {
        Checkpoint { last_block, data }
    }

    fn path(loc: &str) -> String {
        format!("src/checkpoint/data/{}", loc)
    }

    pub fn load_data(loc: &str) -> Option<Self> {
        match fs::read_to_string(Self::path(loc))
            .map_err(|e| e.to_string())
            .and_then(|data| serde_json::from_str(&data).map_err(|e| e.to_string()))
        {
            Ok(checkpoint) => checkpoint,
            Err(_) => None,
        }
    }

    pub fn save_data(&self, loc: &str) {
        let serialized = serde_json::to_string(self).unwrap();
        fs::write(Self::path(loc), serialized).unwrap();
    }
}
