// File: src/exchange/hyperliquid/ws.rs
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use serde_json::json;
use tokio::sync::mpsc;
use log::{info, error};
use crate::exchange::types::L2Snapshot;
use crate::exchange::connector::Trade;

#[derive(Debug)]
#[allow(dead_code)]
pub enum HLEvent {
    Snapshot(L2Snapshot),
    Trade(Trade),
}

pub struct HyperliquidWs {
    url: String,
    tx: mpsc::UnboundedSender<HLEvent>,
    rx: mpsc::UnboundedReceiver<HLEvent>,
}

impl HyperliquidWs {
    pub async fn new() -> Result<Self, crate::exchange::errors::ExchangeError> {
        let (tx, rx) = mpsc::unbounded_channel();
        let is_testnet = true; // Default to testnet for research
        let ws = Self {
            url: if is_testnet { 
                "wss://api.hyperliquid-testnet.xyz/ws".to_string() 
            } else { 
                "wss://api.hyperliquid.xyz/ws".to_string() 
            },
            tx,
            rx,
        };
        
        // Auto-connect to BTC for the research engine
        ws.connect("BTC".to_string()).await;
        Ok(ws)
    }

    pub async fn connect(&self, symbol: String) {
        let url = self.url.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            loop {
                match connect_async(&url).await {
                    Ok((ws_stream, _)) => {
                        info!("✅ Connected to HyperLiquid WS ({})", symbol);
                        let (mut write, mut read) = ws_stream.split();

                        let sub_msg = json!({
                            "method": "subscribe",
                            "subscription": { "type": "l2Book", "coin": symbol }
                        });
                        if let Err(e) = write.send(Message::Text(sub_msg.to_string())).await {
                            error!("Failed to subscribe HL: {}", e);
                            return; 
                        }

                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                                        if let Some(channel) = parsed.get("channel") {
                                            if channel.as_str() == Some("l2Book") {
                                                if let Some(data) = parsed.get("data") {
                                                    if let Ok(snapshot) = serde_json::from_value::<L2Snapshot>(data.clone()) {
                                                        let _ = tx.send(HLEvent::Snapshot(snapshot));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => { error!("HL WS Error: {}", e); break; }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        error!("❌ HL Connection Failed: {}. Retry in 5s...", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
        });
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
