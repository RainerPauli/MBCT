// E:\MBCT\trading-core\src\bin\researcher\modules\archive.rs
use crate::modules::chronos::MBCTFullRecord;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Pool, Sqlite};
use std::fs::OpenOptions;
use std::io::Write;
use std::str::FromStr;

pub struct Archive {
    pool: Pool<Sqlite>,
    csv_path: String,
}

impl Archive {
    pub async fn new(db_url: &str, csv_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let opts = SqliteConnectOptions::from_str(db_url)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal);

        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(opts)
            .await?;

        // Tabelle mit ret_377s erweitert
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS mbct_research_v2 (
                timestamp INTEGER,
                symbol TEXT,
                price REAL,
                entropy REAL,
                pressure REAL,
                nrg REAL,
                regime TEXT,
                symmetry REAL,
                slope REAL,
                ret_3s REAL,
                ret_5s REAL,
                ret_8s REAL,
                ret_13s REAL,
                ret_21s REAL,
                ret_34s REAL,
                ret_55s REAL,
                ret_89s REAL,
                ret_144s REAL,
                ret_233s REAL,
                ret_377s REAL,
                z_entropy_21s REAL,
                z_pressure_21s REAL,
                z_nrg_21s REAL,
                z_entropy_34s REAL,
                z_pressure_34s REAL,
                z_nrg_34s REAL
            )"
        ).execute(&pool).await?;

        Ok(Self { pool, csv_path: csv_path.to_string() })
    }

    /// Ermöglicht dem ParamManager Zugriff auf den DB-Pool für die Kalibrierung
    pub fn get_pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    pub async fn store_batch(&self, records: Vec<MBCTFullRecord>) -> Result<(), sqlx::Error> {
        for record in records {
            sqlx::query(
                "INSERT INTO mbct_research_v2 (
                    timestamp, symbol, price, entropy, pressure, nrg, regime, symmetry, slope,
                    ret_3s, ret_5s, ret_8s, ret_13s, ret_21s, ret_34s, ret_55s, ret_89s, ret_144s, ret_233s, ret_377s,
                    z_entropy_21s, z_pressure_21s, z_nrg_21s, z_entropy_34s, z_pressure_34s, z_nrg_34s
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(record.timestamp as i64)
            .bind(&record.symbol)
            .bind(record.physics.price)
            .bind(record.physics.entropy)
            .bind(record.physics.pressure)
            .bind(record.physics.nrg)
            .bind(format!("{:?}", record.regime.regime))
            .bind(record.regime.symmetry_score)
            .bind(record.regime.slope)
            .bind(record.ret_3s)
            .bind(record.ret_5s)
            .bind(record.ret_8s)
            .bind(record.ret_13s)
            .bind(record.ret_21s)
            .bind(record.ret_34s)
            .bind(record.ret_55s)
            .bind(record.ret_89s)
            .bind(record.ret_144s)
            .bind(record.ret_233s)
            .bind(record.ret_377s)
            .bind(record.z_entropy_21s)
            .bind(record.z_pressure_21s)
            .bind(record.z_nrg_21s)
            .bind(record.z_entropy_34s)
            .bind(record.z_pressure_34s)
            .bind(record.z_nrg_34s)
            .execute(&self.pool)
            .await?;

            self.append_to_csv(&record);
        }
        Ok(())
    }

    fn append_to_csv(&self, record: &MBCTFullRecord) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.csv_path)
            .unwrap();

        // CSV Header mit ret_377s
        if file.metadata().unwrap().len() == 0 {
            writeln!(file, "timestamp,symbol,price,entropy,pressure,nrg,regime,symmetry,slope,ret_3s,ret_5s,ret_8s,ret_13s,ret_21s,ret_34s,ret_55s,ret_89s,ret_144s,ret_233s,ret_377s,z_entropy_21s,z_pressure_21s,z_nrg_21s,z_entropy_34s,z_pressure_34s,z_nrg_34s").unwrap();
        }

        let f_opt = |opt: Option<f64>| {
            opt.map(|v| format!("{:.8}", v)).unwrap_or_else(|| "".to_string())
        };

        let regime_str = format!("{:?}", record.regime.regime);
        
        writeln!(
            file,
            "{},{},{:.8},{:.4},{:.4},{:.4},{},{:.4},{:.8},{},{},{},{},{},{},{},{},{},{},{},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4}",
            record.timestamp,
            record.symbol,
            record.physics.price,
            record.physics.entropy,
            record.physics.pressure,
            record.physics.nrg,
            regime_str,
            record.regime.symmetry_score,
            record.regime.slope,
            f_opt(record.ret_3s),
            f_opt(record.ret_5s),
            f_opt(record.ret_8s),
            f_opt(record.ret_13s),
            f_opt(record.ret_21s),
            f_opt(record.ret_34s),
            f_opt(record.ret_55s),
            f_opt(record.ret_89s),
            f_opt(record.ret_144s),
            f_opt(record.ret_233s),
            f_opt(record.ret_377s), // Neu eingereiht
            record.z_entropy_21s,
            record.z_pressure_21s,
            record.z_nrg_21s,
            record.z_entropy_34s,
            record.z_pressure_34s,
            record.z_nrg_34s
        ).unwrap();
    }
}