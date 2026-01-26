// E:\mbct\trading-core\src\bin\research_evolution_profiler.rs
// THE ALLIANCE - Clean Stream Profiler v2.0 "SHARPENED KINETICS"
// Fokus: Confidence-Scores, Time-to-Symmetry & Thermodynamische Schärfe

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::time::Instant;

const CSV_PATH: &str = "e:/mbct/data/researcher.csv";
const OUTPUT_PATH: &str = "e:/mbct/data/mee_active_universe_new.json";

#[derive(Default, Clone, Serialize, Deserialize, Debug)]
pub struct DeepCoinProfile {
    pub symbol: String,
    pub avg_entropy: f64,
    pub avg_nrg: f64,
    pub avg_pressure: f64,
    pub thermal_efficiency: f64,
    pub symmetry_consistency: f64,
    pub trend_dominance: f64,
    pub vola_3s: f64,
    pub vola_21s: f64,
    pub vola_89s: f64,
    pub sample_count: usize,
    pub reliability: f64,
    pub confidence_score: f64, // 0.0 - 1.0 (Die finale Allianz-Metrik)
    pub symmetry_speed: f64,   // "Time-to-Symmetry" Faktor
}

/// Der "Alliance-Parser": Entfernt Quotes, wandelt Komma zu Punkt
#[inline(always)]
fn alliance_parse(s: &str) -> f64 {
    let clean = s.trim_matches('"').replace(',', ".");
    if clean.starts_with("Some(") {
        clean[5..clean.len() - 1].parse::<f64>().unwrap_or(0.0)
    } else {
        clean.parse::<f64>().unwrap_or(0.0)
    }
}

fn main() -> std::io::Result<()> {
    let start = Instant::now();
    println!("🛡️ THE ALLIANCE: Starte High-Precision Scan...");

    let file = File::open(CSV_PATH)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // Header überspringen
    if let Some(_) = lines.next() {}

    let mut global_profiles: HashMap<String, DeepCoinProfile> = HashMap::new();
    let mut total_lines = 0;

    for line in lines {
        let line = line?;
        let p: Vec<&str> = line.split(',').collect();
        if p.len() < 14 {
            continue;
        }

        let symbol = p[1].to_string();
        let entry = global_profiles
            .entry(symbol.clone())
            .or_insert(DeepCoinProfile {
                symbol,
                ..Default::default()
            });

        // Datenextraktion
        let entropy = alliance_parse(p[3]);
        let pressure = alliance_parse(p[4]);
        let nrg = alliance_parse(p[5]);
        let symmetry = alliance_parse(p[7]);
        let v3 = alliance_parse(p[9]).abs();
        let v21 = alliance_parse(p[11]).abs();
        let v89 = alliance_parse(p[13]).abs();

        entry.avg_entropy += entropy;
        entry.avg_pressure += pressure;
        entry.avg_nrg += nrg;
        entry.symmetry_consistency += symmetry;

        // TTS-Logik: Korrelation von Vola zu Symmetrie
        // Ein Asset ist "schnell", wenn Symmetrie hoch bleibt trotz hoher Vola
        if v3 > 0.0 {
            entry.symmetry_speed += symmetry / (1.0 + v3);
        }

        entry.vola_3s += v3;
        entry.vola_21s += v21;
        entry.vola_89s += v89;

        entry.sample_count += 1;
        total_lines += 1;

        if total_lines % 5_000_000 == 0 {
            println!(
                "⏳ Fortschritt: {} Mio. Zeilen | Aktuell: {}",
                total_lines / 1_000_000,
                p[1]
            );
        }
    }

    println!(
        "✅ Scan abgeschlossen. Berechne Allianz-Confidence für {} Assets...",
        global_profiles.len()
    );

    let mut results: Vec<DeepCoinProfile> = global_profiles.into_values().collect();

    for p in results.iter_mut() {
        let n = p.sample_count as f64;
        if n > 0.0 {
            p.avg_entropy /= n;
            p.avg_pressure /= n;
            p.avg_nrg /= n;
            p.symmetry_consistency /= n;
            p.symmetry_speed /= n;
            p.vola_3s /= n;
            p.vola_21s /= n;
            p.vola_89s /= n;

            p.thermal_efficiency = if p.avg_nrg.abs() > 0.000001 {
                p.avg_pressure / p.avg_nrg
            } else {
                0.0
            };

            // --- ALLIANCE CONFIDENCE FORMULA ---
            // 1. Basis: Symmetrie (Wie oft ist das Signal wahr?)
            let base_rel = p.symmetry_consistency;

            // 2. Kinetik: Symmetry Speed (Wie schnell kehrt Ruhe ein?)
            let speed_factor = (p.symmetry_speed * 2.0).min(1.0);

            // 3. Thermische Arbeit (Leistet das Asset Widerstand?)
            let work_factor = (p.thermal_efficiency.abs() * 0.5).min(0.2);

            // 4. Stabilitäts-Check (Entropie-Bremse)
            let entropy_penalty = if p.avg_entropy > 3.5 { 0.1 } else { 0.0 };

            p.confidence_score =
                (base_rel * 0.6) + (speed_factor * 0.3) + work_factor - entropy_penalty;
            p.reliability = p.confidence_score; // Update für den Report
        }
    }

    // Sortierung nach Confidence Score (Absteigend)
    results.sort_by(|a, b| b.confidence_score.partial_cmp(&a.confidence_score).unwrap());

    let json = serde_json::to_string_pretty(&results).unwrap();
    let mut file = File::create(OUTPUT_PATH)?;
    file.write_all(json.as_bytes())?;

    println!("🏆 THE ALLIANCE: Report unter {} gespeichert.", OUTPUT_PATH);
    println!("Dauer: {:?}", start.elapsed());

    Ok(())
}
