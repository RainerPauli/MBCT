// service/market_data.rs
// TEMPORARILY STUBBED - To be refactored for thermodynamic framework
// The old implementation called Exchange::subscribe_trades which doesn't exist in our simplified trait

use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::debug;

use super::errors::ServiceError;
use crate::exchange::traits::Exchange;

pub struct MarketDataService {
    _exchange: Arc<dyn Exchange>,
}

impl MarketDataService {
    pub fn new(exchange: Arc<dyn Exchange>) -> Self {
        Self {
            _exchange: exchange,
        }
    }

    pub async fn start(
        &self,
        _symbols: Vec<String>,
        _shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<(), ServiceError> {
        debug!("MarketDataService stubbed - will be implemented for thermodynamic framework");
        Ok(())
    }
}
