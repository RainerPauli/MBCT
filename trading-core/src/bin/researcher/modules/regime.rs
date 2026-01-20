// E:\MBCT\trading-core\src\bin\researcher\modules\regime.rs
// THE ALLIANCE - MBCT Regime Modul

use crate::modules::physicist::PhysicsState;
use std::collections::VecDeque;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MarketRegime {
    Compression,
    Oscillatory,
    Ballistic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeState {
    pub regime: MarketRegime,
    pub symmetry_score: f64,
    pub slope: f64,
    pub reversion_speed: f64,
    pub confidence: f64,
}

pub struct RegimeClassifier {
    window_size: usize,
}

impl RegimeClassifier {
    pub fn new(window_size: usize) -> Self {
        Self { window_size }
    }

    pub fn classify(&self, history: &VecDeque<PhysicsState>) -> RegimeState {
        if history.len() < self.window_size {
            return RegimeState {
                regime: MarketRegime::Compression,
                symmetry_score: 0.5,
                slope: 0.0,
                reversion_speed: 0.0,
                confidence: 0.0,
            };
        }

        let prices: Vec<f64> = history.iter().map(|s| s.price).collect();
        let slope = self.calculate_slope(&prices);
        let symmetry = self.calculate_symmetry(&prices);
        let reversion = self.calculate_reversion_speed(&prices);

        let regime = if slope.abs() > 0.0005 && symmetry < 0.3 {
            MarketRegime::Ballistic
        } else if symmetry > 0.6 {
            MarketRegime::Oscillatory
        } else {
            MarketRegime::Compression
        };

        RegimeState {
            regime,
            symmetry_score: symmetry,
            slope,
            reversion_speed: reversion,
            confidence: 1.0 - (1.0 / (history.len() as f64)),
        }
    }

    fn calculate_slope(&self, data: &[f64]) -> f64 {
        let n = data.len() as f64;
        let x_mean = (n - 1.0) / 2.0;
        let y_mean: f64 = data.iter().sum::<f64>() / n;
        let (mut num, mut den) = (0.0, 0.0);
        for (i, &y) in data.iter().enumerate() {
            num += (i as f64 - x_mean) * (y - y_mean);
            den += (i as f64 - x_mean).powi(2);
        }
        if den == 0.0 { 0.0 } else { num / den }
    }

    fn calculate_symmetry(&self, data: &[f64]) -> f64 {
        let (mut ups, mut downs) = (0.0, 0.0);
        for w in data.windows(2) {
            let diff = w[1] - w[0];
            if diff > 0.0 { ups += diff; } else { downs += diff.abs(); }
        }
        if ups + downs == 0.0 { 1.0 } else { (ups.min(downs)) / (ups.max(downs)) }
    }

    fn calculate_reversion_speed(&self, data: &[f64]) -> f64 {
        let n = data.len() as f64;
        let mean: f64 = data.iter().sum::<f64>() / n;
        let mut crossings = 0;
        for w in data.windows(2) {
            if (w[0] - mean) * (w[1] - mean) < 0.0 { crossings += 1; }
        }
        crossings as f64 / n
    }
}