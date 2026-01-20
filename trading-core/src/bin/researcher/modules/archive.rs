// E:\MBCT\trading-core\src\bin\researcher\modules\archive.rs
// THE ALLIANCE - MBCT Archive Modul
// Fokus: Hochperformante Persistenz (SQLite WAL + CSV)

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
    pub async fn new(db_url: &str, csv_path: &str) -> Self {
        // WAL-Mode Konfiguration für THE ALLIANCE Signalgeber
        let opts = SqliteConnectOptions::from_str(db_url)
            .unwrap()
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal) // WAL für paralleles Lesen/Schreiben
            .synchronous(SqliteSynchronous::Normal); // Optimale Balance zwischen Speed & Sicherheit

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await
            .expect("Fehler beim Initialisieren der MBCT-Datenbank");

        // Tabelle anlegen, falls nicht vorhanden
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS mbct_research (
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
                ret_8s REAL,
                ret_21s REAL,
                ret_55s REAL,
                ret_89s REAL
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        Self {
            pool,
            csv_path: csv_path.to_string(),
        }
    }

    pub async fn store_records(&self, records: Vec<MBCTFullRecord>) {
        for record in records {
            // 1. In SQLite speichern
            let _ = sqlx::query(
                "INSERT INTO mbct_research VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
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
            .bind(record.ret_8s)
            .bind(record.ret_21s)
            .bind(record.ret_55s)
            .bind(record.ret_89s)
            .execute(&self.pool)
            .await;

            // 2. In CSV anhängen
            self.append_to_csv(&record);
        }
    }

    fn append_to_csv(&self, record: &MBCTFullRecord) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.csv_path)
            .unwrap();

        // Header schreiben, falls Datei neu
        if file.metadata().unwrap().len() == 0 {
            writeln!(file, "timestamp,symbol,price,entropy,pressure,nrg,regime,symmetry,slope,ret_3s,ret_8s,ret_21s,ret_55s,ret_89s").unwrap();
        }

        let regime_str = format!("{:?}", record.regime.regime);
        writeln!(
            file,
            "{},{},{:.8},{:.4},{:.4},{:.4},{},{:.4},{:.8},{:?},{:?},{:?},{:?},{:?}",
            record.timestamp,
            record.symbol,
            record.physics.price,
            record.physics.entropy,
            record.physics.pressure,
            record.physics.nrg,
            regime_str,
            record.regime.symmetry_score,
            record.regime.slope,
            record.ret_3s,
            record.ret_8s,
            record.ret_21s,
            record.ret_55s,
            record.ret_89s
        ).unwrap();
    }
}