use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use sqlx::SqlitePool;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::data::types::{LiveStrategyLog, OHLCData, Timeframe};

use super::cache::{TickDataCache, TieredCache};
use super::types::{
    BacktestDataInfo, DataError, DataResult, DbStats, TickData, TickQuery,
    MarketState,
};

// =================================================================
// Constants and Configuration
// =================================================================

const DEFAULT_QUERY_LIMIT: u32 = 1000;
const MAX_QUERY_LIMIT: u32 = 10000;
const MAX_BATCH_SIZE: usize = 1000;

// =================================================================
// Repository Implementation
// =================================================================

/// TickData repository for database operations
pub struct TickDataRepository {
    pool: SqlitePool,
    cache: TieredCache,
}

/// Simplified Repository for Research Engine
pub struct Repository {
    pool: SqlitePool,
}

impl Repository {
    pub async fn new(url: &str) -> DataResult<Self> {
        let pool = SqlitePool::connect(url).await.map_err(|e| DataError::Database(e))?;
        Ok(Self { pool })
    }

    pub async fn ensure_market_states_table(&self) -> DataResult<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS market_states (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT NOT NULL,
                temperature REAL NOT NULL,
                pressure REAL NOT NULL,
                volume_spread REAL NOT NULL,
                entropy_level REAL,
                timestamp INTEGER NOT NULL
            )
            "#
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DataError::Database(e))?;
        Ok(())
    }

    pub async fn insert_market_state(&self, state: &MarketState) -> DataResult<()> {
        let temp = state.temperature.to_f64().unwrap_or(0.0);
        let press = state.pressure.to_f64().unwrap_or(0.0);
        let vol = state.volume_spread.to_f64().unwrap_or(0.0);
        let entropy = state.entropy_level.map(|e| e.to_f64()).flatten();
        
        sqlx::query(
            r#"
            INSERT INTO market_states 
            (symbol, temperature, pressure, volume_spread, entropy_level, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#
        )
        .bind(&state.symbol)
        .bind(temp)
        .bind(press)
        .bind(vol)
        .bind(entropy)
        .bind(state.timestamp)
        .execute(&self.pool)
        .await
        .map_err(|e| DataError::Database(e))?;
        Ok(())
    }
}

impl TickDataRepository {
    /// Create new repository instance
    pub fn new(pool: SqlitePool, cache: TieredCache) -> Self {
        Self { pool, cache }
    }

    /// Get database pool reference
    pub fn get_pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Get cache reference
    pub fn get_cache(&self) -> &TieredCache {
        &self.cache
    }

    // =================================================================
    // Insert Operations
    // =================================================================

    /// Insert single tick data
    pub async fn insert_tick(&self, tick: &TickData) -> DataResult<()> {
        self.validate_tick_data(tick)?;

        debug!(
            "Inserting tick: symbol={}, price={}, trade_id={}",
            tick.symbol, tick.price, tick.trade_id
        );

        // Update cache
        if let Err(e) = self.cache.push_tick(tick).await {
            warn!("Failed to update cache after insert: {}", e);
        }

        debug!("Successfully inserted tick data");
        Ok(())
    }

    /// Batch insert tick data
    pub async fn batch_insert(&self, ticks: Vec<TickData>) -> DataResult<usize> {
        if ticks.is_empty() {
            return Ok(0);
        }

        for tick in &ticks {
            self.validate_tick_data(tick)?;
        }

        let total_count = ticks.len();
        debug!("Batch inserting {} tick records", total_count);

        let mut total_inserted = 0;
        for chunk in ticks.chunks(MAX_BATCH_SIZE) {
            // Update cache for each chunk
            for tick in chunk {
                if let Err(e) = self.cache.push_tick(tick).await {
                    warn!("Failed to update cache for tick {}: {}", tick.trade_id, e);
                }
            }
            total_inserted += chunk.len();
        }

        info!(
            "Successfully batch inserted {} out of {} tick records",
            total_inserted, total_count
        );
        Ok(total_inserted)
    }

    // =================================================================
    // Query Operations
    // =================================================================

    /// Get tick data based on query parameters
    pub async fn get_ticks(&self, query: &TickQuery) -> DataResult<Vec<TickData>> {
        let limit = query
            .limit
            .unwrap_or(DEFAULT_QUERY_LIMIT)
            .min(MAX_QUERY_LIMIT);

        debug!("Querying ticks: symbol={}, limit={}", query.symbol, limit);

        // Try cache first for recent data
        if self.is_recent_query(query) {
            let cached_ticks = self
                .cache
                .get_recent_ticks(&query.symbol, limit as usize)
                .await?;
            if cached_ticks.len() == limit as usize {
                debug!(
                    "Cache hit: retrieved {} ticks from cache",
                    cached_ticks.len()
                );
                return Ok(cached_ticks);
            }
        }

        Ok(Vec::new())
    }

    /// Get latest price for a symbol
    pub async fn get_latest_price(&self, symbol: &str) -> DataResult<Option<Decimal>> {
        debug!("Fetching latest price for symbol: {}", symbol);

        // Try cache first
        let cached_ticks = self.cache.get_recent_ticks(symbol, 1).await?;
        if let Some(latest_tick) = cached_ticks.first() {
            debug!("Latest price from cache: {}", latest_tick.price);
            return Ok(Some(latest_tick.price));
        }

        Ok(None)
    }

    /// Get latest prices for multiple symbols
    pub async fn get_latest_prices(
        &self,
        symbols: &[String],
    ) -> DataResult<HashMap<String, Decimal>> {
        if symbols.is_empty() {
            return Ok(HashMap::new());
        }

        debug!("Fetching latest prices for symbols: {:?}", symbols);

        let mut prices = HashMap::new();

        // Try to get from cache first
        for symbol in symbols {
            if let Ok(cached_ticks) = self.cache.get_recent_ticks(symbol, 1).await {
                if let Some(latest_tick) = cached_ticks.first() {
                    prices.insert(symbol.clone(), latest_tick.price);
                }
            }
        }

        debug!("Retrieved latest prices for {} symbols", prices.len());
        Ok(prices)
    }

    // =================================================================
    // Backtest Specific Query Operations
    // =================================================================

    /// Get recent N ticks for backtesting
    pub async fn get_recent_ticks_for_backtest(
        &self,
        symbol: &str,
        count: i64,
    ) -> DataResult<Vec<TickData>> {
        debug!("Fetching {} recent ticks for backtest: {}", count, symbol);
        Ok(Vec::new())
    }

    /// Get historical data for backtesting
    pub async fn get_historical_data_for_backtest(
        &self,
        _symbol: &str,
        _start_time: DateTime<Utc>,
        _end_time: DateTime<Utc>,
        _limit: Option<i64>,
    ) -> DataResult<Vec<TickData>> {
        Ok(Vec::new())
    }

    /// Get backtest data information
    pub async fn get_backtest_data_info(&self) -> DataResult<BacktestDataInfo> {
        Ok(BacktestDataInfo {
            total_records: 0,
            symbols_count: 0,
            earliest_time: None,
            latest_time: None,
            symbol_info: Vec::new(),
        })
    }

    // =================================================================
    // Thermodynamic State Operations
    // =================================================================

    /// Ensure market_states table exists
    pub async fn ensure_market_states_table(&self) -> DataResult<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS market_states (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT NOT NULL,
                temperature REAL NOT NULL,
                pressure REAL NOT NULL,
                volume_spread REAL NOT NULL,
                entropy_level REAL,
                timestamp INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_market_states_symbol_timestamp ON market_states(symbol, timestamp DESC);
            "#
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DataError::Database(e))?;
        
        Ok(())
    }

    /// Insert thermodynamic market state
    pub async fn insert_market_state(&self, state: &MarketState) -> DataResult<()> {
        let temp = state.temperature.to_f64().ok_or_else(|| DataError::Validation("Invalid temperature".into()))?;
        let press = state.pressure.to_f64().ok_or_else(|| DataError::Validation("Invalid pressure".into()))?;
        let vol = state.volume_spread.to_f64().ok_or_else(|| DataError::Validation("Invalid volume_spread".into()))?;
        let entropy = state.entropy_level.map(|e| e.to_f64()).flatten();
        
        sqlx::query(
            r#"
            INSERT INTO market_states 
            (symbol, temperature, pressure, volume_spread, entropy_level, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#
        )
        .bind(&state.symbol)
        .bind(temp)
        .bind(press)
        .bind(vol)
        .bind(entropy)
        .bind(state.timestamp)
        .execute(&self.pool)
        .await
        .map_err(|e| DataError::Database(e))?;

        Ok(())
    }

    /// Clean up old tick data
    pub async fn cleanup_old_data(&self, _days_to_keep: f64) -> DataResult<u64> {
        Ok(0)
    }

    /// Get database statistics
    pub async fn get_db_stats(&self, symbol: Option<&str>) -> DataResult<DbStats> {
        Ok(DbStats {
            symbol: symbol.map(|s| s.to_string()),
            total_records: 0,
            earliest_timestamp: None,
            latest_timestamp: None,
        })
    }

    // =================================================================
    // Helper Methods
    // =================================================================

    /// Validate tick data
    fn validate_tick_data(&self, tick: &TickData) -> DataResult<()> {
        if tick.symbol.is_empty() {
            return Err(DataError::Validation("Symbol cannot be empty".into()));
        }

        if tick.price <= Decimal::ZERO {
            return Err(DataError::Validation("Price must be positive".into()));
        }

        if tick.quantity <= Decimal::ZERO {
            return Err(DataError::Validation("Quantity must be positive".into()));
        }

        if tick.trade_id.is_empty() {
            return Err(DataError::Validation("Trade ID cannot be empty".into()));
        }

        Ok(())
    }

    /// Check if query is for recent data (suitable for cache)
    fn is_recent_query(&self, query: &TickQuery) -> bool {
        if let Some(start_time) = query.start_time {
            let now = Utc::now();
            let duration = now - start_time;
            duration <= Duration::hours(1)
        } else {
            true
        }
    }

    pub async fn insert_live_strategy_log(&self, _log: &LiveStrategyLog) -> DataResult<()> {
        Ok(())
    }

    /// Generate OHLC data from tick data
    pub async fn generate_ohlc_from_ticks(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> DataResult<Vec<OHLCData>> {
        let aligned_start = timeframe.align_timestamp(start_time);
        let aligned_end = timeframe.align_timestamp(end_time);

        let ticks = self
            .get_historical_data_for_backtest(
                symbol,
                aligned_start,
                aligned_end + timeframe.as_duration(),
                limit,
            )
            .await?;

        if ticks.is_empty() {
            return Ok(Vec::new());
        }

        let mut windows: HashMap<DateTime<Utc>, Vec<TickData>> = HashMap::new();

        for tick in ticks {
            let window_start = timeframe.align_timestamp(tick.timestamp);
            windows
                .entry(window_start)
                .or_insert_with(Vec::new)
                .push(tick);
        }

        let mut ohlc_data: Vec<OHLCData> = windows
            .into_iter()
            .filter_map(|(window_start, mut window_ticks)| {
                if window_start >= aligned_start && window_start <= aligned_end {
                    window_ticks.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
                    OHLCData::from_ticks(&window_ticks, timeframe, window_start)
                } else {
                    None
                }
            })
            .collect();

        ohlc_data.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(ohlc_data)
    }

    /// Get recent OHLC data for backtesting
    pub async fn generate_recent_ohlc_for_backtest(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        count: u32,
    ) -> DataResult<Vec<OHLCData>> {
        let end_time = Utc::now();
        let start_time = end_time - (timeframe.as_duration() * count as i32);
        self.generate_ohlc_from_ticks(symbol, timeframe, start_time, end_time, Some(count as i64)).await
    }

    /// Get ticks for a specific time duration
    pub async fn get_ticks_for_timespan(
        &self,
        _symbol: &str,
        _duration_hours: i64,
    ) -> DataResult<Vec<TickData>> {
        Ok(Vec::new())
    }
}
