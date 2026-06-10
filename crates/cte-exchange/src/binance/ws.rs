use std::sync::Arc;

use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use tracing::{instrument, warn};

use cte_core::{Candle, CteError, Exchange, MarketType, OrderBook, Result, Symbol, Timeframe, Trade};

use super::normalize;
use super::types::*;

struct WsSubscription {
    cancel: CancellationToken,
}

pub struct BinanceWsManager {
    spot_ws_base: String,
    futures_ws_base: String,
    subscriptions: Arc<DashMap<String, WsSubscription>>,
    candle_senders: Arc<DashMap<String, broadcast::Sender<Candle>>>,
    trade_senders: Arc<DashMap<String, broadcast::Sender<Trade>>>,
    orderbook_senders: Arc<DashMap<String, broadcast::Sender<OrderBook>>>,
    global_cancel: CancellationToken,
}

impl BinanceWsManager {
    pub fn new(spot_ws_base: &str, futures_ws_base: &str) -> Self {
        Self {
            spot_ws_base: spot_ws_base.to_string(),
            futures_ws_base: futures_ws_base.to_string(),
            subscriptions: Arc::new(DashMap::new()),
            candle_senders: Arc::new(DashMap::new()),
            trade_senders: Arc::new(DashMap::new()),
            orderbook_senders: Arc::new(DashMap::new()),
            global_cancel: CancellationToken::new(),
        }
    }

    fn ws_base(&self, market_type: MarketType) -> &str {
        match market_type {
            MarketType::Spot => &self.spot_ws_base,
            _ => &self.futures_ws_base,
        }
    }

    pub async fn subscribe_candles(
        &self,
        symbol: &Symbol,
        timeframe: Timeframe,
    ) -> Result<broadcast::Receiver<Candle>> {
        let stream_name = format!(
            "{}@kline_{}",
            symbol.raw_symbol.to_lowercase(),
            timeframe.to_binance_str()
        );
        let key = format!("candle:{}:{}", symbol.raw_symbol, timeframe);

        if let Some(sender) = self.candle_senders.get(&key) {
            return Ok(sender.subscribe());
        }

        let (tx, rx) = broadcast::channel(1024);
        self.candle_senders.insert(key.clone(), tx.clone());

        let ws_url = format!("{}/{}", self.ws_base(symbol.market_type), stream_name);
        let cancel = self.global_cancel.child_token();
        let symbol_clone = symbol.clone();

        let subscriptions = self.subscriptions.clone();
        let sub_key = key.clone();

        tokio::spawn(async move {
            Self::run_kline_ws(ws_url, tx, symbol_clone, timeframe, cancel.clone()).await;
            subscriptions.remove(&sub_key);
        });

        self.subscriptions.insert(
            key,
            WsSubscription {
                cancel: self.global_cancel.child_token(),
            },
        );

        Ok(rx)
    }

    pub async fn subscribe_trades(
        &self,
        symbol: &Symbol,
    ) -> Result<broadcast::Receiver<Trade>> {
        let stream_name = format!("{}@aggTrade", symbol.raw_symbol.to_lowercase());
        let key = format!("trade:{}", symbol.raw_symbol);

        if let Some(sender) = self.trade_senders.get(&key) {
            return Ok(sender.subscribe());
        }

        let (tx, rx) = broadcast::channel(4096);
        self.trade_senders.insert(key.clone(), tx.clone());

        let ws_url = format!("{}/{}", self.ws_base(symbol.market_type), stream_name);
        let cancel = self.global_cancel.child_token();
        let symbol_clone = symbol.clone();
        let subscriptions = self.subscriptions.clone();
        let sub_key = key.clone();

        tokio::spawn(async move {
            Self::run_trade_ws(ws_url, tx, symbol_clone, cancel.clone()).await;
            subscriptions.remove(&sub_key);
        });

        self.subscriptions.insert(
            key,
            WsSubscription {
                cancel: self.global_cancel.child_token(),
            },
        );

        Ok(rx)
    }

    pub async fn subscribe_orderbook(
        &self,
        symbol: &Symbol,
    ) -> Result<broadcast::Receiver<OrderBook>> {
        let stream_name = format!("{}@depth20@100ms", symbol.raw_symbol.to_lowercase());
        let key = format!("depth:{}", symbol.raw_symbol);

        if let Some(sender) = self.orderbook_senders.get(&key) {
            return Ok(sender.subscribe());
        }

        let (tx, rx) = broadcast::channel(512);
        self.orderbook_senders.insert(key.clone(), tx.clone());

        let ws_url = format!("{}/{}", self.ws_base(symbol.market_type), stream_name);
        let cancel = self.global_cancel.child_token();
        let symbol_clone = symbol.clone();
        let subscriptions = self.subscriptions.clone();
        let sub_key = key.clone();

        tokio::spawn(async move {
            Self::run_depth_ws(ws_url, tx, symbol_clone, cancel.clone()).await;
            subscriptions.remove(&sub_key);
        });

        self.subscriptions.insert(
            key,
            WsSubscription {
                cancel: self.global_cancel.child_token(),
            },
        );

        Ok(rx)
    }

    pub async fn shutdown(&self) {
        self.global_cancel.cancel();
        self.subscriptions.clear();
        tracing::info!(exchange = "binance", "WebSocket manager shut down");
    }

    async fn run_kline_ws(
        url: String,
        tx: broadcast::Sender<Candle>,
        symbol: Symbol,
        timeframe: Timeframe,
        cancel: CancellationToken,
    ) {
        let mut backoff = 1u64;

        loop {
            if cancel.is_cancelled() {
                break;
            }

            tracing::info!(exchange = "binance", url = %url, "Connecting WebSocket (kline)");

            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    backoff = 1;
                    tracing::info!(exchange = "binance", stream = "kline", symbol = %symbol.raw_symbol, "WebSocket connected");

                    let (mut _write, mut read) = ws_stream.split();

                    loop {
                        tokio::select! {
                            _ = cancel.cancelled() => break,
                            msg = read.next() => {
                                match msg {
                                    Some(Ok(Message::Text(text))) => {
                                        match serde_json::from_str::<BinanceWsKlineEvent>(&text) {
                                            Ok(event) => {
                                                let candle = normalize::normalize_ws_kline(&event, &symbol, timeframe);
                                                let _ = tx.send(candle);
                                            }
                                            Err(e) => {
                                                tracing::trace!(exchange = "binance", error = %e, "Failed to parse kline WS message");
                                            }
                                        }
                                    }
                                    Some(Ok(Message::Ping(data))) => {
                                        let _ = _write.send(Message::Pong(data)).await;
                                    }
                                    Some(Ok(Message::Close(_))) | None => {
                                        tracing::warn!(exchange = "binance", "WebSocket closed (kline)");
                                        break;
                                    }
                                    Some(Err(e)) => {
                                        tracing::warn!(exchange = "binance", error = %e, "WebSocket error (kline)");
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        exchange = "binance",
                        error = %e,
                        backoff_secs = backoff,
                        "WebSocket connection failed (kline), retrying"
                    );
                }
            }

            if cancel.is_cancelled() {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
            backoff = (backoff * 2).min(60);
        }
    }

    async fn run_trade_ws(
        url: String,
        tx: broadcast::Sender<Trade>,
        symbol: Symbol,
        cancel: CancellationToken,
    ) {
        let mut backoff = 1u64;

        loop {
            if cancel.is_cancelled() {
                break;
            }

            tracing::info!(exchange = "binance", url = %url, "Connecting WebSocket (trade)");

            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    backoff = 1;
                    tracing::info!(exchange = "binance", stream = "trade", symbol = %symbol.raw_symbol, "WebSocket connected");

                    let (mut _write, mut read) = ws_stream.split();

                    loop {
                        tokio::select! {
                            _ = cancel.cancelled() => break,
                            msg = read.next() => {
                                match msg {
                                    Some(Ok(Message::Text(text))) => {
                                        match serde_json::from_str::<BinanceWsTradeEvent>(&text) {
                                            Ok(event) => {
                                                let trade = normalize::normalize_ws_trade(&event, &symbol);
                                                let _ = tx.send(trade);
                                            }
                                            Err(e) => {
                                                tracing::trace!(exchange = "binance", error = %e, "Failed to parse trade WS message");
                                            }
                                        }
                                    }
                                    Some(Ok(Message::Ping(data))) => {
                                        let _ = _write.send(Message::Pong(data)).await;
                                    }
                                    Some(Ok(Message::Close(_))) | None => {
                                        tracing::warn!(exchange = "binance", "WebSocket closed (trade)");
                                        break;
                                    }
                                    Some(Err(e)) => {
                                        tracing::warn!(exchange = "binance", error = %e, "WebSocket error (trade)");
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        exchange = "binance",
                        error = %e,
                        backoff_secs = backoff,
                        "WebSocket connection failed (trade), retrying"
                    );
                }
            }

            if cancel.is_cancelled() {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
            backoff = (backoff * 2).min(60);
        }
    }

    async fn run_depth_ws(
        url: String,
        tx: broadcast::Sender<OrderBook>,
        symbol: Symbol,
        cancel: CancellationToken,
    ) {
        let mut backoff = 1u64;

        loop {
            if cancel.is_cancelled() {
                break;
            }

            tracing::info!(exchange = "binance", url = %url, "Connecting WebSocket (depth)");

            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    backoff = 1;
                    tracing::info!(exchange = "binance", stream = "depth", symbol = %symbol.raw_symbol, "WebSocket connected");

                    let (mut _write, mut read) = ws_stream.split();

                    loop {
                        tokio::select! {
                            _ = cancel.cancelled() => break,
                            msg = read.next() => {
                                match msg {
                                    Some(Ok(Message::Text(text))) => {
                                        match serde_json::from_str::<BinanceWsDepthEvent>(&text) {
                                            Ok(event) => {
                                                let book = normalize::normalize_ws_depth(&event, &symbol);
                                                let _ = tx.send(book);
                                            }
                                            Err(e) => {
                                                tracing::trace!(exchange = "binance", error = %e, "Failed to parse depth WS message");
                                            }
                                        }
                                    }
                                    Some(Ok(Message::Ping(data))) => {
                                        let _ = _write.send(Message::Pong(data)).await;
                                    }
                                    Some(Ok(Message::Close(_))) | None => {
                                        tracing::warn!(exchange = "binance", "WebSocket closed (depth)");
                                        break;
                                    }
                                    Some(Err(e)) => {
                                        tracing::warn!(exchange = "binance", error = %e, "WebSocket error (depth)");
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        exchange = "binance",
                        error = %e,
                        backoff_secs = backoff,
                        "WebSocket connection failed (depth), retrying"
                    );
                }
            }

            if cancel.is_cancelled() {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
            backoff = (backoff * 2).min(60);
        }
    }
}
