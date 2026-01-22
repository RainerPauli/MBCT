// E:\mbct\trading-core\src\bin\research_chunk_analyzer.rs
// THE ALLIANCE - Evolutionary Asset Profiler (Chunk-Based)

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

const CHUNK_SIZE: usize = 10_000_000; // 10 Millionen Zeilen pro Batch

#[derive(Default, Clone)]
struct CoinMetrics {
    count: usize,
    sum_entropy: f64,
    sum_nrg: f64,
    sum_pressure: f64,
    sum_abs_ret: f64,
    regime_counts: HashMap<String, usize>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = "e:/mbct/data/researcher.csv";
    println!("🚀 THE ALLIANCE: Starting Evolutionary Scan...");

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // Header überspringen
    let _header = lines.next();

    let mut global_metrics: HashMap<String, CoinMetrics> = HashMap::new();
    let mut chunk_metrics: HashMap<String, CoinMetrics> = HashMap::new();
    
    let mut line_counter: usize = 0;
    let mut total_counter: usize = 0;
    let start_time = Instant::now();

    for line in lines {
        let line_str = line?;
        let parts: Vec<&str> = line_str.split(',').collect();
        
        // CSV Layout laut archive.rs: 
        // 0:timestamp, 1:symbol, 2:price, 3:entropy, 4:pressure, 5:nrg, 6:regime, 7:symmetry, 8:slope...
        if parts.len() < 7 { continue; }

        let symbol = parts[1].to_string();
        let entropy: f64 = parts[3].parse().unwrap_or(0.0);
        let pressure: f64 = parts[4].parse().unwrap_or(0.0);
        let nrg: f64 = parts[5].parse().unwrap_or(0.0);
        let regime = parts[6].to_string();
        let ret_21s: f64 = parts[11].trim_start_matches("Some(").trim_end_matches(')').parse().unwrap_or(0.0);

        // Update Chunk Data
        let m = chunk_metrics.entry(symbol.clone()).or_default();
        m.count += 1;
        m.sum_entropy += entropy;
        m.sum_nrg += nrg;
        m.sum_pressure += pressure;
        m.sum_abs_ret += ret_21s.abs();
        *m.regime_counts.entry(regime).or_insert(0) += 1;

        line_counter += 1;
        total_counter += 1;

        // Wenn Chunk voll -> Zwischenbericht
        if line_counter >= CHUNK_SIZE {
            print_chunk_report(total_counter, &chunk_metrics, start_time.elapsed().as_secs());
            
            // Merge in Global & Reset Chunk
            for (sym, metrics) in chunk_metrics.drain() {
                let g = global_metrics.entry(sym).or_default();
                g.count += metrics.count;
                g.sum_entropy += metrics.sum_entropy;
                // ... andere Felder mergen
            }
            line_counter = 0;
        }
    }

    println!("\n✅ FINISHED. Total processed: {} lines", total_counter);
    Ok(())
}

fn print_chunk_report(total: usize, metrics: &HashMap<String, CoinMetrics>, elapsed: u64) {
    println!("\n--- CHUNK REPORT @ {} Mio Lines (Elapsed: {}s) ---", total / 1_000_000, elapsed);
    println!("{:<10} | {:<8} | {:<10} | {:<10} | {:<10}", "Symbol", "Samples", "Avg Ent", "Avg NRG", "Vola 21s");
    
    // Zeige Top 5 Assets dieses Chunks (sortiert nach Aktivität)
    let mut sorted: Vec<_> = metrics.iter().collect();
    sorted.sort_by(|a, b| b.1.count.cmp(&a.1.count));

    for (sym, m) in sorted.iter().take(8) {
        println!("{:<10} | {:<8} | {:>10.4} | {:>10.4} | {:>10.6}", 
            sym, m.count, m.sum_entropy / m.count as f64, m.sum_nrg / m.count as f64, m.sum_abs_ret / m.count as f64);
    }
}