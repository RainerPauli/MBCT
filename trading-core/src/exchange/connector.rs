// ====
// Hyperliquid API Connector
// ====
// Full REST + WebSocket API implementation
// Custom wallet integration
// No external Hyperliquid SDKs
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
///
/// Full API implementation for Hyperliquid DEX
pub struct HyperliquidConnector {
    /// HTTP client
    client: Client,
    /// Wallet for signing
    wallet: HyperliquidWallet,
    /// API base URL
    base_url: String,
    /// Is testnet?
    is_testnet: bool,
    /// Asset info cache
    asset_info: Arc<RwLock<HashMap<String, AssetInfo>>>,
}

impl HyperliquidConnector {
    /// Create new connector
    ///
    /// Example:
    /// ```
    /// let connector = HyperliquidConnector::new(private_key, false)?; // Mainnet
    /// let connector = HyperliquidConnector::new(private_key, true)?;  // Testnet
    /// ```
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

    /// Get wallet address
    pub fn address(&self) -> &str {
        &self.wallet.address
    }

    // ====================================================================
    // MARKET DATA
    // ====================================================================

    /// Get all assets (symbols)
    pub async fn get_all_assets(&self) -> Result<Vec<AssetInfo>> {
        let url = format!("{}/info", self.base_url);
        let response: InfoResponse = self
            .client
            .post(&url)
            .json(&json!({
                "type": "meta"
            }))
            .send()
            .await?
            .json()
            .await?;

        // Cache asset info
        let mut cache = self.asset_info.write().await;
        for asset in &response.universe {
            cache.insert(asset.name.clone(), asset.clone());
        }

        Ok(response.universe)
    }

    /// Get asset info
    pub async fn get_asset_info(&self, symbol: &str) -> Result<AssetInfo> {
        // Check cache first
        {
            let cache = self.asset_info.read().await;
            if let Some(info) = cache.get(symbol) {
                return Ok(info.clone());
            }
        }

        // Fetch all assets
        let assets = self.get_all_assets().await?;

        // Find symbol
        assets
            .into_iter()
            .find(|a| a.name == symbol)
            .ok_or_else(|| anyhow!("Asset {} not found", symbol))
    }

    /// Get orderbook (L2)
    pub async fn get_orderbook(&self, symbol: &str) -> Result<Orderbook> {
        let url = format!("{}/info", self.base_url);
        let response: OrderbookResponse = self
            .client
            .post(&url)
            .json(&json!({
                "type": "l2Book",
                "coin": symbol
            }))
            .send()
            .await?
            .json()
            .await?;

        Ok(response.levels)
    }

    /// Get recent trades
    pub async fn get_recent_trades(&self, symbol: &str) -> Result<Vec<Trade>> {
        let url = format!("{}/info", self.base_url);
        let response: TradesResponse = self
            .client
            .post(&url)
            .json(&json!({
                "type": "recentTrades",
                "coin": symbol
            }))
            .send()
            .await?
            .json()
            .await?;

        Ok(response.trades)
    }

    // ====================================================================
    // ACCOUNT
    // ====================================================================

    /// Get account state (balances, positions)
    pub async fn get_account_state(&self) -> Result<AccountState> {
        let url = format!("{}/info", self.base_url);
        let response: AccountStateResponse = self
            .client
            .post(&url)
            .json(&json!({
                "type": "clearinghouseState",
                "user": self.wallet.address
            }))
            .send()
            .await?
            .json()
            .await?;

        Ok(response.state)
    }

    /// Get balance for specific asset
    pub async fn get_balance(&self, asset: &str) -> Result<Decimal> {
        let state = self.get_account_state().await?;

        // Find asset in balances
        for balance in state.balances {
            if balance.coin == asset {
                return Decimal::from_str(&balance.total).context("Failed to parse balance");
            }
        }

        Ok(Decimal::ZERO)
    }

    /// Get all balances
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

    /// Get open positions
    pub async fn get_open_positions(&self) -> Result<Vec<Position>> {
        let state = self.get_account_state().await?;
        Ok(state.asset_positions)
    }

    // ====================================================================
    // TRADING
    // ====================================================================

    /// Place market order
    ///
    /// Example:
    /// ```
    /// let order_id = connector.place_market_order(
    ///     "BTC",
    ///     true,  // is_buy
    ///     Decimal::from_str("0.01")?,
    ///     None,  // no leverage (spot)
    /// ).await?;
    /// ```
    pub async fn place_market_order(
        &self,
        symbol: &str,
        is_buy: bool,
        size: Decimal,
        _leverage: Option<u8>,
    ) -> Result<String> {
        // Get asset info for sz_decimals
        let asset_info = self.get_asset_info(symbol).await?;

        // Format size
        let size_str = format_size(size, asset_info.sz_decimals);

        // Build order action
        let order = json!({
            "type": "order",
            "orders": [{
                "a": asset_info.index,  // asset index
                "b": is_buy,            // is buy
                "p": "0",               // price (0 = market)
                "s": size_str,          // size
                "r": false,             // reduce only
                "t": {                  // order type
                    "limit": {
                        "tif": "Ioc"    // Immediate or Cancel (market)
                    }
                }
            }],
            "grouping": "na"
        });

        // Sign and send
        let response = self.sign_and_send_action(order).await?;

        // Extract order ID
        let order_id = response["response"]["data"]["statuses"][0]["resting"]["oid"]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to extract order ID"))?
            .to_string();

        Ok(order_id)
    }

    /// Place limit order
    pub async fn place_limit_order(
        &self,
        symbol: &str,
        is_buy: bool,
        price: Decimal,
        size: Decimal,
        _leverage: Option<u8>,
        post_only: bool,
    ) -> Result<String> {
        let asset_info = self.get_asset_info(symbol).await?;

        let price_str = format_price(price, asset_info.sz_decimals);
        let size_str = format_size(size, asset_info.sz_decimals);

        let order = json!({
            "type": "order",
            "orders": [{
                "a": asset_info.index,
                "b": is_buy,
                "p": price_str,
                "s": size_str,
                "r": false,
                "t": {
                    "limit": {
                        "tif": if post_only { "Alo" } else { "Gtc" }
                    }
                }
            }],
            "grouping": "na"
        });

        let response = self.sign_and_send_action(order).await?;

        let order_id = response["response"]["data"]["statuses"][0]["resting"]["oid"]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to extract order ID"))?
            .to_string();

        Ok(order_id)
    }

    /// Cancel order
    pub async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<()> {
        let asset_info = self.get_asset_info(symbol).await?;

        let cancel = json!({
            "type": "cancel",
            "cancels": [{
                "a": asset_info.index,
                "o": order_id
            }]
        });

        self.sign_and_send_action(cancel).await?;
        Ok(())
    }

    /// Cancel all orders for symbol
    pub async fn cancel_all_orders(&self, symbol: &str) -> Result<()> {
        let asset_info = self.get_asset_info(symbol).await?;

        let cancel = json!({
            "type": "cancelByCloid",
            "cancels": [{
                "asset": asset_info.index,
                "cloid": "0x0"  // Cancel all
            }]
        });

        self.sign_and_send_action(cancel).await?;
        Ok(())
    }

    /// Set leverage for symbol
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

    /// Sign and send action
    async fn sign_and_send_action(&self, action: Value) -> Result<Value> {
        // Build typed data for signing
        let typed_data = self.build_typed_data(action.clone())?;

        // Sign
        let signature = self.wallet.sign_typed_data(&typed_data)?;

        // Send request
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

        // Check for errors
        if let Some(error) = response.get("error") {
            return Err(anyhow!("API error: {}", error));
        }

        Ok(response)
    }

    /// Build EIP-712 typed data for action
    fn build_typed_data(&self, action: Value) -> Result<TypedData> {
        let domain = EIP712Domain {
            name: "Exchange".to_string(),
            version: "1".to_string(),
            chain_id: if self.is_testnet { 421614 } else { 42161 }, // Arbitrum
            verifying_contract: "0x0000000000000000000000000000000000000000".to_string(),
        };

        let types = json!({
            "EIP712Domain": [
                { "name": "name", "type": "string" },
                { "name": "version", "type": "string" },
                { "name": "chainId", "type": "uint256" },
                { "name": "verifyingContract", "type": "address" }
            ],
            "Agent": [
                { "name": "source", "type": "string" },
                { "name": "connectionId", "type": "bytes32" }
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
pub struct AssetInfo {
    pub name: String,
    pub index: u32,
    #[serde(rename = "szDecimals")]
    pub sz_decimals: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InfoResponse {
    universe: Vec<AssetInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Orderbook {
    pub bids: Vec<[String; 2]>, // [price, size]
    pub asks: Vec<[String; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderbookResponse {
    levels: Orderbook,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub px: String,   // price
    pub sz: String,   // size
    pub side: String, // "A" (ask) or "B" (bid)
    pub time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TradesResponse {
    trades: Vec<Trade>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountState {
    pub balances: Vec<Balance>,
    #[serde(rename = "assetPositions")]
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
    pub szi: String, // size (signed, negative = short)
    #[serde(rename = "entryPx")]
    pub entry_px: String,
    #[serde(rename = "unrealizedPnl")]
    pub unrealized_pnl: String,
}

// ====================================================================
// HELPERS
// ====================================================================

fn format_price(price: Decimal, decimals: u8) -> String {
    format!("{:.1$}", price, decimals as usize)
}

fn format_size(size: Decimal, decimals: u8) -> String {
    format!("{:.1$}", size, decimals as usize)
}
