use std::sync::Arc;
use trading_common::data::{repository::TickDataRepository, cache::TieredCache};
use sqlx::SqlitePool;
use std::time::Duration;

pub struct AppState {
    pub repository: Arc<TickDataRepository>,
}

#[derive(Debug, Clone)]
pub struct DatabaseSettings {
    pub database_url: String,
    pub redis_url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub max_lifetime: u64,
}

impl AppState {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        tracing::info!("Initializing Trading Core application state...");
        
        let settings = create_settings_from_env()?;
        tracing::info!("Configuration loaded successfully");

        let pool = create_database_pool(&settings).await?;
        tracing::info!("Database connection established");

        let cache = create_gui_cache(&settings).await?;
        tracing::info!("Cache initialized");

        let repository = TickDataRepository::new(pool, cache);

        Ok(Self {
            repository: Arc::new(repository),
        })
    }
}

fn create_settings_from_env() -> Result<DatabaseSettings, Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:data/mbct_research.db?mode=rwc".to_string());
    
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    Ok(DatabaseSettings {
        database_url,
        redis_url,
        max_connections: 5,
        min_connections: 1,
        max_lifetime: 1800,
    })
}

async fn create_database_pool(settings: &DatabaseSettings) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(settings.max_connections)
        .min_connections(settings.min_connections)
        .max_lifetime(Duration::from_secs(settings.max_lifetime))
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600))
        .connect(&settings.database_url)
        .await?;

    Ok(pool)
}

async fn create_gui_cache(settings: &DatabaseSettings) -> Result<TieredCache, Box<dyn std::error::Error>> {
    let memory_config = (50, 300);
    let redis_config = (
        settings.redis_url.as_str(),
        100,
        600
    );
    
    match TieredCache::new(memory_config, redis_config).await {
        Ok(cache) => {
            tracing::info!("Cache initialized successfully");
            Ok(cache)
        },
        Err(e) => {
            tracing::warn!("Failed to initialize full cache, using minimal cache: {}", e);
            create_minimal_cache().await
        }
    }
}

async fn create_minimal_cache() -> Result<TieredCache, Box<dyn std::error::Error>> {
    let memory_config = (10, 60);
    let redis_config = ("redis://127.0.0.1:6379", 10, 60);
    
    TieredCache::new(memory_config, redis_config).await
        .map_err(|e| e.into())
}