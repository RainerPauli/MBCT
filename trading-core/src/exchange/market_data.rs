// E:\MBCT\trading-core\src\exchange\market_data.rs
// ====
// THE ALLIANCE - Market Data Analysis v4.1
// ====

use super::types::{L2Snapshot, MarketState};
use rust_decimal::prelude::FromPrimitive; // Ermöglicht Decimal::from_f64()
use rust_decimal::Decimal;
use std::str::FromStr;

pub struct HyperliquidMarketData {}

impl HyperliquidMarketData {
    pub fn new() -> Self {
        Self {}
    }

    /// Adaptive Physics: Wandelt Orderbuch-Snapshots in thermodynamische Zustände um
    pub fn derive_market_state(&self, snapshot: &L2Snapshot) -> MarketState {
        let symbol = snapshot.coin.clone();

        // 1. Temperatur (T) = Mid-Price
        let best_bid = snapshot
            .levels
            .bids
            .first()
            .and_then(|l| Decimal::from_str(&l.px).ok())
            .unwrap_or(Decimal::ZERO);

        let best_ask = snapshot
            .levels
            .asks
            .first()
            .and_then(|l| Decimal::from_str(&l.px).ok())
            .unwrap_or(Decimal::ZERO);

        let temperature = if best_bid > Decimal::ZERO && best_ask > Decimal::ZERO {
            (best_bid + best_ask) / Decimal::from(2)
        } else {
            Decimal::ZERO
        };

        // 2. Volumen/Spread (V)
        let volume_spread = if temperature > Decimal::ZERO {
            (best_ask - best_bid) / temperature
        } else {
            Decimal::ZERO
        };

        // 3. Druck (P) = Gewichtete Liquidität der ersten 5 Ebenen
        let mut pressure = Decimal::ZERO;

        // Bids gewichten
        for (j, level) in snapshot.levels.bids.iter().take(5).enumerate() {
            if let Ok(sz) = Decimal::from_str(&level.sz) {
                let weight = Decimal::from_f64(1.0 - (j as f64 * 0.1)).unwrap_or(Decimal::ONE);
                pressure += sz * weight;
            }
        }

        // Asks gewichten
        for (j, level) in snapshot.levels.asks.iter().take(5).enumerate() {
            if let Ok(sz) = Decimal::from_str(&level.sz) {
                let weight = Decimal::from_f64(1.0 - (j as f64 * 0.1)).unwrap_or(Decimal::ONE);
                pressure += sz * weight;
            }
        }

        // 4. Entropie (S) berechnen
        let mut all_levels = Vec::new();
        for level in snapshot
            .levels
            .bids
            .iter()
            .chain(snapshot.levels.asks.iter())
        {
            if let (Ok(px), Ok(sz)) = (Decimal::from_str(&level.px), Decimal::from_str(&level.sz)) {
                all_levels.push((px, sz));
            }
        }

        let entropy_raw = MarketState::calculate_entropy(&all_levels);
        let entropy_dec = Decimal::from_f64(entropy_raw);

        MarketState {
            symbol,
            temperature,
            pressure,
            volume_spread,
            entropy: entropy_dec,
        }
    }
}
