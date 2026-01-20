// E:\mbct\trading-core\src\bin\research_engine.rs
// MBCT THERMODYNAMIC RESEARCH ENGINE v4.1.4 - FINAL ERROR-FREE

use dashmap::DashMap;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::str::FromStr;
use tokio::sync::Mutex;
use tokio::signal;
use tokio::time;

// Interne MEE Module
use trading_common::data::repository::Repository;
use trading_common::data::types::MarketState;
use trading_core::exchange::envelope_detection::EnvelopeDetector;
use trading_core::exchange::market_data::HyperliquidMarketData;
use trading_core::exchange::ws::HyperliquidWs;
use trading_core::exchange::types::L2Snapshot;

// ============================================================================
// KONFIGURATION & KONSTANTEN
// ============================================================================

const HISTORY_SIZE: usize = 100;
const REGRESSION_WINDOW: usize = 15;
const MIN_REGRESSION_SAMPLES: usize = 5;
const SPREAD_THRESHOLD: f64 = 0.001;
const MIN_LIQUIDITY: f64 = 100.0;
const CSV_FLUSH_INTERVAL_MS: u64 = 5000;

// Performance-Monitoring
static PROCESSED_COUNT: AtomicUsize = AtomicUsize::new(0);
static VALIDATION_RECORDS_COUNT: AtomicUsize = AtomicUsize::new(0);
static ERROR_COUNT: AtomicUsize = AtomicUsize::new(0);
static CSV_WRITES_COUNT: AtomicUsize = AtomicUsize::new(0);

// ============================================================================
// PRICE EXTRACTION FROM L2 SNAPSHOT
// ============================================================================

fn extract_mid_price_from_snapshot(snapshot: &L2Snapshot) -> Option<f64> {
    // Hyperliquid L2: levels[0] sind Bids, levels[1] sind Asks
    if snapshot.levels.len() < 2 {
        return None;
    }
    
    let mut best_bid = f64::MIN;
    let mut best_ask = f64::MAX;
    
    // Bids verarbeiten
    for level in &snapshot.levels[0] {
        if let Ok(price) = level.px.parse::<f64>() {
            best_bid = best_bid.max(price);
        }
    }
    
    // Asks verarbeiten
    for level in &snapshot.levels[1] {
        if let Ok(price) = level.px.parse::<f64>() {
            best_ask = best_ask.min(price);
        }
    }
    
    if best_bid == f64::MIN || best_ask == f64::MAX {
        return None;
    }
    
    Some((best_bid + best_ask) / 2.0)
}

fn extract_total_volume_from_snapshot(snapshot: &L2Snapshot) -> f64 {
    let mut total = 0.0;
    for level_vec in &snapshot.levels {
        for level in level_vec {
            if let Ok(volume) = level.sz.parse::<f64>() {
                total += volume;
            }
        }
    }
    total
}

fn extract_spread_from_snapshot(snapshot: &L2Snapshot) -> Option<f64> {
    if snapshot.levels.len() < 2 {
        return None;
    }
    
    let mut best_bid = f64::MIN;
    let mut best_ask = f64::MAX;
    
    for level in &snapshot.levels[0] {
        if let Ok(price) = level.px.parse::<f64>() {
            best_bid = best_bid.max(price);
        }
    }
    
    for level in &snapshot.levels[1] {
        if let Ok(price) = level.px.parse::<f64>() {
            best_ask = best_ask.min(price);
        }
    }
    
    if best_bid > 0.0 && best_ask != f64::MAX {
        Some((best_ask - best_bid) / best_bid)
    } else {
        None
    }
}

fn extract_bid_ask_volumes(snapshot: &L2Snapshot) -> (f64, f64) {
    let mut bid_volume = 0.0;
    let mut ask_volume = 0.0;
    
    if snapshot.levels.len() >= 2 {
        for level in &snapshot.levels[0] {
            if let Ok(volume) = level.sz.parse::<f64>() {
                bid_volume += volume;
            }
        }
        for level in &snapshot.levels[1] {
            if let Ok(volume) = level.sz.parse::<f64>() {
                ask_volume += volume;
            }
        }
    }
    
    (bid_volume, ask_volume)
}

// ============================================================================
// DATENSTRUKTUREN FÜR VALIDIERUNG
// ============================================================================

#[derive(Debug, Clone, Serialize)]
struct ValidationRecord {
    timestamp: i64,
    symbol: String,
    price_at_t0: f64,
    spread_at_t0: f64,
    
    entropy: f64,
    pressure: f64,
    temperature: f64,
    volume_spread: f64,
    total_volume: f64,
    bid_volume: f64,
    ask_volume: f64,
    
    movement_energy: f64,
    symmetry_score: f64,
    decay_slope: f64,
    z_score: f64,
    confidence: f64,
    regime: String,
    regime_consistency: f64,
    liquidity_score: f64,
    
    return_5s: Option<f64>,
    return_10s: Option<f64>,
    return_30s: Option<f64>,
    return_60s: Option<f64>,
    
    is_complete: bool,
    processing_time_us: u128,
    queue_time_us: u128,
    created_at: i64,
}

impl ValidationRecord {
    fn new(
        state: &MarketState, 
        metrics: &RegimeClassifier, 
        snapshot: &L2Snapshot,
        processing_time: Duration,
        queue_time: Duration
    ) -> Self {
        let price = extract_mid_price_from_snapshot(snapshot).unwrap_or(0.0);
        let spread = extract_spread_from_snapshot(snapshot).unwrap_or(0.0);
        let total_volume = extract_total_volume_from_snapshot(snapshot);
        let (bid_volume, ask_volume) = extract_bid_ask_volumes(snapshot);
        
        Self {
            timestamp: state.timestamp,
            symbol: state.symbol.clone(),
            price_at_t0: price,
            spread_at_t0: spread,
            
            entropy: state.entropy_level.and_then(|e| e.to_f64()).unwrap_or(0.0),
            pressure: state.pressure.to_f64().unwrap_or(0.0),
            temperature: state.temperature.to_f64().unwrap_or(0.0),
            volume_spread: state.volume_spread.to_f64().unwrap_or(0.0),
            total_volume,
            bid_volume,
            ask_volume,
            
            movement_energy: metrics.movement_energy,
            symmetry_score: metrics.symmetry_score,
            decay_slope: metrics.decay_slope,
            z_score: metrics.z_score,
            confidence: metrics.confidence,
            regime: state.regime.as_deref().unwrap_or("Unknown").to_string(),
            regime_consistency: metrics.regime_consistency,
            liquidity_score: metrics.liquidity_score,
            
            return_5s: None,
            return_10s: None,
            return_30s: None,
            return_60s: None,
            
            is_complete: false,
            processing_time_us: processing_time.as_micros(),
            queue_time_us: queue_time.as_micros(),
            created_at: chrono::Utc::now().timestamp(),
        }
    }
    
    fn calculate_return(&self, future_price: f64) -> Option<f64> {
        if self.price_at_t0 > 0.0 && future_price > 0.0 {
            Some((future_price - self.price_at_t0) / self.price_at_t0)
        } else {
            None
        }
    }
    
    fn to_csv_line(&self) -> String {
        format!(
            "{},{},{:.8},{:.6},{:.6},{:.6},{:.6},{:.2},{:.2},{:.2},{:.2},{:.6e},{:.4},{:.6},{:.4},{:.4},{},{:.4},{:.4},{:?},{:?},{:?},{:?},{},{},{},{}\n",
            self.timestamp,
            self.symbol,
            self.price_at_t0,
            self.spread_at_t0,
            self.entropy,
            self.pressure,
            self.temperature,
            self.volume_spread,
            self.total_volume,
            self.bid_volume,
            self.ask_volume,
            self.movement_energy,
            self.symmetry_score,
            self.decay_slope,
            self.z_score,
            self.confidence,
            self.regime,
            self.regime_consistency,
            self.liquidity_score,
            self.return_5s,
            self.return_10s,
            self.return_30s,
            self.return_60s,
            self.is_complete,
            self.processing_time_us,
            self.queue_time_us,
            self.created_at
        )
    }
    
    fn csv_header() -> String {
        "timestamp,symbol,price,spread,entropy,pressure,temperature,volume_spread,total_volume,bid_volume,ask_volume,nrg,sym,slope,zscore,confidence,regime,regime_consistency,liquidity_score,return_5s,return_10s,return_30s,return_60s,complete,processing_us,queue_us,created_at\n".to_string()
    }
}

// ============================================================================
// REGIME CLASSIFIER
// ============================================================================

#[derive(Debug, Clone)]
struct RegimeClassifier {
    movement_energy: f64,
    symmetry_score: f64,
    decay_slope: f64,
    volatility_heat: f64,
    confidence: f64,
    z_score: f64,
    regime_consistency: f64,
    liquidity_score: f64,
}

// ============================================================================
// THERMODYNAMIC PHYSICIST MIT LIVE CSV WRITER
// ============================================================================

struct ThermodynamicPhysicist {
    entropy_cache: DashMap<String, Vec<f64>>,
    price_history: DashMap<String, Vec<(i64, f64)>>,
    validation_queue: DashMap<String, Vec<ValidationRecord>>,
    correlation_stats: DashMap<String, CorrelationStats>,
    csv_writer: Arc<Mutex<BufWriter<std::fs::File>>>,
}

#[derive(Debug, Clone)]
struct CorrelationStats {
    nrg_5s_correlation: f64,
    nrg_10s_correlation: f64,
    nrg_5s_samples: usize,
    sym_oscillatory_precision: f64,
    last_updated: Instant,
}

impl ThermodynamicPhysicist {
    async fn new() -> anyhow::Result<Self> {
        let csv_path = "e:/mbct/data/validation_live.csv";
        
        let file_exists = std::path::Path::new(csv_path).exists();
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(csv_path)
            .map_err(|e| anyhow::anyhow!("Failed to open CSV file: {}", e))?;
        
        let writer = BufWriter::new(file);
        
        if !file_exists {
            let temp_file = OpenOptions::new()
                .create(true)
                .write(true)
                .open(csv_path)
                .map_err(|e| anyhow::anyhow!("Failed to create CSV file: {}", e))?;
            let mut temp_writer = BufWriter::new(temp_file);
            temp_writer.write_all(ValidationRecord::csv_header().as_bytes())
                .map_err(|e| anyhow::anyhow!("Failed to write CSV header: {}", e))?;
            temp_writer.flush()?;
        }
        
        Ok(Self {
            entropy_cache: DashMap::new(),
            price_history: DashMap::new(),
            validation_queue: DashMap::new(),
            correlation_stats: DashMap::new(),
            csv_writer: Arc::new(Mutex::new(writer)),
        })
    }
    
    fn calculate_decay_slope(&self, history: &[f64]) -> f64 {
        let n = history.len() as f64;
        if n < MIN_REGRESSION_SAMPLES as f64 {
            return 0.0;
        }
        
        let sum_x: f64 = (0..history.len()).map(|i| i as f64).sum();
        let sum_y: f64 = history.iter().sum();
        let sum_xy: f64 = history.iter().enumerate()
            .map(|(i, &y)| i as f64 * y)
            .sum();
        let sum_x2: f64 = (0..history.len())
            .map(|i| (i as f64).powi(2))
            .sum();
        
        let denominator = n * sum_x2 - sum_x.powi(2);
        if denominator.abs() < 1e-9 {
            return 0.0;
        }
        
        (n * sum_xy - sum_x * sum_y) / denominator
    }
    
    fn calculate_pearson_correlation(&self, x: &[f64], y: &[f64]) -> (f64, usize) {
        if x.len() != y.len() || x.len() < 2 {
            return (0.0, 0);
        }
        
        let n = x.len() as f64;
        let sum_x: f64 = x.iter().sum();
        let sum_y: f64 = y.iter().sum();
        let sum_xy: f64 = x.iter().zip(y.iter()).map(|(&xi, &yi)| xi * yi).sum();
        let sum_x2: f64 = x.iter().map(|&xi| xi * xi).sum();
        let sum_y2: f64 = y.iter().map(|&yi| yi * yi).sum();
        
        let numerator = n * sum_xy - sum_x * sum_y;
        let denominator = ((n * sum_x2 - sum_x * sum_x) * (n * sum_y2 - sum_y * sum_y)).sqrt();
        
        if denominator.abs() > 1e-9 {
            (numerator / denominator, x.len())
        } else {
            (0.0, x.len())
        }
    }
    
    fn record_price(&self, symbol: &str, timestamp: i64, price: f64) {
        let mut history = self.price_history.entry(symbol.to_string()).or_insert_with(Vec::new);
        history.push((timestamp, price));
        
        if history.len() > 5000 {
            history.remove(0);
        }
        
        self.update_pending_records(symbol, timestamp, price);
    }
    
    fn update_pending_records(&self, symbol: &str, current_timestamp: i64, current_price: f64) {
        if let Some(mut records) = self.validation_queue.get_mut(symbol) {
            let mut to_remove = Vec::new();
            let mut nrg_values_5s = Vec::new();
            let mut returns_5s = Vec::new();
            let mut nrg_values_10s = Vec::new();
            let mut returns_10s = Vec::new();
            
            for (i, record) in records.iter_mut().enumerate() {
                let time_diff = current_timestamp - record.timestamp;
                
                if time_diff >= 5 && record.return_5s.is_none() {
                    record.return_5s = record.calculate_return(current_price);
                }
                if time_diff >= 10 && record.return_10s.is_none() {
                    record.return_10s = record.calculate_return(current_price);
                }
                if time_diff >= 30 && record.return_30s.is_none() {
                    record.return_30s = record.calculate_return(current_price);
                }
                if time_diff >= 60 && record.return_60s.is_none() {
                    record.return_60s = record.calculate_return(current_price);
                    record.is_complete = true;
                    
                    if let Some(return_5s) = record.return_5s {
                        nrg_values_5s.push(record.movement_energy);
                        returns_5s.push(return_5s);
                    }
                    if let Some(return_10s) = record.return_10s {
                        nrg_values_10s.push(record.movement_energy);
                        returns_10s.push(return_10s);
                    }
                    
                    to_remove.push(i);
                }
            }
            
            if !nrg_values_5s.is_empty() || !nrg_values_10s.is_empty() {
                let mut stats = self.correlation_stats.entry(symbol.to_string()).or_insert_with(|| CorrelationStats {
                    nrg_5s_correlation: 0.0,
                    nrg_10s_correlation: 0.0,
                    nrg_5s_samples: 0,
                    sym_oscillatory_precision: 0.0,
                    last_updated: Instant::now(),
                });
                
                if !nrg_values_5s.is_empty() {
                    let (correlation_5s, samples_5s) = self.calculate_pearson_correlation(&nrg_values_5s, &returns_5s);
                    let alpha = 0.1;
                    stats.nrg_5s_correlation = alpha * correlation_5s + (1.0 - alpha) * stats.nrg_5s_correlation;
                    stats.nrg_5s_samples += samples_5s;
                }
                
                if !nrg_values_10s.is_empty() {
                    let (correlation_10s, _) = self.calculate_pearson_correlation(&nrg_values_10s, &returns_10s);
                    let alpha = 0.1;
                    stats.nrg_10s_correlation = alpha * correlation_10s + (1.0 - alpha) * stats.nrg_10s_correlation;
                }
                
                stats.last_updated = Instant::now();
                
                if stats.nrg_5s_samples % 50 == 0 && stats.nrg_5s_samples > 0 {
                    println!(
                        "📊 CORRELATION {}: 5s: {:.3} | 10s: {:.3} | Samples: {}",
                        symbol, stats.nrg_5s_correlation, stats.nrg_10s_correlation, stats.nrg_5s_samples
                    );
                }
            }
            
            for &idx in to_remove.iter().rev() {
                if idx < records.len() {
                    let complete_record = records.remove(idx);
                    VALIDATION_RECORDS_COUNT.fetch_add(1, Ordering::Relaxed);
                    
                    let csv_writer = self.csv_writer.clone();
                    let record_line = complete_record.to_csv_line();
                    
                    tokio::spawn(async move {
                        let mut writer = csv_writer.lock().await;
                        if writer.write_all(record_line.as_bytes()).is_ok() {
                            CSV_WRITES_COUNT.fetch_add(1, Ordering::Relaxed);
                        }
                    });
                    
                    if complete_record.confidence > 0.7 {
                        if let Some(return_5s) = complete_record.return_5s {
                            let abs_return = return_5s.abs();
                            if abs_return > 0.001 {
                                let direction = if return_5s > 0.0 { "↑" } else { "↓" };
                                println!(
                                    "📈 SIGNAL {}: {} | NRG: {:.3e} → 5s: {:.4}{} | Conf: {:.0}%",
                                    complete_record.symbol,
                                    complete_record.regime,
                                    complete_record.movement_energy,
                                    abs_return * 100.0,
                                    direction,
                                    complete_record.confidence * 100.0
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    
    fn queue_validation_record(&self, symbol: &str, record: ValidationRecord) {
        let mut records = self.validation_queue.entry(symbol.to_string()).or_insert_with(Vec::new);
        records.push(record);
        
        if records.len() > 1000 {
            records.remove(0);
        }
    }
    
    fn analyze(&self, state: &MarketState, history: &[MarketState], snapshot: &L2Snapshot) -> Result<RegimeClassifier, String> {
        let _analysis_start = Instant::now();
        let entropy = state.entropy_level.and_then(|e| e.to_f64()).unwrap_or(0.0);
        let symbol = &state.symbol;
        
        if let Some(price) = extract_mid_price_from_snapshot(snapshot) {
            self.record_price(symbol, state.timestamp, price);
        }
        
        let (optimal_entropy, std_dev, slope, mean_entropy) = {
            let mut cache = self.entropy_cache.entry(symbol.clone()).or_insert_with(Vec::new);
            cache.push(entropy);
            if cache.len() > HISTORY_SIZE {
                cache.remove(0);
            }
            
            let n = cache.len() as f64;
            let mean = cache.iter().sum::<f64>() / n;
            let variance = cache.iter()
                .map(|&x| (x - mean).powi(2))
                .sum::<f64>() / n;
            let std_dev = variance.sqrt();
            
            let slope = self.calculate_decay_slope(&cache);
            
            let mut sorted = cache.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let median = sorted[sorted.len() / 2];
            
            (median, std_dev, slope, mean)
        };
        
        let z_score = if std_dev > 0.0 {
            (entropy - optimal_entropy).abs() / std_dev
        } else {
            0.0
        };
        
        let mut conf = 1.0;
        
        if z_score > 2.0 {
            conf *= 0.8;
        }
        if z_score > 3.0 {
            conf *= 0.5;
        }
        
        let spread = state.volume_spread.to_f64().unwrap_or(0.0);
        if spread > SPREAD_THRESHOLD {
            conf *= 0.7;
        }
        
        let total_volume = extract_total_volume_from_snapshot(snapshot);
        let liquidity_score = (total_volume / MIN_LIQUIDITY).min(1.0);
        conf *= liquidity_score;
        
        let mut regime_consistency = 1.0;
        if history.len() >= 10 {
            let recent_regimes: Vec<&str> = history[history.len().saturating_sub(10)..]
                .iter()
                .filter_map(|s| s.regime.as_deref())
                .collect();
            
            if !recent_regimes.is_empty() {
                let current_regime = state.regime.as_deref().unwrap_or("");
                let same_regime_count = recent_regimes.iter()
                    .filter(|&&r| r == current_regime)
                    .count();
                regime_consistency = same_regime_count as f64 / recent_regimes.len() as f64;
                
                if regime_consistency < 0.7 {
                    conf *= regime_consistency;
                }
            }
        }
        
        let entropy_stability = if mean_entropy > 0.0 {
            1.0 / (1.0 + (std_dev / mean_entropy).abs())
        } else {
            0.5
        };
        conf *= entropy_stability;
        
        let pressure = state.pressure.to_f64().unwrap_or(0.0).abs();
        let raw_movement_energy = pressure / (entropy + 1e-9);
        let movement_energy = (raw_movement_energy + 1.0).ln_1p() * entropy_stability;
        
        let symmetry_score = 1.0 / (1.0 + z_score);
        
        Ok(RegimeClassifier {
            movement_energy,
            symmetry_score,
            decay_slope: slope,
            volatility_heat: state.temperature.to_f64().unwrap_or(0.0),
            confidence: conf.clamp(0.0, 1.0),
            z_score,
            regime_consistency,
            liquidity_score,
        })
    }
    
    async fn flush_csv(&self) -> anyhow::Result<()> {
        let mut writer = self.csv_writer.lock().await;
        writer.flush()?;
        Ok(())
    }
}

// ============================================================================
// LIVE CSV FLUSH TASK
// ============================================================================

async fn run_csv_flusher(physicist: Arc<ThermodynamicPhysicist>) {
    let mut interval = time::interval(Duration::from_millis(CSV_FLUSH_INTERVAL_MS));
    
    loop {
        interval.tick().await;
        if let Err(e) = physicist.flush_csv().await {
            eprintln!("❌ CSV flush failed: {}", e);
        }
    }
}

// ============================================================================
// HAUPTFUNKTION
// ============================================================================

#[derive(Debug, Deserialize)]
struct SymbolConfig {
    symbol: String,
    base_asset: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("{}", "=".repeat(80));
    println!("🚀 MBCT THERMODYNAMIC RESEARCH ENGINE v4.1.4");
    println!("💾 LIVE CSV WRITING ENABLED - FINAL VERSION");
    println!("📊 REAL-TIME VALIDATION & CORRELATION TRACKING");
    println!("{}", "=".repeat(80));
    
    let config_path = "config/mee_active_universe.json";
    let config_data = fs::read_to_string(config_path)
        .map_err(|e| anyhow::anyhow!("Failed to read universe config: {}", e))?;
    
    let universe: HashMap<String, SymbolConfig> = serde_json::from_str(&config_data)
        .map_err(|e| anyhow::anyhow!("Failed to parse universe config: {}", e))?;
    
    let symbols: Vec<String> = universe.values()
        .map(|cfg| cfg.base_asset.clone())
        .collect();
    
    println!("📦 Loaded {} symbols", symbols.len());
    
    let db_path = "e:/mbct/data/mbct_research.db";
    let db_conn = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))?
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .create_if_missing(true);
    
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(db_conn)
        .await
        .map_err(|e| anyhow::anyhow!("Database connection failed: {}", e))?;
    
    let repo = Arc::new(Repository::from_pool(pool));
    repo.ensure_market_states_table()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to ensure tables: {}", e))?;
    
    let physicist = Arc::new(ThermodynamicPhysicist::new().await?);
    let detector = EnvelopeDetector::new(20);
    
    let csv_flusher_handle = tokio::spawn(run_csv_flusher(physicist.clone()));
    
    let stats_handle = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(30));
        let mut last_processed = 0;
        let mut last_validations = 0;
        let mut last_csv_writes = 0;
        
        loop {
            interval.tick().await;
            let processed = PROCESSED_COUNT.load(Ordering::Relaxed);
            let validations = VALIDATION_RECORDS_COUNT.load(Ordering::Relaxed);
            let errors = ERROR_COUNT.load(Ordering::Relaxed);
            let csv_writes = CSV_WRITES_COUNT.load(Ordering::Relaxed);
            
            let processed_diff = processed - last_processed;
            let validations_diff = validations - last_validations;
            let csv_writes_diff = csv_writes - last_csv_writes;
            
            println!(
                "\n📈 STATS 30s: Processed: {} ({}/s) | Validations: {} ({}/s) | CSV: {} ({}/s) | Errors: {} ({:.1}%)",
                processed,
                processed_diff / 30,
                validations,
                validations_diff / 30,
                csv_writes,
                csv_writes_diff / 30,
                errors,
                if processed > 0 { errors as f64 / processed as f64 * 100.0 } else { 0.0 }
            );
            
            last_processed = processed;
            last_validations = validations;
            last_csv_writes = csv_writes;
        }
    });
    
    let mut ws = HyperliquidWs::new()
        .await
        .map_err(|e| anyhow::anyhow!("WebSocket connection failed: {}", e))?;
    
    let market_data = HyperliquidMarketData::new();
    
    println!("📡 Subscribing to symbols...");
    for symbol in &symbols {
        if let Err(e) = ws.subscribe_l2(symbol).await {
            eprintln!("⚠️  Failed to subscribe to {}: {}", symbol, e);
            continue;
        }
        time::sleep(Duration::from_millis(40)).await;
    }
    println!("✅ Subscriptions complete");
    
    let history_map: Arc<DashMap<String, Vec<MarketState>>> = Arc::new(DashMap::new());
    
    println!("{}", "=".repeat(80));
    println!("🔄 Starting live validation with CSV writing...");
    println!("💾 CSV file: e:/mbct/data/validation_live.csv");
    println!("{}", "=".repeat(80));
    
    let mut consecutive_errors = 0;
    
    loop {
        tokio::select! {
            snapshot = ws.next_snapshot() => {
                let processing_start = Instant::now();
                PROCESSED_COUNT.fetch_add(1, Ordering::Relaxed);
                
                match snapshot {
                    Some(l2_snapshot) => {
                        consecutive_errors = 0;
                        
                        let state = market_data.derive_market_state(&l2_snapshot);
                        let symbol = state.symbol.clone();
                        
                        let mut history = history_map.entry(symbol.clone()).or_insert_with(Vec::new);
                        history.push(state.clone());
                        
                        if history.len() > HISTORY_SIZE {
                            history.remove(0);
                        }
                        
                        let regime = detector.classify(&state, &history);
                        let mut state_with_regime = state.clone();
                        state_with_regime.regime = Some(regime.as_str().to_string());
                        
                        match physicist.analyze(&state_with_regime, &history, &l2_snapshot) {
                            Ok(metrics) => {
                                let processing_time = processing_start.elapsed();
                                
                                let validation_record = ValidationRecord::new(
                                    &state_with_regime,
                                    &metrics,
                                    &l2_snapshot,
                                    processing_time,
                                    Duration::from_secs(0)
                                );
                                
                                physicist.queue_validation_record(&symbol, validation_record);
                                
                                let processed = PROCESSED_COUNT.load(Ordering::Relaxed);
                                if processed % 200 == 0 {
                                    if let Some(stats) = physicist.correlation_stats.get(&symbol) {
                                        if stats.nrg_5s_samples > 0 {
                                            let signal_strength = if stats.nrg_5s_correlation.abs() > 0.3 { "💪" } 
                                                else if stats.nrg_5s_correlation.abs() > 0.2 { "👌" } 
                                                else { "🤏" };
                                            
                                            println!(
                                                "🔬 {} {}: Corr 5s: {:.3} | 10s: {:.3} | Samples: {} | Conf: {:.0}%",
                                                signal_strength,
                                                symbol,
                                                stats.nrg_5s_correlation,
                                                stats.nrg_10s_correlation,
                                                stats.nrg_5s_samples,
                                                metrics.confidence * 100.0
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                ERROR_COUNT.fetch_add(1, Ordering::Relaxed);
                                eprintln!("❌ Analysis failed for {}: {}", symbol, e);
                            }
                        }
                    }
                    None => {
                        ERROR_COUNT.fetch_add(1, Ordering::Relaxed);
                        consecutive_errors += 1;
                        
                        if consecutive_errors > 5 {
                            eprintln!("⚠️  Multiple connection errors, attempting reconnect...");
                            time::sleep(Duration::from_secs(5)).await;
                            
                            match HyperliquidWs::new().await {
                                Ok(new_ws) => {
                                    ws = new_ws;
                                    for symbol in &symbols {
                                        let _ = ws.subscribe_l2(symbol).await;
                                        time::sleep(Duration::from_millis(40)).await;
                                    }
                                    println!("✅ Reconnected and resubscribed");
                                    consecutive_errors = 0;
                                }
                                Err(e) => {
                                    eprintln!("❌ Reconnection failed: {}", e);
                                    time::sleep(Duration::from_secs(10)).await;
                                }
                            }
                        }
                    }
                }
            }
            
            _ = signal::ctrl_c() => {
                println!("\n{}", "=".repeat(80));
                println!("🛑 Shutdown signal received");
                
                println!("💾 Flushing CSV data...");
                if let Err(e) = physicist.flush_csv().await {
                    eprintln!("❌ Final CSV flush failed: {}", e);
                }
                
                println!("📊 Final Statistics:");
                println!("   Total processed: {}", PROCESSED_COUNT.load(Ordering::Relaxed));
                println!("   Complete validation records: {}", VALIDATION_RECORDS_COUNT.load(Ordering::Relaxed));
                println!("   CSV writes: {}", CSV_WRITES_COUNT.load(Ordering::Relaxed));
                println!("   Total errors: {}", ERROR_COUNT.load(Ordering::Relaxed));
                
                println!("\n📈 FINAL CORRELATION STATISTICS:");
                for entry in physicist.correlation_stats.iter() {
                    let symbol = entry.key();
                    let stats = entry.value();
                    if stats.nrg_5s_samples > 0 {
                        let significance = if stats.nrg_5s_correlation.abs() > 0.3 { "✅ SIGNIFICANT" }
                            else if stats.nrg_5s_correlation.abs() > 0.2 { "⚠️  MODERATE" }
                            else { "❌ WEAK" };
                        
                        println!("   {}: {} | 5s: {:.4} | 10s: {:.4} | Samples: {}",
                            symbol, significance, stats.nrg_5s_correlation, 
                            stats.nrg_10s_correlation, stats.nrg_5s_samples);
                    }
                }
                
                println!("{}", "=".repeat(80));
                
                drop(csv_flusher_handle);
                drop(stats_handle);
                
                println!("✅ Shutdown complete");
                break;
            }
        }
    }
    
    Ok(())
}