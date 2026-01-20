// E:\mbct\trading_core\src\exchange\market_data.rs

use rust_decimal::Decimal;
use std::str::FromStr;

use super::types::{MarketState, L2Snapshot};

pub struct HyperliquidMarketData {}

impl HyperliquidMarketData {
    pub fn new() -> Self {
        Self {}
    }

    /// Adaptive Physics: Berechnet den thermodynamischen Zustand mit dynamischen Schwellenwerten
    pub fn derive_market_state(&self, snapshot: &L2Snapshot) -> MarketState {
        let symbol = snapshot.coin.clone();
        
        // 1. Temperatur (T) = Mid-Price
        let best_bid = if !snapshot.levels[0].is_empty() {
             Decimal::from_str(&snapshot.levels[0][0].px).unwrap_or(Decimal::ZERO)
        } else {
            Decimal::ZERO
        };
        
        let best_ask = if !snapshot.levels[1].is_empty() {
            Decimal::from_str(&snapshot.levels[1][0].px).unwrap_or(Decimal::ZERO)
        } else {
            Decimal::ZERO
        };
        
        let temperature = if best_bid > Decimal::ZERO && best_ask > Decimal::ZERO {
            (best_bid + best_ask) / Decimal::from(2)
        } else {
            Decimal::ZERO
        };

        // 2. Volumen (V) = Spread
        let volume_spread = if temperature > Decimal::ZERO {
            (best_ask - best_bid) / temperature
        } else {
            Decimal::ZERO
        };

        // 3. Druck (P) = Adaptive Liquiditätsmessung
        // Wir gewichten die Liquidität näher am Mid-Price stärker (Erdbeben vs Rippel)
        let mut pressure = Decimal::ZERO;
        for side in snapshot.levels.iter() {
            for (j, level) in side.iter().take(5).enumerate() {
                let sz = Decimal::from_str(&level.sz).unwrap_or(Decimal::ZERO);
                // Gewichtung: 1.0, 0.8, 0.6, 0.4, 0.2
                let weight = Decimal::from_str(&format!("{:.1}", 1.0 - (j as f64 * 0.2))).unwrap_or(Decimal::ONE);
                pressure += sz * weight;
            }
        }

        // 4. Entropie (S) = Shannon Entropy der Liquiditätsverteilung
        let mut all_levels = Vec::new();
        for side in snapshot.levels.iter() {
            for level in side.iter() {
                let px = Decimal::from_str(&level.px).unwrap_or(Decimal::ZERO);
                let sz = Decimal::from_str(&level.sz).unwrap_or(Decimal::ZERO);
                all_levels.push((px, sz));
            }
        }
        let entropy = MarketState::calculate_entropy(&all_levels);
        let entropy_level = Some(Decimal::from_str(&format!("{:.4}", entropy)).unwrap_or(Decimal::ZERO));

        MarketState {
            symbol,
            temperature,
            pressure,
            volume_spread,
            entropy_level,
            timestamp: chrono::Utc::now().timestamp_millis(),
            regime: None,
        }
    }
}
