use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;

use cte_core::{Candle, Exchange, MarketType, OrderBook, OrderBookLevel, Symbol, Timeframe, Trade};

use super::types::*;

pub fn normalize_spot_candle(
    raw: &GateSpotCandleRaw,
    symbol: &Symbol,
    timeframe: Timeframe,
) -> Option<Candle> {
    if raw.len() < 8 {
        return None;
    }

    let ts_str = &raw[0];
    let ts: i64 = ts_str.parse().ok()?;
    // Gate.io spot candle timestamps are in seconds
    let open_time_ms = ts * 1000;
    let close_time_ms = open_time_ms + (timeframe.to_seconds() as i64 * 1000);

    Some(Candle {
        symbol: symbol.clone(),
        timeframe,
        open_time: DateTime::from_timestamp_millis(open_time_ms)?,
        close_time: DateTime::from_timestamp_millis(close_time_ms)?,
        open: Decimal::from_str(&raw[5]).unwrap_or_default(),
        high: Decimal::from_str(&raw[3]).unwrap_or_default(),
        low: Decimal::from_str(&raw[4]).unwrap_or_default(),
        close: Decimal::from_str(&raw[2]).unwrap_or_default(),
        volume: Decimal::from_str(&raw[6]).unwrap_or_default(),
        quote_volume: Decimal::from_str(&raw[1]).unwrap_or_default(),
        trades_count: 0,
        is_closed: raw.get(7).map(|s| s == "true").unwrap_or(true),
    })
}

pub fn normalize_futures_candle(
    raw: &GateFuturesCandleRaw,
    symbol: &Symbol,
    timeframe: Timeframe,
) -> Option<Candle> {
    let open_time_ms = raw.t * 1000;
    let close_time_ms = open_time_ms + (timeframe.to_seconds() as i64 * 1000);

    Some(Candle {
        symbol: symbol.clone(),
        timeframe,
        open_time: DateTime::from_timestamp_millis(open_time_ms)?,
        close_time: DateTime::from_timestamp_millis(close_time_ms)?,
        open: Decimal::from_str(&raw.o).unwrap_or_default(),
        high: Decimal::from_str(&raw.h).unwrap_or_default(),
        low: Decimal::from_str(&raw.l).unwrap_or_default(),
        close: Decimal::from_str(&raw.c).unwrap_or_default(),
        volume: Decimal::from(raw.v.unwrap_or(0)),
        quote_volume: raw
            .sum
            .as_deref()
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or_default(),
        trades_count: 0,
        is_closed: true,
    })
}

pub fn normalize_ws_candle(raw: &GateWsCandle, symbol: &Symbol, timeframe: Timeframe) -> Option<Candle> {
    let ts: i64 = raw.t.parse().ok()?;
    let open_time_ms = ts * 1000;
    let close_time_ms = open_time_ms + (timeframe.to_seconds() as i64 * 1000);

    Some(Candle {
        symbol: symbol.clone(),
        timeframe,
        open_time: DateTime::from_timestamp_millis(open_time_ms)?,
        close_time: DateTime::from_timestamp_millis(close_time_ms)?,
        open: Decimal::from_str(&raw.o).unwrap_or_default(),
        high: Decimal::from_str(&raw.h).unwrap_or_default(),
        low: Decimal::from_str(&raw.l).unwrap_or_default(),
        close: Decimal::from_str(&raw.c).unwrap_or_default(),
        volume: raw
            .v
            .as_deref()
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or_default(),
        quote_volume: Decimal::ZERO,
        trades_count: 0,
        is_closed: true,
    })
}

pub fn normalize_trade(raw: &GateTradeRaw, symbol: &Symbol) -> Trade {
    let ts_ms: i64 = raw
        .create_time_ms
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            raw.create_time
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .map(|t| t * 1000)
                .unwrap_or(0)
        });

    Trade {
        symbol: symbol.clone(),
        id: raw.id.clone(),
        price: Decimal::from_str(&raw.price).unwrap_or_default(),
        quantity: Decimal::from_str(&raw.amount).unwrap_or_default(),
        timestamp: DateTime::from_timestamp_millis(ts_ms).unwrap_or_else(|| Utc::now()),
        is_buyer_maker: raw.side == "sell",
    }
}

pub fn normalize_ws_trade(raw: &GateWsTrade, symbol: &Symbol) -> Trade {
    let ts_ms: i64 = raw
        .create_time_ms
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            raw.create_time
                .map(|t| (t * 1000.0) as i64)
                .unwrap_or(0)
        });

    Trade {
        symbol: symbol.clone(),
        id: raw.id.map(|i| i.to_string()).unwrap_or_default(),
        price: Decimal::from_str(&raw.price).unwrap_or_default(),
        quantity: Decimal::from_str(&raw.amount).unwrap_or_default(),
        timestamp: DateTime::from_timestamp_millis(ts_ms).unwrap_or_else(|| Utc::now()),
        is_buyer_maker: raw.side == "sell",
    }
}

pub fn normalize_depth(raw: &GateOrderBookRaw, symbol: &Symbol) -> OrderBook {
    OrderBook {
        symbol: symbol.clone(),
        timestamp: Utc::now(),
        bids: raw
            .bids
            .iter()
            .map(|b| OrderBookLevel {
                price: Decimal::from_str(&b[0]).unwrap_or_default(),
                quantity: Decimal::from_str(&b[1]).unwrap_or_default(),
            })
            .collect(),
        asks: raw
            .asks
            .iter()
            .map(|a| OrderBookLevel {
                price: Decimal::from_str(&a[0]).unwrap_or_default(),
                quantity: Decimal::from_str(&a[1]).unwrap_or_default(),
            })
            .collect(),
        sequence: raw.id.unwrap_or(0),
    }
}

pub fn normalize_ws_depth(raw: &GateWsDepth, symbol: &Symbol) -> OrderBook {
    OrderBook {
        symbol: symbol.clone(),
        timestamp: raw
            .t
            .and_then(|t| DateTime::from_timestamp_millis(t as i64))
            .unwrap_or_else(Utc::now),
        bids: raw
            .bids
            .iter()
            .map(|b| OrderBookLevel {
                price: Decimal::from_str(&b[0]).unwrap_or_default(),
                quantity: Decimal::from_str(&b[1]).unwrap_or_default(),
            })
            .collect(),
        asks: raw
            .asks
            .iter()
            .map(|a| OrderBookLevel {
                price: Decimal::from_str(&a[0]).unwrap_or_default(),
                quantity: Decimal::from_str(&a[1]).unwrap_or_default(),
            })
            .collect(),
        sequence: raw.last_update_id.unwrap_or(0),
    }
}

pub fn build_symbol(id: &str, base: &str, quote: &str, market_type: MarketType) -> Symbol {
    Symbol {
        base: base.to_uppercase(),
        quote: quote.to_uppercase(),
        market_type,
        exchange: Exchange::GateIo,
        raw_symbol: id.to_string(),
    }
}

/// Convert cte-core symbol to Gate.io pair format: BTC_USDT
pub fn to_gate_pair(symbol: &Symbol) -> String {
    // If raw_symbol already contains underscore, use it directly
    if symbol.raw_symbol.contains('_') {
        return symbol.raw_symbol.clone();
    }
    // Otherwise construct from base/quote
    format!("{}_{}", symbol.base, symbol.quote)
}
