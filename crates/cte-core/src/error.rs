use rust_decimal::Decimal;
use thiserror::Error;

use crate::Exchange;

#[derive(Debug, Error)]
pub enum CteError {
    #[error("Exchange connection failed: {exchange} - {message}")]
    ConnectionFailed { exchange: Exchange, message: String },

    #[error("WebSocket error on {exchange}: {message}")]
    WebSocket { exchange: Exchange, message: String },

    #[error("REST API error: {exchange} {endpoint} returned {status}: {body}")]
    RestApi {
        exchange: Exchange,
        endpoint: String,
        status: u16,
        body: String,
    },

    #[error("Rate limited by {exchange}, retry after {retry_after_ms}ms")]
    RateLimited {
        exchange: Exchange,
        retry_after_ms: u64,
    },

    #[error("Deserialization failed for {exchange} {context}: {source}")]
    Deserialization {
        exchange: Exchange,
        context: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Strategy error: {0}")]
    Strategy(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance {
        required: Decimal,
        available: Decimal,
    },

    #[error("Position limit reached: max {max} positions")]
    PositionLimitReached { max: usize },

    #[error("Scraper error: {origin} - {message}")]
    Scraper { origin: String, message: String },

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, CteError>;
