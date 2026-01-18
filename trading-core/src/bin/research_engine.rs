// E:\mbct\trading-core\src\bin\research_engine.rs

use trading_core::exchange::ws::HyperliquidWs;
use trading_core::exchange::market_data::HyperliquidMarketData;
use trading_common::data::repository::Repository;
use std::sync::Arc;
use tokio::signal;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 THE ALLIANCE - Thermodynamic Research Engine v1.0");
    
    // 1. Initialisiere SQLite Repository (E:\mbct\data\mbct_research.db)
    std::fs::create_dir_all("e:/mbct/data")?;
    let db_url = "sqlite:e:/mbct/data/mbct_research.db?mode=rwc";
    let repo = Arc::new(Repository::new(db_url).await?);
    repo.ensure_market_states_table().await?;

    // 2. Initialisiere Sensorik (Hyperliquid)
    let market_data = HyperliquidMarketData::new();
    let mut ws = HyperliquidWs::new().await?;
    
    println!("🔬 Cybernetic Loop active. Monitoring BTC/USDC Compression...");

    // 3. Main Loop: Messung von Entropie, Druck und Temperatur
    loop {
        tokio::select! {
            snapshot = ws.next_snapshot() => {
                if let Some(l2) = snapshot {
                    // Adaptive Physics: Berechne Zustand
                    let state = market_data.derive_market_state(&l2);
                    
                    // SENS-Reward: Nur signifikante Druckänderungen loggen (Deduping)
                    if state.pressure > rust_decimal::Decimal::ZERO {
                        println!("🌡️ T: {} | 💨 P: {} | 🌊 V: {} | 🌀 S: {:?}", 
                            state.temperature, state.pressure, state.volume_spread, state.entropy_level);
                        
                        let _ = repo.insert_market_state(&state).await;
                    }
                }
            }
            _ = signal::ctrl_c() => {
                println!("🛑 Shutdown initiated. Preserving State of MEE...");
                break;
            }
        }
    }
    Ok(())
}
