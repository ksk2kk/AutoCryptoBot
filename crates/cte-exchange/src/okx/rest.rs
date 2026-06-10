use chrono::{DateTime, Utc};
use reqwest::Client;
use tracing::instrument;

use cte_core::{Candle, CteError, Exchange, MarketType, OrderBook, Result, Symbol, Timeframe, Trade};

use super::normalize;
use super::types::*;

pub struct OkxRestClient {
    client: Client,
    base_url: String,
}

impl OkxRestClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to build HTTP client"),
            base_url: base_url.to_string(),
        }
    }

    #[instrument(skip(self), fields(exchange = "okx"))]
    pub async fn ping(&self) -> Result<()> {
        let url = format!("{}/api/v5/public/time", self.base_url);
        let resp = self.client.get(&url).send().await.map_err(|e| {
            CteError::ConnectionFailed {
                exchange: Exchange::Okx,
                message: e.to_string(),
            }
        })?;

        if !resp.status().is_success() {
            return Err(CteError::ConnectionFailed {
                exchange: Exchange::Okx,
                message: format!("Ping returned status {}", resp.status()),
            });
        }

        tracing::debug!(exchange = "okx", "Ping successful");
        Ok(())
    }

    #[instrument(skip(self), fields(exchange = "okx", market_type = %market_type))]
    pub async fn fetch_symbols(&self, market_type: MarketType) -> Result<Vec<Symbol>> {
        let inst_type = normalize::okx_inst_type(market_type);
        let url = format!("{}/api/v5/public/instruments?instType={}", self.base_url, inst_type);

        let resp = self.client.get(&url).send().await.map_err(|e| {
            CteError::ConnectionFailed {
                exchange: Exchange::Okx,
                message: e.to_string(),
            }
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::Okx,
                endpoint: "/api/v5/public/instruments".to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let data: OkxResponse<OkxInstrument> = resp.json().await.map_err(|e| {
            CteError::RestApi {
                exchange: Exchange::Okx,
                endpoint: "/api/v5/public/instruments".to_string(),
                status: 200,
                body: format!("Parse error: {e}"),
            }
        })?;

        let symbols = data
            .data
            .iter()
            .filter(|i| i.state == "live")
            .map(|i| normalize::build_symbol(&i.inst_id, &i.base_ccy, &i.quote_ccy, market_type))
            .collect();

        Ok(symbols)
    }

    #[instrument(skip(self), fields(exchange = "okx", symbol = %symbol.raw_symbol, timeframe = %timeframe))]
    pub async fn fetch_candles(
        &self,
        symbol: &Symbol,
        timeframe: Timeframe,
        start: Option<DateTime<Utc>>,
        limit: Option<u32>,
    ) -> Result<Vec<Candle>> {
        let bar = timeframe.to_okx_str();
        let limit = limit.unwrap_or(100).min(300);
        let mut url = format!(
            "{}/api/v5/market/candles?instId={}&bar={}&limit={}",
            self.base_url, symbol.raw_symbol, bar, limit
        );

        if let Some(start_time) = start {
            url.push_str(&format!("&after={}", start_time.timestamp_millis()));
        }

        let resp = self.client.get(&url).send().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::Okx,
            endpoint: "/api/v5/market/candles".to_string(),
            status: 0,
            body: e.to_string(),
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::Okx,
                endpoint: "/api/v5/market/candles".to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let data: OkxResponse<OkxCandleRaw> = resp.json().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::Okx,
            endpoint: "/api/v5/market/candles".to_string(),
            status: 200,
            body: format!("Parse error: {e}"),
        })?;

        let candles: Vec<Candle> = data
            .data
            .iter()
            .filter_map(|raw| normalize::normalize_candle(raw, symbol, timeframe))
            .collect();

        tracing::debug!(exchange = "okx", symbol = %symbol.raw_symbol, count = candles.len(), "Fetched candles");
        Ok(candles)
    }

    #[instrument(skip(self), fields(exchange = "okx", symbol = %symbol.raw_symbol))]
    pub async fn fetch_orderbook(&self, symbol: &Symbol, depth: u32) -> Result<OrderBook> {
        let sz = depth.min(400);
        let url = format!(
            "{}/api/v5/market/books?instId={}&sz={}",
            self.base_url, symbol.raw_symbol, sz
        );

        let resp = self.client.get(&url).send().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::Okx,
            endpoint: "/api/v5/market/books".to_string(),
            status: 0,
            body: e.to_string(),
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::Okx,
                endpoint: "/api/v5/market/books".to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let data: OkxResponse<OkxDepthRaw> = resp.json().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::Okx,
            endpoint: "/api/v5/market/books".to_string(),
            status: 200,
            body: format!("Parse error: {e}"),
        })?;

        let book = data
            .data
            .first()
            .map(|raw| normalize::normalize_depth(raw, symbol))
            .unwrap_or(OrderBook {
                symbol: symbol.clone(),
                timestamp: Utc::now(),
                bids: vec![],
                asks: vec![],
                sequence: 0,
            });

        Ok(book)
    }

    #[instrument(skip(self), fields(exchange = "okx", symbol = %symbol.raw_symbol))]
    pub async fn fetch_recent_trades(&self, symbol: &Symbol, limit: u32) -> Result<Vec<Trade>> {
        let limit = limit.min(500);
        let url = format!(
            "{}/api/v5/market/trades?instId={}&limit={}",
            self.base_url, symbol.raw_symbol, limit
        );

        let resp = self.client.get(&url).send().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::Okx,
            endpoint: "/api/v5/market/trades".to_string(),
            status: 0,
            body: e.to_string(),
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::Okx,
                endpoint: "/api/v5/market/trades".to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let data: OkxResponse<OkxTradeRaw> = resp.json().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::Okx,
            endpoint: "/api/v5/market/trades".to_string(),
            status: 200,
            body: format!("Parse error: {e}"),
        })?;

        let trades = data.data.iter().map(|t| normalize::normalize_trade(t, symbol)).collect();
        Ok(trades)
    }
}
