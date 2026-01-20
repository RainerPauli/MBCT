// E:\MBCT\trading-core\src\bin\researcher\modules\collector.rs
// THE ALLIANCE - MBCT Collector Modul
// Fokus: Präzises 100ms Sampling & WebSocket Ingestion

use dashmap::DashMap;
use std::sync::Arc;
use tokio::time::{self, Duration};
use trading_core::exchange::ws::HyperliquidWs;
use trading_core::exchange::L2Snapshot;
use std::sync::atomic::{AtomicUsize, Ordering};

// Statistik-Counter für den kybernetischen Loop
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

    /// Startet den HyperLiquid Stream und füllt die DashMap mit den neuesten Snapshots
    pub async fn stream_provider(self: Arc<Self>, symbols: Vec<String>) {
        println!("[COLLECTOR] Verbinde mit HyperLiquid WebSocket...");
        
        // Initialisierung des WS laut ws.rs
        let ws_result = HyperliquidWs::new().await;
        
        match ws_result {
            Ok(mut ws) => {
                // Wir abonnieren L2-Snapshots für jedes Symbol einzeln (&str)
                for symbol in &symbols {
                    if let Err(e) = ws.subscribe_l2(symbol).await {
                        eprintln!("[COLLECTOR] Fehler beim Abo für {}: {:?}", symbol, e);
                        return;
                    }
                }

                println!("[COLLECTOR] Stream gestartet für {} Symbole", symbols.len());

                // Hauptschleife für die Ingestion
                loop {
                    // Laut ws.rs: pub async fn next_snapshot(&mut self) -> Option<L2Snapshot>
                    match ws.next_snapshot().await {
                        Some(snapshot) => {
                            self.stats.messages_received.fetch_add(1, Ordering::Relaxed);
                            // Update den neuesten Snapshot für das Symbol (Feld 'coin' in L2Snapshot)
                            self.market_data.insert(snapshot.coin.clone(), snapshot);
                        }
                        None => {
                            eprintln!("[COLLECTOR] Stream beendet oder Kanal geschlossen.");
                            // Kurze Pause vor potentiellem Reconnect (Logik in ws.rs vorhanden)
                            time::sleep(Duration::from_secs(1)).await;
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[COLLECTOR] Konnte WebSocket nicht initialisieren: {:?}", e);
            }
        }
    }

    /// Erzeugt den 100ms Herzschlag (3-6-9er Magie Fundament)
    /// Extrahiert den aktuellen Zustand aus der DashMap und gibt ihn an den Physicist weiter
    pub async fn heartbeat_loop<F>(self: Arc<Self>, mut callback: F) 
    where 
        F: FnMut(String, L2Snapshot) + Send + 'static 
    {
        let mut interval = time::interval(Duration::from_millis(100));
        // Missed ticks werden übersprungen, um die Zeit-Integrität zu wahren (Kybernetik)
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        println!("[COLLECTOR] Heartbeat Loop (100ms) aktiv.");

        loop {
            // Der Taktgeber für die gesamte nachgelagerte Physik
            interval.tick().await;
            
            // Iteriere über alle Coins in der Map und sende den aktuellen Stand an die Pipeline
            for entry in self.market_data.iter() {
                let symbol = entry.key().clone();
                let snapshot = entry.value().clone();
                
                self.stats.snapshots_sampled.fetch_add(1, Ordering::Relaxed);
                
                // Callback an das nächste Modul (Physicist) zur thermodynamischen Analyse
                callback(symbol, snapshot);
            }
        }
    }
}
