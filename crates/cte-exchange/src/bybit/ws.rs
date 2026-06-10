use std::sync::Arc;

use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

use cte_core::{Candle, MarketType, OrderBook, Result, Symbol, Timeframe, Trade};

use super::normalize;
use super::types::*;

pub struct BybitWsManager {
    linear_ws_url: String,
    spot_ws_url: String,
    candle_senders: Arc<DashMap<String, broadcast::Sender<Candle>>>,
    trade_senders: Arc<DashMap<String, broadcast::Sender<Trade>>>,
    orderbook_senders: Arc<DashMap<String, broadcast::Sender<OrderBook>>>,
    global_cancel: CancellationToken,
}

impl BybitWsManager {
    pub fn new(linear_ws_url: &str, spot_ws_url: &str) -> Self {
        Self {
            linear_ws_url: linear_ws_url.to_string(),
            spot_ws_url: spot_ws_url.to_string(),
            candle_senders: Arc::new(DashMap::new()),
            trade_senders: Arc::new(DashMap::new()),
            orderbook_senders: Arc::new(DashMap::new()),
            global_cancel: CancellationToken::new(),
        }
    }

    fn ws_url(&self, market_type: MarketType) -> &str {
        match market_type {
            MarketType::Spot => &self.spot_ws_url,
            _ => &self.linear_ws_url,
        }
    }

    pub async fn subscribe_candles(
        &self,
        symbol: &Symbol,
        timeframe: Timeframe,
    ) -> Result<broadcast::Receiver<Candle>> {
        let key = format!("candle:{}:{}", symbol.raw_symbol, timeframe);
        if let Some(sender) = self.candle_senders.get(&key) {
            return Ok(sender.subscribe());
        }

        let (tx, rx) = broadcast::channel(1024);
        self.candle_senders.insert(key.clone(), tx.clone());

        let interval = timeframe.to_bybit_str();
        let topic = format!("kline.{}.{}", interval, symbol.raw_symbol);
        let sub_msg = serde_json::json!({
            "op": "subscribe",
            "args": [topic]
        });

        let ws_url = self.ws_url(symbol.market_type).to_string();
        let cancel = self.global_cancel.child_token();
        let symbol_clone = symbol.clone();

        tokio::spawn(async move {
            Self::run_ws(ws_url, sub_msg.to_string(), cancel, move |msg_text| {
                if let Ok(msg) = serde_json::from_str::<BybitWsMessage>(&msg_text) {
                    if let Some(topic) = &msg.topic {
                        if topic.starts_with("kline.") {
                            if let Some(data) = &msg.data {
                                if let Ok(klines) =
                                    serde_json::from_value::<Vec<BybitWsKline>>(data.clone())
                                {
                                    for raw in &klines {
                                        let candle = normalize::normalize_ws_kline(
                                            raw,
                                            &symbol_clone,
                                            timeframe,
                                        );
                                        let _ = tx.send(candle);
                                    }
                                }
                            }
                        }
                    }
                }
            })
            .await;
        });

        Ok(rx)
    }

    pub async fn subscribe_trades(
        &self,
        symbol: &Symbol,
    ) -> Result<broadcast::Receiver<Trade>> {
        let key = format!("trade:{}", symbol.raw_symbol);
        if let Some(sender) = self.trade_senders.get(&key) {
            return Ok(sender.subscribe());
        }

        let (tx, rx) = broadcast::channel(4096);
        self.trade_senders.insert(key.clone(), tx.clone());

        let topic = format!("publicTrade.{}", symbol.raw_symbol);
        let sub_msg = serde_json::json!({
            "op": "subscribe",
            "args": [topic]
        });

        let ws_url = self.ws_url(symbol.market_type).to_string();
        let cancel = self.global_cancel.child_token();
        let symbol_clone = symbol.clone();

        tokio::spawn(async move {
            Self::run_ws(ws_url, sub_msg.to_string(), cancel, move |msg_text| {
                if let Ok(msg) = serde_json::from_str::<BybitWsMessage>(&msg_text) {
                    if let Some(topic) = &msg.topic {
                        if topic.starts_with("publicTrade.") {
                            if let Some(data) = &msg.data {
                                if let Ok(trades) =
                                    serde_json::from_value::<Vec<BybitWsTrade>>(data.clone())
                                {
                                    for raw in &trades {
                                        let trade =
                                            normalize::normalize_ws_trade(raw, &symbol_clone);
                                        let _ = tx.send(trade);
                                    }
                                }
                            }
                        }
                    }
                }
            })
            .await;
        });

        Ok(rx)
    }

    pub async fn subscribe_orderbook(
        &self,
        symbol: &Symbol,
    ) -> Result<broadcast::Receiver<OrderBook>> {
        let key = format!("depth:{}", symbol.raw_symbol);
        if let Some(sender) = self.orderbook_senders.get(&key) {
            return Ok(sender.subscribe());
        }

        let (tx, rx) = broadcast::channel(512);
        self.orderbook_senders.insert(key.clone(), tx.clone());

        let topic = format!("orderbook.50.{}", symbol.raw_symbol);
        let sub_msg = serde_json::json!({
            "op": "subscribe",
            "args": [topic]
        });

        let ws_url = self.ws_url(symbol.market_type).to_string();
        let cancel = self.global_cancel.child_token();
        let symbol_clone = symbol.clone();

        tokio::spawn(async move {
            Self::run_ws(ws_url, sub_msg.to_string(), cancel, move |msg_text| {
                if let Ok(msg) = serde_json::from_str::<BybitWsMessage>(&msg_text) {
                    if let Some(topic) = &msg.topic {
                        if topic.starts_with("orderbook.") {
                            if let Some(data) = &msg.data {
                                if let Ok(depth) =
                                    serde_json::from_value::<BybitWsDepth>(data.clone())
                                {
                                    let book =
                                        normalize::normalize_ws_depth(&depth, &symbol_clone);
                                    let _ = tx.send(book);
                                }
                            }
                        }
                    }
                }
            })
            .await;
        });

        Ok(rx)
    }

    pub async fn shutdown(&self) {
        self.global_cancel.cancel();
        tracing::info!(exchange = "bybit", "WebSocket manager shut down");
    }

    async fn run_ws<F>(url: String, subscribe_msg: String, cancel: CancellationToken, handler: F)
    where
        F: Fn(String) + Send + 'static,
    {
        let mut backoff = 1u64;

        loop {
            if cancel.is_cancelled() {
                break;
            }

            tracing::info!(exchange = "bybit", url = %url, "Connecting WebSocket");

            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    backoff = 1;
                    let (mut write, mut read) = ws_stream.split();

                    if let Err(e) = write.send(Message::Text(subscribe_msg.clone())).await {
                        tracing::warn!(exchange = "bybit", error = %e, "Failed to send subscribe");
                        continue;
                    }

                    tracing::info!(exchange = "bybit", "WebSocket connected and subscribed");

                    loop {
                        tokio::select! {
                            _ = cancel.cancelled() => break,
                            msg = read.next() => {
                                match msg {
                                    Some(Ok(Message::Text(text))) => {
                                        handler(text);
                                    }
                                    Some(Ok(Message::Ping(data))) => {
                                        let _ = write.send(Message::Pong(data)).await;
                                    }
                                    Some(Ok(Message::Close(_))) | None => {
                                        tracing::warn!(exchange = "bybit", "WebSocket closed");
                                        break;
                                    }
                                    Some(Err(e)) => {
                                        tracing::warn!(exchange = "bybit", error = %e, "WebSocket error");
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(exchange = "bybit", error = %e, backoff_secs = backoff, "WebSocket connection failed");
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
