use chrono::Utc;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;

use cte_core::{CteError, Exchange, LeadTrader, Result};

#[derive(Debug, Deserialize)]
struct BinanceLeaderboardResponse {
    data: Option<Vec<BinanceLeader>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceLeader {
    #[serde(default)]
    encrypted_uid: String,
    #[serde(default)]
    nick_name: String,
    #[serde(default)]
    pnl: f64,
    #[serde(default)]
    roi: f64,
    #[serde(default)]
    follower_count: u64,
}

pub struct BinanceLeaderboardScraper {
    client: Client,
}

impl BinanceLeaderboardScraper {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    pub async fn fetch_lead_traders(&self, limit: usize) -> Result<Vec<LeadTrader>> {
        let url = "https://www.binance.com/bapi/futures/v1/public/future/copy-trade/lead-portfolio/rank";

        let body = serde_json::json!({
            "pageSize": limit,
            "pageNumber": 1,
            "timeRange": "30D",
            "dataType": "ROI",
            "favoriteOnly": false,
        });

        let resp = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| CteError::Scraper {
                origin: "binance_leaderboard".to_string(),
                message: e.to_string(),
            })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(CteError::Scraper {
                origin: "binance_leaderboard".to_string(),
                message: format!("HTTP {status}: {body_text}"),
            });
        }

        let data: BinanceLeaderboardResponse = resp.json().await.map_err(|e| CteError::Scraper {
            origin: "binance_leaderboard".to_string(),
            message: format!("Parse error: {e}"),
        })?;

        let now = Utc::now();
        let traders = data
            .data
            .unwrap_or_default()
            .iter()
            .map(|t| LeadTrader {
                id: t.encrypted_uid.clone(),
                nickname: t.nick_name.clone(),
                exchange: Exchange::Binance,
                roi_percent: Decimal::from_str(&format!("{:.2}", t.roi * 100.0))
                    .unwrap_or_default(),
                pnl_usd: Decimal::from_str(&format!("{:.2}", t.pnl)).unwrap_or_default(),
                win_rate: Decimal::ZERO,
                followers: t.follower_count,
                total_trades: 0,
                current_positions: vec![],
                fetched_at: now,
            })
            .collect::<Vec<LeadTrader>>();

        tracing::info!(source = "binance", count = traders.len(), "Fetched leaderboard");
        Ok(traders)
    }
}
