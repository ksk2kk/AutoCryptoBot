use cte_core::AppConfig;

pub fn execute(config: &AppConfig) -> anyhow::Result<()> {
    println!("=== CTE Configuration ===\n");

    println!("[General]");
    println!("  Log Level:  {}", config.general.log_level);
    println!("  Log Format: {}", config.general.log_format);
    println!("  Log File:   {}", config.general.log_file.display());
    println!("  Data Dir:   {}", config.general.data_dir.display());

    println!("\n[Trading]");
    println!("  Capital:          ${:.2}", config.trading.starting_capital);
    println!("  Max Positions:    {}", config.trading.max_positions);
    println!("  Max Position USD: ${:.2}", config.trading.max_position_size_usd);
    println!("  Leverage:         {}x", config.trading.default_leverage);
    println!("  Auto Trade:       {}", config.trading.auto_trade_on_start);
    println!("  Strategy:         {}", config.trading.default_strategy);
    println!("  Symbols:          {:?}", config.trading.symbols);

    println!("\n[Exchanges]");
    for (name, ex_config) in &config.exchanges {
        let status = if ex_config.enabled { "enabled" } else { "disabled" };
        println!("  {}: {}", name, status);
    }

    println!("\n[Scraper]");
    println!("  OKX:     {}", if config.scraper.okx_enabled { "enabled" } else { "disabled" });
    println!("  Bybit:   {}", if config.scraper.bybit_enabled { "enabled" } else { "disabled" });
    println!("  Binance: {}", if config.scraper.binance_enabled { "enabled" } else { "disabled" });
    println!("  Interval: {}s", config.scraper.scrape_interval_secs);

    println!("\n[Strategies]");
    for (name, strat) in &config.strategies {
        println!("  {}: type={}", name, strat.strategy_type);
    }

    println!("\nConfiguration valid.");
    Ok(())
}
