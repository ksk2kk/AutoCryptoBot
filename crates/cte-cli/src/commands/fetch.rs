use cte_core::{AppConfig, Exchange, MarketType, Symbol, Timeframe};
use cte_exchange::ExchangeRegistry;

pub async fn execute(
    exchange_name: String,
    symbol_raw: String,
    timeframe_str: String,
    market_str: String,
    limit: u32,
    orderbook: bool,
    trades: bool,
    config: &AppConfig,
) -> anyhow::Result<()> {
    let exchange: Exchange = exchange_name.parse().map_err(|e| anyhow::anyhow!("{e}"))?;
    let timeframe: Timeframe = timeframe_str.parse().map_err(|e| anyhow::anyhow!("{e}"))?;
    let market_type: MarketType = market_str.parse().map_err(|e| anyhow::anyhow!("{e}"))?;

    let registry = ExchangeRegistry::from_config(&config.exchanges);
    let connector = registry.get(exchange).ok_or_else(|| {
        anyhow::anyhow!("Exchange {exchange} not configured or disabled")
    })?;

    println!("Connecting to {}...", exchange);
    connector.connect().await.map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Connected.");

    let symbol = Symbol {
        base: String::new(),
        quote: String::new(),
        market_type,
        exchange,
        raw_symbol: symbol_raw.clone(),
    };

    if orderbook {
        println!("Fetching orderbook for {} (depth: {})...", symbol_raw, limit);
        let book = connector.fetch_orderbook(&symbol, limit).await.map_err(|e| anyhow::anyhow!("{e}"))?;
        println!("\n=== ORDER BOOK: {} ===", symbol_raw);
        println!("{:<20} {:>15}", "PRICE", "QUANTITY");
        println!("--- ASKS (sell) ---");
        for level in book.asks.iter().take(limit as usize).rev() {
            println!("{:<20} {:>15}", level.price, level.quantity);
        }
        if let Some(spread) = book.spread() {
            println!("--- SPREAD: {} ---", spread);
        }
        println!("--- BIDS (buy) ---");
        for level in book.bids.iter().take(limit as usize) {
            println!("{:<20} {:>15}", level.price, level.quantity);
        }
    } else if trades {
        println!("Fetching {} recent trades for {}...", limit, symbol_raw);
        let trade_list = connector.fetch_recent_trades(&symbol, limit).await.map_err(|e| anyhow::anyhow!("{e}"))?;
        println!("\n=== RECENT TRADES: {} ({} results) ===", symbol_raw, trade_list.len());
        println!("{:<24} {:>12} {:>12} {:>6}", "TIME", "PRICE", "QTY", "SIDE");
        for t in &trade_list {
            let side = if t.is_buyer_maker { "SELL" } else { "BUY" };
            println!(
                "{:<24} {:>12} {:>12} {:>6}",
                t.timestamp.format("%Y-%m-%d %H:%M:%S"),
                t.price,
                t.quantity,
                side
            );
        }
    } else {
        println!("Fetching {} {} candles for {}...", limit, timeframe, symbol_raw);
        let candles = connector
            .fetch_candles(&symbol, timeframe, None, Some(limit))
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        println!("\n=== CANDLES: {} {} ({} results) ===", symbol_raw, timeframe, candles.len());
        println!(
            "{:<20} {:>12} {:>12} {:>12} {:>12} {:>14}",
            "TIME", "OPEN", "HIGH", "LOW", "CLOSE", "VOLUME"
        );
        for c in &candles {
            println!(
                "{:<20} {:>12} {:>12} {:>12} {:>12} {:>14}",
                c.open_time.format("%Y-%m-%d %H:%M"),
                c.open,
                c.high,
                c.low,
                c.close,
                c.volume,
            );
        }
    }

    connector.disconnect().await.map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}
