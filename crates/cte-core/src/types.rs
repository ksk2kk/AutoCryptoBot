use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::timeframe::Timeframe;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Exchange {
    Binance,
    Okx,
    Bybit,
    GateIo,
    Bitget,
}

impl fmt::Display for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Binance => write!(f, "binance"),
            Self::Okx => write!(f, "okx"),
            Self::Bybit => write!(f, "bybit"),
            Self::GateIo => write!(f, "gateio"),
            Self::Bitget => write!(f, "bitget"),
        }
    }
}

impl std::str::FromStr for Exchange {
    type Err = crate::error::CteError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "binance" => Ok(Self::Binance),
            "okx" => Ok(Self::Okx),
            "bybit" => Ok(Self::Bybit),
            "gateio" | "gate.io" | "gate" => Ok(Self::GateIo),
            "bitget" => Ok(Self::Bitget),
            _ => Err(crate::error::CteError::Config(format!(
                "Unknown exchange: {s}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketType {
    Spot,
    LinearPerpetual,
    InversePerpetual,
    Futures,
}

impl fmt::Display for MarketType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spot => write!(f, "spot"),
            Self::LinearPerpetual => write!(f, "linear"),
            Self::InversePerpetual => write!(f, "inverse"),
            Self::Futures => write!(f, "futures"),
        }
    }
}

impl std::str::FromStr for MarketType {
    type Err = crate::error::CteError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "spot" => Ok(Self::Spot),
            "linear" | "linear_perpetual" | "perp" | "swap" => Ok(Self::LinearPerpetual),
            "inverse" | "inverse_perpetual" => Ok(Self::InversePerpetual),
            "futures" => Ok(Self::Futures),
            _ => Err(crate::error::CteError::Config(format!(
                "Unknown market type: {s}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol {
    pub base: String,
    pub quote: String,
    pub market_type: MarketType,
    pub exchange: Exchange,
    pub raw_symbol: String,
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}/{} ({})", self.exchange, self.base, self.quote, self.market_type)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub symbol: Symbol,
    pub timeframe: Timeframe,
    pub open_time: DateTime<Utc>,
    pub close_time: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub quote_volume: Decimal,
    pub trades_count: u64,
    pub is_closed: bool,
}

impl fmt::Display for Candle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} O:{} H:{} L:{} C:{} V:{}",
            self.open_time.format("%Y-%m-%d %H:%M:%S"),
            self.timeframe,
            self.open,
            self.high,
            self.low,
            self.close,
            self.volume
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub symbol: Symbol,
    pub id: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub timestamp: DateTime<Utc>,
    pub is_buyer_maker: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub price: Decimal,
    pub quantity: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub symbol: Symbol,
    pub timestamp: DateTime<Utc>,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub sequence: u64,
}

impl OrderBook {
    pub fn best_bid(&self) -> Option<&OrderBookLevel> {
        self.bids.first()
    }

    pub fn best_ask(&self) -> Option<&OrderBookLevel> {
        self.asks.first()
    }

    pub fn spread(&self) -> Option<Decimal> {
        match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => Some(ask.price - bid.price),
            _ => None,
        }
    }

    pub fn mid_price(&self) -> Option<Decimal> {
        match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => Some((ask.price + bid.price) / Decimal::TWO),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    Long,
    Short,
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Long => write!(f, "LONG"),
            Self::Short => write!(f, "SHORT"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Filled,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimOrder {
    pub id: uuid::Uuid,
    pub symbol: Symbol,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub filled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimPosition {
    pub id: uuid::Uuid,
    pub symbol: Symbol,
    pub side: Side,
    pub entry_price: Decimal,
    pub quantity: Decimal,
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub usd_size: Decimal,
}

impl SimPosition {
    pub fn update_pnl(&mut self, current_price: Decimal) {
        let price_diff = current_price - self.entry_price;
        self.unrealized_pnl = match self.side {
            Side::Long => price_diff * self.quantity,
            Side::Short => -price_diff * self.quantity,
        };
        self.usd_size = self.quantity * current_price;
    }

    pub fn pnl_percent(&self) -> Decimal {
        if self.entry_price.is_zero() {
            return Decimal::ZERO;
        }
        let entry_value = self.entry_price * self.quantity;
        if entry_value.is_zero() {
            return Decimal::ZERO;
        }
        (self.unrealized_pnl / entry_value) * Decimal::ONE_HUNDRED
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeadTrader {
    pub id: String,
    pub nickname: String,
    pub exchange: Exchange,
    pub roi_percent: Decimal,
    pub pnl_usd: Decimal,
    pub win_rate: Decimal,
    pub followers: u64,
    pub total_trades: u64,
    pub current_positions: Vec<TraderPosition>,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraderPosition {
    pub symbol: String,
    pub side: Side,
    pub entry_price: Decimal,
    pub mark_price: Decimal,
    pub size_usd: Decimal,
    pub pnl_percent: Decimal,
}

#[derive(Debug, Clone)]
pub enum MarketEvent {
    CandleUpdate(Candle),
    TradeUpdate(Trade),
    OrderBookUpdate(OrderBook),
}

#[derive(Debug, Clone)]
pub enum TradingSignal {
    OpenLong {
        symbol: Symbol,
        size_usd: Decimal,
        reason: String,
    },
    OpenShort {
        symbol: Symbol,
        size_usd: Decimal,
        reason: String,
    },
    CloseLong {
        symbol: Symbol,
        reason: String,
    },
    CloseShort {
        symbol: Symbol,
        reason: String,
    },
}
