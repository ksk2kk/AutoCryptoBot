use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;

use cte_core::{Candle, Exchange, MarketType, OrderBook, OrderBookLevel, Symbol, Timeframe, Trade};

use super::types::*;

pub fn normalize_kline(raw: &[String], symbol: &Symbol, timeframe: Timeframe) -> Option<Candle> {
    if raw.len() < 7 {
        return None;
    }

    let open_time_ms: i64 = raw[0].parse().ok()?;
    let close_time_ms = open_time_ms + (timeframe.to_seconds() as i64 * 1000);

    Some(Candle {
        symbol: symbol.clone(),
        timeframe,
        open_time: DateTime::from_timestamp_millis(open_time_ms)?,
        close_time: DateTime::from_timestamp_millis(close_time_ms)?,
        open: Decimal::from_str(&raw[1]).unwrap_or_default(),
        high: Decimal::from_str(&raw[2]).unwrap_or_default(),
        low: Decimal::from_str(&raw[3]).unwrap_or_default(),
        close: Decimal::from_str(&raw[4]).unwrap_or_default(),
        volume: Decimal::from_str(&raw[5]).unwrap_or_default(),
        quote_volume: Decimal::from_str(&raw[6]).unwrap_or_default(),
        trades_count: 0,
        is_closed: true,
    })
}

pub fn normalize_ws_kline(raw: &BybitWsKline, symbol: &Symbol, timeframe: Timeframe) -> Candle {
    Candle {
        symbol: symbol.clone(),
        timeframe,
        open_time: DateTime::from_timestamp_millis(raw.start as i64)
            .unwrap_or_else(|| Utc::now()),
        close_time: DateTime::from_timestamp_millis(raw.end as i64)
            .unwrap_or_else(|| Utc::now()),
        open: Decimal::from_str(&raw.open).unwrap_or_default(),
        high: Decimal::from_str(&raw.high).unwrap_or_default(),
        low: Decimal::from_str(&raw.low).unwrap_or_default(),
        close: Decimal::from_str(&raw.close).unwrap_or_default(),
        volume: Decimal::from_str(&raw.volume).unwrap_or_default(),
        quote_volume: Decimal::from_str(&raw.turnover).unwrap_or_default(),
        trades_count: 0,
        is_closed: raw.confirm,
    }
}

pub fn normalize_trade(raw: &BybitTradeRaw, symbol: &Symbol) -> Trade {
    let ts: i64 = raw.time.parse().unwrap_or(0);
    Trade {
        symbol: symbol.clone(),
        id: raw.exec_id.clone(),
        price: Decimal::from_str(&raw.price).unwrap_or_default(),
        quantity: Decimal::from_str(&raw.size).unwrap_or_default(),
        timestamp: DateTime::from_timestamp_millis(ts).unwrap_or_else(|| Utc::now()),
        is_buyer_maker: raw.side.eq_ignore_ascii_case("sell"),
    }
}

pub fn normalize_ws_trade(raw: &BybitWsTrade, symbol: &Symbol) -> Trade {
    Trade {
        symbol: symbol.clone(),
        id: raw.i.clone().unwrap_or_default(),
        price: Decimal::from_str(&raw.p).unwrap_or_default(),
        quantity: Decimal::from_str(&raw.v).unwrap_or_default(),
        timestamp: DateTime::from_timestamp_millis(raw.timestamp as i64)
            .unwrap_or_else(|| Utc::now()),
        is_buyer_maker: raw.side.eq_ignore_ascii_case("sell"),
    }
}

pub fn normalize_depth(raw: &BybitOrderBookResult, symbol: &Symbol) -> OrderBook {
    let ts = raw.ts.unwrap_or(0);
    OrderBook {
        symbol: symbol.clone(),
        timestamp: DateTime::from_timestamp_millis(ts as i64).unwrap_or_else(|| Utc::now()),
        bids: raw
            .b
            .iter()
            .map(|b| OrderBookLevel {
                price: Decimal::from_str(&b[0]).unwrap_or_default(),
                quantity: Decimal::from_str(&b[1]).unwrap_or_default(),
            })
            .collect(),
        asks: raw
            .a
            .iter()
            .map(|a| OrderBookLevel {
                price: Decimal::from_str(&a[0]).unwrap_or_default(),
                quantity: Decimal::from_str(&a[1]).unwrap_or_default(),
            })
            .collect(),
        sequence: raw.u.unwrap_or(0),
    }
}

pub fn normalize_ws_depth(raw: &BybitWsDepth, symbol: &Symbol) -> OrderBook {
    OrderBook {
        symbol: symbol.clone(),
        timestamp: Utc::now(),
        bids: raw
            .b
            .iter()
            .map(|b| OrderBookLevel {
                price: Decimal::from_str(&b[0]).unwrap_or_default(),
                quantity: Decimal::from_str(&b[1]).unwrap_or_default(),
            })
            .collect(),
        asks: raw
            .a
            .iter()
            .map(|a| OrderBookLevel {
                price: Decimal::from_str(&a[0]).unwrap_or_default(),
                quantity: Decimal::from_str(&a[1]).unwrap_or_default(),
            })
            .collect(),
        sequence: raw.u.unwrap_or(0),
    }
}

pub fn build_symbol(raw: &str, base: &str, quote: &str, market_type: MarketType) -> Symbol {
    Symbol {
        base: base.to_uppercase(),
        quote: quote.to_uppercase(),
        market_type,
        exchange: Exchange::Bybit,
        raw_symbol: raw.to_string(),
    }
}

pub fn bybit_category(market_type: MarketType) -> &'static str {
    match market_type {
        MarketType::Spot => "spot",
        MarketType::LinearPerpetual => "linear",
        MarketType::InversePerpetual => "inverse",
        MarketType::Futures => "linear",
    }
}
