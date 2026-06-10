use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rusqlite::{params, Connection};
use std::path::Path;
use std::str::FromStr;
use std::sync::Mutex;

use cte_core::{Candle, CteError, Exchange, MarketType, Result, Symbol, Timeframe};

use crate::schema;

pub struct CandleStore {
    conn: Mutex<Connection>,
}

impl CandleStore {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).map_err(|e| {
            CteError::Storage(format!("Failed to open SQLite at {}: {}", path.display(), e))
        })?;

        conn.execute_batch(schema::CREATE_TABLES).map_err(|e| {
            CteError::Storage(format!("Failed to create tables: {e}"))
        })?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| CteError::Storage(format!("Failed to set pragmas: {e}")))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert_candles(&self, candles: &[Candle]) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| {
            CteError::Storage(format!("Lock poisoned: {e}"))
        })?;

        let mut count = 0;
        let mut stmt = conn
            .prepare_cached(
                "INSERT OR REPLACE INTO candles (exchange, symbol, timeframe, open_time, close_time, open, high, low, close, volume, quote_volume, trades_count)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            )
            .map_err(|e| CteError::Storage(format!("Prepare failed: {e}")))?;

        for candle in candles {
            stmt.execute(params![
                candle.symbol.exchange.to_string(),
                candle.symbol.raw_symbol,
                candle.timeframe.to_string(),
                candle.open_time.timestamp_millis(),
                candle.close_time.timestamp_millis(),
                candle.open.to_string(),
                candle.high.to_string(),
                candle.low.to_string(),
                candle.close.to_string(),
                candle.volume.to_string(),
                candle.quote_volume.to_string(),
                candle.trades_count as i64,
            ])
            .map_err(|e| CteError::Storage(format!("Insert failed: {e}")))?;
            count += 1;
        }

        Ok(count)
    }

    pub fn query_candles(
        &self,
        exchange: Exchange,
        symbol_raw: &str,
        timeframe: Timeframe,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        limit: Option<u32>,
    ) -> Result<Vec<Candle>> {
        let conn = self.conn.lock().map_err(|e| {
            CteError::Storage(format!("Lock poisoned: {e}"))
        })?;

        let mut sql = String::from(
            "SELECT exchange, symbol, timeframe, open_time, close_time, open, high, low, close, volume, quote_volume, trades_count
             FROM candles WHERE exchange = ?1 AND symbol = ?2 AND timeframe = ?3",
        );

        if start.is_some() {
            sql.push_str(" AND open_time >= ?4");
        }
        if end.is_some() {
            sql.push_str(" AND open_time <= ?5");
        }
        sql.push_str(" ORDER BY open_time ASC");
        if let Some(lim) = limit {
            sql.push_str(&format!(" LIMIT {lim}"));
        }

        let mut stmt = conn.prepare(&sql).map_err(|e| {
            CteError::Storage(format!("Query prepare failed: {e}"))
        })?;

        let start_ms = start.map(|s| s.timestamp_millis()).unwrap_or(0);
        let end_ms = end.map(|e| e.timestamp_millis()).unwrap_or(i64::MAX);

        let rows = stmt
            .query_map(
                params![
                    exchange.to_string(),
                    symbol_raw,
                    timeframe.to_string(),
                    start_ms,
                    end_ms,
                ],
                |row| {
                    let exchange_str: String = row.get(0)?;
                    let symbol_str: String = row.get(1)?;
                    let tf_str: String = row.get(2)?;
                    let open_time_ms: i64 = row.get(3)?;
                    let close_time_ms: i64 = row.get(4)?;
                    let open_str: String = row.get(5)?;
                    let high_str: String = row.get(6)?;
                    let low_str: String = row.get(7)?;
                    let close_str: String = row.get(8)?;
                    let vol_str: String = row.get(9)?;
                    let qvol_str: String = row.get(10)?;
                    let trades: i64 = row.get(11)?;

                    Ok((
                        exchange_str,
                        symbol_str,
                        tf_str,
                        open_time_ms,
                        close_time_ms,
                        open_str,
                        high_str,
                        low_str,
                        close_str,
                        vol_str,
                        qvol_str,
                        trades,
                    ))
                },
            )
            .map_err(|e| CteError::Storage(format!("Query failed: {e}")))?;

        let mut candles = Vec::new();
        for row in rows {
            let (
                _exchange_str,
                symbol_str,
                tf_str,
                open_time_ms,
                close_time_ms,
                open_str,
                high_str,
                low_str,
                close_str,
                vol_str,
                qvol_str,
                trades,
            ) = row.map_err(|e| CteError::Storage(format!("Row read failed: {e}")))?;

            let tf: Timeframe = tf_str.parse().unwrap_or(Timeframe::M1);

            candles.push(Candle {
                symbol: Symbol {
                    base: String::new(),
                    quote: String::new(),
                    market_type: MarketType::LinearPerpetual,
                    exchange,
                    raw_symbol: symbol_str,
                },
                timeframe: tf,
                open_time: DateTime::from_timestamp_millis(open_time_ms).unwrap_or_else(|| Utc::now()),
                close_time: DateTime::from_timestamp_millis(close_time_ms).unwrap_or_else(|| Utc::now()),
                open: Decimal::from_str(&open_str).unwrap_or_default(),
                high: Decimal::from_str(&high_str).unwrap_or_default(),
                low: Decimal::from_str(&low_str).unwrap_or_default(),
                close: Decimal::from_str(&close_str).unwrap_or_default(),
                volume: Decimal::from_str(&vol_str).unwrap_or_default(),
                quote_volume: Decimal::from_str(&qvol_str).unwrap_or_default(),
                trades_count: trades as u64,
                is_closed: true,
            });
        }

        Ok(candles)
    }

    pub fn candle_count(&self, exchange: Exchange, symbol_raw: &str, timeframe: Timeframe) -> Result<u64> {
        let conn = self.conn.lock().map_err(|e| {
            CteError::Storage(format!("Lock poisoned: {e}"))
        })?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM candles WHERE exchange = ?1 AND symbol = ?2 AND timeframe = ?3",
                params![exchange.to_string(), symbol_raw, timeframe.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| CteError::Storage(format!("Count query failed: {e}")))?;

        Ok(count as u64)
    }
}
