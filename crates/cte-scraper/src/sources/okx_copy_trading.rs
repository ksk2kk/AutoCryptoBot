use chrono::Utc;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;

use cte_core::{CteError, Exchange, LeadTrader, Result, Side, TraderPosition};

#[derive(Debug, Deserialize)]
struct OkxLeadTradersResponse {
    code: String,
    data: Vec<OkxLeadTradersData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxLeadTradersData {
    #[serde(default)]
    ranks: Vec<OkxRankEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxRankEntry {
    #[serde(default)]
    nick_name: String,
    #[serde(default)]
    pnl: String,
    #[serde(default)]
    pnl_ratio: String,
    #[serde(default)]
    copy_trader_num: String,
    #[serde(default)]
    acc_copy_trader_num: String,
    #[serde(default)]
    lead_days: String,
    #[serde(default)]
    aum: String,
}

#[derive(Debug, Deserialize)]
struct OkxPositionsResponse {
    code: String,
    data: Vec<OkxPosition>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxPosition {
    #[serde(default)]
    inst_id: String,
    #[serde(default)]
    pos_side: String,
    #[serde(default)]
    avg_px: String,
    #[serde(default)]
    mark_px: String,
    #[serde(default)]
    upl_ratio: String,
    #[serde(default)]
    notional_usd: String,
}

pub struct OkxCopyTradingScraper {
    client: Client,
    base_url: String,
}

impl OkxCopyTradingScraper {
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
            "{}/api/v5/copytrading/public-lead-traders?instType=SWAP&sortType=pnl&state=1&limit={}",
            self.base_url, limit
        );

        let resp = self.client.get(&url).send().await.map_err(|e| CteError::Scraper {
            origin: "okx_copy_trading".to_string(),
            message: e.to_string(),
        })?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CteError::Scraper {
                origin: "okx_copy_trading".to_string(),
                message: format!("HTTP error: {}", body),
            });
        }

        let data: OkxLeadTradersResponse = resp.json().await.map_err(|e| CteError::Scraper {
            origin: "okx_copy_trading".to_string(),
            message: format!("Parse error: {e}"),
        })?;

        if data.code != "0" {
            return Err(CteError::Scraper {
                origin: "okx_copy_trading".to_string(),
                message: format!("API error code: {}", data.code),
            });
        }

        let now = Utc::now();
        let ranks = data.data.first().map(|d| &d.ranks).cloned().unwrap_or_default();

        let traders = ranks
            .iter()
            .take(limit)
            .map(|t| LeadTrader {
                id: t.nick_name.clone(),
                nickname: t.nick_name.clone(),
                exchange: Exchange::Okx,
                roi_percent: Decimal::from_str(&t.pnl_ratio).unwrap_or_default(),
                pnl_usd: Decimal::from_str(&t.pnl).unwrap_or_default(),
                win_rate: Decimal::ZERO,
                followers: t.acc_copy_trader_num.parse().unwrap_or(0),
                total_trades: t.lead_days.parse().unwrap_or(0),
                current_positions: vec![],
                fetched_at: now,
            })
            .collect::<Vec<LeadTrader>>();

        tracing::info!(source = "okx", count = traders.len(), "Fetched lead traders");
        Ok(traders)
    }

    pub async fn fetch_trader_positions(&self, trader_id: &str) -> Result<Vec<TraderPosition>> {
        let url = format!(
            "{}/api/v5/copytrading/public-current-subpositions?uniqueCode={}",
            self.base_url, trader_id
        );

        let resp = self.client.get(&url).send().await.map_err(|e| CteError::Scraper {
            origin: "okx_copy_trading".to_string(),
            message: e.to_string(),
        })?;

        let data: OkxPositionsResponse = resp.json().await.map_err(|e| CteError::Scraper {
            origin: "okx_copy_trading".to_string(),
            message: format!("Parse error: {e}"),
        })?;

        let positions = data
            .data
            .iter()
            .map(|p| TraderPosition {
                symbol: p.inst_id.clone(),
                side: if p.pos_side == "long" { Side::Long } else { Side::Short },
                entry_price: Decimal::from_str(&p.avg_px).unwrap_or_default(),
                mark_price: Decimal::from_str(&p.mark_px).unwrap_or_default(),
                size_usd: Decimal::from_str(&p.notional_usd).unwrap_or_default(),
                pnl_percent: Decimal::from_str(&p.upl_ratio).unwrap_or_default() * Decimal::ONE_HUNDRED,
            })
            .collect();

        Ok(positions)
    }
}
