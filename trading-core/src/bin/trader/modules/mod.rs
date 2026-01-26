// E:\MBCT\trading-core\src\bin\trader\modules\mod.rs

pub mod chronos;
pub mod collector; // WebSocket & Heartbeat Loop
pub mod physicist; // Thermodynamische Transformation (Entropy, Pressure, NRG)
pub mod regime; // Markt-Zustands-Klassifizierung (Symmetry & Slope) // (Optional) Falls der Trader eigene Ausführungen loggen soll
