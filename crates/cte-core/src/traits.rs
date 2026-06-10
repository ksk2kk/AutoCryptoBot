use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::broadcast;

use crate::{
    Candle, Exchange, MarketType, OrderBook, Result, Symbol, Timeframe, Trade,
    TradingSignal,
};

#[async_trait]
pub trait ExchangeConnector: Send + Sync + 'static {
    fn exchange(&self) -> Exchange;
    fn is_connected(&self) -> bool;

    async fn connect(&self) -> Result<()>;
    async fn disconnect(&self) -> Result<()>;

    async fn fetch_symbols(&self, market_type: MarketType) -> Result<Vec<Symbol>>;

    async fn fetch_candles(
        &self,
        symbol: &Symbol,
        timeframe: Timeframe,
        start: Option<DateTime<Utc>>,
        limit: Option<u32>,
    ) -> Result<Vec<Candle>>;

    async fn fetch_orderbook(&self, symbol: &Symbol, depth: u32) -> Result<OrderBook>;
    async fn fetch_recent_trades(&self, symbol: &Symbol, limit: u32) -> Result<Vec<Trade>>;

    async fn subscribe_candles(
        &self,
        symbol: &Symbol,
        timeframe: Timeframe,
    ) -> Result<broadcast::Receiver<Candle>>;

    async fn subscribe_trades(&self, symbol: &Symbol) -> Result<broadcast::Receiver<Trade>>;

    async fn subscribe_orderbook(
        &self,
        symbol: &Symbol,
    ) -> Result<broadcast::Receiver<OrderBook>>;
}

pub trait Strategy: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn on_candle(&mut self, candle: &Candle) -> Vec<TradingSignal>;
    fn on_trade(&mut self, trade: &Trade) -> Vec<TradingSignal>;
    fn on_orderbook(&mut self, book: &OrderBook) -> Vec<TradingSignal>;
    fn reset(&mut self);
}
