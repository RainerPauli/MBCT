// E:\mbct\trading_core\src\exchange\types.rs

use serde::{Deserialize, Serialize};


// Re-export MarketState from common
pub use trading_common::data::types::MarketState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Snapshot {
    pub coin: String,
    pub levels: Vec<Vec<LevelData>>, // [bids, asks]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelData {
    pub px: String,
    pub sz: String,
    pub n: u32,
}
