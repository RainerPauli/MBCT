-- research/queries.sql
-- Thermodynamic Market Research Queries für MBCT (Movement Based Crypto Trading)
-- Diese Queries extrahieren die "physikalische Realität" aus den Marktdaten

-- ============================================================================
-- 1. KRITISCHE KOMPRESSION: Identifiziere Phasen maximaler Spannung
-- ============================================================================
-- Zeigt Momente, in denen hoher Druck auf minimalen Spread trifft
-- Dies sind die "gespannten Federn" kurz vor der Explosion

SELECT 
    symbol, 
    temperature as price, 
    pressure, 
    volume_spread, 
    (pressure / volume_spread) as tension,
    datetime(timestamp/1000, 'unixepoch') as time,
    CASE 
        WHEN (pressure / volume_spread) > 1000 THEN 'CRITICAL'
        WHEN (pressure / volume_spread) > 500 THEN 'HIGH'
        WHEN (pressure / volume_spread) > 100 THEN 'MODERATE'
        ELSE 'LOW'
    END as tension_level
FROM market_states
WHERE volume_spread > 0 
  AND volume_spread < (SELECT AVG(volume_spread) FROM market_states WHERE volume_spread > 0)
ORDER BY tension DESC
LIMIT 100;

-- ============================================================================
-- 2. DRUCK-ENTWICKLUNG: Zeitreihenanalyse der Liquiditätsdynamik
-- ============================================================================
-- Zeigt, wie sich der Druck über die Zeit entwickelt

SELECT 
    symbol,
    datetime(timestamp/1000, 'unixepoch') as time,
    pressure,
    volume_spread,
    temperature as price,
    (pressure / volume_spread) as tension,
    LAG(pressure, 1) OVER (PARTITION BY symbol ORDER BY timestamp) as prev_pressure,
    ((pressure - LAG(pressure, 1) OVER (PARTITION BY symbol ORDER BY timestamp)) / LAG(pressure, 1) OVER (PARTITION BY symbol ORDER BY timestamp)) * 100 as pressure_change_pct
FROM market_states
WHERE volume_spread > 0
ORDER BY symbol, timestamp DESC
LIMIT 500;

-- ============================================================================
-- 3. SPREAD-ANOMALIEN: Finde unnatürlich enge Spreads
-- ============================================================================
-- Extrem enge Spreads bei hohem Volumen = institutionelle Akkumulation

SELECT 
    symbol,
    datetime(timestamp/1000, 'unixepoch') as time,
    temperature as price,
    volume_spread,
    pressure,
    (SELECT AVG(volume_spread) FROM market_states WHERE symbol = ms.symbol AND volume_spread > 0) as avg_spread,
    volume_spread / (SELECT AVG(volume_spread) FROM market_states WHERE symbol = ms.symbol AND volume_spread > 0) as spread_ratio
FROM market_states ms
WHERE volume_spread > 0
  AND volume_spread < (SELECT AVG(volume_spread) * 0.5 FROM market_states WHERE symbol = ms.symbol AND volume_spread > 0)
ORDER BY spread_ratio ASC
LIMIT 100;

-- ============================================================================
-- 4. THERMODYNAMISCHE SIGNALE: Kombinierte Indikatoren
-- ============================================================================
-- Hoher Druck + Sinkender Spread + Steigende Temperatur = Kaufsignal

SELECT 
    symbol,
    datetime(timestamp/1000, 'unixepoch') as time,
    temperature as price,
    pressure,
    volume_spread,
    (pressure / volume_spread) as tension,
    LAG(volume_spread, 1) OVER (PARTITION BY symbol ORDER BY timestamp) as prev_spread,
    LAG(temperature, 1) OVER (PARTITION BY symbol ORDER BY timestamp) as prev_temp,
    CASE 
        WHEN pressure > 100 
         AND volume_spread < LAG(volume_spread, 1) OVER (PARTITION BY symbol ORDER BY timestamp)
         AND temperature > LAG(temperature, 1) OVER (PARTITION BY symbol ORDER BY timestamp)
        THEN 'BUY_SIGNAL'
        WHEN pressure < 50
         AND volume_spread > LAG(volume_spread, 1) OVER (PARTITION BY symbol ORDER BY timestamp)
        THEN 'SELL_SIGNAL'
        ELSE 'NEUTRAL'
    END as signal
FROM market_states
WHERE volume_spread > 0
ORDER BY timestamp DESC
LIMIT 200;

-- ============================================================================
-- 5. STATISTIK: Gesamtübersicht der gespeicherten Daten
-- ============================================================================

SELECT 
    symbol,
    COUNT(*) as total_states,
    MIN(datetime(timestamp/1000, 'unixepoch')) as earliest_data,
    MAX(datetime(timestamp/1000, 'unixepoch')) as latest_data,
    AVG(temperature) as avg_price,
    AVG(pressure) as avg_pressure,
    AVG(volume_spread) as avg_spread,
    AVG(pressure / NULLIF(volume_spread, 0)) as avg_tension
FROM market_states
WHERE volume_spread > 0
GROUP BY symbol
ORDER BY total_states DESC;
