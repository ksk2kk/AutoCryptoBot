use cte_core::AppConfig;
use cte_scraper::{BinanceLeaderboardScraper, BybitCopyTradingScraper, OkxCopyTradingScraper};

pub async fn execute(source: String, top: usize, json: bool, config: &AppConfig) -> anyhow::Result<()> {
    println!("Scraping {} for top {} traders...\n", source, top);

    let traders = match source.to_lowercase().as_str() {
        "okx" => {
            let scraper = OkxCopyTradingScraper::new("https://www.okx.com");
            scraper.fetch_lead_traders(top).await.map_err(|e| anyhow::anyhow!("{e}"))?
        }
        "bybit" => {
            let base_url = config.exchanges.get("bybit")
                .and_then(|c| c.rest.as_deref())
                .unwrap_or("https://api.bybit.com");
            let scraper = BybitCopyTradingScraper::new(base_url);
            scraper.fetch_lead_traders(top).await.map_err(|e| anyhow::anyhow!("{e}"))?
        }
        "binance" => {
            let scraper = BinanceLeaderboardScraper::new();
            scraper.fetch_lead_traders(top).await.map_err(|e| anyhow::anyhow!("{e}"))?
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown source: {}. Use: okx, bybit, binance", source));
        }
    };

    if json {
        for trader in &traders {
            println!("{}", serde_json::to_string(trader).unwrap_or_default());
        }
    } else {
        println!("=== Top Traders from {} ({} results) ===\n", source.to_uppercase(), traders.len());
        println!(
            "{:<20} {:>10} {:>12} {:>10} {:>10}",
            "NICKNAME", "ROI%", "PNL (USD)", "WIN RATE", "FOLLOWERS"
        );
        println!("{}", "-".repeat(70));
        for t in &traders {
            println!(
                "{:<20} {:>9}% {:>12} {:>9}% {:>10}",
                truncate_str(&t.nickname, 18),
                t.roi_percent,
                format!("${}", t.pnl_usd),
                t.win_rate,
                t.followers,
            );
        }
    }

    Ok(())
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max.saturating_sub(3)])
    } else {
        s.to_string()
    }
}
