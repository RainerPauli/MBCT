// E:\mbct\trading-core\src\bin\universe_ranker_v2.rs
// THE ALLIANCE - Universe Ranker "KINETIC SHARPENER"
// Ziel: Identifikation der besten Shlong-Kandidaten basierend auf TTS

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;

#[derive(Debug, Deserialize, Serialize)]
struct AssetProfile {
    symbol: String,
    avg_entropy: f64,
    avg_nrg: f64,
    avg_pressure: f64,
    thermal_efficiency: f64,
    symmetry_consistency: f64,
    confidence_score: f64,
    symmetry_speed: f64,
    sample_count: usize,
}

fn main() -> std::io::Result<()> {
    let mut file = File::open("e:/mbct/data/mee_active_universe_new.json")?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;

    let mut assets: Vec<AssetProfile> = serde_json::from_str(&data).expect("JSON Fehler");

    // Filter: Wir ignorieren Assets mit zu wenig Samples oder ohne thermische Arbeit
    assets.retain(|a| a.sample_count > 400_000 && a.avg_entropy > 0.1);

    // Sortierung nach dem neuen Allianz-Kinetik-Score
    // Wir priorisieren (Symmetry Speed * Confidence)
    assets.sort_by(|a, b| {
        let score_a = a.symmetry_speed * a.confidence_score;
        let score_b = b.symmetry_speed * b.confidence_score;
        score_b.partial_cmp(&score_a).unwrap()
    });

    println!("\n🛡️ --- THE ALLIANCE: UNIVERSE RANKING (KINETIC SHARPENER) ---");
    println!("{:<10} | {:<10} | {:<10} | {:<12} | {:<10}", "SYMBOL", "CONFIDENCE", "TTS-SPEED", "EFFICIENCY", "STATUS");
    println!("{:-<65}", "");

    for asset in assets.iter().take(40) {
        let status = if asset.symmetry_speed > 0.1 {
            "🚀 SNIPER"
        } else if asset.confidence_score > 0.7 {
            "🛡️ TANKER"
        } else {
            "💤 SLEEPER"
        };

        println!(
            "{:<10} | {:<10.4} | {:<10.4} | {:<12.4} | {}",
            asset.symbol,
            asset.confidence_score,
            asset.symmetry_speed,
            asset.thermal_efficiency,
            status
        );
    }

    println!("{:-<65}", "");
    println!("INFO: SNIPER = Schnelle Roundtrips | TANKER = Hohe Sicherheit | SLEEPER = Zu wenig Kinetik");
    
    Ok(())
}