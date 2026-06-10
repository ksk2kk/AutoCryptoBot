use std::sync::Arc;

use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

use cte_core::{Candle, OrderBook, Result, Symbol, Timeframe, Trade};

use super::normalize;
use super::types::*;

pub struct GateWsManager {
    ws_url: String,
    candle_senders: Arc<DashMap<String, broadcast::Sender<Candle>>>,
    trade_senders: Arc<DashMap<String, broadcast::Sender<Trade>>>,
    orderbook_senders: Arc<DashMap<String, broadcast::Sender<OrderBook>>>,
    global_cancel: CancellationToken,
}

impl GateWsManager {
    pub fn new(ws_url: &str) -> Self {
        Self {
            ws_url: ws_url.to_string(),
            candle_senders: Arc::new(DashMap::new()),
            trade_senders: Arc::new(DashMap::new()),
            orderbook_senders: Arc::new(DashMap::new()),
            global_cancel: CancellationToken::new(),
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

        let pair = normalize::to_gate_pair(symbol);
        let interval = timeframe.to_gateio_str();
        let now = chrono::Utc::now().timestamp();

        let sub_msg = serde_json::json!({
            "time": now,
            "channel": "spot.candlesticks",
            "event": "subscribe",
            "payload": [interval, pair]
        });

        let ws_url = self.ws_url.clone();
        let cancel = self.global_cancel.child_token();
        let symbol_clone = symbol.clone();

        tokio::spawn(async move {
            Self::run_ws(ws_url, sub_msg.to_string(), cancel, move |msg_text| {
                if let Ok(msg) = serde_json::from_str::<GateWsMessage>(&msg_text) {
                    if let Some(channel) = &msg.channel {
                        if channel == "spot.candlesticks" {
                            if let Some(event) = &msg.event {
                                if event == "update" {
                                    if let Some(result) = &msg.result {
                                        if let Ok(candle_data) =
                                            serde_json::from_value::<GateWsCandle>(result.clone())
                                        {
                                            if let Some(candle) = normalize::normalize_ws_candle(
                                                &candle_data,
                                                &symbol_clone,
                                                timeframe,
                                            ) {
                                                let _ = tx.send(candle);
                                            }
                                        }
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

        let pair = normalize::to_gate_pair(symbol);
        let now = chrono::Utc::now().timestamp();

        let sub_msg = serde_json::json!({
            "time": now,
            "channel": "spot.trades",
            "event": "subscribe",
            "payload": [pair]
        });

        let ws_url = self.ws_url.clone();
        let cancel = self.global_cancel.child_token();
        let symbol_clone = symbol.clone();

        tokio::spawn(async move {
            Self::run_ws(ws_url, sub_msg.to_string(), cancel, move |msg_text| {
                if let Ok(msg) = serde_json::from_str::<GateWsMessage>(&msg_text) {
                    if let Some(channel) = &msg.channel {
                        if channel == "spot.trades" {
                            if let Some(event) = &msg.event {
                                if event == "update" {
                                    if let Some(result) = &msg.result {
                                        if let Ok(trade_data) =
                                            serde_json::from_value::<GateWsTrade>(result.clone())
                                        {
                                            let trade = normalize::normalize_ws_trade(
                                                &trade_data,
                                                &symbol_clone,
                                            );
                                            let _ = tx.send(trade);
                                        }
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

        let pair = normalize::to_gate_pair(symbol);
        let now = chrono::Utc::now().timestamp();

        let sub_msg = serde_json::json!({
            "time": now,
            "channel": "spot.order_book_update",
            "event": "subscribe",
            "payload": [pair, "100ms"]
        });

        let ws_url = self.ws_url.clone();
        let cancel = self.global_cancel.child_token();
        let symbol_clone = symbol.clone();

        tokio::spawn(async move {
            Self::run_ws(ws_url, sub_msg.to_string(), cancel, move |msg_text| {
                if let Ok(msg) = serde_json::from_str::<GateWsMessage>(&msg_text) {
                    if let Some(channel) = &msg.channel {
                        if channel.contains("order_book") {
                            if let Some(event) = &msg.event {
                                if event == "update" {
                                    if let Some(result) = &msg.result {
                                        if let Ok(depth_data) =
                                            serde_json::from_value::<GateWsDepth>(result.clone())
                                        {
                                            let book = normalize::normalize_ws_depth(
                                                &depth_data,
                                                &symbol_clone,
                                            );
                                            let _ = tx.send(book);
                                        }
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

    pub async fn shutdown(&self) {
        self.global_cancel.cancel();
        tracing::info!(exchange = "gateio", "WebSocket manager shut down");
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

            tracing::info!(exchange = "gateio", url = %url, "Connecting WebSocket");

            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    backoff = 1;
                    let (mut write, mut read) = ws_stream.split();

                    if let Err(e) = write.send(Message::Text(subscribe_msg.clone())).await {
                        tracing::warn!(exchange = "gateio", error = %e, "Failed to send subscribe");
                        continue;
                    }

                    tracing::info!(exchange = "gateio", "WebSocket connected and subscribed");

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
                                        tracing::warn!(exchange = "gateio", "WebSocket closed");
                                        break;
                                    }
                                    Some(Err(e)) => {
                                        tracing::warn!(exchange = "gateio", error = %e, "WebSocket error");
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(exchange = "gateio", error = %e, backoff_secs = backoff, "WebSocket connection failed");
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
