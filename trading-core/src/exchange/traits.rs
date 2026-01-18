// E:\mbct\trading-core\src\exchange\traits.rs

use async_trait::async_trait;
use trading_common::data::types::MarketState;
use crate::exchange::errors::ExchangeError;
use crate::exchange::types::L2Snapshot;

#[async_trait]
pub trait Exchange: Send + Sync {
    /// Initialisiert die Verbindung zum Hyperliquid-L1
    async fn connect(&self) -> Result<(), ExchangeError>;
    
    /// Liefert den aktuellen thermodynamischen Zustand
    fn derive_state(&self, snapshot: &L2Snapshot) -> MarketState;
}

#[async_trait]
pub trait MarketDataProvider: Send + Sync {
    /// Streamt die thermodynamische Bewegung (Cybernetic Loop)
    async fn subscribe_movement(
        &self, 
        symbol: &str
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<MarketState>, ExchangeError>;
}
