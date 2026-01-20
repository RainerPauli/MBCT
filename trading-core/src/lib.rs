// E:\MBCT\trading-core\src\lib.rs
// THE ALLIANCE - Core Library Definitions

pub mod config;
pub mod exchange;
pub mod live_trading;
pub mod service;

// Re-export trading-common for convenience
pub use trading_common::{backtest, data};