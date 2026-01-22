// E:\mbct\trading-core\src\bin\research_evolution_profiler.rs

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::time::Instant;
use serde::{Serialize, Deserialize};

// Wir analysieren in 1-Mio-Schritten für maximale Transparenz
const CHUNK_SIZE: usize = 1_000_000; 
const CSV_PATH: &str = "e:/mbct/data/researcher.csv"; 

#[derive(Default, Clone, Serialize, Deserialize, Debug)]
pub struct DeepCoinProfile {
    pub symbol: String,
    // --- Kybernetik (Signalverlässlichkeit) ---
    pub avg_entropy: f64,          // Bestimmt den SENS-Boden
    pub symmetry_consistency: f64, // Vertrauenswürdigkeit des Vektors
    pub trend_dominance: f64,      // Regime-Verteilung
    // --- Thermodynamik (Physik) ---
    pub avg_nrg: f64,              // Trägheit / Masse
    pub avg_pressure: f64,         // Ladungspotenzial
    pub thermal_efficiency: f64,   // (Pressure / NRG) -> Explosivität
    // --- Vola-Vektoren (Fibonacci) ---
    pub vola_3s: f64,
    pub vola_21s: f64,
    pub vola_89s: f64,
    // --- Metadaten ---
    pub sample_count: usize,
    pub last_update_ts: u64,
}

fn clean_v(val: &str) -> f64 {
    val.trim().trim_start_matches("Some(").trim_end_matches(')').parse().unwrap_or(0.0)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start_total = Instant::now();
    
    println!("🔍 Prüfe Datenquelle: {}", CSV_PATH);
    if !std::path::Path::new(CSV_PATH).exists() {
        println!("❌ FEHLER: Datei nicht in /data gefunden! Bitte Pfad prüfen.");
        return Ok(());
    }

    let file = File::open(CSV_PATH)?;
    let metadata = file.metadata()?;
    println!("📂 Allianz-Daten geladen: {:.2} GB", metadata.len() as f64 / 1024.0 / 1024.0 / 1024.0);

    let reader = BufReader::with_capacity(1024 * 1024, file);
    let mut lines = reader.lines();
    
    // Header-Check (Basierend auf archive.rs)
    if let Some(Ok(header)) = lines.next() {
        println!("📝 Header-Struktur: {}", header);
    }

    let mut global_data: HashMap<String, DeepCoinProfile> = HashMap::new();
    let mut chunk_data: HashMap<String, DeepCoinProfile> = HashMap::new();
    
    let mut line_counter = 0;
    let mut total_lines = 0;
    let mut chunk_idx = 0;

    println!("🚀 Scan gestartet (Punkt = 100k Zeilen)...");

    for line in lines {
        let line_str = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let p: Vec<&str> = line_str.split(',').collect();
        if p.len() < 14 { continue; }

        let sym = p[1].to_string();
        let ts: u64 = p[0].parse().unwrap_or(0);
        let ent = p[3].parse::<f64>().unwrap_or(0.0);
        let pres = p[4].parse::<f64>().unwrap_or(0.0);
        let nrg = p[5].parse::<f64>().unwrap_or(0.0);
        let reg = p[6];
        let symm = p[7].parse::<f64>().unwrap_or(0.0);
        
        let v3 = clean_v(p[9]);
        let v21 = clean_v(p[11]);
        let v89 = clean_v(p[13]);

        let s = chunk_data.entry(sym.clone()).or_insert(DeepCoinProfile {
            symbol: sym,
            ..Default::default()
        });

        s.sample_count += 1;
        s.avg_entropy += ent;
        s.avg_pressure += pres;
        s.avg_nrg += nrg;
        s.symmetry_consistency += symm;
        s.vola_3s += v3.abs();
        s.vola_21s += v21.abs();
        s.vola_89s += v89.abs();
        s.last_update_ts = ts;
        if reg.contains("Trending") { s.trend_dominance += 1.0; }

        line_counter += 1;
        total_lines += 1;

        if total_lines % 100_000 == 0 {
            print!("."); 
            std::io::stdout().flush().unwrap();
        }

        if line_counter >= CHUNK_SIZE {
            chunk_idx += 1;
            println!("\n✅ Chunk #{} verarbeitet ({} Mio Zeilen total).", chunk_idx, total_lines / 1_000_000);
            process_chunk_end(chunk_idx, &mut global_data, &mut chunk_data, start_total);
            line_counter = 0;
        }
    }

    println!("\n🏁 ANALYSE KOMPLETT. {} Zeilen analysiert.", total_lines);
    Ok(())
}

fn process_chunk_end(idx: usize, global: &mut HashMap<String, DeepCoinProfile>, chunk: &mut HashMap<String, DeepCoinProfile>, start: Instant) {
    for (sym, c) in chunk.drain() {
        let g = global.entry(sym.clone()).or_insert(DeepCoinProfile { symbol: sym, ..Default::default() });
        
        g.sample_count += c.sample_count;
        g.avg_entropy += c.avg_entropy;
        g.avg_pressure += c.avg_pressure;
        g.avg_nrg += c.avg_nrg;
        g.symmetry_consistency += c.symmetry_consistency;
        g.vola_3s += c.vola_3s;
        g.vola_21s += c.vola_21s;
        g.vola_89s += c.vola_89s;
        g.trend_dominance += c.trend_dominance;
        g.last_update_ts = c.last_update_ts;
    }

    // Statistisches Update für THE ALLIANCE (Beispiel BTC)
    if let Some(btc) = global.get("BTC") {
        let n = btc.sample_count as f64;
        println!(">>> Snapshot BTC: Ent: {:.4}, Vola21: {:.6}, Eff: {:.4}", 
                 btc.avg_entropy/n, btc.vola_21s/n, btc.avg_pressure/btc.avg_nrg);
    }

    // Fortschritt speichern
    let out_path = format!("e:/mbct/data/profiles_evolution_v4.json");
    let mut file = File::create(out_path).unwrap();
    
    // Wir berechnen für den Export die echten Durchschnitte
    let mut export_map = global.clone();
    for p in export_map.values_mut() {
        let n = p.sample_count as f64;
        p.avg_entropy /= n;
        p.symmetry_consistency /= n;
        p.vola_3s /= n;
        p.vola_21s /= n;
        p.vola_89s /= n;
        p.trend_dominance /= n;
        p.thermal_efficiency = p.avg_pressure / p.avg_nrg;
    }
    
    let json = serde_json::to_string_pretty(&export_map).unwrap();
    file.write_all(json.as_bytes()).unwrap();
}