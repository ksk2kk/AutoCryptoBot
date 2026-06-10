use chrono::{DateTime, Utc};
use reqwest::Client;
use tracing::instrument;

use cte_core::{Candle, CteError, Exchange, MarketType, OrderBook, Result, Symbol, Timeframe, Trade};

use super::normalize;
use super::types::*;

pub struct BitgetRestClient {
    client: Client,
    base_url: String,
}

impl BitgetRestClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to build HTTP client"),
            base_url: base_url.to_string(),
        }
    }

    #[instrument(skip(self), fields(exchange = "bitget"))]
    pub async fn ping(&self) -> Result<()> {
        let url = format!(
            "{}/api/v2/mix/market/tickers?productType=USDT-FUTURES",
            self.base_url
        );
        let resp = self.client.get(&url).send().await.map_err(|e| {
            CteError::ConnectionFailed {
                exchange: Exchange::Bitget,
                message: e.to_string(),
            }
        })?;

        if !resp.status().is_success() {
            return Err(CteError::ConnectionFailed {
                exchange: Exchange::Bitget,
                message: format!("Ping returned status {}", resp.status()),
            });
        }

        tracing::debug!(exchange = "bitget", "Ping successful");
        Ok(())
    }

    #[instrument(skip(self), fields(exchange = "bitget", market_type = %market_type))]
    pub async fn fetch_symbols(&self, market_type: MarketType) -> Result<Vec<Symbol>> {
        match market_type {
            MarketType::Spot => self.fetch_spot_symbols().await,
            _ => self.fetch_futures_symbols(market_type).await,
        }
    }

    async fn fetch_spot_symbols(&self) -> Result<Vec<Symbol>> {
        let endpoint = "/api/v2/spot/public/symbols";
        let url = format!("{}{}", self.base_url, endpoint);

        let resp = self.client.get(&url).send().await.map_err(|e| {
            CteError::ConnectionFailed {
                exchange: Exchange::Bitget,
                message: e.to_string(),
            }
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::Bitget,
                endpoint: endpoint.to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let data: BitgetResponse<Vec<BitgetSpotSymbol>> =
            resp.json().await.map_err(|e| CteError::RestApi {
                exchange: Exchange::Bitget,
                endpoint: endpoint.to_string(),
                status: 200,
                body: format!("Parse error: {e}"),
            })?;

        if data.code != "00000" {
            return Err(CteError::RestApi {
                exchange: Exchange::Bitget,
                endpoint: endpoint.to_string(),
                status: 200,
                body: format!("API error: {} - {:?}", data.code, data.msg),
            });
        }

        let symbols = data
            .data
            .iter()
            .filter(|s| s.status == "online")
            .map(|s| normalize::build_symbol(&s.symbol, &s.base_coin, &s.quote_coin, MarketType::Spot))
            .collect();

        Ok(symbols)
    }

    async fn fetch_futures_symbols(&self, market_type: MarketType) -> Result<Vec<Symbol>> {
        let product_type = normalize::bitget_product_type(market_type);
        let endpoint = "/api/v2/mix/market/tickers";
        let url = format!("{}{}?productType={}", self.base_url, endpoint, product_type);

        let resp = self.client.get(&url).send().await.map_err(|e| {
            CteError::ConnectionFailed {
                exchange: Exchange::Bitget,
                message: e.to_string(),
            }
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::Bitget,
                endpoint: endpoint.to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let data: BitgetResponse<Vec<BitgetTicker>> =
            resp.json().await.map_err(|e| CteError::RestApi {
                exchange: Exchange::Bitget,
                endpoint: endpoint.to_string(),
                status: 200,
                body: format!("Parse error: {e}"),
            })?;

        if data.code != "00000" {
            return Err(CteError::RestApi {
                exchange: Exchange::Bitget,
                endpoint: endpoint.to_string(),
                status: 200,
                body: format!("API error: {} - {:?}", data.code, data.msg),
            });
        }

        let symbols = data
            .data
            .iter()
            .map(|t| {
                let base = t.base_coin.as_deref().unwrap_or("");
                let quote = t.quote_coin.as_deref().unwrap_or("USDT");
                normalize::build_symbol(&t.symbol, base, quote, market_type)
            })
            .collect();

        Ok(symbols)
    }

    #[instrument(skip(self), fields(exchange = "bitget", symbol = %symbol.raw_symbol, timeframe = %timeframe))]
    pub async fn fetch_candles(
        &self,
        symbol: &Symbol,
        timeframe: Timeframe,
        start: Option<DateTime<Utc>>,
        limit: Option<u32>,
    ) -> Result<Vec<Candle>> {
        let granularity = timeframe.to_bitget_str();
        let limit = limit.unwrap_or(200).min(1000);

        let endpoint = match symbol.market_type {
            MarketType::Spot => "/api/v2/spot/market/candles",
            _ => "/api/v2/mix/market/candles",
        };

        let mut req = self.client.get(format!("{}{}", self.base_url, endpoint));

        match symbol.market_type {
            MarketType::Spot => {
                req = req.query(&[
                    ("symbol", symbol.raw_symbol.as_str()),
                    ("granularity", granularity),
                ]);
            }
            _ => {
                let product_type = normalize::bitget_product_type(symbol.market_type);
                req = req.query(&[
                    ("productType", product_type),
                    ("symbol", symbol.raw_symbol.as_str()),
                    ("granularity", granularity),
                ]);
            }
        }

        req = req.query(&[("limit", &limit.to_string())]);

        if let Some(start_time) = start {
            req = req.query(&[("startTime", &start_time.timestamp_millis().to_string())]);
        }

        let resp = req.send().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::Bitget,
            endpoint: endpoint.to_string(),
            status: 0,
            body: e.to_string(),
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::Bitget,
                endpoint: endpoint.to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let data: BitgetResponse<Vec<BitgetKlineRaw>> =
            resp.json().await.map_err(|e| CteError::RestApi {
                exchange: Exchange::Bitget,
                endpoint: endpoint.to_string(),
                status: 200,
                body: format!("Parse error: {e}"),
            })?;

        if data.code != "00000" {
            return Err(CteError::RestApi {
                exchange: Exchange::Bitget,
                endpoint: endpoint.to_string(),
                status: 200,
                body: format!("API error: {} - {:?}", data.code, data.msg),
            });
        }

        let candles: Vec<Candle> = data
            .data
            .iter()
            .filter_map(|k| normalize::normalize_kline(k, symbol, timeframe))
            .collect();

        tracing::debug!(
            exchange = "bitget",
            symbol = %symbol.raw_symbol,
            count = candles.len(),
            "Fetched candles"
        );

        Ok(candles)
    }

    #[instrument(skip(self), fields(exchange = "bitget", symbol = %symbol.raw_symbol))]
    pub async fn fetch_orderbook(&self, symbol: &Symbol, depth: u32) -> Result<OrderBook> {
        let limit = depth.min(150);

        let endpoint = match symbol.market_type {
            MarketType::Spot => "/api/v2/spot/market/merge-depth",
            _ => "/api/v2/mix/market/merge-depth",
        };

        let mut req = self
            .client
            .get(format!("{}{}", self.base_url, endpoint))
            .query(&[("symbol", symbol.raw_symbol.as_str())])
            .query(&[("limit", limit)]);

        if symbol.market_type != MarketType::Spot {
            let product_type = normalize::bitget_product_type(symbol.market_type);
            req = req.query(&[("productType", product_type)]);
        }

        let resp = req.send().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::Bitget,
            endpoint: endpoint.to_string(),
            status: 0,
            body: e.to_string(),
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::Bitget,
                endpoint: endpoint.to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let data: BitgetResponse<BitgetOrderBookRaw> =
            resp.json().await.map_err(|e| CteError::RestApi {
                exchange: Exchange::Bitget,
                endpoint: endpoint.to_string(),
                status: 200,
                body: format!("Parse error: {e}"),
            })?;

        if data.code != "00000" {
            return Err(CteError::RestApi {
                exchange: Exchange::Bitget,
                endpoint: endpoint.to_string(),
                status: 200,
                body: format!("API error: {} - {:?}", data.code, data.msg),
            });
        }

        Ok(normalize::normalize_depth(&data.data, symbol))
    }

    #[instrument(skip(self), fields(exchange = "bitget", symbol = %symbol.raw_symbol))]
    pub async fn fetch_recent_trades(&self, symbol: &Symbol, limit: u32) -> Result<Vec<Trade>> {
        let limit = limit.min(500);

        let endpoint = match symbol.market_type {
            MarketType::Spot => "/api/v2/spot/market/fills",
            _ => "/api/v2/mix/market/fills",
        };

        let mut req = self
            .client
            .get(format!("{}{}", self.base_url, endpoint))
            .query(&[("symbol", symbol.raw_symbol.as_str())])
            .query(&[("limit", limit)]);

        if symbol.market_type != MarketType::Spot {
            let product_type = normalize::bitget_product_type(symbol.market_type);
            req = req.query(&[("productType", product_type)]);
        }

        let resp = req.send().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::Bitget,
            endpoint: endpoint.to_string(),
            status: 0,
            body: e.to_string(),
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::Bitget,
                endpoint: endpoint.to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let data: BitgetResponse<Vec<BitgetTradeRaw>> =
            resp.json().await.map_err(|e| CteError::RestApi {
                exchange: Exchange::Bitget,
                endpoint: endpoint.to_string(),
                status: 200,
                body: format!("Parse error: {e}"),
            })?;

        if data.code != "00000" {
            return Err(CteError::RestApi {
                exchange: Exchange::Bitget,
                endpoint: endpoint.to_string(),
                status: 200,
                body: format!("API error: {} - {:?}", data.code, data.msg),
            });
        }

        let trades = data
            .data
            .iter()
            .map(|t| normalize::normalize_trade(t, symbol))
            .collect();

        Ok(trades)
    }
}
