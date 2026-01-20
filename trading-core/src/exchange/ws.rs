// File: src/exchange/hyperliquid/ws.rs
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use serde_json::json;
use tokio::sync::mpsc;
use crate::exchange::types::L2Snapshot;
use crate::exchange::connector::Trade;

#[derive(Debug)]
#[allow(dead_code)]
pub enum HLEvent {
    Snapshot(L2Snapshot),
    Trade(Trade),
}

pub struct HyperliquidWs {
    rx: mpsc::UnboundedReceiver<HLEvent>,
    sub_tx: mpsc::UnboundedSender<String>,
}

impl HyperliquidWs {
    pub async fn new() -> Result<Self, crate::exchange::errors::ExchangeError> {
        let (tx, rx) = mpsc::unbounded_channel();
        let (sub_tx, mut sub_rx) = mpsc::unbounded_channel::<String>();
        let is_testnet = false; 
        
        let url = if is_testnet { 
            "wss://api.hyperliquid-testnet.xyz/ws".to_string() 
        } else { 
            "wss://api.hyperliquid.xyz/ws".to_string() 
        };

        let event_tx = tx.clone();
        let ws_url = url.clone();

        tokio::spawn(async move {
            let mut active_subs = std::collections::HashSet::new();
            
            loop {
                match connect_async(&ws_url).await {
                    Ok((ws_stream, _)) => {
                        println!("✅ Connected to HyperLiquid WS");
                        let (mut write, mut read) = ws_stream.split();

                        // Re-subscribe to existing symbols on reconnect
                        for symbol in &active_subs {
                            let sub_msg = json!({
                                "method": "subscribe",
                                "subscription": { "type": "l2Book", "coin": symbol }
                            });
                            let _ = write.send(Message::Text(sub_msg.to_string())).await;
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }

                        loop {
                            tokio::select! {
                                Some(symbol) = sub_rx.recv() => {
                                    if active_subs.insert(symbol.clone()) {
                                        let sub_msg = json!({
                                            "method": "subscribe",
                                            "subscription": { "type": "l2Book", "coin": symbol }
                                        });
                                        println!("📡 Subscribing to: {}", symbol);
                                        let _ = write.send(Message::Text(sub_msg.to_string())).await;
                                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                    }
                                }
                                Some(msg) = read.next() => {
                                    match msg {
                                        Ok(Message::Text(text)) => {
                                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                                                if let Some(channel) = parsed.get("channel") {
                                                    if channel.as_str() == Some("l2Book") {
                                                        if let Some(data) = parsed.get("data") {
                                                            if let Ok(snapshot) = serde_json::from_value::<L2Snapshot>(data.clone()) {
                                                                let _ = event_tx.send(HLEvent::Snapshot(snapshot));
                                                            }
                                                        }
                                                    } else if channel.as_str() == Some("error") {
                                                        println!("❌ HL WS Server Error: {}", text);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => { println!("❌ HL WS Error: {}", e); break; }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ HL Connection Failed: {}. Retry in 5s...", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
        });

        Ok(Self {
            rx,
            sub_tx,
        })
    }

    pub async fn subscribe_l2(&self, symbol: &str) -> Result<(), crate::exchange::errors::ExchangeError> {
        self.sub_tx.send(symbol.to_string()).map_err(|_| {
            crate::exchange::errors::ExchangeError::NetworkError("Failed to send subscription request".into())
        })
    }

    pub async fn next_snapshot(&mut self) -> Option<L2Snapshot> {
        while let Some(event) = self.rx.recv().await {
            if let HLEvent::Snapshot(s) = event {
                return Some(s);
            }
        }
        None
    }
}
