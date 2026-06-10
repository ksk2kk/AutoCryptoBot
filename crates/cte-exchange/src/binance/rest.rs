use chrono::{DateTime, Utc};
use reqwest::Client;
use tracing::instrument;

use cte_core::{
    Candle, CteError, Exchange, MarketType, OrderBook, Result, Symbol, Timeframe, Trade,
};

use super::normalize;
use super::types::*;

pub struct BinanceRestClient {
    client: Client,
    spot_base: String,
    futures_base: String,
}

impl BinanceRestClient {
    pub fn new(spot_base: &str, futures_base: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to build HTTP client"),
            spot_base: spot_base.to_string(),
            futures_base: futures_base.to_string(),
        }
    }

    fn base_url(&self, market_type: MarketType) -> &str {
        match market_type {
            MarketType::Spot => &self.spot_base,
            _ => &self.futures_base,
        }
    }

    #[instrument(skip(self), fields(exchange = "binance"))]
    pub async fn ping(&self) -> Result<()> {
        let url = format!("{}/api/v3/ping", self.spot_base);
        let resp = self.client.get(&url).send().await.map_err(|e| {
            CteError::ConnectionFailed {
                exchange: Exchange::Binance,
                message: e.to_string(),
            }
        })?;

        if !resp.status().is_success() {
            return Err(CteError::ConnectionFailed {
                exchange: Exchange::Binance,
                message: format!("Ping returned status {}", resp.status()),
            });
        }

        tracing::debug!(exchange = "binance", "Ping successful");
        Ok(())
    }

    #[instrument(skip(self), fields(exchange = "binance", market_type = %market_type))]
    pub async fn fetch_symbols(&self, market_type: MarketType) -> Result<Vec<Symbol>> {
        match market_type {
            MarketType::Spot => self.fetch_spot_symbols().await,
            MarketType::LinearPerpetual => self.fetch_futures_symbols().await,
            _ => self.fetch_futures_symbols().await,
        }
    }

    async fn fetch_spot_symbols(&self) -> Result<Vec<Symbol>> {
        let url = format!("{}/api/v3/exchangeInfo", self.spot_base);
        let resp = self.client.get(&url).send().await.map_err(|e| {
            CteError::ConnectionFailed {
                exchange: Exchange::Binance,
                message: e.to_string(),
            }
        })?;

        let info: BinanceExchangeInfo = resp.json().await.map_err(|e| {
            CteError::ConnectionFailed {
                exchange: Exchange::Binance,
                message: format!("Failed to parse exchange info: {e}"),
            }
        })?;

        let symbols = info
            .symbols
            .iter()
            .filter(|s| s.status == "TRADING")
            .map(|s| normalize::build_symbol(&s.symbol, &s.base_asset, &s.quote_asset, MarketType::Spot))
            .collect();

        Ok(symbols)
    }

    async fn fetch_futures_symbols(&self) -> Result<Vec<Symbol>> {
        let url = format!("{}/fapi/v1/exchangeInfo", self.futures_base);
        let resp = self.client.get(&url).send().await.map_err(|e| {
            CteError::ConnectionFailed {
                exchange: Exchange::Binance,
                message: e.to_string(),
            }
        })?;

        let info: BinanceFuturesExchangeInfo = resp.json().await.map_err(|e| {
            CteError::ConnectionFailed {
                exchange: Exchange::Binance,
                message: format!("Failed to parse futures exchange info: {e}"),
            }
        })?;

        let symbols = info
            .symbols
            .iter()
            .filter(|s| s.status == "TRADING" && s.contract_type == "PERPETUAL")
            .map(|s| {
                normalize::build_symbol(
                    &s.symbol,
                    &s.base_asset,
                    &s.quote_asset,
                    MarketType::LinearPerpetual,
                )
            })
            .collect();

        Ok(symbols)
    }

    #[instrument(skip(self), fields(exchange = "binance", symbol = %symbol.raw_symbol, timeframe = %timeframe))]
    pub async fn fetch_candles(
        &self,
        symbol: &Symbol,
        timeframe: Timeframe,
        start: Option<DateTime<Utc>>,
        limit: Option<u32>,
    ) -> Result<Vec<Candle>> {
        let base = self.base_url(symbol.market_type);
        let path = match symbol.market_type {
            MarketType::Spot => "/api/v3/klines",
            _ => "/fapi/v1/klines",
        };
        let url = format!("{}{}", base, path);
        let limit = limit.unwrap_or(200).min(1500);

        let mut req = self
            .client
            .get(&url)
            .query(&[
                ("symbol", symbol.raw_symbol.as_str()),
                ("interval", timeframe.to_binance_str()),
            ])
            .query(&[("limit", limit)]);

        if let Some(start_time) = start {
            req = req.query(&[("startTime", start_time.timestamp_millis())]);
        }

        let resp = req.send().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::Binance,
            endpoint: path.to_string(),
            status: 0,
            body: e.to_string(),
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::Binance,
                endpoint: path.to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let raw_candles: Vec<BinanceKlineRaw> =
            resp.json().await.map_err(|e| CteError::RestApi {
                exchange: Exchange::Binance,
                endpoint: path.to_string(),
                status: 200,
                body: format!("Parse error: {e}"),
            })?;

        let candles = raw_candles
            .iter()
            .map(|k| normalize::normalize_kline(k, symbol, timeframe))
            .collect();

        tracing::debug!(
            exchange = "binance",
            symbol = %symbol.raw_symbol,
            count = raw_candles.len(),
            "Fetched candles"
        );

        Ok(candles)
    }

    #[instrument(skip(self), fields(exchange = "binance", symbol = %symbol.raw_symbol))]
    pub async fn fetch_orderbook(&self, symbol: &Symbol, depth: u32) -> Result<OrderBook> {
        let base = self.base_url(symbol.market_type);
        let path = match symbol.market_type {
            MarketType::Spot => "/api/v3/depth",
            _ => "/fapi/v1/depth",
        };
        let url = format!("{}{}", base, path);
        let limit = depth.min(1000);

        let resp = self
            .client
            .get(&url)
            .query(&[("symbol", symbol.raw_symbol.as_str())])
            .query(&[("limit", limit)])
            .send()
            .await
            .map_err(|e| CteError::RestApi {
                exchange: Exchange::Binance,
                endpoint: path.to_string(),
                status: 0,
                body: e.to_string(),
            })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::Binance,
                endpoint: path.to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let raw: BinanceDepthEvent = resp.json().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::Binance,
            endpoint: path.to_string(),
            status: 200,
            body: format!("Parse error: {e}"),
        })?;

        Ok(normalize::normalize_depth(&raw, symbol))
    }

    #[instrument(skip(self), fields(exchange = "binance", symbol = %symbol.raw_symbol))]
    pub async fn fetch_recent_trades(&self, symbol: &Symbol, limit: u32) -> Result<Vec<Trade>> {
        let base = self.base_url(symbol.market_type);
        let path = match symbol.market_type {
            MarketType::Spot => "/api/v3/trades",
            _ => "/fapi/v1/trades",
        };
        let url = format!("{}{}", base, path);
        let limit = limit.min(1000);

        let resp = self
            .client
            .get(&url)
            .query(&[("symbol", symbol.raw_symbol.as_str())])
            .query(&[("limit", limit)])
            .send()
            .await
            .map_err(|e| CteError::RestApi {
                exchange: Exchange::Binance,
                endpoint: path.to_string(),
                status: 0,
                body: e.to_string(),
            })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::RestApi {
                exchange: Exchange::Binance,
                endpoint: path.to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let raw: Vec<BinanceTradeRaw> = resp.json().await.map_err(|e| CteError::RestApi {
            exchange: Exchange::Binance,
            endpoint: path.to_string(),
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
