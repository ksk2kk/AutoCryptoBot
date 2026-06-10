use cte_core::AppConfig;

pub async fn execute(
    config: AppConfig,
    symbols_str: String,
    exchanges_str: String,
    timeframe_str: String,
    capital: f64,
    no_auto_trade: bool,
) -> anyhow::Result<()> {
    println!("=== CTE - Crypto Trading Engine ===");
    println!("Starting GUI with live trading dashboard...\n");
    println!("  Symbols:    {}", symbols_str);
    println!("  Exchanges:  {}", exchanges_str);
    println!("  Timeframe:  {}", timeframe_str);
    println!("  Capital:    ${:.2}", capital);
    println!("  Auto Trade: {}", !no_auto_trade);
    println!();

    cte_gui::run_app(config, symbols_str, exchanges_str, timeframe_str, capital, no_auto_trade)
}
