// E:\MBCT\trading-core\src\bin\researcher\modules\collector.rs
// THE ALLIANCE - MBCT Collector Modul v2.7
// Fokus: Watchdog-geschütztes 100ms Sampling & Auto-Reconnect

use dashmap::DashMap;
use std::sync::Arc;
use tokio::time::{self, Duration, timeout};
use trading_core::exchange::ws::HyperliquidWs;
use trading_core::exchange::L2Snapshot;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct CollectorStats {
    pub messages_received: AtomicUsize,
    pub snapshots_sampled: AtomicUsize,
}

pub struct Collector {
    pub market_data: Arc<DashMap<String, L2Snapshot>>,
    pub stats: Arc<CollectorStats>,
}

impl Collector {
    pub fn new() -> Self {
        Self {
            market_data: Arc::new(DashMap::new()),
            stats: Arc::new(CollectorStats {
                messages_received: AtomicUsize::new(0),
                snapshots_sampled: AtomicUsize::new(0),
            }),
        }
    }

    pub fn get_stats(&self) -> (usize, usize) {
        (
            self.stats.messages_received.load(Ordering::Relaxed),
            self.stats.snapshots_sampled.load(Ordering::Relaxed),
        )
    }

    pub async fn stream_provider(self: Arc<Self>, symbols: Vec<String>) {
        loop {
            println!("[COLLECTOR] Allianz-Kanal wird aufgebaut (HyperLiquid)...");
            
            let ws_result = HyperliquidWs::new().await;
            
            match ws_result {
                Ok(mut ws) => {
                    for symbol in &symbols {
                        if let Err(e) = ws.subscribe_l2(symbol).await {
                            eprintln!("[COLLECTOR] Abo-Fehler für {}: {:?}", symbol, e);
                        }
                    }

                    println!("[COLLECTOR] Stream aktiv. Watchdog scharf geschaltet (30s).");

                    loop {
                        // Der entscheidende Watchdog: 30s Timeout für den nächsten Snapshot
                        let next_res = timeout(Duration::from_secs(30), ws.next_snapshot()).await;

                        match next_res {
                            Ok(Some(snapshot)) => {
                                self.stats.messages_received.fetch_add(1, Ordering::Relaxed);
                                self.market_data.insert(snapshot.coin.clone(), snapshot);
                            }
                            Ok(None) => {
                                eprintln!("[COLLECTOR] Stream-Ende detektiert. Reconnect...");
                                break;
                            }
                            Err(_) => {
                                eprintln!("[COLLECTOR] 🚨 WATCHDOG: Silent Timeout! Keine Daten seit 30s. Erzwinge Reconnect...");
                                break; // Bricht den inneren Loop ab -> Reconnect
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[COLLECTOR] Verbindungsfehler: {:?}. Versuch in 10s...", e);
                    time::sleep(Duration::from_secs(10)).await;
                }
            }
        }
    }

    pub async fn heartbeat_loop<F>(self: Arc<Self>, mut callback: F) 
    where 
        F: FnMut(String, L2Snapshot) + Send + 'static 
    {
        let mut interval = time::interval(Duration::from_millis(100));
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        println!("[COLLECTOR] Heartbeat Loop (100ms) aktiv.");

        loop {
            interval.tick().await;
            for entry in self.market_data.iter() {
                let symbol = entry.key().clone();
                let snapshot = entry.value().clone();
                self.stats.snapshots_sampled.fetch_add(1, Ordering::Relaxed);
                callback(symbol, snapshot);
            }
        }
    }
}