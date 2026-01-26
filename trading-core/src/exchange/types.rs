// E:\MBCT\trading-core\src\exchange\types.rs
// ====
// THE ALLIANCE - Core Types v4.1 (Resilient Trait Edition)
// ====

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize}; // Ermöglicht .to_f64() für Entropie

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct L2Snapshot {
    pub coin: String,
    pub time: u64,
    pub levels: L2Levels,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Levels {
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Level {
    pub px: String,
    pub sz: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketState {
    pub symbol: String,
    pub temperature: Decimal,
    pub pressure: Decimal,
    pub volume_spread: Decimal,
    pub entropy: Option<Decimal>,
}

impl MarketState {
    /// Berechnet die Shannon-Entropie der Liquiditätsverteilung
    pub fn calculate_entropy(levels: &[(Decimal, Decimal)]) -> f64 {
        let total_sz: Decimal = levels.iter().map(|(_, sz)| sz).sum();
        if total_sz == Decimal::ZERO {
            return 0.0;
        }

        let mut entropy = 0.0;
        for (_, sz) in levels {
            // Hier wird ToPrimitive benötigt
            let p = (sz / total_sz).to_f64().unwrap_or(0.0);
            if p > 0.0 {
                entropy -= p * p.ln();
            }
        }
        entropy
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub name: String,
    pub index: u32,
    pub sz_decimals: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub coin: String,
    pub side: String,
    pub px: Decimal,
    pub sz: Decimal,
    pub hash: String,
    pub time: u64,
}
