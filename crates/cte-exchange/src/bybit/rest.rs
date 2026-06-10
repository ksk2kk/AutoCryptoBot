use chrono::{DateTime, Utc};
use reqwest::Client;
use tracing::instrument;

use cte_core::{Candle, CteError, Exchange, MarketType, OrderBook, Result, Symbol, Timeframe, Trade};

use super::normalize;
use super::types::*;

pub struct BybitRestClient {
    client: Client,
    base_url: String,
}

impl BybitRestClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to build HTTP client"),
            base_url: base_url.to_string(),
        }
    }

    #[instrument(skip(self), fields(exchange = "bybit"))]
    pub async fn ping(&self) -> Result<()> {
        let url = format!("{}/v5/market/time", self.base_url);
        let resp = self.client.get(&url).send().await.map_err(|e| {
            CteError::ConnectionFailed {
                exchange: Exchange::Bybit,
                message: e.to_string(),
            }
        })?;

        if !resp.status().is_success() {
            return Err(CteError::ConnectionFailed {
                exchange: Exchange::Bybit,
                message: format!("Ping returned status {}", resp.status()),
            });
        }

        tracing::debug!(exchange = "bybit", "Ping successful");
        Ok(())
    }

    #[instrument(skip(self), fields(exchange = "bybit", market_type = %market_type))]
    pub async fn fetch_symbols(&self, market_type: MarketType) -> Result<Vec<Symbol>> {
        let category = normalize::bybit_category(market_type);
        let url = format!(
            "{}/v5/market/instruments-info?category={}",
            self.base_url, category
        );

        let resp = self.client.get(&url).send().await.map_err(|e| {
            CteError::ConnectionFailed {
                exchange: Exchange::Bybit,
                message: e.to_string(),
            }
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::Bybit,
                endpoint: "/v5/market/instruments-info".to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let data: BybitResponse<BybitInstrumentsResult> =
            resp.json().await.map_err(|e| CteError::RestApi {
                exchange: Exchange::Bybit,
                endpoint: "/v5/market/instruments-info".to_string(),
                status: 200,
                body: format!("Parse error: {e}"),
            })?;

        if data.ret_code != 0 {
            return Err(CteError::RestApi {
                exchange: Exchange::Bybit,
                endpoint: "/v5/market/instruments-info".to_string(),
                status: 200,
                body: format!("API error code: {}, msg: {:?}", data.ret_code, data.ret_msg),
            });
        }

        let symbols = data
            .result
            .list
            .iter()
            .filter(|i| i.status == "Trading")
            .map(|i| {
                normalize::build_symbol(
                    &i.symbol,
                    i.base_coin.as_deref().unwrap_or(""),
                    i.quote_coin.as_deref().unwrap_or(""),
                    market_type,
                )
            })
            .collect();

        Ok(symbols)
    }

    #[instrument(skip(self), fields(exchange = "bybit", symbol = %symbol.raw_symbol, timeframe = %timeframe))]
    pub async fn fetch_candles(
        &self,
        symbol: &Symbol,
        timeframe: Timeframe,
        start: Option<DateTime<Utc>>,
        limit: Option<u32>,
    ) -> Result<Vec<Candle>> {
        let category = normalize::bybit_category(symbol.market_type);
        let interval = timeframe.to_bybit_str();
        let limit = limit.unwrap_or(200).min(1000);
        let endpoint = "/v5/market/kline";

        let mut req = self
            .client
            .get(format!("{}{}", self.base_url, endpoint))
            .query(&[
                ("category", category),
                ("symbol", &symbol.raw_symbol),
                ("interval", interval),
            ])
            .query(&[("limit", limit)]);

        if let Some(start_time) = start {
            req = req.query(&[("start", start_time.timestamp_millis() as u64)]);
        }

        let resp = req.send().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::Bybit,
            endpoint: endpoint.to_string(),
            status: 0,
            body: e.to_string(),
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::Bybit,
                endpoint: endpoint.to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let data: BybitResponse<BybitKlineResult> =
            resp.json().await.map_err(|e| CteError::RestApi {
                exchange: Exchange::Bybit,
                endpoint: endpoint.to_string(),
                status: 200,
                body: format!("Parse error: {e}"),
            })?;

        if data.ret_code != 0 {
            return Err(CteError::RestApi {
                exchange: Exchange::Bybit,
                endpoint: endpoint.to_string(),
                status: 200,
                body: format!("API error code: {}, msg: {:?}", data.ret_code, data.ret_msg),
            });
        }

        let candles: Vec<Candle> = data
            .result
            .list
            .iter()
            .filter_map(|k| normalize::normalize_kline(k, symbol, timeframe))
            .collect();

        tracing::debug!(
            exchange = "bybit",
            symbol = %symbol.raw_symbol,
            count = candles.len(),
            "Fetched candles"
        );

        Ok(candles)
    }

    #[instrument(skip(self), fields(exchange = "bybit", symbol = %symbol.raw_symbol))]
    pub async fn fetch_orderbook(&self, symbol: &Symbol, depth: u32) -> Result<OrderBook> {
        let category = normalize::bybit_category(symbol.market_type);
        let limit = depth.min(200);
        let endpoint = "/v5/market/orderbook";

        let resp = self
            .client
            .get(format!("{}{}", self.base_url, endpoint))
            .query(&[
                ("category", category),
                ("symbol", symbol.raw_symbol.as_str()),
            ])
            .query(&[("limit", limit)])
            .send()
            .await
            .map_err(|e| CteError::RestApi {
                exchange: Exchange::Bybit,
                endpoint: endpoint.to_string(),
                status: 0,
                body: e.to_string(),
            })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::Bybit,
                endpoint: endpoint.to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let data: BybitResponse<BybitOrderBookResult> =
            resp.json().await.map_err(|e| CteError::RestApi {
                exchange: Exchange::Bybit,
                endpoint: endpoint.to_string(),
                status: 200,
                body: format!("Parse error: {e}"),
            })?;

        if data.ret_code != 0 {
            return Err(CteError::RestApi {
                exchange: Exchange::Bybit,
                endpoint: endpoint.to_string(),
                status: 200,
                body: format!("API error code: {}, msg: {:?}", data.ret_code, data.ret_msg),
            });
        }

        Ok(normalize::normalize_depth(&data.result, symbol))
    }

    #[instrument(skip(self), fields(exchange = "bybit", symbol = %symbol.raw_symbol))]
    pub async fn fetch_recent_trades(&self, symbol: &Symbol, limit: u32) -> Result<Vec<Trade>> {
        let category = normalize::bybit_category(symbol.market_type);
        let limit = limit.min(1000);
        let endpoint = "/v5/market/recent-trade";

        let resp = self
            .client
            .get(format!("{}{}", self.base_url, endpoint))
            .query(&[
                ("category", category),
                ("symbol", symbol.raw_symbol.as_str()),
            ])
            .query(&[("limit", limit)])
            .send()
            .await
            .map_err(|e| CteError::RestApi {
                exchange: Exchange::Bybit,
                endpoint: endpoint.to_string(),
                status: 0,
                body: e.to_string(),
            })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::Bybit,
                endpoint: endpoint.to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let data: BybitResponse<BybitTradeResult> =
            resp.json().await.map_err(|e| CteError::RestApi {
                exchange: Exchange::Bybit,
                endpoint: endpoint.to_string(),
                status: 200,
                body: format!("Parse error: {e}"),
            })?;

        if data.ret_code != 0 {
            return Err(CteError::RestApi {
                exchange: Exchange::Bybit,
                endpoint: endpoint.to_string(),
                status: 200,
                body: format!("API error code: {}, msg: {:?}", data.ret_code, data.ret_msg),
            });
        }

        let trades = data
            .result
            .list
            .iter()
            .map(|t| normalize::normalize_trade(t, symbol))
            .collect();

        Ok(trades)
    }
}
