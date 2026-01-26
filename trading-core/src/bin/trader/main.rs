// E:\MBCT\trading-core\src\bin\trader\main.rs
// THE ALLIANCE - MBCT Main Control Unit v7.7 "Quantum-Res"
// Fokus: Dynamische JSON-Thresholds, Trailing-SL & Präzisions-Anzeige
// Vollständige Datei - compilierbar und ohne Platzhalter.

mod modules;

use dotenvy::dotenv;
use modules::{
    chronos::Chronos,
    collector::Collector,
    physicist::{Physicist, PhysicsState},
    regime::{RegimeClassifier, RegimeState},
};
use rust_decimal::prelude::*;
use serde::Deserialize;
use std::sync::atomic::{AtomicI64, Ordering};
use std::{
    collections::{HashMap, VecDeque},
    env, fs,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::mpsc,
    sync::Mutex,
    time::{sleep, timeout},
};
use trading_core::exchange::connector::HyperliquidConnector;

#[derive(Debug, PartialEq, Clone, Copy)]
enum TradeState {
    Flat,
    Observing,
    SetupDetected,
    PendingEntry,
    InPosition,
    Exiting,
    Cooldown,
}

#[derive(Deserialize, Clone, Debug)]
struct CoinProfile {
    pub symbol: String,
    pub allocation_weight: f64,
    #[allow(dead_code)]
    pub price_precision: u32,
    #[allow(dead_code)]
    pub volatility_factor: f64,
    #[allow(dead_code)]
    pub sens_long_trigger: f64,
    #[allow(dead_code)]
    pub sens_short_trigger: f64,
    pub nrg_long_threshold: f64,
    pub nrg_short_threshold: f64,
    pub slope_min: f64,
    pub cooldown_seconds: u64,
    pub entropy_max: f64,
    pub hard_stop_pct: f64,
    pub max_duration_seconds: u64,
    #[allow(dead_code)]
    pub optimal_raster: Vec<usize>,
}

struct ShlongMachine {
    state: TradeState,
    _symbol: String,
    entry_price: Option<f64>,
    is_long: bool,
    opened_at: Option<Instant>,
    last_action: Instant,
    is_executing: bool,
    executing_since: Option<Instant>,
    highest_pnl: f64,
}

impl ShlongMachine {
    fn new(symbol: String) -> Self {
        Self {
            state: TradeState::Flat,
            _symbol: symbol,
            entry_price: None,
            is_long: true,
            opened_at: None,
            last_action: Instant::now(),
            is_executing: false,
            executing_since: None,
            highest_pnl: 0.0,
        }
    }

    fn get_pnl(&self, current_price: f64) -> f64 {
        if let Some(entry) = self.entry_price {
            if entry == 0.0 { return 0.0; }
            let direction = if self.is_long { 1.0 } else { -1.0 };
            return ((current_price - entry) / entry) * 100.0 * direction;
        }
        0.0
    }

    fn update(
        &mut self,
        physics: &PhysicsState,
        regime: &RegimeState,
        profile: &CoinProfile,
        active_count: usize,
        buffer_ready: bool,
        chronos_hit: bool,
    ) {
        if self.is_executing {
            if let Some(start) = self.executing_since {
                if start.elapsed() > Duration::from_secs(10) {
                    self.is_executing = false;
                    self.executing_since = None;
                }
            }
            return;
        }

        // --- TRAILING SL LOGIK (v7.7) ---
        if self.state == TradeState::InPosition && physics.price > 0.0 {
            let pnl = self.get_pnl(physics.price);
            if pnl > self.highest_pnl {
                self.highest_pnl = pnl;
            }

            let mut should_exit = false;

            // 1. Hard Stop (aus JSON)
            if pnl < -profile.hard_stop_pct { should_exit = true; }

            // 2. Break-Even (Sicherung bei +0.12%)
            if self.highest_pnl > 0.12 && pnl < 0.02 { should_exit = true; }

            // 3. Trail (Abstand 0.15% ab 0.30% Profit)
            if self.highest_pnl > 0.30 && pnl < (self.highest_pnl - 0.15) { should_exit = true; }

            // 4. Take Profit (Thermodynamisches Limit)
            if pnl > 0.70 { should_exit = true; }

            // 5. Zeit-Limit
            let elapsed = self.opened_at.map(|t| t.elapsed().as_secs()).unwrap_or(0);
            if elapsed > profile.max_duration_seconds { should_exit = true; }

            if should_exit {
                self.state = TradeState::Exiting;
                self.last_action = Instant::now();
            }
        }

        // --- STATE MACHINE ---
        match self.state {
            TradeState::Flat => {
                self.state = TradeState::Observing;
                self.highest_pnl = 0.0;
            }
            TradeState::Observing => {
                // Dynamische Threshold-Prüfung aus JSON
                let nrg_valid = physics.nrg > profile.nrg_long_threshold || physics.nrg < profile.nrg_short_threshold;
                let slope_valid = regime.slope.abs() > profile.slope_min;
                let entropy_valid = physics.entropy < profile.entropy_max;

                if buffer_ready && active_count < 3 && nrg_valid && slope_valid && entropy_valid {
                    if chronos_hit {
                        self.state = TradeState::SetupDetected;
                        self.last_action = Instant::now();
                    }
                }
            }
            TradeState::SetupDetected => {
                if self.last_action.elapsed().as_secs() > 1 { // Kurze Bestätigung
                    self.state = TradeState::PendingEntry;
                    self.last_action = Instant::now();
                }
            }
            TradeState::Cooldown => {
                if self.last_action.elapsed().as_secs() > profile.cooldown_seconds {
                    self.state = TradeState::Flat;
                    self.highest_pnl = 0.0;
                }
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let pk = env::var("HL_PRIVATE_KEY").expect("HL_PRIVATE_KEY missing");
    let main_addr = env::var("HL_MAIN_ADDRESS").expect("HL_MAIN_ADDRESS missing");
    let is_testnet = env::var("IS_TESTNET").unwrap_or("true".to_string()) == "true";

    let conn = Arc::new(HyperliquidConnector::new(&pk, is_testnet)?);
    let collector = Arc::new(Collector::new(is_testnet));
    let chronos_arc = Arc::new(Mutex::new(Chronos::new()));
    let account_value = Arc::new(AtomicI64::new(0));

    let profiles_raw = fs::read_to_string("E:/MBCT/data/coin_profiles.json")?;
    let profiles: Vec<CoinProfile> = serde_json::from_str(&profiles_raw)?;
    let profile_map: HashMap<String, CoinProfile> = profiles.iter().map(|p| (p.symbol.clone(), p.clone())).collect();

    let machines_map = Arc::new(Mutex::new(
        profiles.iter().map(|p| (p.symbol.clone(), ShlongMachine::new(p.symbol.clone()))).collect::<HashMap<String, ShlongMachine>>()
    ));
    let histories_map = Arc::new(Mutex::new(HashMap::<String, VecDeque<PhysicsState>>::new()));

    let (tx_order_res, mut rx_order_res) = mpsc::channel::<(String, bool, f64, bool)>(100);

    // Account Watcher
    let conn_acc = conn.clone();
    let acc_val = account_value.clone();
    let addr_acc = main_addr.clone();
    tokio::spawn(async move {
        loop {
            if let Ok(info) = conn_acc.get_user_state(&addr_acc).await {
                let val = info.withdrawable_equity.to_f64().unwrap_or(0.0);
                acc_val.store((val * 100.0) as i64, Ordering::Relaxed);
            }
            sleep(Duration::from_secs(10)).await;
        }
    });

    let c_listen = collector.clone();
    let symbols: Vec<String> = profiles.iter().map(|p| p.symbol.clone()).collect();
    tokio::spawn(async move {
        c_listen.stream_provider(symbols).await;
    });

    let c_heart = collector.clone();
    let h_arc = histories_map.clone();
    let m_arc = machines_map.clone();
    let co_arc = conn.clone();
    let tx_res = tx_order_res.clone();
    let chr_arc = chronos_arc.clone();
    let p_map_heart = profile_map.clone();

    tokio::spawn(async move {
        c_heart.heartbeat_loop(move |updates| {
            let h_lock = h_arc.clone();
            let m_lock = m_arc.clone();
            let p_map = p_map_heart.clone();
            let co_call = co_arc.clone();
            let tx_call = tx_res.clone();
            let chr_lock = chr_arc.clone();

            async move {
                let mut h_map = h_lock.lock().await;
                let mut m_map = m_lock.lock().await;
                let mut chr_map = chr_lock.lock().await;
                
                let active_trades = m_map.values().filter(|m| m.state == TradeState::InPosition).count();

                for (symbol, snapshot) in updates {
                    let physics = Physicist::process_snapshot(&snapshot);
                    
                    let hist = h_map.entry(symbol.clone()).or_insert_with(|| VecDeque::with_capacity(90));
                    hist.push_back(physics.clone());
                    if hist.len() > 90 { hist.pop_front(); }

                    let classifier = RegimeClassifier::new(90);
                    let regime = classifier.classify(hist);
                    let ready = hist.len() >= 90;

                    let hit = chr_map.observe_potential_hit(&symbol, &physics, &regime, 0.15, 0.85);

                    if let (Some(m), Some(profile)) = (m_map.get_mut(&symbol), p_map.get(&symbol)) {
                        m.update(&physics, &regime, profile, active_trades, ready, hit);

                        if (m.state == TradeState::PendingEntry || m.state == TradeState::Exiting) && !m.is_executing {
                            m.is_executing = true;
                            m.executing_since = Some(Instant::now());
                            let is_entry = m.state == TradeState::PendingEntry;
                            let is_long = if is_entry { regime.symmetry_score < 0.5 } else { m.is_long };
                            
                            // Quantisierte Size-Berechnung
                            let size = Decimal::from_f64((12.0 / physics.price.max(0.000001)) * profile.allocation_weight).unwrap_or(Decimal::ZERO).round_dp(2);

                            let s_order = symbol.clone();
                            let p_now = physics.price;
                            let co_call_inner = co_call.clone();
                            let tx_call_inner = tx_call.clone();
                            tokio::spawn(async move {
                                let res = timeout(Duration::from_secs(6), co_call_inner.place_market_order(&s_order, is_long, size, None)).await;
                                let success = matches!(res, Ok(Ok(_)));
                                let _ = tx_call_inner.send((s_order, success, p_now, is_entry)).await;
                            });
                        }
                    }
                }
            }
        }).await;
    });

    loop {
        while let Ok((sym, ok, price, entry)) = rx_order_res.try_recv() {
            let mut m_map = machines_map.lock().await;
            if let Some(m) = m_map.get_mut(&sym) {
                m.is_executing = false;
                if ok {
                    m.state = if entry { TradeState::InPosition } else { TradeState::Cooldown };
                    if entry {
                        m.entry_price = Some(price);
                        m.opened_at = Some(Instant::now());
                        m.highest_pnl = 0.0;
                    }
                } else {
                    m.state = if entry { TradeState::Observing } else { TradeState::InPosition };
                }
                m.last_action = Instant::now();
            }
        }

        {
            let m_map = machines_map.lock().await;
            let h_map = histories_map.lock().await;
            let stats = collector.get_stats();
            let rec = stats.0;
            let equity = account_value.load(Ordering::Relaxed) as f64 / 100.0;

            print!("{}[H", 27 as char);
            println!("╔══════════════════════════════════════════════════════════════════════════════════════════╗");
            println!("║ 🛡️  THE ALLIANCE v7.7 | WS-RCV: {:<10} | EQUITY: {:>10.2} USD                ║", rec, equity);
            println!("╠══════════════════════════════════════════════════════════════════════════════════════════╣");
            println!("║ SYMBOL   | PRICE        | SYM   | Z-NRG  | PnL %   | MAX % | STATE                     ║");
            println!("╟──────────┼────────────┼───────┼────────┼─────────┼───────┼───────────────────────────╢");

            let mut keys: Vec<_> = m_map.keys().collect();
            keys.sort();
            for k in keys {
                if let (Some(m), Some(h), Some(profile)) = (m_map.get(k), h_map.get(k), profile_map.get(k)) {
                    let last_p = h.back().cloned().unwrap_or_default();
                    let z_nrg = RegimeClassifier::calculate_z_score(last_p.nrg, h, "nrg");
                    let classifier = RegimeClassifier::new(90);
                    let reg = classifier.classify(h);
                    let pnl = if m.state == TradeState::InPosition { format!("{:>+7.2}%", m.get_pnl(last_p.price)) } else { "---".to_string() };
                    let max_pnl = if m.state == TradeState::InPosition { format!("{:>+5.2}%", m.highest_pnl) } else { "---".to_string() };

                    // Dynamische Präzision für die Anzeige
                    let prec = profile.price_precision as usize;
                    println!(
                        "║ {:<8} | {:<12.*} | {:<5.3} | {:>+6.1} | {:<7} | {:<5} | {:<25} ║",
                        k, prec, last_p.price, reg.symmetry_score, z_nrg, pnl, max_pnl, format!("{:?}", m.state)
                    );
                }
            }
            println!("╚══════════════════════════════════════════════════════════════════════════════════════════╝");
        }
        sleep(Duration::from_millis(600)).await;
    }
}