// E:\MBCT\trading-core\src\bin\trader\modules\collector.rs
// THE ALLIANCE - MBCT Collector v4.6 "Researcher-Sync"
// Simplified to match the working researcher implementation

use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::time::{self, timeout, Duration};
use trading_core::exchange::ws::HyperliquidWs;
use trading_core::exchange::L2Snapshot;

pub struct CollectorStats {
    pub messages_received: AtomicUsize,
}

pub struct Collector {
    pub market_data: Arc<DashMap<String, L2Snapshot>>,
    pub stats: Arc<CollectorStats>,
    is_testnet: bool,
}

impl Collector {
    pub fn new(is_testnet: bool) -> Self {
        Self {
            market_data: Arc::new(DashMap::new()),
            stats: Arc::new(CollectorStats {
                messages_received: AtomicUsize::new(0),
            }),
            is_testnet,
        }
    }

    pub fn get_stats(&self) -> (usize, usize) {
        let received = self.stats.messages_received.load(Ordering::Relaxed);
        (received, 0)
    }

    pub async fn stream_provider(
        self: Arc<Self>,
        symbols: Vec<String>,
    ) {
        loop {
            println!("[COLLECTOR] Allianz-Kanal wird aufgebaut...");

            let ws_result = HyperliquidWs::new(self.is_testnet).await;

            match ws_result {
                Ok(mut ws) => {
                    // Settle time for connection
                    tokio::time::sleep(Duration::from_secs(1)).await;

                    // Subscribe to all symbols (simple loop like researcher)
                    for symbol in &symbols {
                        if let Err(e) = ws.subscribe_l2(symbol).await {
                            eprintln!("[COLLECTOR] Abo-Fehler für {}: {:?}", symbol, e);
                        }
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }

                    println!("[COLLECTOR] ✅ Stream aktiv. Watchdog (30s).");

                    loop {
                        // Watchdog timeout like researcher
                        let next_res = timeout(Duration::from_secs(30), ws.next_snapshot()).await;

                        match next_res {
                            Ok(Some(snapshot)) => {
                                self.stats.messages_received.fetch_add(1, Ordering::Relaxed);
                                self.market_data.insert(snapshot.coin.clone(), snapshot);
                            }
                            Ok(None) => {
                                eprintln!("[COLLECTOR] Stream-Ende. Reconnect...");
                                break;
                            }
                            Err(_) => {
                                eprintln!("[COLLECTOR] 🚨 Watchdog (30s). Reconnect...");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[COLLECTOR] Verbindungsfehler: {:?}. Retry in 10s...", e);
                    time::sleep(Duration::from_secs(10)).await;
                }
            }
        }
    }

    pub async fn heartbeat_loop<F, Fut>(self: Arc<Self>, mut callback: F)
    where
        F: FnMut(Vec<(String, L2Snapshot)>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let mut interval = time::interval(Duration::from_millis(100));
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        println!("[COLLECTOR] Heartbeat Loop (100ms) aktiv.");

        loop {
            interval.tick().await;
            let updates: Vec<(String, L2Snapshot)> = self.market_data.iter()
                .map(|entry| (entry.key().clone(), entry.value().clone()))
                .collect();
            
            if !updates.is_empty() {
                callback(updates).await;
            }
        }
    }
}
