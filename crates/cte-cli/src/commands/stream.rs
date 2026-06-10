use cte_core::{AppConfig, Exchange, MarketType, Symbol, Timeframe};
use cte_exchange::ExchangeRegistry;
use tokio::sync::broadcast;

pub async fn execute(
    exchange_name: String,
    symbol_raw: String,
    channel: String,
    timeframe_str: String,
    duration: u64,
    config: &AppConfig,
) -> anyhow::Result<()> {
    let exchange: Exchange = exchange_name.parse().map_err(|e| anyhow::anyhow!("{e}"))?;
    let timeframe: Timeframe = timeframe_str.parse().map_err(|e| anyhow::anyhow!("{e}"))?;

    let registry = ExchangeRegistry::from_config(&config.exchanges);
    let connector = registry.get(exchange).ok_or_else(|| {
        anyhow::anyhow!("Exchange {exchange} not configured or disabled")
    })?;

    println!("Connecting to {}...", exchange);
    connector.connect().await.map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Connected. Streaming {} for {} seconds...\n", channel, duration);

    let symbol = Symbol {
        base: String::new(),
        quote: String::new(),
        market_type: MarketType::LinearPerpetual,
        exchange,
        raw_symbol: symbol_raw.clone(),
    };

    match channel.as_str() {
        "kline" => {
            let mut rx = connector
                .subscribe_candles(&symbol, timeframe)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            println!("{:<20} {:>12} {:>12} {:>12} {:>12} {:>8}", "TIME", "OPEN", "HIGH", "LOW", "CLOSE", "CLOSED");

            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(duration);
            loop {
                tokio::select! {
                    _ = tokio::time::sleep_until(deadline) => break,
                    result = rx.recv() => {
                        match result {
                            Ok(candle) => {
                                println!(
                                    "{:<20} {:>12} {:>12} {:>12} {:>12} {:>8}",
                                    candle.open_time.format("%H:%M:%S"),
                                    candle.open,
                                    candle.high,
                                    candle.low,
                                    candle.close,
                                    if candle.is_closed { "YES" } else { "no" },
                                );
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                println!("  [lagged by {} messages]", n);
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
        }
        "trade" => {
            let mut rx = connector
                .subscribe_trades(&symbol)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            println!("{:<12} {:>12} {:>12} {:>6}", "TIME", "PRICE", "QTY", "SIDE");

            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(duration);
            loop {
                tokio::select! {
                    _ = tokio::time::sleep_until(deadline) => break,
                    result = rx.recv() => {
                        match result {
                            Ok(trade) => {
                                let side = if trade.is_buyer_maker { "SELL" } else { "BUY" };
                                println!(
                                    "{:<12} {:>12} {:>12} {:>6}",
                                    trade.timestamp.format("%H:%M:%S"),
                                    trade.price,
                                    trade.quantity,
                                    side,
                                );
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                println!("  [lagged by {} messages]", n);
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
        }
        "depth" => {
            let mut rx = connector
                .subscribe_orderbook(&symbol)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(duration);
            loop {
                tokio::select! {
                    _ = tokio::time::sleep_until(deadline) => break,
                    result = rx.recv() => {
                        match result {
                            Ok(book) => {
                                println!("[{}] Best Bid: {} | Best Ask: {} | Spread: {}",
                                    book.timestamp.format("%H:%M:%S"),
                                    book.best_bid().map(|b| b.price.to_string()).unwrap_or("--".into()),
                                    book.best_ask().map(|a| a.price.to_string()).unwrap_or("--".into()),
                                    book.spread().map(|s| s.to_string()).unwrap_or("--".into()),
                                );
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                println!("  [lagged by {} messages]", n);
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown channel: {}. Use: kline, trade, depth", channel));
        }
    }

    println!("\nStream ended after {} seconds.", duration);
    connector.disconnect().await.map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}
