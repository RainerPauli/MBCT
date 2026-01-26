// E:\MBCT\trading-core\src\bin\researcher\main.rs
mod modules;

use modules::archive::Archive;
use modules::chronos::Chronos;
use modules::collector::Collector;
use modules::physicist::{Physicist, PhysicsState};
use modules::regime::RegimeClassifier;

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tokio::sync::{mpsc, watch, Mutex};

fn clear_screen() {
    print!("{}[2J{}[1;1H", 27 as char, 27 as char);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sens_path = "e:/mbct/data/sens_config_top18.json";
    let sens_content = fs::read_to_string(sens_path).expect("❌ SENS-Konfiguration fehlt!");
    let sens_data: serde_json::Value = serde_json::from_str(&sens_content)?;

    let mut sens_map_internal = HashMap::new();
    let mut symbols = Vec::new();
    if let Some(list) = sens_data.as_array() {
        for item in list {
            if let Some(sym) = item["symbol"].as_str() {
                symbols.push(sym.to_string());
                sens_map_internal.insert(sym.to_string(), item.clone());
            }
        }
    }

    let sens_map = Arc::new(sens_map_internal);
    let collector = Arc::new(Collector::new());
    let archive = Arc::new(
        Archive::new(
            "sqlite:e:/mbct/data/researcher_v2.db",
            "e:/mbct/data/researcher_v2.csv",
        )
        .await?,
    );
    let chronos = Arc::new(Mutex::new(Chronos::new()));
    let classifier = Arc::new(RegimeClassifier::new(21));
    let histories: Arc<Mutex<HashMap<String, VecDeque<PhysicsState>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let ui_events: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::with_capacity(10)));

    let (shutdown_tx, _shutdown_rx) = watch::channel(false);
    let (tx, mut rx) = mpsc::channel(10000);

    clear_screen();

    let stats_collector = collector.clone();
    let chronos_monitor = chronos.clone();
    let ui_log = ui_events.clone();
    let mut ui_shutdown = shutdown_tx.subscribe();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(200));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let (rcv, smp) = stats_collector.get_stats();
                    let pending = chronos_monitor.lock().await.get_pending_count();
                    let logs = ui_log.lock().await;

                    print!("{}[1;1H", 27 as char);
                    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
                    println!("║  🛡️  THE ALLIANCE - QUANTUM RESEARCHER CENTER v2.6                           ║");
                    println!("╠══════════════════════════════════════════════════════════════════════════════╣");
                    println!("║  INGESTED: {:<12} | SAMPLED: {:<12} | PENDING: {:<11} ║", rcv, smp, pending);
                    println!("╠══════════════════════════════════════════════════════════════════════════════╣");
                    println!("║  SYMBOL      | SYMMETRY  | PRICE          | REGIME       | STATUS            ║");
                    println!("╟──────────────┼───────────┼────────────────┼──────────────┼──────────────────╢");
                    for i in 0..10 {
                        if let Some(line) = logs.get(i) { println!("║ {:<76} ║", line); }
                        else { println!("║ {:<76} ║", ""); }
                    }
                    println!("╚══════════════════════════════════════════════════════════════════════════════╝");
                }
                _ = ui_shutdown.changed() => break,
            }
        }
    });

    let archive_worker = archive.clone();
    let archive_handle = tokio::spawn(async move {
        while let Some(records) = rx.recv().await {
            let _ = archive_worker.store_batch(records).await;
        }
    });

    let collector_clone = collector.clone();
    let symbols_clone = symbols.clone();
    tokio::spawn(async move {
        collector_clone.stream_provider(symbols_clone).await;
    });

    let tx_channel = tx.clone();
    let histories_lock = histories.clone();
    let chronos_lock = chronos.clone();
    let classifier_arc = classifier.clone();
    let sens_ref = sens_map.clone();
    let ui_event_log = ui_events.clone();
    let heart_shutdown = shutdown_tx.subscribe();

    let heartbeat_handle = tokio::spawn(async move {
        collector
            .heartbeat_loop(move |symbol, snapshot| {
                if *heart_shutdown.borrow() {
                    return;
                }

                let s_name = symbol.clone();
                let current_physics = Physicist::process_snapshot(&snapshot);
                let s_config = sens_ref.get(&s_name).cloned();
                let h_lock = histories_lock.clone();
                let c_lock = chronos_lock.clone();
                let tx_chan = tx_channel.clone();
                let classifier_ref = classifier_arc.clone();
                let ui_log_trigger = ui_event_log.clone();

                tokio::spawn(async move {
                    let mut hist = h_lock.lock().await;
                    let entry = hist
                        .entry(s_name.clone())
                        .or_insert_with(|| VecDeque::with_capacity(100));
                    entry.push_back(current_physics.clone());
                    if entry.len() > 89 {
                        entry.pop_front();
                    }

                    let regime_state = classifier_ref.classify(entry);
                    let z_scores = (
                        RegimeClassifier::calculate_z_score(
                            current_physics.entropy,
                            entry,
                            "entropy",
                        ),
                        RegimeClassifier::calculate_z_score(
                            current_physics.pressure,
                            entry,
                            "pressure",
                        ),
                        RegimeClassifier::calculate_z_score(current_physics.nrg, entry, "nrg"),
                    );

                    if let Some(cfg) = s_config {
                        let l_floor = cfg["sens_long_trigger"].as_f64().unwrap_or(0.40);
                        let s_ceiling = cfg["sens_short_trigger"].as_f64().unwrap_or(0.60);

                        let mut c_guard = c_lock.lock().await;
                        if c_guard.observe_potential_hit(
                            &s_name,
                            &current_physics,
                            &regime_state,
                            l_floor,
                            s_ceiling,
                        ) {
                            let log_line = format!(
                                "{:<11} | {:<9.3} | {:<14.4} | {:<12?} | LOCKED ✅",
                                s_name,
                                regime_state.symmetry_score,
                                current_physics.price,
                                regime_state.regime
                            );
                            let mut ui_guard = ui_log_trigger.lock().await;
                            ui_guard.push_front(log_line);
                            if ui_guard.len() > 10 {
                                ui_guard.pop_back();
                            }
                        }

                        let completed_records = c_guard.update_and_flush(
                            &s_name,
                            current_physics.price,
                            z_scores,
                            z_scores,
                        );
                        if !completed_records.is_empty() {
                            let _ = tx_chan.send(completed_records).await;
                        }
                    }
                });
            })
            .await;
    });

    signal::ctrl_c().await?;
    println!("\n🛑 Shutdown-Signal empfangen...");
    let _ = shutdown_tx.send(true);
    heartbeat_handle.abort();
    drop(tx);
    let _ = archive_handle.await;
    println!("✅ System sauber beendet.");
    Ok(())
}
