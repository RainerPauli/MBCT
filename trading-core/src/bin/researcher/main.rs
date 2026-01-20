// E:\MBCT\trading-core\src\bin\researcher\main.rs
// THE ALLIANCE - MBCT Researcher Main Engine
// Fokus: Sauberes Terminal durch Discovery-Throttling

mod modules;

use modules::collector::Collector;
use modules::physicist::{Physicist, PhysicsState};
use modules::regime::{RegimeClassifier};
use modules::chronos::Chronos;
use modules::archive::Archive;

use trading_core::config::Settings;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use std::collections::{VecDeque, HashMap};
use tokio::signal;
use std::fs;
use std::time::{Instant, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 MBCT SIGNAL-RESEARCHER STARTING...");
    println!("🛡️ THE ALLIANCE - DYNAMIC MODE");

    let settings = Settings::new().expect("Konnte Konfiguration nicht laden");
    let data_dir = "E:/MBCT/data";
    let _ = fs::create_dir_all(data_dir);

    let db_path = format!("sqlite:{}/researcher.db", data_dir);
    let csv_path = format!("{}/researcher.csv", data_dir);

    let symbols = settings.symbols.clone();
    let collector = Arc::new(Collector::new());
    let classifier = Arc::new(RegimeClassifier::new(20)); 
    let chronos = Arc::new(Mutex::new(Chronos::new()));
    let archive = Arc::new(Archive::new(&db_path, &csv_path).await);
    let histories: Arc<dashmap::DashMap<String, VecDeque<PhysicsState>>> = Arc::new(dashmap::DashMap::new());

    // Gedächtnis für Terminal-Ausgaben (Asset -> Letzter Log-Zeitpunkt)
    let log_throttle: Arc<Mutex<HashMap<String, Instant>>> = Arc::new(Mutex::new(HashMap::new()));

    let (tx, mut rx) = mpsc::channel(50000); 
    let archive_worker = Arc::clone(&archive);
    
    tokio::spawn(async move {
        while let Some(records) = rx.recv().await {
            archive_worker.store_records(records).await;
        }
    });

    let collector_clone = Arc::clone(&collector);
    let symbols_clone = symbols.clone();
    tokio::spawn(async move {
        collector_clone.stream_provider(symbols_clone).await;
    });

    println!("💓 Heartbeat active | 📊 Monitoring {} Assets", symbols.len());
    println!("🔭 Discovery-Filter: >0.5% Return | 10s Cooldown pro Asset");

    let collector_handle = Arc::clone(&collector);
    let chronos_main = Arc::clone(&chronos);
    let classifier_main = Arc::clone(&classifier);
    let histories_main = Arc::clone(&histories);
    let throttle_main = Arc::clone(&log_throttle);

    tokio::spawn(async move {
        collector_handle.heartbeat_loop(move |symbol, snapshot| {
            let s_name = symbol.clone();
            let snap = snapshot;
            let chr = Arc::clone(&chronos_main);
            let cls = Arc::clone(&classifier_main);
            let hist_map = Arc::clone(&histories_main);
            let thr = Arc::clone(&throttle_main);
            let tx_channel = tx.clone();

            tokio::spawn(async move {
                let current_physics = Physicist::process_snapshot(&snap);
                
                let mut history = hist_map.entry(s_name.clone()).or_insert_with(|| VecDeque::with_capacity(100));
                if history.len() >= 100 { history.pop_front(); }
                history.push_back(current_physics.clone());
                let current_regime = cls.classify(&history);
                drop(history);

                let mut chronos_lock = chr.lock().await;
                chronos_lock.register_observation(&s_name, current_physics.clone(), current_regime);
                let completed_records = chronos_lock.update_and_flush(&s_name, current_physics.price);
                drop(chronos_lock);

                if !completed_records.is_empty() {
                    for rec in &completed_records {
                        if let Some(r89) = rec.ret_89s {
                            if r89.abs() > 0.5 {
                                let mut throttle_lock = thr.lock().await;
                                let last_log = throttle_lock.get(&rec.symbol);
                                
                                // Nur loggen, wenn neu oder Cooldown abgelaufen
                                if last_log.is_none() || last_log.unwrap().elapsed() > Duration::from_secs(10) {
                                    let icon = if r89 > 0.0 { "📈" } else { "📉" };
                                    println!("✨ [DISCOVERY] {} {:>7} | Return: {:>+6.3}% | Regime: {:?}", 
                                        icon, rec.symbol, r89, rec.regime.regime);
                                    throttle_lock.insert(rec.symbol.clone(), Instant::now());
                                }
                            }
                        }
                    }
                    let _ = tx_channel.send(completed_records).await;
                }
            });
        }).await;
    });

    signal::ctrl_c().await?;
    println!("\n🛑 Shutdown Signal. THE ALLIANCE sichert Daten...");
    tokio::time::sleep(Duration::from_secs(2)).await;
    Ok(())
}