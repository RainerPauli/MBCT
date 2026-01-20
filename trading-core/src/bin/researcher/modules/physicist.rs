// E:\MBCT\trading-core\src\bin\researcher\modules\physicist.rs
// THE ALLIANCE - MBCT Physicist Modul
// Fokus: Thermodynamische Transformation (Entropy, Pressure, NRG)

use serde::{Serialize, Deserialize};
use trading_core::exchange::L2Snapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsState {
    pub price: f64,
    pub spread: f64,
    pub entropy: f64,
    pub pressure: f64,
    pub temperature: f64,
    pub nrg: f64,
    pub total_volume: f64,
    pub bid_volume: f64,
    pub ask_volume: f64,
    pub timestamp: i64,
}

pub struct Physicist;

impl Physicist {
    /// Transformiert einen L2Snapshot in einen thermodynamischen PhysicsState
    pub fn process_snapshot(snapshot: &L2Snapshot) -> PhysicsState {
        let (bid_vol, ask_vol) = Self::calculate_volumes(snapshot);
        let entropy = Self::calculate_entropy(snapshot);
        let pressure = Self::calculate_pressure(bid_vol, ask_vol);
        
        // Die Felder px und sz sind Strings in der neuen Library
        let mid_price = if !snapshot.levels[0].is_empty() && !snapshot.levels[1].is_empty() {
            let best_bid = snapshot.levels[0][0].px.parse::<f64>().unwrap_or(0.0);
            let best_ask = snapshot.levels[1][0].px.parse::<f64>().unwrap_or(0.0);
            (best_bid + best_ask) / 2.0
        } else {
            0.0
        };

        let spread = if mid_price > 0.0 {
            let best_bid = snapshot.levels[0][0].px.parse::<f64>().unwrap_or(0.0);
            let best_ask = snapshot.levels[1][0].px.parse::<f64>().unwrap_or(0.0);
            (best_ask - best_bid) / mid_price
        } else {
            0.0
        };

        // UNSERE ALLIANZ-FORMEL: PEP = pi * E * phi
        let nrg = std::f64::consts::PI * entropy * pressure.abs().ln_1p();

        PhysicsState {
            price: mid_price,
            spread,
            entropy,
            pressure,
            temperature: mid_price, 
            nrg,
            total_volume: bid_vol + ask_vol,
            bid_volume: bid_vol,
            ask_volume: ask_vol,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        }
    }

    /// Berechnet die Shannon-Entropie basierend auf der Volumenverteilung im Orderbuch
    fn calculate_entropy(snapshot: &L2Snapshot) -> f64 {
        let mut total_vol = 0.0;
        let mut probabilities = Vec::new();

        for level in snapshot.levels[0].iter().chain(snapshot.levels[1].iter()) {
            let vol = level.sz.parse::<f64>().unwrap_or(0.0); 
            total_vol += vol;
            probabilities.push(vol);
        }

        if total_vol == 0.0 { return 0.0; }

        probabilities.iter()
            .map(|v| v / total_vol)
            .filter(|p| *p > 0.0)
            .map(|p| -p * p.ln())
            .sum()
    }

    /// Berechnet den Orderbuch-Druck (Imbalance)
    fn calculate_pressure(bid_vol: f64, ask_vol: f64) -> f64 {
        if bid_vol + ask_vol == 0.0 { return 0.0; }
        (bid_vol - ask_vol) / (bid_vol + ask_vol) * 100.0
    }

    /// Extrahiert kumulierte Volumina aus den ersten Ebenen
    fn calculate_volumes(snapshot: &L2Snapshot) -> (f64, f64) {
        let bid_vol: f64 = snapshot.levels[0].iter()
            .take(10) 
            .map(|l| l.sz.parse::<f64>().unwrap_or(0.0))
            .sum();
        
        let ask_vol: f64 = snapshot.levels[1].iter()
            .take(10)
            .map(|l| l.sz.parse::<f64>().unwrap_or(0.0))
            .sum();

        (bid_vol, ask_vol)
    }
}