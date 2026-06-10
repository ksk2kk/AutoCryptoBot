use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rusqlite::{params, Connection};
use std::path::Path;
use std::str::FromStr;
use std::sync::Mutex;

use cte_core::{CteError, Exchange, MarketType, Result, Side, SimPosition, Symbol};

use crate::schema;

pub struct PositionStore {
    conn: Mutex<Connection>,
}

impl PositionStore {
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

    pub fn save_position(&self, pos: &SimPosition) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| {
            CteError::Storage(format!("Lock poisoned: {e}"))
        })?;

        conn.execute(
            "INSERT OR REPLACE INTO positions (id, exchange, symbol, side, entry_price, quantity, unrealized_pnl, realized_pnl, opened_at, closed_at, usd_size)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                pos.id.to_string(),
                pos.symbol.exchange.to_string(),
                pos.symbol.raw_symbol,
                format!("{}", pos.side),
                pos.entry_price.to_string(),
                pos.quantity.to_string(),
                pos.unrealized_pnl.to_string(),
                pos.realized_pnl.to_string(),
                pos.opened_at.timestamp_millis(),
                pos.closed_at.map(|t| t.timestamp_millis()),
                pos.usd_size.to_string(),
            ],
        )
        .map_err(|e| CteError::Storage(format!("Insert position failed: {e}")))?;

        Ok(())
    }

    pub fn get_open_positions(&self) -> Result<Vec<SimPosition>> {
        let conn = self.conn.lock().map_err(|e| {
            CteError::Storage(format!("Lock poisoned: {e}"))
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT id, exchange, symbol, side, entry_price, quantity, unrealized_pnl, realized_pnl, opened_at, closed_at, usd_size
                 FROM positions WHERE closed_at IS NULL ORDER BY opened_at DESC",
            )
            .map_err(|e| CteError::Storage(format!("Prepare failed: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let exchange_str: String = row.get(1)?;
                let symbol_str: String = row.get(2)?;
                let side_str: String = row.get(3)?;
                let entry_str: String = row.get(4)?;
                let qty_str: String = row.get(5)?;
                let upnl_str: String = row.get(6)?;
                let rpnl_str: String = row.get(7)?;
                let opened_ms: i64 = row.get(8)?;
                let closed_ms: Option<i64> = row.get(9)?;
                let usd_str: String = row.get(10)?;

                Ok((
                    id_str,
                    exchange_str,
                    symbol_str,
                    side_str,
                    entry_str,
                    qty_str,
                    upnl_str,
                    rpnl_str,
                    opened_ms,
                    closed_ms,
                    usd_str,
                ))
            })
            .map_err(|e| CteError::Storage(format!("Query failed: {e}")))?;

        let mut positions = Vec::new();
        for row in rows {
            let (id_str, exchange_str, symbol_str, side_str, entry_str, qty_str, upnl_str, rpnl_str, opened_ms, closed_ms, usd_str) =
                row.map_err(|e| CteError::Storage(format!("Row read failed: {e}")))?;

            let exchange: Exchange = exchange_str.parse().unwrap_or(Exchange::Binance);
            let side = if side_str == "LONG" { Side::Long } else { Side::Short };

            positions.push(SimPosition {
                id: uuid::Uuid::parse_str(&id_str).unwrap_or_else(|_| uuid::Uuid::new_v4()),
                symbol: Symbol {
                    base: String::new(),
                    quote: String::new(),
                    market_type: MarketType::LinearPerpetual,
                    exchange,
                    raw_symbol: symbol_str,
                },
                side,
                entry_price: Decimal::from_str(&entry_str).unwrap_or_default(),
                quantity: Decimal::from_str(&qty_str).unwrap_or_default(),
                unrealized_pnl: Decimal::from_str(&upnl_str).unwrap_or_default(),
                realized_pnl: Decimal::from_str(&rpnl_str).unwrap_or_default(),
                opened_at: DateTime::from_timestamp_millis(opened_ms).unwrap_or_else(|| Utc::now()),
                closed_at: closed_ms.and_then(DateTime::from_timestamp_millis),
                usd_size: Decimal::from_str(&usd_str).unwrap_or_default(),
            });
        }

        Ok(positions)
    }

    pub fn get_closed_positions(&self, limit: u32) -> Result<Vec<SimPosition>> {
        let conn = self.conn.lock().map_err(|e| {
            CteError::Storage(format!("Lock poisoned: {e}"))
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT id, exchange, symbol, side, entry_price, quantity, unrealized_pnl, realized_pnl, opened_at, closed_at, usd_size
                 FROM positions WHERE closed_at IS NOT NULL ORDER BY closed_at DESC LIMIT ?1",
            )
            .map_err(|e| CteError::Storage(format!("Prepare failed: {e}")))?;

        let rows = stmt
            .query_map(params![limit], |row| {
                let id_str: String = row.get(0)?;
                let exchange_str: String = row.get(1)?;
                let symbol_str: String = row.get(2)?;
                let side_str: String = row.get(3)?;
                let entry_str: String = row.get(4)?;
                let qty_str: String = row.get(5)?;
                let upnl_str: String = row.get(6)?;
                let rpnl_str: String = row.get(7)?;
                let opened_ms: i64 = row.get(8)?;
                let closed_ms: Option<i64> = row.get(9)?;
                let usd_str: String = row.get(10)?;

                Ok((
                    id_str, exchange_str, symbol_str, side_str, entry_str, qty_str, upnl_str,
                    rpnl_str, opened_ms, closed_ms, usd_str,
                ))
            })
            .map_err(|e| CteError::Storage(format!("Query failed: {e}")))?;

        let mut positions = Vec::new();
        for row in rows {
            let (id_str, exchange_str, symbol_str, side_str, entry_str, qty_str, upnl_str, rpnl_str, opened_ms, closed_ms, usd_str) =
                row.map_err(|e| CteError::Storage(format!("Row read: {e}")))?;

            let exchange: Exchange = exchange_str.parse().unwrap_or(Exchange::Binance);
            let side = if side_str == "LONG" { Side::Long } else { Side::Short };

            positions.push(SimPosition {
                id: uuid::Uuid::parse_str(&id_str).unwrap_or_else(|_| uuid::Uuid::new_v4()),
                symbol: Symbol {
                    base: String::new(),
                    quote: String::new(),
                    market_type: MarketType::LinearPerpetual,
                    exchange,
                    raw_symbol: symbol_str,
                },
                side,
                entry_price: Decimal::from_str(&entry_str).unwrap_or_default(),
                quantity: Decimal::from_str(&qty_str).unwrap_or_default(),
                unrealized_pnl: Decimal::from_str(&upnl_str).unwrap_or_default(),
                realized_pnl: Decimal::from_str(&rpnl_str).unwrap_or_default(),
                opened_at: DateTime::from_timestamp_millis(opened_ms).unwrap_or_else(|| Utc::now()),
                closed_at: closed_ms.and_then(DateTime::from_timestamp_millis),
                usd_size: Decimal::from_str(&usd_str).unwrap_or_default(),
            });
        }

        Ok(positions)
    }

    pub fn total_realized_pnl(&self) -> Result<Decimal> {
        let conn = self.conn.lock().map_err(|e| {
            CteError::Storage(format!("Lock poisoned: {e}"))
        })?;

        let mut stmt = conn
            .prepare("SELECT realized_pnl FROM positions WHERE closed_at IS NOT NULL")
            .map_err(|e| CteError::Storage(format!("Prepare failed: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                let pnl_str: String = row.get(0)?;
                Ok(pnl_str)
            })
            .map_err(|e| CteError::Storage(format!("Query failed: {e}")))?;

        let mut total = Decimal::ZERO;
        for row in rows {
            let pnl_str = row.map_err(|e| CteError::Storage(format!("Row: {e}")))?;
            total += Decimal::from_str(&pnl_str).unwrap_or_default();
        }

        Ok(total)
    }
}
