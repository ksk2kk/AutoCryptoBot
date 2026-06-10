pub mod normalize;
pub mod rest;
pub mod types;
pub mod ws;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::broadcast;

use cte_core::{
    traits::ExchangeConnector, Candle, Exchange, MarketType, OrderBook, Result, Symbol, Timeframe,
    Trade,
};

use self::rest::GateRestClient;
use self::ws::GateWsManager;

pub struct GateIoConnector {
    rest: GateRestClient,
    ws: GateWsManager,
    connected: Arc<AtomicBool>,
}

impl GateIoConnector {
    pub fn new(rest_url: &str, ws_url: &str) -> Self {
        Self {
            rest: GateRestClient::new(rest_url),
            ws: GateWsManager::new(ws_url),
            connected: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait]
impl ExchangeConnector for GateIoConnector {
    fn exchange(&self) -> Exchange {
        Exchange::GateIo
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    async fn connect(&self) -> Result<()> {
        self.rest.ping().await?;
        self.connected.store(true, Ordering::Relaxed);
        tracing::info!(exchange = "gateio", "Connected to Gate.io");
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        self.ws.shutdown().await;
        self.connected.store(false, Ordering::Relaxed);
        tracing::info!(exchange = "gateio", "Disconnected from Gate.io");
        Ok(())
    }

    async fn fetch_symbols(&self, market_type: MarketType) -> Result<Vec<Symbol>> {
        self.rest.fetch_symbols(market_type).await
    }

    async fn fetch_candles(
        &self,
        symbol: &Symbol,
        timeframe: Timeframe,
        start: Option<DateTime<Utc>>,
        limit: Option<u32>,
    ) -> Result<Vec<Candle>> {
        self.rest.fetch_candles(symbol, timeframe, start, limit).await
    }

    async fn fetch_orderbook(&self, symbol: &Symbol, depth: u32) -> Result<OrderBook> {
        self.rest.fetch_orderbook(symbol, depth).await
    }

    async fn fetch_recent_trades(&self, symbol: &Symbol, limit: u32) -> Result<Vec<Trade>> {
        self.rest.fetch_recent_trades(symbol, limit).await
    }

    async fn subscribe_candles(
        &self,
        symbol: &Symbol,
        timeframe: Timeframe,
    ) -> Result<broadcast::Receiver<Candle>> {
        self.ws.subscribe_candles(symbol, timeframe).await
    }

    async fn subscribe_trades(&self, symbol: &Symbol) -> Result<broadcast::Receiver<Trade>> {
        self.ws.subscribe_trades(symbol).await
    }

    async fn subscribe_orderbook(
        &self,
        symbol: &Symbol,
    ) -> Result<broadcast::Receiver<OrderBook>> {
        self.ws.subscribe_orderbook(symbol).await
    }
}
