use std::collections::HashMap;
use std::sync::Arc;

use cte_core::{config::ExchangeConfig, traits::ExchangeConnector, Exchange, Result};

use crate::binance::BinanceConnector;
use crate::okx::OkxConnector;
use crate::bybit::BybitConnector;
use crate::gateio::GateIoConnector;
use crate::bitget::BitgetConnector;

pub struct ExchangeRegistry {
    connectors: HashMap<Exchange, Arc<dyn ExchangeConnector>>,
}

impl ExchangeRegistry {
    pub fn new() -> Self {
        Self {
            connectors: HashMap::new(),
        }
    }

    pub fn from_config(exchanges: &HashMap<String, ExchangeConfig>) -> Self {
        let mut registry = Self::new();

        for (name, config) in exchanges {
            if !config.enabled {
                continue;
            }

            match name.as_str() {
                "binance" => {
                    let connector = BinanceConnector::new(
                        config.spot_rest.as_deref().unwrap_or("https://api.binance.com"),
                        config.futures_rest.as_deref().unwrap_or("https://fapi.binance.com"),
                        config.spot_ws.as_deref().unwrap_or("wss://stream.binance.com:9443/ws"),
                        config.futures_ws.as_deref().unwrap_or("wss://fstream.binance.com/ws"),
                    );
                    registry.register(Exchange::Binance, Arc::new(connector));
                }
                "okx" => {
                    let connector = OkxConnector::new(
                        config.rest.as_deref().unwrap_or("https://www.okx.com"),
                        config.ws_public.as_deref().unwrap_or("wss://ws.okx.com:8443/ws/v5/public"),
                    );
                    registry.register(Exchange::Okx, Arc::new(connector));
                }
                "bybit" => {
                    let connector = BybitConnector::new(
                        config.rest.as_deref().unwrap_or("https://api.bybit.com"),
                        config.ws_public_linear.as_deref().unwrap_or("wss://stream.bybit.com/v5/public/linear"),
                        config.ws_public_spot.as_deref().unwrap_or("wss://stream.bybit.com/v5/public/spot"),
                    );
                    registry.register(Exchange::Bybit, Arc::new(connector));
                }
                "gateio" => {
                    let connector = GateIoConnector::new(
                        config.rest.as_deref().unwrap_or("https://api.gateio.ws/api/v4"),
                        config.ws.as_deref().unwrap_or("wss://api.gateio.ws/ws/v4/"),
                    );
                    registry.register(Exchange::GateIo, Arc::new(connector));
                }
                "bitget" => {
                    let connector = BitgetConnector::new(
                        config.rest.as_deref().unwrap_or("https://api.bitget.com"),
                        config.ws_public.as_deref().unwrap_or("wss://ws.bitget.com/v2/ws/public"),
                    );
                    registry.register(Exchange::Bitget, Arc::new(connector));
                }
                _ => {
                    tracing::warn!(exchange = name, "Unknown exchange in config, skipping");
                }
            }
        }

        registry
    }

    pub fn register(&mut self, exchange: Exchange, connector: Arc<dyn ExchangeConnector>) {
        self.connectors.insert(exchange, connector);
    }

    pub fn get(&self, exchange: Exchange) -> Option<&Arc<dyn ExchangeConnector>> {
        self.connectors.get(&exchange)
    }

    pub fn all(&self) -> Vec<(&Exchange, &Arc<dyn ExchangeConnector>)> {
        self.connectors.iter().collect()
    }

    pub async fn connect_all(&self) -> Vec<(Exchange, Result<()>)> {
        let mut results = Vec::new();
        for (exchange, connector) in &self.connectors {
            let result = connector.connect().await;
            results.push((*exchange, result));
        }
        results
    }

    pub async fn disconnect_all(&self) {
        for (_, connector) in &self.connectors {
            let _ = connector.disconnect().await;
        }
    }
}
