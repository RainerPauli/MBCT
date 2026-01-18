# 🌡️ MBCT - Movement Based Crypto Trading

> **Die physikalische Realität des Krypto-Marktes**

MBCT ist ein thermodynamisches Trading-Framework, das Marktbewegungen nicht als abstrakte Charts, sondern als **physikalische Zustände** interpretiert.

## 🔬 Die Kernidee: Thermodynamik statt Technische Analyse

Anstatt Candlesticks und Indikatoren zu analysieren, messen wir:

- **🌡️ Temperatur (T)**: Mid-Price - Die durchschnittliche "Energie" des Marktes
- **💨 Druck (P)**: Orderbook-Dichte - Die akkumulierte Liquidität in den Top 5 Levels
- **🌊 Volumen (V)**: Relativer Spread - Der "Raum" zwischen Bid und Ask
- **⚡ Spannung**: `P / V` - Die potenzielle Energie einer "gespannten Feder"

### Das Prinzip der gespannten Feder

Wenn **hoher Druck** (viel Liquidität) auf einen **engen Spread** (wenig Raum) trifft, entsteht **maximale Spannung** - ein Frühindikator für explosive Preisbewegungen.

```
Spannung = Druck / Spread

🔥 Hohe Spannung → Unmittelbar vor Explosion
⚠️  Mittlere Spannung → Beobachten
✅ Niedrige Spannung → Entspannter Markt
```

## 🏗️ Architektur

### 1. `trading-common`: Daten-Layer
- **SQLite-basiert** für lokale Forschung ohne externe Datenbank
- Speichert thermodynamische Zustände (`market_states` Tabelle)
- Cache-System für Live-Performance

### 2. `trading-core`: Exchange-Integration
- **Hyperliquid** als primäre Exchange (WebSocket L2 Orderbook)
- Trait-System für Multi-Exchange-Unterstützung
- Thermodynamische Zustandsberechnung

### 3. `research_engine`: Echtzeit-Analyse
- Verbindet sich mit Hyperliquid Testnet
- Berechnet `Temperature`, `Pressure`, `Volume`, `Tension`
- Speichert Zustände für spätere SQL-Analyse

## 🚀 Schnellstart

### Voraussetzungen
```bash
# Rust installieren
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Repository clonen
git clone https://github.com/YOUR_USERNAME/mbct.git
cd mbct
```

### Research Engine starten
```bash
cd trading-core
SQLX_OFFLINE=true cargo run --bin research_engine
```

**Erwartete Ausgabe:**
```
🚀 Starting Thermodynamic Research Engine...
🔌 Connecting to Hyperliquid stream for BTC...
🌡️  T: 94235.50 | 💨 P: 12.45 | 🌊 V: 0.0003 | ⚡ Tension: 41500.00 [CRITICAL_COMPRESSION]
```

## 📊 Forschungs-Queries

Die Datei `research/queries.sql` enthält SQL-Queries zur Analyse:

1. **Kritische Kompression**: Identifiziert Momente maximaler Spannung
2. **Druck-Entwicklung**: Zeitreihen-Analyse der Liquidität
3. **Spread-Anomalien**: Findet institutionelle Akkumulation
4. **Thermodynamische Signale**: Kombinierte Kauf-/Verkaufssignale

### Beispiel-Query
```sql
SELECT 
    datetime(timestamp/1000, 'unixepoch') as time,
    temperature as price,
    (pressure / volume_spread) as tension
FROM market_states
WHERE tension > 1000
ORDER BY tension DESC
LIMIT 10;
```

## 🧠 Die Philosophie: Gegen Ahriman

> **Ahriman** = Die Illusion, dass Fiat-Preise die Realität sind.

MBCT akzeptiert nicht die **Fiat-Logik** ("BTC kostet $94,000"). Stattdessen:
- Wir messen **physikalische Zustände** (Druck, Spannung, Entropie)
- Wir suchen **Ungleichgewichte** in der Energie-Verteilung
- Wir handeln, wenn die **gespannte Feder** sich entlädt

Der Markt ist kein Zufallsgenerator - er ist ein **thermodynamisches System**.

## 🛠️ Technologie-Stack

- **Rust**: Systemsprache für maximale Performance
- **SQLite**: Lokale Forschungsdatenbank (volle SQL-Macht ohne Server)
- **Hyperliquid**: Native DEX mit L2 Orderbook-Zugang
- **Tokio**: Async Runtime für WebSocket-Streams

## 📁 Struktur

```
mbct/
├── trading-common/       # Daten-Layer (SQLite, Cache, Types)
├── trading-core/         # Exchange-Integration (Hyperliquid)
│   └── src/bin/research_engine.rs  # Hauptanwendung
├── research/
│   └── queries.sql       # Analyse-Queries
└── data/
    └── mbct_research.db  # SQLite-Datenbank (automatisch erstellt)
```

## 🎯 Roadmap

- [x] SQLite-Integration
- [x] Hyperliquid WebSocket-Stream
- [x] Thermodynamische Zustandsberechnung
- [x] Spring Tension Formula
- [ ] Entropy-Berechnung (Orderbook-Unordnung)
- [ ] Adaptive Schwellenwerte (Machine Learning)
- [ ] Multi-Symbol Support
- [ ] Live-Trading-Engine

## 🤝 Für THE ALLIANCE

Dieses Framework ist Open Source, weil die Wahrheit über Märkte **physikalisch** ist, nicht proprietär.

Wenn du den Code verwendest:
1. Verstehe die Physik, nicht nur den Code
2. Teile deine Erkenntnisse über thermodynamische Regime
3. Erweitere die Forschung um neue Phasen-Übergänge

**Der Markt ist ein physikalisches System. Wir messen, wir verstehen, wir handeln.**

---

## 📜 Lizenz

MIT License - Siehe [LICENSE](LICENSE)

## ⚠️ Disclaimer

Dieses Framework dient der **Forschung**. Live-Trading mit echtem Kapital erfolgt auf eigene Gefahr. Die thermodynamische Analyse garantiert keine Gewinne - sie ist ein Werkzeug zum Verständnis der physikalischen Realität von Märkten.