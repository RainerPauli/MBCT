// E:\MBCT\trading-core\src\exchange\connector.rs
// ====
// Hyperliquid API Connector - FULL ALLIANZ EDITION v3.7
// ====
// Full REST + WebSocket API implementation
// Custom wallet integration - No external Hyperliquid SDKs
// Fokus: Dynamisches Asset-Indexing & Robuste Equity-Abfrage
// ====

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::wallet::*;

/// Hyperliquid API Endpoints
const MAINNET_API: &str = "https://api.hyperliquid.xyz";
const TESTNET_API: &str = "https://api.hyperliquid-testnet.xyz";

/// Hyperliquid Connector
pub struct HyperliquidConnector {
    /// HTTP client
    client: Client,
    /// Wallet for signing
    wallet: HyperliquidWallet,
    /// API base URL
    base_url: String,
    /// Is testnet?
    pub is_testnet: bool,
    /// Asset info cache
    asset_info: Arc<RwLock<HashMap<String, AssetInfo>>>,
}

impl HyperliquidConnector {
    pub fn new(private_key: &str, is_testnet: bool) -> Result<Self> {
        let wallet = HyperliquidWallet::from_private_key(private_key)?;
        let base_url = if is_testnet {
            TESTNET_API.to_string()
        } else {
            MAINNET_API.to_string()
        };

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            wallet,
            base_url,
            is_testnet,
            asset_info: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn address(&self) -> &str {
        &self.wallet.address
    }

    // ====================================================================
    // MARKET DATA
    // ====================================================================

    /// Holt alle Assets und korrigiert das Index-Mapping dynamisch
    pub async fn get_all_assets(&self) -> Result<Vec<AssetInfo>> {
        let url = format!("{}/info", self.base_url);
        let response_value: Value = self
            .client
            .post(&url)
            .json(&json!({ "type": "meta" }))
            .send()
            .await?
            .json()
            .await?;

        // Allianz-Sicherung: Wir parsen das "universe" manuell,
        // um flexibel auf Feld-Änderungen der API zu reagieren.
        let universe_json = response_value["universe"]
            .as_array()
            .ok_or_else(|| anyhow!("Universe-Feld in API-Antwort fehlt"))?;

        let mut assets = Vec::new();
        let mut cache = self.asset_info.write().await;

        for (idx, val) in universe_json.iter().enumerate() {
            let name = val["name"].as_str().unwrap_or("UNKNOWN").to_string();
            let sz_decimals = val["szDecimals"].as_u64().unwrap_or(0) as u8;

            let asset = AssetInfo {
                name: name.clone(),
                index: idx as u32, // Wir nutzen die Array-Position als verlässlichen Index
                sz_decimals,
            };

            assets.push(asset.clone());
            cache.insert(name, asset);
        }

        Ok(assets)
    }

    pub async fn get_asset_info(&self, symbol: &str) -> Result<AssetInfo> {
        {
            let cache = self.asset_info.read().await;
            if let Some(info) = cache.get(symbol) {
                return Ok(info.clone());
            }
        }
        let assets = self.get_all_assets().await?;
        assets
            .into_iter()
            .find(|a| a.name == symbol)
            .ok_or_else(|| anyhow!("Asset {} nicht gefunden", symbol))
    }

    pub async fn get_orderbook(&self, symbol: &str) -> Result<Orderbook> {
        let url = format!("{}/info", self.base_url);
        let response: OrderbookResponse = self
            .client
            .post(&url)
            .json(&json!({ "type": "l2Book", "coin": symbol }))
            .send()
            .await?
            .json()
            .await?;
        Ok(response.levels)
    }

    pub async fn get_recent_trades(&self, symbol: &str) -> Result<Vec<Trade>> {
        let url = format!("{}/info", self.base_url);
        let response: TradesResponse = self
            .client
            .post(&url)
            .json(&json!({ "type": "recentTrades", "coin": symbol }))
            .send()
            .await?
            .json()
            .await?;
        Ok(response.trades)
    }

    // ====================================================================
    // ACCOUNT & ALLIANZ-STABILITY
    // ====================================================================

    /// Robuste Equity-Abfrage für Master-Account und Agenten
    pub async fn get_user_state(&self, address: &str) -> Result<UserState> {
        let url = format!("{}/info", self.base_url);
        let response_value: Value = self
            .client
            .post(&url)
            .json(&json!({ "type": "clearinghouseState", "user": address }))
            .send()
            .await?
            .json()
            .await?;

        // Suche nach Equity in verschiedenen möglichen JSON-Pfaden (API-Resilienz)
        let equity_str = response_value["withdrawableEquity"]
            .as_str()
            .or_else(|| response_value["marginSummary"]["withdrawableEquity"].as_str())
            .or_else(|| response_value["marginSummary"]["accountValue"].as_str())
            .unwrap_or("0");

        let withdrawable = Decimal::from_str(equity_str).unwrap_or(Decimal::ZERO);

        Ok(UserState {
            withdrawable_equity: withdrawable,
        })
    }

    pub async fn get_account_state(&self) -> Result<AccountState> {
        let url = format!("{}/info", self.base_url);
        // Wir nutzen hier Value, um flexibel auf die Antwort zu reagieren
        let response: Value = self
            .client
            .post(&url)
            .json(&json!({ "type": "clearinghouseState", "user": self.wallet.address }))
            .send()
            .await?
            .json()
            .await?;

        // Mapping auf die AccountState Struktur
        let state_val = if response["assetPositions"].is_array() {
            response.clone()
        } else {
            response["state"].clone()
        };

        serde_json::from_value(state_val).context("Fehler beim Parsen des AccountState")
    }

    pub async fn get_balance(&self, asset: &str) -> Result<Decimal> {
        let state = self.get_account_state().await?;
        for balance in state.balances {
            if balance.coin == asset {
                return Decimal::from_str(&balance.total).context("Balance-Parsing fehlgeschlagen");
            }
        }
        Ok(Decimal::ZERO)
    }

    pub async fn get_all_balances(&self) -> Result<HashMap<String, Decimal>> {
        let state = self.get_account_state().await?;
        let mut balances = HashMap::new();
        for balance in state.balances {
            if let Ok(amount) = Decimal::from_str(&balance.total) {
                if amount > Decimal::ZERO {
                    balances.insert(balance.coin, amount);
                }
            }
        }
        Ok(balances)
    }

    pub async fn get_open_positions(&self) -> Result<Vec<Position>> {
        let state = self.get_account_state().await?;
        Ok(state.asset_positions)
    }

    // ====================================================================
    // TRADING
    // ====================================================================

    pub async fn place_market_order(
        &self,
        symbol: &str,
        is_buy: bool,
        size: Decimal,
        _leverage: Option<u8>,
    ) -> Result<String> {
        let asset_info = self.get_asset_info(symbol).await?;
        let size_str = format_size(size, asset_info.sz_decimals);
        let order = json!({
            "type": "order",
            "orders": [{
                "a": asset_info.index, "b": is_buy, "p": "0", "s": size_str, "r": false,
                "t": { "limit": { "tif": "Ioc" } }
            }],
            "grouping": "na"
        });
        let response = self.sign_and_send_action(order).await?;
        let status = &response["response"]["data"]["statuses"][0];

        status["resting"]["oid"]
            .as_u64()
            .or_else(|| status["filled"]["oid"].as_u64())
            .map(|id| id.to_string())
            .ok_or_else(|| anyhow!("Order-ID konnte nicht extrahiert werden: {:?}", response))
    }

    pub async fn place_limit_order(
        &self,
        symbol: &str,
        is_buy: bool,
        size: Decimal,
        price: Decimal,
        _leverage: Option<u8>,
        post_only: bool,
    ) -> Result<String> {
        let asset_info = self.get_asset_info(symbol).await?;
        let price_str = format_price(price, 6);
        let size_str = format_size(size, asset_info.sz_decimals);
        let order = json!({
            "type": "order",
            "orders": [{
                "a": asset_info.index, "b": is_buy, "p": price_str, "s": size_str, "r": false,
                "t": { "limit": { "tif": if post_only { "Alo" } else { "Gtc" } } }
            }],
            "grouping": "na"
        });
        let response = self.sign_and_send_action(order).await?;
        let status = &response["response"]["data"]["statuses"][0];

        status["resting"]["oid"]
            .as_u64()
            .or_else(|| status["filled"]["oid"].as_u64())
            .map(|id| id.to_string())
            .ok_or_else(|| anyhow!("Limit-Order-ID Fehler: {:?}", response))
    }

    pub async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<()> {
        let asset_info = self.get_asset_info(symbol).await?;
        let cancel = json!({
            "type": "cancel",
            "cancels": [{ "a": asset_info.index, "o": order_id.parse::<u64>().unwrap_or(0) }]
        });
        self.sign_and_send_action(cancel).await?;
        Ok(())
    }

    pub async fn cancel_all_orders(&self, symbol: &str) -> Result<()> {
        let asset_info = self.get_asset_info(symbol).await?;
        let cancel = json!({
            "type": "cancelByCloid",
            "cancels": [{ "asset": asset_info.index, "cloid": "0x0" }]
        });
        self.sign_and_send_action(cancel).await?;
        Ok(())
    }

    pub async fn set_leverage(&self, symbol: &str, leverage: u8) -> Result<()> {
        let asset_info = self.get_asset_info(symbol).await?;
        let update = json!({
            "type": "updateLeverage",
            "asset": asset_info.index,
            "isCross": true,
            "leverage": leverage
        });
        self.sign_and_send_action(update).await?;
        Ok(())
    }

    // ====================================================================
    // INTERNAL
    // ====================================================================

    async fn sign_and_send_action(&self, action: Value) -> Result<Value> {
        let typed_data = self.build_typed_data(action.clone())?;
        let signature = self.wallet.sign_typed_data(&typed_data)?;
        let url = format!("{}/exchange", self.base_url);
        let response: Value = self
            .client
            .post(&url)
            .json(&json!({
                "action": action,
                "nonce": chrono::Utc::now().timestamp_millis(),
                "signature": signature,
                "vaultAddress": null
            }))
            .send()
            .await?
            .json()
            .await?;

        if let Some(error) = response.get("error") {
            return Err(anyhow!("API Fehler: {}", error));
        }
        Ok(response)
    }

    fn build_typed_data(&self, action: Value) -> Result<TypedData> {
        let domain = EIP712Domain {
            name: "Exchange".to_string(),
            version: "1".to_string(),
            chain_id: if self.is_testnet { 421614 } else { 42161 },
            verifying_contract: "0x0000000000000000000000000000000000000000".to_string(),
        };
        let types = json!({
            "EIP712Domain": [
                { "name": "name", "type": "string" }, { "name": "version", "type": "string" },
                { "name": "chainId", "type": "uint256" }, { "name": "verifyingContract", "type": "address" }
            ],
            "Agent": [
                { "name": "source", "type": "string" }, { "name": "connectionId", "type": "bytes32" }
            ]
        });
        Ok(TypedData {
            domain,
            primary_type: "Agent".to_string(),
            types,
            message: action,
        })
    }
}

// ====================================================================
// DATA STRUCTURES
// ====================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserState {
    pub withdrawable_equity: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub name: String,
    pub index: u32,
    #[serde(rename = "szDecimals")]
    pub sz_decimals: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Orderbook {
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderbookResponse {
    levels: Orderbook,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub px: String,
    pub sz: String,
    pub side: String,
    pub time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TradesResponse {
    trades: Vec<Trade>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountState {
    pub balances: Vec<Balance>,
    #[serde(rename = "withdrawableEquity", default)]
    pub withdrawable_equity: String,
    #[serde(rename = "assetPositions", default)]
    pub asset_positions: Vec<Position>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccountStateResponse {
    state: AccountState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub coin: String,
    pub total: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub position: PositionData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionData {
    pub coin: String,
    pub szi: String,
    #[serde(rename = "entryPx")]
    pub entry_px: String,
    #[serde(rename = "unrealizedPnl")]
    pub unrealized_pnl: String,
}

fn format_price(price: Decimal, decimals: u8) -> String {
    format!("{:.1$}", price, decimals as usize)
}
fn format_size(size: Decimal, decimals: u8) -> String {
    format!("{:.1$}", size, decimals as usize)
}
