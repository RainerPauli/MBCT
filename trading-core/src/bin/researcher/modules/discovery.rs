// E:\MBCT\trading-core\src\bin\researcher\modules\discovery.rs
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::{self, Duration};
use reqwest::Client;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssetCtx {
    symbol: String,
    day_nxt: f64,
    day_vel: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum InfoRequest {
    #[serde(rename = "metaAndAssetCtxs")]
    MetaAndAssetCtxs,
}

pub struct Discovery {
    client: Client,
    api_url: String,
}

impl Discovery {
    pub fn new(is_testnet: bool) -> Self {
        let api_url = if is_testnet {
            "https://api.hyperliquid-testnet.xyz/info".to_string()
        } else {
            "https://api.hyperliquid.xyz/info".to_string()
        };
        Self {
            client: Client::new(),
            api_url,
        }
    }

    pub async fn run_continuous_discovery(&self, tx_new_symbols: mpsc::Sender<Vec<String>>) {
        let mut interval = time::interval(Duration::from_secs(300));
        let mut known_symbols = std::collections::HashSet::new();

        loop {
            interval.tick().await;

            let request_body = InfoRequest::MetaAndAssetCtxs;
            let response = self.client
                .post(&self.api_url)
                .json(&request_body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if let Some(universe) = data.get(0).and_then(|v| v.get("universe")) {
                            if let Some(asset_ctxs) = data.get(1) {
                                let mut to_subscribe = Vec::new();
                                
                                if let Some(ctx_array) = asset_ctxs.as_array() {
                                    for (idx, ctx) in ctx_array.iter().enumerate() {
                                        if let Some(sym_info) = universe.get(idx) {
                                            if let Some(name) = sym_info.get("name").and_then(|n| n.as_str()) {
                                                // Filter: Nur Assets mit signifikantem Volumen (Day-Volume > 1M)
                                                let day_vol = ctx.get("dayNxt").and_then(|v| v.as_str())
                                                    .and_then(|v| v.parse::<f64>().ok())
                                                    .unwrap_or(0.0);

                                                if day_vol > 1000000.0 && !known_symbols.contains(name) {
                                                    known_symbols.insert(name.to_string());
                                                    to_subscribe.push(name.to_string());
                                                }
                                            }
                                        }
                                    }
                                }

                                if !to_subscribe.is_empty() {
                                    let _ = tx_new_symbols.send(to_subscribe).await;
                                }
                            }
                        }
                    }
                }
                Err(e) => eprintln!("[DISCOVERY] API Error: {:?}", e),
            }
        }
    }
}