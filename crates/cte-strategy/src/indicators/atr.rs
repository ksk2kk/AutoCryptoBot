use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

/// Average True Range indicator using Wilder's smoothing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atr {
    period: usize,
    current: Decimal,
    prev_close: Option<Decimal>,
    count: usize,
    initialized: bool,
    tr_values: Vec<Decimal>,
}

impl Atr {
    /// Create a new ATR with the given period (typically 14).
    pub fn new(period: usize) -> Self {
        Self {
            period,
            current: Decimal::ZERO,
            prev_close: None,
            count: 0,
            initialized: false,
            tr_values: Vec::with_capacity(period),
        }
    }

    /// Update the ATR with a new candle's high, low, close values.
    /// Returns the current ATR value.
    pub fn update(&mut self, high: Decimal, low: Decimal, close: Decimal) -> Decimal {
        let true_range = match self.prev_close {
            Some(prev_close) => {
                let hl = high - low;
                let hc = (high - prev_close).abs();
                let lc = (low - prev_close).abs();
                hl.max(hc).max(lc)
            }
            None => high - low,
        };

        self.prev_close = Some(close);

        if !self.initialized {
            self.tr_values.push(true_range);
            self.count += 1;

            if self.count >= self.period {
                // Initial ATR is simple average of TRs
                self.current = self.tr_values.iter().copied().sum::<Decimal>()
                    / Decimal::from(self.period as u64);
                self.initialized = true;
            } else {
                self.current = self.tr_values.iter().copied().sum::<Decimal>()
                    / Decimal::from(self.count as u64);
            }
        } else {
            // Wilder's smoothing: ATR = ((period - 1) * prev_ATR + TR) / period
            let period_dec = Decimal::from(self.period as u64);
            self.current = (self.current * (period_dec - dec!(1)) + true_range) / period_dec;
        }

        self.current
    }

    /// Returns the current ATR value.
    pub fn value(&self) -> Decimal {
        self.current
    }

    /// Returns whether the ATR has enough data to be valid.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Returns the configured period.
    pub fn period(&self) -> usize {
        self.period
    }
}
