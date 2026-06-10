mod commands;
mod runtime;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cte", version, about = "Crypto Trading Engine - Production algorithmic trading framework")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Config file path
    #[arg(short, long, default_value = "./config/default.toml")]
    config: String,

    /// Increase log verbosity
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Log file path
    #[arg(long, default_value = "./logs/cte.log")]
    log_file: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch full GUI dashboard with live trading
    Run {
        #[arg(long, default_value = "BTCUSDT,ETHUSDT,SOLUSDT")]
        symbols: String,
        #[arg(long, default_value = "binance,okx,bybit")]
        exchanges: String,
        #[arg(long, default_value = "1m")]
        timeframe: String,
        #[arg(long, default_value = "10000")]
        capital: f64,
        #[arg(long)]
        no_auto_trade: bool,
    },
    /// Test REST API data fetching
    Fetch {
        #[arg(long)]
        exchange: String,
        #[arg(long)]
        symbol: String,
        #[arg(long, default_value = "1h")]
        timeframe: String,
        #[arg(long, default_value = "linear")]
        market: String,
        #[arg(long, default_value = "10")]
        limit: u32,
        #[arg(long)]
        orderbook: bool,
        #[arg(long)]
        trades: bool,
    },
    /// Test WebSocket streaming
    Stream {
        #[arg(long)]
        exchange: String,
        #[arg(long)]
        symbol: String,
        #[arg(long, default_value = "kline")]
        channel: String,
        #[arg(long, default_value = "1m")]
        timeframe: String,
        #[arg(long, default_value = "60")]
        duration: u64,
    },
    /// Run headless simulated trading
    Sim {
        #[arg(long, default_value = "BTCUSDT")]
        symbols: String,
        #[arg(long, default_value = "default")]
        strategy: String,
        #[arg(long, default_value = "0")]
        duration: u64,
        #[arg(long)]
        json: bool,
    },
    /// Test copy-trading scraper
    Scrape {
        #[arg(long)]
        source: String,
        #[arg(long, default_value = "20")]
        top: usize,
        #[arg(long)]
        json: bool,
    },
    /// Run strategy on historical data
    Backtest {
        #[arg(long, default_value = "binance")]
        exchange: String,
        #[arg(long, default_value = "BTCUSDT")]
        symbol: String,
        #[arg(long, default_value = "1h")]
        timeframe: String,
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: String,
        #[arg(long, default_value = "default")]
        strategy: String,
        #[arg(long, default_value = "10000")]
        capital: f64,
    },
    /// Show exchange connection status
    Status,
    /// Validate and display configuration
    Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    runtime::init_logging(&cli.log_file, cli.verbose)?;

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "CTE starting");

    let config = cte_core::AppConfig::load(std::path::Path::new(&cli.config))
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to load config, using defaults");
            cte_core::AppConfig::default_config()
        });

    match cli.command {
        Commands::Run { symbols, exchanges, timeframe, capital, no_auto_trade } => {
            commands::run::execute(config, symbols, exchanges, timeframe, capital, no_auto_trade).await?;
        }
        Commands::Fetch { exchange, symbol, timeframe, market, limit, orderbook, trades } => {
            commands::fetch::execute(exchange, symbol, timeframe, market, limit, orderbook, trades, &config).await?;
        }
        Commands::Stream { exchange, symbol, channel, timeframe, duration } => {
            commands::stream::execute(exchange, symbol, channel, timeframe, duration, &config).await?;
        }
        Commands::Sim { symbols, strategy, duration, json } => {
            commands::sim::execute(config, symbols, strategy, duration, json).await?;
        }
        Commands::Scrape { source, top, json } => {
            commands::scrape::execute(source, top, json, &config).await?;
        }
        Commands::Backtest { exchange, symbol, timeframe, start, end, strategy, capital } => {
            commands::backtest::execute(config, exchange, symbol, timeframe, start, end, strategy, capital).await?;
        }
        Commands::Status => {
            commands::status::execute(&config).await?;
        }
        Commands::Config => {
            commands::config::execute(&config)?;
        }
    }

    tracing::info!("CTE shutdown complete");
    Ok(())
}
