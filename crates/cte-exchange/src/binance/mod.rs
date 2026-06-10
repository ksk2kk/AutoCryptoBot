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

use self::rest::BinanceRestClient;
use self::ws::BinanceWsManager;

pub struct BinanceConnector {
    rest: BinanceRestClient,
    ws: BinanceWsManager,
    connected: Arc<AtomicBool>,
}

impl BinanceConnector {
    pub fn new(
        spot_rest_url: &str,
        futures_rest_url: &str,
        spot_ws_url: &str,
        futures_ws_url: &str,
    ) -> Self {
        Self {
            rest: BinanceRestClient::new(spot_rest_url, futures_rest_url),
            ws: BinanceWsManager::new(spot_ws_url, futures_ws_url),
            connected: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait]
impl ExchangeConnector for BinanceConnector {
    fn exchange(&self) -> Exchange {
        Exchange::Binance
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    async fn connect(&self) -> Result<()> {
        self.rest.ping().await?;
        self.connected.store(true, Ordering::Relaxed);
        tracing::info!(exchange = "binance", "Connected to Binance");
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        self.ws.shutdown().await;
        self.connected.store(false, Ordering::Relaxed);
        tracing::info!(exchange = "binance", "Disconnected from Binance");
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
