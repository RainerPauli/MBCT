// E:\mbct\trading-core\src\bin\research_analyzer.rs
// MEE10 DEEP-CORE ANALYTICS ENGINE v2.0 - THE SOURCE (FIXED)

use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Default)]
struct CoreStats {
    count: usize,
    pos_ret: usize,
    sum_ret: f64,
    sum_abs_ret: f64,
    max_drawdown: f64,
    max_upside: f64,
}

fn clean_val(val: &str) -> Option<f64> {
    let s = val.trim().trim_start_matches("Some(").trim_end_matches(')');
    if s == "None" || s.is_empty() { None } else { s.parse::<f64>().ok() }
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = "e:/mbct/data/validation_live.csv";
    println!("🚀 STARTING MEE10 DEEP-CORE SCAN...");
    
    if !Path::new(path).exists() {
        return Err(format!("Datei nicht gefunden: {}", path).into());
    }

    let file_meta = Path::new(path).metadata()?;
    println!("📦 Data Source: {} ({:.2} MB)", path, file_meta.len() as f64 / 1024.0 / 1024.0);

    let file = File::open(path)?;
    let reader = BufReader::with_capacity(2 * 1024 * 1024, file); 

    let mut nrg_vbi_matrix: BTreeMap<i32, HashMap<i32, CoreStats>> = BTreeMap::new();
    let mut regime_stats: HashMap<String, CoreStats> = HashMap::new();
    let mut symbol_stats: HashMap<String, CoreStats> = HashMap::new();
    
    let mut total_lines = 0;
    let mut processed = 0;

    for line_result in reader.lines() {
        let l = line_result?;
        total_lines += 1;
        if l.starts_with("timestamp") || l.is_empty() { continue; }

        let c: Vec<&str> = l.split(',').collect();
        // Index Check: ts(0), sym(1), entropy(4), nrg(11), vbi(12), regime(16), ret(22), complete(23)
        if c.len() < 24 || c[23].trim() != "true" { continue; }

        let symbol = c[1].trim().to_string();
        let nrg = clean_val(c[11]).unwrap_or(0.0);
        let vbi = clean_val(c[12]).unwrap_or(0.0); 
        let regime = c[16].trim().to_string();
        let ret = clean_val(c[22]).unwrap_or(0.0);

        processed += 1;

        // NRG-VBI Matrix (NRG in 1er Schritten, VBI skaliert auf -5 bis +5)
        let nrg_bucket = nrg.floor() as i32;
        let vbi_bucket = (vbi * 5.0).floor() as i32; 
        
        let bucket = nrg_vbi_matrix.entry(nrg_bucket).or_default()
            .entry(vbi_bucket).or_default();
        
        update_stats(bucket, ret);
        update_stats(regime_stats.entry(regime).or_default(), ret);
        update_stats(symbol_stats.entry(symbol).or_default(), ret);
    }

    print_report(processed, total_lines, regime_stats, symbol_stats, nrg_vbi_matrix);

    Ok(())
}

fn update_stats(s: &mut CoreStats, ret: f64) {
    s.count += 1;
    s.sum_ret += ret;
    s.sum_abs_ret += ret.abs();
    if ret > 0.0 { s.pos_ret += 1; }
    if ret < s.max_drawdown { s.max_drawdown = ret; }
    if ret > s.max_upside { s.max_upside = ret; }
}

fn print_report(proc: usize, total: usize, reg: HashMap<String, CoreStats>, sym: HashMap<String, CoreStats>, matrix: BTreeMap<i32, HashMap<i32, CoreStats>>) {
    let separator = "=".repeat(100);
    println!("\n{}", separator);
    println!("📊 MEE10 THERMODYNAMIC CONSOLIDATED REPORT");
    println!("Processed Samples: {} | Efficiency: {:.1}%", proc, (proc as f64 / total as f64) * 100.0);
    println!("{}", separator);

    println!("\n[1] REGIME EFFICIENCY");
    println!("{:<15} | {:<10} | {:<10} | {:<10} | {:<10}", "REGIME", "SAMPLES", "WINRATE", "AVG RET", "EXPECTANCY");
    for (name, s) in reg {
        let wr = (s.pos_ret as f64 / s.count as f64) * 100.0;
        let avg = s.sum_ret / s.count as f64;
        println!("{:<15} | {:<10} | {:>8.2}%  | {:>10.6} | {:>10.6}", name, s.count, wr, avg, avg);
    }

    println!("\n[2] ASSET VOLATILITY SIGNATURE");
    for (name, s) in sym {
        println!("{:<10} | Samples: {:<8} | Max Upside: {:>8.4}% | Max Drawdown: {:>8.4}%", 
                 name, s.count, s.max_upside*100.0, s.max_drawdown*100.0);
    }

    println!("\n[3] THE GOLDEN MATRIX (NRG vs DIRECTIONAL VECTOR)");
    println!("Goal: Find Winrates > 55% (Trend) or < 40% (Reversion)");
    println!("{:<10} | {:<10} | {:<10} | {:<10} | {:<10}", "NRG BUCKET", "VBI ZONE", "SAMPLES", "WINRATE", "SIGNAL");
    
    // Wir schauen uns die höchsten NRG-Ebenen zuerst an
    for (nrg_b, vbi_map) in matrix.iter().rev().take(15) { 
        for (vbi_b, s) in vbi_map {
            if s.count < 100 { continue; } // Signifikanz-Filter
            
            let wr = (s.pos_ret as f64 / s.count as f64) * 100.0;
            let vbi_desc = match vbi_b {
                v if *v <= -3 => "HEAVY ASK",
                v if *v <= -1 => "ASK BIAS",
                0             => "NEUTRAL",
                v if *v <= 2  => "BID BIAS",
                _             => "HEAVY BID",
            };

            let signal = if wr > 55.0 { "🔥 LONG" } else if wr < 45.0 { "❄️  SHORT" } else { "   ---" };
            
            println!("NRG {:>2}.0  | {:<10} | {:<10} | {:>8.2}%  | {}", 
                     nrg_b, vbi_desc, s.count, wr, signal);
        }
    }
    println!("{}", separator);
}