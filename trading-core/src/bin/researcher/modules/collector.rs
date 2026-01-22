// E:\MBCT\trading-core\src\bin\researcher\modules\collector.rs
// THE ALLIANCE - MBCT Collector Modul
// Fokus: Präzises 100ms Sampling & WebSocket Ingestion (Library-konform)

use dashmap::DashMap;
use std::sync::Arc;
use tokio::time::{self, Duration};
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

    /// Gibt die aktuellen Statistiken (Empfangen, Gesampelt) zurück.
    pub fn get_stats(&self) -> (usize, usize) {
        (
            self.stats.messages_received.load(Ordering::Relaxed),
            self.stats.snapshots_sampled.load(Ordering::Relaxed),
        )
    }

    pub async fn stream_provider(self: Arc<Self>, symbols: Vec<String>) {
        println!("[COLLECTOR] Verbinde mit HyperLiquid WebSocket...");
        
        let ws_result = HyperliquidWs::new().await;
        
        match ws_result {
            Ok(mut ws) => {
                // Symbole einzeln abonnieren
                for symbol in &symbols {
                    if let Err(e) = ws.subscribe_l2(symbol).await {
                        eprintln!("[COLLECTOR] Fehler beim Abo für {}: {:?}", symbol, e);
                    }
                }

                println!("[COLLECTOR] Stream gestartet für {} Symbole", symbols.len());

                loop {
                    match ws.next_snapshot().await {
                        Some(snapshot) => {
                            self.stats.messages_received.fetch_add(1, Ordering::Relaxed);
                            // Nutzt das 'coin' Feld aus dem L2Snapshot als Key
                            self.market_data.insert(snapshot.coin.clone(), snapshot);
                        }
                        None => {
                            eprintln!("[COLLECTOR] Stream unterbrochen. Versuche Reconnect in 5s...");
                            time::sleep(Duration::from_secs(5)).await;
                            break; 
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[COLLECTOR] Kritischer Initialisierungsfehler: {:?}", e);
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