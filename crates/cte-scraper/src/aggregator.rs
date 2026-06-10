use cte_core::LeadTrader;

pub struct ScraperAggregator {
    traders: Vec<LeadTrader>,
}

impl ScraperAggregator {
    pub fn new() -> Self {
        Self { traders: vec![] }
    }

    pub fn add_traders(&mut self, mut new_traders: Vec<LeadTrader>) {
        self.traders.append(&mut new_traders);
    }

    pub fn top_by_roi(&self, n: usize) -> Vec<&LeadTrader> {
        let mut sorted: Vec<&LeadTrader> = self.traders.iter().collect();
        sorted.sort_by(|a, b| b.roi_percent.cmp(&a.roi_percent));
        sorted.truncate(n);
        sorted
    }

    pub fn top_by_pnl(&self, n: usize) -> Vec<&LeadTrader> {
        let mut sorted: Vec<&LeadTrader> = self.traders.iter().collect();
        sorted.sort_by(|a, b| b.pnl_usd.cmp(&a.pnl_usd));
        sorted.truncate(n);
        sorted
    }

    pub fn all_traders(&self) -> &[LeadTrader] {
        &self.traders
    }

    pub fn clear(&mut self) {
        self.traders.clear();
    }
}
