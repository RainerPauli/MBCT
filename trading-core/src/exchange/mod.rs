// E:\MBCT\trading-core\src\exchange\mod.rs
pub mod connector;
pub mod envelope_detection;
pub mod errors;
pub mod filters;
pub mod market_data;
pub mod traits;
pub mod types;
pub mod utils;
pub mod wallet;
pub mod ws;

// Re-exports für die "Movement Based" Engine
pub use connector::HyperliquidConnector as ExchangeConnector;
pub use errors::ExchangeError;
pub use market_data::HyperliquidMarketData as MarketProvider;
pub use traits::{Exchange, MarketDataProvider};
pub use types::*;
pub use wallet::HyperliquidWallet;
pub use ws::HyperliquidWs as WebSocketStream;
