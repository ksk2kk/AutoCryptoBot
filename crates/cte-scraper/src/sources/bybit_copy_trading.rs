use chrono::Utc;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;

use cte_core::{CteError, Exchange, LeadTrader, Result, Side, TraderPosition};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BybitResponse<T> {
    ret_code: i32,
    result: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BybitLeaderboardResult {
    #[serde(default)]
    list: Vec<BybitLeadTrader>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BybitLeadTrader {
    #[serde(default)]
    leader_user_id: String,
    #[serde(default)]
    nick_name: String,
    #[serde(default)]
    pnl: String,
    #[serde(default)]
    roi: String,
    #[serde(default)]
    win_rate: String,
    #[serde(default)]
    follower_count: u64,
    #[serde(default)]
    total_count: u64,
}

pub struct BybitCopyTradingScraper {
    client: Client,
    base_url: String,
}

impl BybitCopyTradingScraper {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("Failed to build HTTP client"),
            base_url: base_url.to_string(),
        }
    }

    pub async fn fetch_lead_traders(&self, limit: usize) -> Result<Vec<LeadTrader>> {
        let url = format!(
            "{}/v5/copy-trading/get-leader-board?limit={}",
            self.base_url, limit
        );

        let resp = self.client.get(&url).send().await.map_err(|e| CteError::Scraper {
            origin: "bybit_copy_trading".to_string(),
            message: e.to_string(),
        })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::Scraper {
                origin: "bybit_copy_trading".to_string(),
                message: format!("HTTP {status}: {body}"),
            });
        }

        let data: BybitResponse<BybitLeaderboardResult> = resp.json().await.map_err(|e| {
            CteError::Scraper {
                origin: "bybit_copy_trading".to_string(),
                message: format!("Parse error: {e}"),
            }
        })?;

        if data.ret_code != 0 {
            return Err(CteError::Scraper {
                origin: "bybit_copy_trading".to_string(),
                message: format!("API error code: {}", data.ret_code),
            });
        }

        let now = Utc::now();
        let traders = data
            .result
            .list
            .iter()
            .map(|t| LeadTrader {
                id: t.leader_user_id.clone(),
                nickname: t.nick_name.clone(),
                exchange: Exchange::Bybit,
                roi_percent: Decimal::from_str(&t.roi).unwrap_or_default(),
                pnl_usd: Decimal::from_str(&t.pnl).unwrap_or_default(),
                win_rate: Decimal::from_str(&t.win_rate).unwrap_or_default(),
                followers: t.follower_count,
                total_trades: t.total_count,
                current_positions: vec![],
                fetched_at: now,
            })
            .collect();

        tracing::info!(source = "bybit", count = data.result.list.len(), "Fetched lead traders");
        Ok(traders)
    }
}
