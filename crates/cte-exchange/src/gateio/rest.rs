use chrono::{DateTime, Utc};
use reqwest::Client;
use tracing::instrument;

use cte_core::{Candle, CteError, Exchange, MarketType, OrderBook, Result, Symbol, Timeframe, Trade};

use super::normalize;
use super::types::*;

pub struct GateRestClient {
    client: Client,
    base_url: String,
}

impl GateRestClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to build HTTP client"),
            base_url: base_url.to_string(),
        }
    }

    #[instrument(skip(self), fields(exchange = "gateio"))]
    pub async fn ping(&self) -> Result<()> {
        let url = format!("{}/spot/time", self.base_url);
        let resp = self.client.get(&url).send().await.map_err(|e| {
            CteError::ConnectionFailed {
                exchange: Exchange::GateIo,
                message: e.to_string(),
            }
        })?;

        if !resp.status().is_success() {
            return Err(CteError::ConnectionFailed {
                exchange: Exchange::GateIo,
                message: format!("Ping returned status {}", resp.status()),
            });
        }

        tracing::debug!(exchange = "gateio", "Ping successful");
        Ok(())
    }

    #[instrument(skip(self), fields(exchange = "gateio", market_type = %market_type))]
    pub async fn fetch_symbols(&self, market_type: MarketType) -> Result<Vec<Symbol>> {
        match market_type {
            MarketType::Spot => self.fetch_spot_symbols().await,
            _ => self.fetch_futures_symbols().await,
        }
    }

    async fn fetch_spot_symbols(&self) -> Result<Vec<Symbol>> {
        let endpoint = "/spot/currency_pairs";
        let url = format!("{}{}", self.base_url, endpoint);

        let resp = self.client.get(&url).send().await.map_err(|e| {
            CteError::ConnectionFailed {
                exchange: Exchange::GateIo,
                message: e.to_string(),
            }
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::GateIo,
                endpoint: endpoint.to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let pairs: Vec<GateCurrencyPair> = resp.json().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::GateIo,
            endpoint: endpoint.to_string(),
            status: 200,
            body: format!("Parse error: {e}"),
        })?;

        let symbols = pairs
            .iter()
            .filter(|p| {
                p.trade_status
                    .as_deref()
                    .map(|s| s == "tradable")
                    .unwrap_or(true)
            })
            .map(|p| normalize::build_symbol(&p.id, &p.base, &p.quote, MarketType::Spot))
            .collect();

        Ok(symbols)
    }

    async fn fetch_futures_symbols(&self) -> Result<Vec<Symbol>> {
        let endpoint = "/futures/usdt/contracts";
        let url = format!("{}{}", self.base_url, endpoint);

        let resp = self.client.get(&url).send().await.map_err(|e| {
            CteError::ConnectionFailed {
                exchange: Exchange::GateIo,
                message: e.to_string(),
            }
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::GateIo,
                endpoint: endpoint.to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let data: Vec<serde_json::Value> = resp.json().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::GateIo,
            endpoint: endpoint.to_string(),
            status: 200,
            body: format!("Parse error: {e}"),
        })?;

        let symbols = data
            .iter()
            .filter_map(|v| {
                let name = v.get("name")?.as_str()?;
                let parts: Vec<&str> = name.split('_').collect();
                if parts.len() == 2 {
                    Some(normalize::build_symbol(
                        name,
                        parts[0],
                        parts[1],
                        MarketType::LinearPerpetual,
                    ))
                } else {
                    None
                }
            })
            .collect();

        Ok(symbols)
    }

    #[instrument(skip(self), fields(exchange = "gateio", symbol = %symbol.raw_symbol, timeframe = %timeframe))]
    pub async fn fetch_candles(
        &self,
        symbol: &Symbol,
        timeframe: Timeframe,
        start: Option<DateTime<Utc>>,
        limit: Option<u32>,
    ) -> Result<Vec<Candle>> {
        let pair = normalize::to_gate_pair(symbol);
        let interval = timeframe.to_gateio_str();
        let limit = limit.unwrap_or(200).min(1000);

        match symbol.market_type {
            MarketType::Spot => {
                self.fetch_spot_candles(symbol, &pair, interval, timeframe, start, limit)
                    .await
            }
            _ => {
                self.fetch_futures_candles(symbol, &pair, interval, timeframe, start, limit)
                    .await
            }
        }
    }

    async fn fetch_spot_candles(
        &self,
        symbol: &Symbol,
        pair: &str,
        interval: &str,
        timeframe: Timeframe,
        start: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<Candle>> {
        let endpoint = "/spot/candlesticks";
        let mut req = self
            .client
            .get(format!("{}{}", self.base_url, endpoint))
            .query(&[("currency_pair", pair), ("interval", interval)])
            .query(&[("limit", limit)]);

        if let Some(start_time) = start {
            req = req.query(&[("from", start_time.timestamp())]);
        }

        let resp = req.send().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::GateIo,
            endpoint: endpoint.to_string(),
            status: 0,
            body: e.to_string(),
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::GateIo,
                endpoint: endpoint.to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let raw: Vec<GateSpotCandleRaw> = resp.json().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::GateIo,
            endpoint: endpoint.to_string(),
            status: 200,
            body: format!("Parse error: {e}"),
        })?;

        let candles: Vec<Candle> = raw
            .iter()
            .filter_map(|k| normalize::normalize_spot_candle(k, symbol, timeframe))
            .collect();

        tracing::debug!(
            exchange = "gateio",
            symbol = %symbol.raw_symbol,
            count = candles.len(),
            "Fetched spot candles"
        );

        Ok(candles)
    }

    async fn fetch_futures_candles(
        &self,
        symbol: &Symbol,
        contract: &str,
        interval: &str,
        timeframe: Timeframe,
        start: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<Candle>> {
        let endpoint = "/futures/usdt/candlesticks";
        let mut req = self
            .client
            .get(format!("{}{}", self.base_url, endpoint))
            .query(&[("contract", contract), ("interval", interval)])
            .query(&[("limit", limit)]);

        if let Some(start_time) = start {
            req = req.query(&[("from", start_time.timestamp())]);
        }

        let resp = req.send().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::GateIo,
            endpoint: endpoint.to_string(),
            status: 0,
            body: e.to_string(),
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::GateIo,
                endpoint: endpoint.to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let raw: Vec<GateFuturesCandleRaw> = resp.json().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::GateIo,
            endpoint: endpoint.to_string(),
            status: 200,
            body: format!("Parse error: {e}"),
        })?;

        let candles: Vec<Candle> = raw
            .iter()
            .filter_map(|k| normalize::normalize_futures_candle(k, symbol, timeframe))
            .collect();

        tracing::debug!(
            exchange = "gateio",
            symbol = %symbol.raw_symbol,
            count = candles.len(),
            "Fetched futures candles"
        );

        Ok(candles)
    }

    #[instrument(skip(self), fields(exchange = "gateio", symbol = %symbol.raw_symbol))]
    pub async fn fetch_orderbook(&self, symbol: &Symbol, depth: u32) -> Result<OrderBook> {
        let pair = normalize::to_gate_pair(symbol);
        let limit = depth.min(100);

        let endpoint = match symbol.market_type {
            MarketType::Spot => "/spot/order_book",
            _ => "/futures/usdt/order_book",
        };

        let query_key = match symbol.market_type {
            MarketType::Spot => "currency_pair",
            _ => "contract",
        };

        let resp = self
            .client
            .get(format!("{}{}", self.base_url, endpoint))
            .query(&[(query_key, pair.as_str())])
            .query(&[("limit", limit)])
            .send()
            .await
            .map_err(|e| CteError::RestApi {
                exchange: Exchange::GateIo,
                endpoint: endpoint.to_string(),
                status: 0,
                body: e.to_string(),
            })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::GateIo,
                endpoint: endpoint.to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let raw: GateOrderBookRaw = resp.json().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::GateIo,
            endpoint: endpoint.to_string(),
            status: 200,
            body: format!("Parse error: {e}"),
        })?;

        Ok(normalize::normalize_depth(&raw, symbol))
    }

    #[instrument(skip(self), fields(exchange = "gateio", symbol = %symbol.raw_symbol))]
    pub async fn fetch_recent_trades(&self, symbol: &Symbol, limit: u32) -> Result<Vec<Trade>> {
        let pair = normalize::to_gate_pair(symbol);
        let limit = limit.min(1000);

        let endpoint = match symbol.market_type {
            MarketType::Spot => "/spot/trades",
            _ => "/futures/usdt/trades",
        };

        let query_key = match symbol.market_type {
            MarketType::Spot => "currency_pair",
            _ => "contract",
        };

        let resp = self
            .client
            .get(format!("{}{}", self.base_url, endpoint))
            .query(&[(query_key, pair.as_str())])
            .query(&[("limit", limit)])
            .send()
            .await
            .map_err(|e| CteError::RestApi {
                exchange: Exchange::GateIo,
                endpoint: endpoint.to_string(),
                status: 0,
                body: e.to_string(),
            })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::GateIo,
                endpoint: endpoint.to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let raw: Vec<GateTradeRaw> = resp.json().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::GateIo,
            endpoint: endpoint.to_string(),
            status: 200,
            body: format!("Parse error: {e}"),
        })?;

        let trades = raw
            .iter()
            .map(|t| normalize::normalize_trade(t, symbol))
            .collect();

        Ok(trades)
    }
}
