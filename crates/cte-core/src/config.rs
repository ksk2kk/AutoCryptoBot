use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub trading: TradingConfig,
    pub exchanges: HashMap<String, ExchangeConfig>,
    pub tui: TuiConfig,
    pub scraper: ScraperConfig,
    pub strategies: HashMap<String, StrategyConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub log_level: String,
    pub log_format: String,
    pub log_file: PathBuf,
    pub data_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    pub starting_capital: f64,
    pub max_positions: usize,
    pub max_position_size_usd: f64,
    pub default_leverage: f64,
    pub auto_trade_on_start: bool,
    pub default_strategy: String,
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub enabled: bool,
    #[serde(default)]
    pub spot_rest: Option<String>,
    #[serde(default)]
    pub futures_rest: Option<String>,
    #[serde(default)]
    pub rest: Option<String>,
    #[serde(default)]
    pub spot_ws: Option<String>,
    #[serde(default)]
    pub futures_ws: Option<String>,
    #[serde(default)]
    pub ws_public: Option<String>,
    #[serde(default)]
    pub ws_public_linear: Option<String>,
    #[serde(default)]
    pub ws_public_spot: Option<String>,
    #[serde(default)]
    pub ws: Option<String>,
    pub rate_limit_per_second: Option<u32>,
    pub rate_limit_weight_per_minute: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    pub refresh_rate_ms: u64,
    pub max_candles_displayed: usize,
    pub max_trades_displayed: usize,
    pub orderbook_depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScraperConfig {
    pub okx_enabled: bool,
    pub bybit_enabled: bool,
    pub binance_enabled: bool,
    pub scrape_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    #[serde(rename = "type")]
    pub strategy_type: String,
    #[serde(default)]
    pub ema_fast: Option<usize>,
    #[serde(default)]
    pub ema_slow: Option<usize>,
    #[serde(default)]
    pub rsi_period: Option<usize>,
    #[serde(default)]
    pub rsi_overbought: Option<f64>,
    #[serde(default)]
    pub rsi_oversold: Option<f64>,
    #[serde(default)]
    pub atr_period: Option<usize>,
    #[serde(default)]
    pub atr_stop_multiplier: Option<f64>,
    #[serde(default)]
    pub bollinger_period: Option<usize>,
    #[serde(default)]
    pub bollinger_std_dev: Option<f64>,
}

impl AppConfig {
    pub fn load(path: &std::path::Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::CteError::Config(format!("Failed to read config file {}: {}", path.display(), e))
        })?;
        toml::from_str(&content).map_err(|e| {
            crate::CteError::Config(format!("Failed to parse config: {e}"))
        })
    }

    pub fn default_config() -> Self {
        Self {
            general: GeneralConfig {
                log_level: "info".to_string(),
                log_format: "json".to_string(),
                log_file: PathBuf::from("./logs/cte.log"),
                data_dir: PathBuf::from("./data"),
            },
            trading: TradingConfig {
                starting_capital: 10000.0,
                max_positions: 5,
                max_position_size_usd: 2000.0,
                default_leverage: 1.0,
                auto_trade_on_start: true,
                default_strategy: "default".to_string(),
                symbols: vec![
                    "BTCUSDT".to_string(),
                    "ETHUSDT".to_string(),
                    "SOLUSDT".to_string(),
                ],
            },
            exchanges: HashMap::new(),
            tui: TuiConfig {
                refresh_rate_ms: 16,
                max_candles_displayed: 200,
                max_trades_displayed: 100,
                orderbook_depth: 20,
            },
            scraper: ScraperConfig {
                okx_enabled: true,
                bybit_enabled: true,
                binance_enabled: false,
                scrape_interval_secs: 300,
            },
            strategies: HashMap::new(),
        }
    }
}
