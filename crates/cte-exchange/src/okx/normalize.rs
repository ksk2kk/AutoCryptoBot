use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;

use cte_core::{Candle, Exchange, MarketType, OrderBook, OrderBookLevel, Symbol, Timeframe, Trade};

use super::types::*;

pub fn normalize_candle(raw: &OkxCandleRaw, symbol: &Symbol, timeframe: Timeframe) -> Option<Candle> {
    if raw.len() < 9 {
        return None;
    }

    let open_time_ms: i64 = raw[0].parse().ok()?;

    Some(Candle {
        symbol: symbol.clone(),
        timeframe,
        open_time: DateTime::from_timestamp_millis(open_time_ms)?,
        close_time: DateTime::from_timestamp_millis(open_time_ms + (timeframe.to_seconds() as i64 * 1000))?,
        open: Decimal::from_str(&raw[1]).unwrap_or_default(),
        high: Decimal::from_str(&raw[2]).unwrap_or_default(),
        low: Decimal::from_str(&raw[3]).unwrap_or_default(),
        close: Decimal::from_str(&raw[4]).unwrap_or_default(),
        volume: Decimal::from_str(&raw[5]).unwrap_or_default(),
        quote_volume: Decimal::from_str(&raw[7]).unwrap_or_default(),
        trades_count: 0,
        is_closed: raw.get(8).map(|s| s == "1").unwrap_or(true),
    })
}

pub fn normalize_trade(raw: &OkxTradeRaw, symbol: &Symbol) -> Trade {
    let ts: i64 = raw.ts.parse().unwrap_or(0);
    Trade {
        symbol: symbol.clone(),
        id: raw.trade_id.clone(),
        price: Decimal::from_str(&raw.px).unwrap_or_default(),
        quantity: Decimal::from_str(&raw.sz).unwrap_or_default(),
        timestamp: DateTime::from_timestamp_millis(ts).unwrap_or_else(|| Utc::now()),
        is_buyer_maker: raw.side == "sell",
    }
}

pub fn normalize_depth(raw: &OkxDepthRaw, symbol: &Symbol) -> OrderBook {
    let ts: i64 = raw.ts.parse().unwrap_or(0);
    OrderBook {
        symbol: symbol.clone(),
        timestamp: DateTime::from_timestamp_millis(ts).unwrap_or_else(|| Utc::now()),
        bids: raw
            .bids
            .iter()
            .filter_map(|b| {
                if b.len() >= 2 {
                    Some(OrderBookLevel {
                        price: Decimal::from_str(&b[0]).unwrap_or_default(),
                        quantity: Decimal::from_str(&b[1]).unwrap_or_default(),
                    })
                } else {
                    None
                }
            })
            .collect(),
        asks: raw
            .asks
            .iter()
            .filter_map(|a| {
                if a.len() >= 2 {
                    Some(OrderBookLevel {
                        price: Decimal::from_str(&a[0]).unwrap_or_default(),
                        quantity: Decimal::from_str(&a[1]).unwrap_or_default(),
                    })
                } else {
                    None
                }
            })
            .collect(),
        sequence: 0,
    }
}

pub fn build_symbol(inst_id: &str, base: &str, quote: &str, market_type: MarketType) -> Symbol {
    Symbol {
        base: base.to_uppercase(),
        quote: quote.to_uppercase(),
        market_type,
        exchange: Exchange::Okx,
        raw_symbol: inst_id.to_string(),
    }
}

pub fn okx_inst_type(market_type: MarketType) -> &'static str {
    match market_type {
        MarketType::Spot => "SPOT",
        MarketType::LinearPerpetual => "SWAP",
        MarketType::InversePerpetual => "SWAP",
        MarketType::Futures => "FUTURES",
    }
}
