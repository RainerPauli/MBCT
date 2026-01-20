// E:\MBCT\trading-core\src\config.rs
// THE ALLIANCE - MBCT Configuration Engine
// Fokus: BOM-Filtering & Saubere Symbol-Strings

use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Database {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub max_lifetime: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MemoryCache {
    pub max_ticks_per_symbol: usize,
    pub ttl_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisCache {
    pub url: String,
    pub ttl_seconds: u64,
    pub max_ticks_per_symbol: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Cache {
    pub memory: MemoryCache,
    pub redis: RedisCache,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PaperTrading {
    pub enabled: bool,
    pub strategy: String,
    pub initial_capital: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub database: Database,
    pub cache: Cache,
    pub symbols: Vec<String>,
    pub paper_trading: PaperTrading,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .set_default("database.url", "sqlite:E:/MBCT/data/researcher.db")?
            .set_default("database.max_connections", 5)?
            .set_default("database.min_connections", 1)?
            .set_default("database.max_lifetime", 30)?
            .set_default("cache.memory.max_ticks_per_symbol", 1000)?
            .set_default("cache.memory.ttl_seconds", 3600)?
            .set_default("cache.redis.url", "redis://127.0.0.1/")?
            .set_default("cache.redis.ttl_seconds", 3600)?
            .set_default("cache.redis.max_ticks_per_symbol", 10000)?
            .set_default("paper_trading.enabled", true)?
            .set_default("paper_trading.strategy", "MBCT-Alpha-1")?
            .set_default("paper_trading.initial_capital", 10000.0)?
            .set_default("symbols", Vec::<String>::new())?
            .add_source(File::with_name("config").required(false))
            .build()?;

        let mut settings: Settings = s.try_deserialize()?;
        let asset_path = "E:/MBCT/data/static/hl_assets.txt";
        
        match Self::load_symbols_from_file(asset_path) {
            Ok(dynamic_symbols) if !dynamic_symbols.is_empty() => {
                println!("✅ THE ALLIANCE: {} Symbole bereinigt geladen.", dynamic_symbols.len());
                settings.symbols = dynamic_symbols;
            },
            Ok(_) => println!("⚠️ hl_assets.txt ist leer."),
            Err(e) => println!("⚠️ Ladefehler: {}", e),
        }
        Ok(settings)
    }

    fn load_symbols_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<String>, std::io::Error> {
        if !path.as_ref().exists() {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Datei fehlt"));
        }
        
        let bytes = fs::read(path)?;
        let content = String::from_utf8_lossy(&bytes);
        
        let symbols: Vec<String> = content.lines()
            .map(|line| {
                // Filtert das BOM (\u{feff}) und Whitespace
                line.trim().trim_start_matches('\u{feff}').to_string()
            })
            .filter(|s| !s.is_empty())
            .collect();
        Ok(symbols)
    }

    pub fn get_db_url(&self) -> String {
        self.database.url.clone()
    }
}