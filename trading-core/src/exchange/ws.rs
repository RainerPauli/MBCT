// E:\MBCT\trading-core\src\exchange\ws.rs
// ====
// Hyperliquid WebSocket Connector - ALLIANZ RESILIENT EDITION v4.2
// One-Shot Pattern: Collector handles reconnection.
// ====

use crate::exchange::types::L2Snapshot;
use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug, serde::Deserialize)]
struct WSResponse {
    data: Option<L2Snapshot>,
}

#[derive(Debug)]
pub enum HLEvent {
    Snapshot(L2Snapshot),
}

pub struct HyperliquidWs {
    rx: mpsc::UnboundedReceiver<HLEvent>,
    sub_tx: mpsc::UnboundedSender<String>,
}

impl HyperliquidWs {
    pub async fn new(is_testnet: bool) -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        let (sub_tx, mut sub_rx) = mpsc::unbounded_channel::<String>();

        let url = if is_testnet {
            "wss://api.hyperliquid-testnet.xyz/ws"
        } else {
            "wss://api.hyperliquid.xyz/ws"
        }
        .to_string();

        let (ws_stream, _) = connect_async(&url).await.map_err(|e| anyhow!("Connect failed: {}", e))?;
        println!("✅ WS: Verbindung zur Allianz-Zentrale steht.");
        
        let (mut write, mut read) = ws_stream.split();
        let event_tx = tx.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Subscriptions from Collector
                    res = sub_rx.recv() => {
                        match res {
                            Some(symbol) => {
                                let sub_msg = json!({
                                    "method": "subscribe",
                                    "subscription": { "type": "l2Book", "coin": symbol }
                                });
                                if let Err(_) = write.send(Message::Text(sub_msg.to_string())).await {
                                    break;
                                }
                            }
                            None => break, // sub_tx was dropped
                        }
                    }
                    
                    // Incoming Messages
                    msg_res = read.next() => {
                        match msg_res {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(resp) = serde_json::from_str::<WSResponse>(&text) {
                                    if let Some(snapshot) = resp.data {
                                        let _ = event_tx.send(HLEvent::Snapshot(snapshot));
                                    }
                                }
                            }
                            Some(Ok(Message::Binary(bin))) => {
                                if let Ok(resp) = serde_json::from_slice::<WSResponse>(&bin) {
                                    if let Some(snapshot) = resp.data {
                                        let _ = event_tx.send(HLEvent::Snapshot(snapshot));
                                    }
                                }
                            }
                            Some(Ok(Message::Ping(payload))) => {
                                // Server -> Client Ping: Respond with Pong
                                let _ = write.send(Message::Pong(payload)).await;
                            }
                            Some(Ok(Message::Close(_))) | Some(Err(_)) | None => {
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            println!("ℹ️ WS-Task beendet.");
        });

        Ok(Self { rx, sub_tx })
    }

    pub async fn subscribe_l2(&self, symbol: &str) -> Result<()> {
        self.sub_tx
            .send(symbol.to_string())
            .map_err(|e| anyhow!("Sub-Error: {}", e))
    }

    pub async fn next_snapshot(&mut self) -> Option<L2Snapshot> {
        while let Some(event) = self.rx.recv().await {
            match event {
                HLEvent::Snapshot(s) => return Some(s),
            }
        }
        None
    }
}
