use chrono::NaiveDate;
use cte_core::config::StrategyConfig;
use cte_core::{AppConfig, Exchange, MarketType, Symbol, Timeframe};
use cte_exchange::ExchangeRegistry;
use cte_strategy::{SimTrader, StrategyEngine};
use rust_decimal::Decimal;
use std::str::FromStr;

pub async fn execute(
    config: AppConfig,
    exchange_name: String,
    symbol_raw: String,
    timeframe_str: String,
    start_str: String,
    end_str: String,
    strategy_name: String,
    capital: f64,
) -> anyhow::Result<()> {
    let exchange: Exchange = exchange_name.parse().map_err(|e| anyhow::anyhow!("{e}"))?;
    let timeframe: Timeframe = timeframe_str.parse().map_err(|e| anyhow::anyhow!("{e}"))?;

    let start_date = NaiveDate::parse_from_str(&start_str, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("Invalid start date: {e}"))?;
    let end_date = NaiveDate::parse_from_str(&end_str, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("Invalid end date: {e}"))?;

    let start_dt = start_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end_dt = end_date.and_hms_opt(23, 59, 59).unwrap().and_utc();

    let registry = ExchangeRegistry::from_config(&config.exchanges);
    let connector = registry.get(exchange).ok_or_else(|| {
        anyhow::anyhow!("Exchange {exchange} not configured")
    })?;

    println!("=== Backtest ===");
    println!("  Exchange:  {}", exchange);
    println!("  Symbol:    {}", symbol_raw);
    println!("  Timeframe: {}", timeframe);
    println!("  Period:    {} to {}", start_str, end_str);
    println!("  Capital:   ${:.2}", capital);
    println!();

    connector.connect().await.map_err(|e| anyhow::anyhow!("{e}"))?;

    let symbol = Symbol {
        base: String::new(),
        quote: String::new(),
        market_type: MarketType::LinearPerpetual,
        exchange,
        raw_symbol: symbol_raw.clone(),
    };

    println!("Fetching historical candles...");
    let candles = connector
        .fetch_candles(&symbol, timeframe, Some(start_dt), Some(1500))
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    println!("Loaded {} candles.\n", candles.len());

    if candles.is_empty() {
        println!("No candles available for the given period.");
        return Ok(());
    }

    let cap = Decimal::from_str(&capital.to_string()).unwrap_or(Decimal::new(10000, 0));
    let max_pos = config.trading.max_positions;
    let max_size = Decimal::from_str(&config.trading.max_position_size_usd.to_string())
        .unwrap_or(Decimal::new(2000, 0));

    let trader = SimTrader::new(cap, max_pos, max_size);
    let strat_config = config.strategies.get(&strategy_name).cloned().unwrap_or(StrategyConfig {
        strategy_type: "combined".to_string(),
        ema_fast: Some(9),
        ema_slow: Some(21),
        rsi_period: Some(14),
        rsi_overbought: Some(70.0),
        rsi_oversold: Some(30.0),
        atr_period: Some(14),
        atr_stop_multiplier: Some(2.0),
        bollinger_period: Some(20),
        bollinger_std_dev: Some(2.0),
    });

    let mut engine = StrategyEngine::new(&strat_config, trader);

    println!("Running backtest...");
    for candle in &candles {
        if candle.open_time > end_dt {
            break;
        }
        engine.process_candle(candle);
    }

    let summary = engine.get_pnl_summary();
    println!("\n=== Backtest Results ===");
    println!("  Candles Processed: {}", candles.len());
    println!("  Closed Trades:     {}", summary.closed_trades);
    println!("  Win Rate:          {}%", summary.win_rate);
    println!("  Realized PnL:      ${}", summary.total_realized_pnl);
    println!("  Unrealized PnL:    ${}", summary.total_unrealized_pnl);
    println!("  Final Equity:      ${}", summary.equity);
    println!("  Open Positions:    {}", summary.open_positions);

    let roi = if cap > Decimal::ZERO {
        ((summary.equity - cap) / cap) * Decimal::ONE_HUNDRED
    } else {
        Decimal::ZERO
    };
    println!("  ROI:               {:.2}%", roi);

    connector.disconnect().await.map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}
