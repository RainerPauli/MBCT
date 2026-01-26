// E:\mbct\trading-core\src\bin\signaler.rs
// MBCT BROAD-SPECTRUM SIGNALER v1.2.6
// THE ALLIANCE - Full File Recovery & Exhaustion Detection

use chrono::Local;
use dashmap::DashMap;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use serde::Deserialize;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};

// Interne MBCT Module
use trading_common::data::types::MarketState;
use trading_core::exchange::envelope_detection::{EnvelopeDetector, MarketRegime};
use trading_core::exchange::types::L2Snapshot;
use trading_core::exchange::ws::HyperliquidWs;

// ============================================================================
// KONFIGURATION
// ============================================================================
const META_PATH: &str = "hl_meta_full.json";
const HISTORY_SIZE: usize = 50;

#[derive(Deserialize, Debug)]
struct UniverseAsset {
    name: String,
    #[serde(rename = "isDelisted", default)]
    is_delisted: bool,
}

#[derive(Deserialize, Debug)]
struct HLMeta {
    universe: Vec<UniverseAsset>,
}

// ============================================================================
// MBCT THERMODYNAMIK KERN
// ============================================================================
struct SignalerPhysicist {
    history: DashMap<String, Vec<MarketState>>,
    detector: EnvelopeDetector,
}

impl SignalerPhysicist {
    fn new() -> Self {
        Self {
            history: DashMap::new(),
            detector: EnvelopeDetector::new(HISTORY_SIZE),
        }
    }

    fn process_snapshot(&self, snapshot: &L2Snapshot) -> Option<(String, MarketRegime, f64)> {
        let symbol = snapshot.coin.clone();

        let (bid_vol, ask_vol) = self.extract_volumes(snapshot);
        let total_vol = bid_vol + ask_vol;
        if total_vol == 0.0 {
            return None;
        }

        let pressure = (bid_vol - ask_vol).abs() / total_vol;
        let entropy = self.calculate_entropy(snapshot);

        let state = MarketState {
            timestamp: Local::now().timestamp_millis(),
            symbol: symbol.clone(),
            pressure: Decimal::from_f64(pressure).unwrap_or_else(|| Decimal::new(0, 0)),
            entropy_level: Some(Decimal::from_f64(entropy).unwrap_or_else(|| Decimal::new(0, 0))),
            regime: None,
            temperature: Decimal::from_f64(pressure * 100.0).unwrap_or_else(|| Decimal::new(0, 0)),
            volume_spread: Decimal::from_f64(total_vol).unwrap_or_else(|| Decimal::new(0, 0)),
        };

        let mut hist = self.history.entry(symbol.clone()).or_insert_with(Vec::new);
        hist.push(state.clone());
        if hist.len() > HISTORY_SIZE {
            hist.remove(0);
        }

        let regime = self.detector.classify(&state, &hist);
        let nrg = self.calculate_nrg(&hist);

        Some((symbol, regime, nrg))
    }

    fn extract_volumes(&self, snapshot: &L2Snapshot) -> (f64, f64) {
        let mut bv = 0.0;
        let mut av = 0.0;
        if snapshot.levels.len() >= 2 {
            for l in &snapshot.levels[0] {
                bv += l.sz.parse::<f64>().unwrap_or(0.0);
            }
            for l in &snapshot.levels[1] {
                av += l.sz.parse::<f64>().unwrap_or(0.0);
            }
        }
        (bv, av)
    }

    fn calculate_entropy(&self, snapshot: &L2Snapshot) -> f64 {
        let mut volumes = Vec::new();
        for side in &snapshot.levels {
            for l in side {
                let sz = l.sz.parse::<f64>().unwrap_or(0.0);
                if sz > 0.0 {
                    volumes.push(sz);
                }
            }
        }
        let total: f64 = volumes.iter().sum();
        if total == 0.0 {
            return 0.0;
        }

        volumes
            .iter()
            .map(|v| v / total)
            .map(|p| -p * p.log2())
            .sum()
    }

    fn calculate_nrg(&self, history: &[MarketState]) -> f64 {
        if history.len() < 2 {
            return 0.0;
        }
        let last = history.last().unwrap();
        let prev = &history[history.len() - 2];
        let dp = (last.pressure - prev.pressure)
            .to_f64()
            .unwrap_or(0.0)
            .abs();
        dp * 1000.0
    }
}

// ============================================================================
// MAIN EXECUTION LOOP
// ============================================================================
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("📡 MBCT SIGNALER v1.2.6 - ALLIANCE EXHAUSTION SCAN");

    let raw_bytes =
        fs::read(META_PATH).map_err(|e| anyhow::anyhow!("Meta-File nicht lesbar: {}", e))?;
    let content = std::str::from_utf8(if raw_bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &raw_bytes[3..]
    } else {
        &raw_bytes
    })?;

    let meta: HLMeta = serde_json::from_str(content)?;
    let active_coins: Vec<String> = meta
        .universe
        .into_iter()
        .filter(|a| !a.is_delisted)
        .map(|a| a.name)
        .collect();

    println!(
        "✅ Universum stabil. Monitoring {} Assets.",
        active_coins.len()
    );

    let mut ws = HyperliquidWs::new().await?;
    let physicist = Arc::new(SignalerPhysicist::new());

    for coin in &active_coins {
        let _ = ws.subscribe_l2(coin).await;
    }

    let mut last_log = Instant::now();
    let mut signal_count = 0;
    let mut total_snapshots = 0;
    let mut top_nrg: (String, f64, MarketRegime) = (String::new(), 0.0, MarketRegime::Oscillatory);

    loop {
        if let Some(snapshot) = ws.next_snapshot().await {
            total_snapshots += 1;
            if let Some((symbol, regime, nrg)) = physicist.process_snapshot(&snapshot) {
                // Leaderboard tracking
                if nrg > top_nrg.1 {
                    top_nrg = (symbol.clone(), nrg, regime.clone());
                }

                // TRIGGER: Rückkehr von Ballistic in Oscillatory Habitat
                if regime == MarketRegime::Oscillatory && nrg > 30.0 {
                    signal_count += 1;
                    let ts = Local::now().format("%H:%M:%S").to_string();
                    println!(
                        "[{}] 🎯 EXHAUSTION: {:<8} | NRG: {:.2} | Re-Entry in Habitat",
                        ts, symbol, nrg
                    );
                }
            }
        }

        if last_log.elapsed() > Duration::from_secs(10) {
            let ts = Local::now().format("%H:%M:%S").to_string();
            println!(
                "[{}] 📊 SCANNER: {} Coins | Snaps: {} | Hot: {} ({:.1}, {:?}) | Signals: {}",
                ts,
                active_coins.len(),
                total_snapshots,
                top_nrg.0,
                top_nrg.1,
                top_nrg.2,
                signal_count
            );

            signal_count = 0;
            total_snapshots = 0;
            top_nrg = (String::new(), 0.0, MarketRegime::Oscillatory);
            last_log = Instant::now();
        }
    }
}
