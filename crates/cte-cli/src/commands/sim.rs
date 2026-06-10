use cte_core::{AppConfig, Exchange, MarketType, Symbol, Timeframe};
use cte_core::config::StrategyConfig;
use cte_exchange::ExchangeRegistry;
use cte_strategy::{SimTrader, StrategyEngine};
use rust_decimal::Decimal;
use std::str::FromStr;
use tokio::sync::broadcast;

pub async fn execute(
    config: AppConfig,
    symbols_str: String,
    strategy_name: String,
    duration: u64,
    json: bool,
) -> anyhow::Result<()> {
    let symbols: Vec<&str> = symbols_str.split(',').collect();
    let capital = Decimal::from_str(&config.trading.starting_capital.to_string())
        .unwrap_or(Decimal::new(10000, 0));
    let max_positions = config.trading.max_positions;
    let max_size = Decimal::from_str(&config.trading.max_position_size_usd.to_string())
        .unwrap_or(Decimal::new(2000, 0));

    let registry = ExchangeRegistry::from_config(&config.exchanges);

    println!("Connecting to exchanges...");
    let results = registry.connect_all().await;
    for (ex, res) in &results {
        match res {
            Ok(()) => println!("  [OK] {}", ex),
            Err(e) => println!("  [FAIL] {} - {}", ex, e),
        }
    }

    let trader = SimTrader::new(capital, max_positions, max_size);
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

    let connector = registry.get(Exchange::Binance)
        .or_else(|| registry.get(Exchange::Okx))
        .ok_or_else(|| anyhow::anyhow!("No exchange available"))?;

    let primary_symbol = symbols.first().unwrap_or(&"BTCUSDT");
    let symbol = Symbol {
        base: String::new(),
        quote: String::new(),
        market_type: MarketType::LinearPerpetual,
        exchange: connector.exchange(),
        raw_symbol: primary_symbol.to_string(),
    };

    println!("\nStarting simulation on {}...", primary_symbol);
    println!("Strategy: {}", strat_config.strategy_type);
    println!("Capital: ${}, Max Positions: {}\n", capital, max_positions);

    let mut rx = connector
        .subscribe_candles(&symbol, Timeframe::M1)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let deadline = if duration > 0 {
        Some(tokio::time::Instant::now() + std::time::Duration::from_secs(duration))
    } else {
        None
    };

    if !json {
        println!("{:<12} {:>10} {:>8} {:>12} {:>10} {:>12}", "TIME", "PRICE", "POS", "UNRL PNL", "REAL PNL", "EQUITY");
    }

    loop {
        let should_stop = if let Some(dl) = deadline {
            tokio::time::Instant::now() >= dl
        } else {
            false
        };
        if should_stop {
            break;
        }

        tokio::select! {
            _ = async {
                if let Some(dl) = deadline {
                    tokio::time::sleep_until(dl).await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => break,
            result = rx.recv() => {
                match result {
                    Ok(candle) => {
                        engine.process_candle(&candle);

                        let summary = engine.get_pnl_summary();
                        if json {
                            let output = serde_json::json!({
                                "time": candle.open_time.to_rfc3339(),
                                "price": candle.close.to_string(),
                                "positions": summary.open_positions,
                                "unrealized_pnl": summary.total_unrealized_pnl.to_string(),
                                "realized_pnl": summary.total_realized_pnl.to_string(),
                                "equity": summary.equity.to_string(),
                            });
                            println!("{}", output);
                        } else {
                            println!(
                                "{:<12} {:>10} {:>8} {:>12} {:>10} {:>12}",
                                candle.open_time.format("%H:%M:%S"),
                                candle.close,
                                summary.open_positions,
                                format!("${}", summary.total_unrealized_pnl),
                                format!("${}", summary.total_realized_pnl),
                                format!("${}", summary.equity),
                            );
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(lagged = n, "Sim receiver lagged");
                    }
                    Err(_) => break,
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\nSimulation stopped by user.");
                break;
            }
        }
    }

    let summary = engine.get_pnl_summary();
    println!("\n=== Simulation Summary ===");
    println!("  Total Realized PnL: ${}", summary.total_realized_pnl);
    println!("  Total Unrealized PnL: ${}", summary.total_unrealized_pnl);
    println!("  Open Positions: {}", summary.open_positions);
    println!("  Equity: ${}", summary.equity);
    println!("  Closed Trades: {}", summary.closed_trades);
    println!("  Win Rate: {}%", summary.win_rate);

    registry.disconnect_all().await;
    Ok(())
}
