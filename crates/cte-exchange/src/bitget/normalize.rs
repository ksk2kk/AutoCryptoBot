use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;

use cte_core::{Candle, Exchange, MarketType, OrderBook, OrderBookLevel, Symbol, Timeframe, Trade};

use super::types::*;

pub fn normalize_kline(raw: &BitgetKlineRaw, symbol: &Symbol, timeframe: Timeframe) -> Option<Candle> {
    if raw.len() < 7 {
        return None;
    }

    let ts: i64 = raw[0].parse().ok()?;
    let close_time = ts + (timeframe.to_seconds() as i64 * 1000);

    Some(Candle {
        symbol: symbol.clone(),
        timeframe,
        open_time: DateTime::from_timestamp_millis(ts)?,
        close_time: DateTime::from_timestamp_millis(close_time)?,
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

pub fn normalize_ws_kline(raw: &BitgetKlineRaw, symbol: &Symbol, timeframe: Timeframe) -> Option<Candle> {
    // WS kline has the same format: ["ts","o","h","l","c","vol","quoteVol"]
    normalize_kline(raw, symbol, timeframe)
}

pub fn normalize_trade(raw: &BitgetTradeRaw, symbol: &Symbol) -> Trade {
    let ts: i64 = raw.ts.parse().unwrap_or(0);
    Trade {
        symbol: symbol.clone(),
        id: raw.trade_id.clone(),
        price: Decimal::from_str(&raw.price).unwrap_or_default(),
        quantity: Decimal::from_str(&raw.size).unwrap_or_default(),
        timestamp: DateTime::from_timestamp_millis(ts).unwrap_or_else(|| Utc::now()),
        is_buyer_maker: raw.side.eq_ignore_ascii_case("sell"),
    }
}

pub fn normalize_ws_trade(raw: &BitgetWsTrade, symbol: &Symbol) -> Trade {
    let ts: i64 = raw.ts.parse().unwrap_or(0);
    Trade {
        symbol: symbol.clone(),
        id: String::new(),
        price: Decimal::from_str(&raw.px).unwrap_or_default(),
        quantity: Decimal::from_str(&raw.sz).unwrap_or_default(),
        timestamp: DateTime::from_timestamp_millis(ts).unwrap_or_else(|| Utc::now()),
        is_buyer_maker: raw.side.eq_ignore_ascii_case("sell"),
    }
}

pub fn normalize_depth(raw: &BitgetOrderBookRaw, symbol: &Symbol) -> OrderBook {
    let ts: i64 = raw.ts.as_deref().and_then(|s| s.parse().ok()).unwrap_or(0);
    OrderBook {
        symbol: symbol.clone(),
        timestamp: DateTime::from_timestamp_millis(ts).unwrap_or_else(|| Utc::now()),
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
        sequence: 0,
    }
}

pub fn normalize_ws_depth(raw: &BitgetWsDepth, symbol: &Symbol) -> OrderBook {
    let ts: i64 = raw.ts.as_deref().and_then(|s| s.parse().ok()).unwrap_or(0);
    OrderBook {
        symbol: symbol.clone(),
        timestamp: DateTime::from_timestamp_millis(ts).unwrap_or_else(|| Utc::now()),
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
        sequence: 0,
    }
}

pub fn build_symbol(raw: &str, base: &str, quote: &str, market_type: MarketType) -> Symbol {
    Symbol {
        base: base.to_uppercase(),
        quote: quote.to_uppercase(),
        market_type,
        exchange: Exchange::Bitget,
        raw_symbol: raw.to_string(),
    }
}

pub fn bitget_product_type(market_type: MarketType) -> &'static str {
    match market_type {
        MarketType::Spot => "SPOT",
        MarketType::LinearPerpetual => "USDT-FUTURES",
        MarketType::InversePerpetual => "COIN-FUTURES",
        MarketType::Futures => "USDT-FUTURES",
    }
}

pub fn bitget_inst_type(market_type: MarketType) -> &'static str {
    match market_type {
        MarketType::Spot => "SPOT",
        MarketType::LinearPerpetual => "USDT-FUTURES",
        MarketType::InversePerpetual => "COIN-FUTURES",
        MarketType::Futures => "USDT-FUTURES",
    }
}
