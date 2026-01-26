// E:\mbct\trading-core\src\bin\alliance_sens_configurator.rs
// THE ALLIANCE - SENS Configurator v1.0
// Berechnet SENS-Böden für die Top 18 Sniper & Tanker

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

#[derive(Debug, Deserialize, Serialize)]
struct AssetProfile {
    symbol: String,
    confidence_score: f64,
    symmetry_speed: f64,
    symmetry_consistency: f64,
    thermal_efficiency: f64,
}

#[derive(Debug, Serialize)]
struct SensConfig {
    symbol: String,
    sens_long_trigger: f64,
    sens_short_trigger: f64,
    cooldown_seconds: u64,
    trade_mode: String,
}

fn main() -> std::io::Result<()> {
    let mut file = File::open("e:/mbct/data/mee_active_universe_new.json")?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;

    let assets: Vec<AssetProfile> = serde_json::from_str(&data).expect("JSON Fehler");

    // Wir nehmen die Top 18 nach dem Allianz-Kinetik-Score
    let mut top_18 = assets;
    top_18.sort_by(|a, b| {
        let score_a = a.symmetry_speed * a.confidence_score;
        let score_b = b.symmetry_speed * b.confidence_score;
        score_b.partial_cmp(&score_a).unwrap()
    });
    let top_18: Vec<AssetProfile> = top_18.into_iter().take(18).collect();

    let mut final_configs = Vec::new();

    println!("\n🛡️ THE ALLIANCE: Berechne SENS-Böden für Top 18...");

    for asset in top_18 {
        // Logik: Je höher die Speed, desto enger (aggressiver) kann der Trigger sein.
        // Sniper brauchen schnellere Trigger, Tanker brauchen mehr "Raum".

        let base_threshold = 1.0 - asset.symmetry_consistency;
        let speed_adjustment = asset.symmetry_speed * 0.5;

        // Long Trigger: Wenn Symmetrie-Integrität einbricht
        let long_t = (0.35 + speed_adjustment).clamp(0.2, 0.45);
        // Short Trigger: Spiegelbildlich oder basierend auf Symmetrie-Peak
        let short_t = (0.65 - speed_adjustment).clamp(0.55, 0.8);

        let mode = if asset.symmetry_speed > 0.1 {
            "SNIPER_FAST"
        } else {
            "TANKER_STABLE"
        };

        final_configs.push(SensConfig {
            symbol: asset.symbol.clone(),
            sens_long_trigger: long_t,
            sens_short_trigger: short_t,
            cooldown_seconds: if mode == "SNIPER_FAST" { 60 } else { 180 },
            trade_mode: mode.to_string(),
        });

        println!(
            "🎯 {} | Mode: {} | L: {:.3} | S: {:.3}",
            asset.symbol, mode, long_t, short_t
        );
    }

    let json = serde_json::to_string_pretty(&final_configs).unwrap();
    let mut file = File::create("e:/mbct/data/sens_config_top18.json")?;
    file.write_all(json.as_bytes())?;

    println!("\n✅ Konfiguration für Testnet gespeichert: sens_config_top18.json");
    Ok(())
}
