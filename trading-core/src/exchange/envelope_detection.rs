// E:\mbct\trading-core\src\exchange\envelope_detection.rs

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use trading_common::data::types::MarketState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketRegime {
    CompressionExpansion = 1, // Regime 1: Der Schrei
    Oscillatory = 2,          // Regime 2: Die Atmung (Unser Habitat)
    BallisticDrift = 3,       // Regime 3: Die Trägheit
    Unknown = 0,
}

impl MarketRegime {
    pub fn as_str(&self) -> &'static str {
        match self {
            MarketRegime::CompressionExpansion => "COMPRESSION",
            MarketRegime::Oscillatory => "OSCILLATORY",
            MarketRegime::BallisticDrift => "BALLISTIC",
            MarketRegime::Unknown => "UNKNOWN",
        }
    }
}

pub struct TimeMetrics {
    pub start_time: u64,
    pub crossover_count: u32, // Wie oft kreuzt der Preis den EMA/Mid
}

pub struct EnvelopeDetector {
    // Historie zur Berechnung von Velocity und Reversion
    pub window_size: usize,
}

impl EnvelopeDetector {
    pub fn new(window_size: usize) -> Self {
        Self { window_size }
    }

    pub fn classify(&self, state: &MarketState, history: &[MarketState]) -> MarketRegime {
        // 1. Velocity-Check (dV/dt) -> Indikator für Schreie
        // 2. Entropy-Check (S) -> Indikator für Atmung
        // 3. Directional Persistence -> Indikator für Trägheit

        let entropy_val = state
            .entropy_level
            .unwrap_or(Decimal::MAX)
            .to_f64()
            .unwrap_or(10.0);
        let threshold = Decimal::new(50, 0); // Beispiel-Threshold für Druck

        // Logik-Kern:
        if entropy_val < 0.5 && state.pressure > threshold {
            MarketRegime::CompressionExpansion
        } else if self.is_oscillating(history) {
            MarketRegime::Oscillatory
        } else {
            MarketRegime::BallisticDrift
        }
    }

    fn is_oscillating(&self, history: &[MarketState]) -> bool {
        if history.len() < 5 {
            return false;
        }

        let mut sign_changes = 0;
        for i in 1..history.len() {
            let prev_p = history[i - 1].pressure;
            let curr_p = history[i].pressure;

            // Wenn Druck um 0 oszilliert
            if (prev_p > Decimal::ZERO && curr_p < Decimal::ZERO)
                || (prev_p < Decimal::ZERO && curr_p > Decimal::ZERO)
            {
                sign_changes += 1;
            }
        }

        sign_changes >= 2
    }
}
