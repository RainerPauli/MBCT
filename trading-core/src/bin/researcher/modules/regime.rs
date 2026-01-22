// E:\MBCT\trading-core\src\bin\researcher\modules\regime.rs
// THE ALLIANCE - MBCT Regime Modul v2.0
// Fokus: Kybernetische Symmetrie & Z-Score Anomalie-Detektion

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

    /// Klassifiziert den Marktzustand basierend auf der energetischen Symmetrie
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

        let prices: Vec<f64> = history.iter().map(|h| h.price).collect();
        let slope = self.calculate_slope(&prices);
        let symmetry = self.calculate_symmetry(&prices);
        
        // Reversion Speed: Delta der Symmetrie über die letzten 5 Samples (Kinetischer Vektor)
        let reversion = if history.len() > 5 {
            let prev_sym = self.calculate_symmetry(&prices[..prices.len()-5]);
            symmetry - prev_sym
        } else {
            0.0
        };

        // Definition der Regime-Zonen (Allianz-Standard)
        let regime = if symmetry > 0.8 || symmetry < 0.2 {
            MarketRegime::Ballistic   // Einseitiger Energiefluss
        } else if symmetry > 0.4 && symmetry < 0.6 {
            MarketRegime::Compression // Maximaler Druckaufbau
        } else {
            MarketRegime::Oscillatory // Rauschen / Seitwärts
        };

        RegimeState {
            regime,
            symmetry_score: symmetry,
            slope,
            reversion_speed: reversion,
            confidence: 1.0 - (1.0 / (history.len() as f64)),
        }
    }

    /// Statistische Überlegenheit: Z-Score Berechnung für physikalische Parameter
    /// Erlaubt die Identifizierung von 3-Sigma-Events in Echtzeit.
    pub fn calculate_z_score(current_val: f64, history: &VecDeque<PhysicsState>, field: &str) -> f64 {
        let values: Vec<f64> = match field {
            "entropy" => history.iter().map(|h| h.entropy).collect(),
            "pressure" => history.iter().map(|h| h.pressure).collect(),
            "nrg" => history.iter().map(|h| h.nrg).collect(),
            _ => return 0.0,
        };

        let n = values.len() as f64;
        if n < 2.0 { return 0.0; }
        
        let mean = values.iter().sum::<f64>() / n;
        let variance = values.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let std_dev = variance.sqrt();
        
        // Division durch Null Schutz bei "toter" Materie
        if std_dev < 0.000000001 { 
            0.0 
        } else { 
            (current_val - mean) / std_dev 
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
            if diff > 0.0 { ups += diff; }
            else { downs += diff.abs(); }
        }
        let total = ups + downs;
        if total == 0.0 { 0.5 } else { ups / total }
    }
}