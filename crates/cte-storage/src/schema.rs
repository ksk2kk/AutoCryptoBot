pub const CREATE_TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS candles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    exchange TEXT NOT NULL,
    symbol TEXT NOT NULL,
    timeframe TEXT NOT NULL,
    open_time INTEGER NOT NULL,
    close_time INTEGER NOT NULL,
    open TEXT NOT NULL,
    high TEXT NOT NULL,
    low TEXT NOT NULL,
    close TEXT NOT NULL,
    volume TEXT NOT NULL,
    quote_volume TEXT NOT NULL,
    trades_count INTEGER NOT NULL,
    UNIQUE(exchange, symbol, timeframe, open_time)
);

CREATE INDEX IF NOT EXISTS idx_candles_lookup
    ON candles(exchange, symbol, timeframe, open_time);

CREATE TABLE IF NOT EXISTS positions (
    id TEXT PRIMARY KEY,
    exchange TEXT NOT NULL,
    symbol TEXT NOT NULL,
    side TEXT NOT NULL,
    entry_price TEXT NOT NULL,
    quantity TEXT NOT NULL,
    unrealized_pnl TEXT NOT NULL DEFAULT '0',
    realized_pnl TEXT NOT NULL DEFAULT '0',
    opened_at INTEGER NOT NULL,
    closed_at INTEGER,
    usd_size TEXT NOT NULL DEFAULT '0'
);

CREATE INDEX IF NOT EXISTS idx_positions_open
    ON positions(closed_at) WHERE closed_at IS NULL;

CREATE TABLE IF NOT EXISTS trades_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    position_id TEXT NOT NULL,
    action TEXT NOT NULL,
    price TEXT NOT NULL,
    quantity TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    reason TEXT
);
"#;
