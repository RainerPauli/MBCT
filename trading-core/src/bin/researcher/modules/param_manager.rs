// E:\MBCT\trading-core\src\bin\researcher\modules\param_manager.rs
use sqlx::{Pool, Sqlite, Row};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingParams {
    pub symbol: String,
    pub l_floor: f64,    
    pub s_ceiling: f64,  
    pub is_active: bool,
    pub sample_count: i64,
    pub last_updated: i64,
}

pub struct ParamManager {
    pool: Pool<Sqlite>,
}

impl ParamManager {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Erstellt die Steuerungstabelle mit erweiterten Metriken für die Allianz
    pub async fn initialize_table(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS active_trading_params (
                symbol TEXT PRIMARY KEY,
                l_floor REAL DEFAULT 0.35,
                s_ceiling REAL DEFAULT 0.65,
                is_active INTEGER DEFAULT 1,
                sample_count INTEGER DEFAULT 0,
                last_updated INTEGER
            )"
        ).execute(&self.pool).await?;
        Ok(())
    }

    /// Der kybernetische Loop zur Selbst-Justierung (Self-Sharpening)
    /// Er nutzt die letzten 30 Minuten Markterfahrung, um die Trigger zu schärfen.
    pub async fn auto_calibrate(&self, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        let timeframe_ms = 30 * 60 * 1000; // 30 Minuten Erfahrung
        let now = Utc::now().timestamp_millis();
        let start_ts = now - timeframe_ms;

        // 1. Datenextraktion: Wir holen alle Symmetrie-Werte der Periode
        // Geändert auf sqlx::query(), um Compile-Zeit Abhängigkeiten zu vermeiden
        let rows = sqlx::query(
            "SELECT symmetry FROM mbct_research_v2 
             WHERE symbol = ? AND timestamp > ? 
             AND symmetry IS NOT NULL
             ORDER BY symmetry ASC"
        )
        .bind(symbol)
        .bind(start_ts)
        .fetch_all(&self.pool).await?;

        let sample_count = rows.len() as i64;

        // 2. Validierung: Haben wir genug Daten für eine statistische Aussage?
        if sample_count < 1000 {
            println!("⚠️ [PARAM] Zu wenig Daten für {}: {} Samples. Kalibrierung übersprungen.", symbol, sample_count);
            return Ok(());
        }

        // 3. Perzentil-Berechnung (P15 / P85)
        let p15_idx = (sample_count as f64 * 0.15) as usize;
        let p85_idx = (sample_count as f64 * 0.85) as usize;

        let mut new_l_floor = rows[p15_idx.min(rows.len() - 1)].get::<Option<f64>, _>(0).unwrap_or(0.35);
        let mut new_s_ceiling = rows[p85_idx.min(rows.len() - 1)].get::<Option<f64>, _>(0).unwrap_or(0.65);

        // 4. Allianz-Schutzmechanismus (Sanity Check)
        let min_distance = 0.08; // Abstand von der Mitte (0.5)
        if new_l_floor > (0.5 - min_distance) { new_l_floor = 0.5 - min_distance; }
        if new_s_ceiling < (0.5 + min_distance) { new_s_ceiling = 0.5 + min_distance; }

        // 5. Persistenz: Wir machen die Erfahrung zum Gesetz
        sqlx::query(
            "INSERT INTO active_trading_params (symbol, l_floor, s_ceiling, sample_count, last_updated)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(symbol) DO UPDATE SET
             l_floor = excluded.l_floor,
             s_ceiling = excluded.s_ceiling,
             sample_count = excluded.sample_count,
             last_updated = excluded.last_updated"
        )
        .bind(symbol)
        .bind(new_l_floor)
        .bind(new_s_ceiling)
        .bind(sample_count)
        .bind(now)
        .execute(&self.pool).await?;

        println!("⚖️ [CALIBRATED] {} | Samples: {} | L-Floor: {:.3} | S-Ceiling: {:.3}", 
                 symbol, sample_count, new_l_floor, new_s_ceiling);

        Ok(())
    }

    /// Lädt die aktuell gültigen "Gesetze" für den Signalgeber
    pub async fn get_current_params(&self) -> HashMap<String, TradingParams> {
        let rows = sqlx::query(
            "SELECT symbol, l_floor, s_ceiling, is_active, sample_count, last_updated 
             FROM active_trading_params 
             WHERE is_active = 1"
        ).fetch_all(&self.pool).await.unwrap_or_default();

        let mut map = HashMap::new();
        for r in rows {
            let symbol: String = r.get(0);
            map.insert(symbol.clone(), TradingParams {
                symbol,
                l_floor: r.get::<Option<f64>, _>(1).unwrap_or(0.35),
                s_ceiling: r.get::<Option<f64>, _>(2).unwrap_or(0.65),
                is_active: r.get::<i64, _>(3) != 0,
                sample_count: r.get::<Option<i64>, _>(4).unwrap_or(0),
                last_updated: r.get::<Option<i64>, _>(5).unwrap_or(0),
            });
        }
        map
    }
}