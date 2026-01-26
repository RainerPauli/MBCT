// E:\MBCT\trading-core\src\universe.rs
// MBCT - Kinetic Universe Selection

use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;

#[derive(Debug, Deserialize)]
pub struct CoinProfile {
    pub symbol: String,
    pub avg_entropy: f64,
    pub symmetry_consistency: f64,
    pub thermal_efficiency: f64,
    pub vola_3s: f64,
    pub sample_count: usize,
}

pub struct KineticUniverse;

impl KineticUniverse {
    pub fn get_active_symbols(json_path: &str) -> Vec<String> {
        // Die physikalische White-List der Movement-Carrier (aus deiner Analyse)
        let white_list: HashSet<&str> = [
            "SOL", "GMX", "BIGTIME", "ZK", "KAITO", "TURBO", "MOODENG", "ZRO", "PURR", "PROVE",
        ]
        .iter()
        .cloned()
        .collect();

        let mut active_symbols = Vec::new();

        if let Ok(content) = fs::read_to_string(json_path) {
            if let Ok(profiles) = serde_json::from_str::<HashMap<String, CoinProfile>>(&content) {
                for (sym, p) in profiles {
                    if white_list.contains(sym.as_str()) {
                        // Sicherheits-Check: Nur wenn das Asset im Research nicht "tot" war
                        if p.vola_3s > 0.0 && p.avg_entropy > 0.0 {
                            active_symbols.push(sym);
                        }
                    }
                }
            }
        }

        // Fallback, falls JSON nicht lesbar oder Pfad falsch
        if active_symbols.is_empty() {
            println!("⚠️ MBCT: Nutze statische Fallback-Liste.");
            active_symbols = white_list.into_iter().map(|s| s.to_string()).collect();
        }

        active_symbols
    }
}
