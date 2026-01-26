// E:\MBCT\trading-core\src\bin\balance_check.rs
use anyhow::Result;
use trading_core::exchange::connector::HyperliquidConnector;

#[tokio::main]
async fn main() -> Result<()> {
    // Ersetze dies durch deinen EXAKTEN Private Key des Master-Wallets
    let private_key = "f5e96e307d258bb8f26d3d356522c30d79fede6ef11970878374de4a54a277b4";

    println!("🧪 Allianz-Check: Initialisiere Testnet-Verbindung...");
    let connector = HyperliquidConnector::new(private_key, true)?;

    println!("🛰️ Abfrage für Adresse: {}", connector.address());

    // Test 1: User State (Equity)
    match connector.get_user_state(connector.address()).await {
        Ok(state) => {
            println!("✅ ACCOUNT GEFUNDEN!");
            println!("💰 Withdrawable Equity: ${}", state.withdrawable_equity);
        }
        Err(e) => println!("❌ Fehler bei UserState: {:?}", e),
    }

    // Test 2: Asset Info (Marktdaten-Verbindung)
    match connector.get_all_assets().await {
        Ok(assets) => println!(
            "📈 Marktdaten-Zugriff: OK ({} Assets gefunden)",
            assets.len()
        ),
        Err(e) => println!("❌ Fehler bei Asset-Info: {:?}", e),
    }

    Ok(())
}
