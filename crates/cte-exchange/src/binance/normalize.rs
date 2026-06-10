use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;

use cte_core::{Candle, Exchange, MarketType, OrderBook, OrderBookLevel, Symbol, Timeframe, Trade};

use super::types::*;

pub fn normalize_kline(raw: &BinanceKlineRaw, symbol: &Symbol, timeframe: Timeframe) -> Candle {
    Candle {
        symbol: symbol.clone(),
        timeframe,
        open_time: DateTime::from_timestamp_millis(raw.0)
            .unwrap_or_else(|| Utc::now()),
        close_time: DateTime::from_timestamp_millis(raw.6)
            .unwrap_or_else(|| Utc::now()),
        open: Decimal::from_str(&raw.1).unwrap_or_default(),
        high: Decimal::from_str(&raw.2).unwrap_or_default(),
        low: Decimal::from_str(&raw.3).unwrap_or_default(),
        close: Decimal::from_str(&raw.4).unwrap_or_default(),
        volume: Decimal::from_str(&raw.5).unwrap_or_default(),
        quote_volume: Decimal::from_str(&raw.7).unwrap_or_default(),
        trades_count: raw.8,
        is_closed: true,
    }
}

pub fn normalize_ws_kline(event: &BinanceWsKlineEvent, symbol: &Symbol, timeframe: Timeframe) -> Candle {
    Candle {
        symbol: symbol.clone(),
        timeframe,
        open_time: DateTime::from_timestamp_millis(event.kline.open_time)
            .unwrap_or_else(|| Utc::now()),
        close_time: DateTime::from_timestamp_millis(event.kline.close_time)
            .unwrap_or_else(|| Utc::now()),
        open: Decimal::from_str(&event.kline.open).unwrap_or_default(),
        high: Decimal::from_str(&event.kline.high).unwrap_or_default(),
        low: Decimal::from_str(&event.kline.low).unwrap_or_default(),
        close: Decimal::from_str(&event.kline.close).unwrap_or_default(),
        volume: Decimal::from_str(&event.kline.volume).unwrap_or_default(),
        quote_volume: Decimal::from_str(&event.kline.quote_volume).unwrap_or_default(),
        trades_count: event.kline.trades,
        is_closed: event.kline.is_closed,
    }
}

pub fn normalize_trade(raw: &BinanceTradeRaw, symbol: &Symbol) -> Trade {
    Trade {
        symbol: symbol.clone(),
        id: raw.id.to_string(),
        price: Decimal::from_str(&raw.price).unwrap_or_default(),
        quantity: Decimal::from_str(&raw.qty).unwrap_or_default(),
        timestamp: DateTime::from_timestamp_millis(raw.time)
            .unwrap_or_else(|| Utc::now()),
        is_buyer_maker: raw.is_buyer_maker,
    }
}

pub fn normalize_ws_trade(event: &BinanceWsTradeEvent, symbol: &Symbol) -> Trade {
    Trade {
        symbol: symbol.clone(),
        id: event.trade_id.to_string(),
        price: Decimal::from_str(&event.price).unwrap_or_default(),
        quantity: Decimal::from_str(&event.quantity).unwrap_or_default(),
        timestamp: DateTime::from_timestamp_millis(event.trade_time)
            .unwrap_or_else(|| Utc::now()),
        is_buyer_maker: event.is_buyer_maker,
    }
}

pub fn normalize_depth(raw: &BinanceDepthEvent, symbol: &Symbol) -> OrderBook {
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
        sequence: raw.last_update_id,
    }
}

pub fn normalize_ws_depth(event: &BinanceWsDepthEvent, symbol: &Symbol) -> OrderBook {
    OrderBook {
        symbol: symbol.clone(),
        timestamp: DateTime::from_timestamp_millis(event.event_time)
            .unwrap_or_else(|| Utc::now()),
        bids: event
            .bids
            .iter()
            .map(|b| OrderBookLevel {
                price: Decimal::from_str(&b[0]).unwrap_or_default(),
                quantity: Decimal::from_str(&b[1]).unwrap_or_default(),
            })
            .collect(),
        asks: event
            .asks
            .iter()
            .map(|a| OrderBookLevel {
                price: Decimal::from_str(&a[0]).unwrap_or_default(),
                quantity: Decimal::from_str(&a[1]).unwrap_or_default(),
            })
            .collect(),
        sequence: event.final_update_id.or(event.last_update_id).unwrap_or(0),
    }
}

pub fn build_symbol(raw: &str, base: &str, quote: &str, market_type: MarketType) -> Symbol {
    Symbol {
        base: base.to_uppercase(),
        quote: quote.to_uppercase(),
        market_type,
        exchange: Exchange::Binance,
        raw_symbol: raw.to_string(),
    }
}
