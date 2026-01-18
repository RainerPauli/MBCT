// E:\mbct\trading_core\src\exchange\mod.rs

pub mod wallet;
pub mod connector;
pub mod market_data;
pub mod ws;
pub mod types;
pub mod traits;
pub mod errors;
pub mod utils;
pub mod filters;

// Re-exports für die "Movement Based" Engine
pub use connector::HyperliquidConnector as ExchangeConnector;
pub use market_data::HyperliquidMarketData as MarketProvider;
pub use ws::HyperliquidWs as WebSocketStream;
pub use wallet::HyperliquidWallet;
pub use errors::ExchangeError;
pub use traits::{Exchange, MarketDataProvider};
pub use types::*;