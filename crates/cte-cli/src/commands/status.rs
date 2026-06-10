use cte_core::AppConfig;
use cte_exchange::ExchangeRegistry;

pub async fn execute(config: &AppConfig) -> anyhow::Result<()> {
    println!("=== CTE Exchange Connection Status ===\n");

    let registry = ExchangeRegistry::from_config(&config.exchanges);
    let results = registry.connect_all().await;

    let mut connected = 0u32;
    let mut failed = 0u32;

    for (exchange, result) in &results {
        match result {
            Ok(()) => {
                println!("  [OK] {} - Connected", exchange);
                connected += 1;
            }
            Err(e) => {
                println!("  [FAIL] {} - {}", exchange, e);
                failed += 1;
            }
        }
    }

    println!("\n--- Summary ---");
    println!("  Connected: {}", connected);
    println!("  Failed:    {}", failed);
    println!("  Total:     {}", connected + failed);

    if failed > 0 {
        println!("\nSome exchanges failed to connect. Check network and configuration.");
    } else {
        println!("\nAll exchanges connected successfully.");
    }

    registry.disconnect_all().await;
    Ok(())
}
