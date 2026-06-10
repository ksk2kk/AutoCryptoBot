use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

/// Exponential Moving Average indicator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ema {
    period: usize,
    multiplier: Decimal,
    current: Decimal,
    initialized: bool,
    count: usize,
    sum: Decimal,
}

impl Ema {
    /// Create a new EMA with the given period.
    pub fn new(period: usize) -> Self {
        let multiplier = dec!(2) / Decimal::from(period as u64 + 1);
        Self {
            period,
            multiplier,
            current: Decimal::ZERO,
            initialized: false,
            count: 0,
            sum: Decimal::ZERO,
        }
    }

    /// Update the EMA with a new price and return the current EMA value.
    pub fn update(&mut self, price: Decimal) -> Decimal {
        if !self.initialized {
            self.count += 1;
            self.sum += price;
            if self.count >= self.period {
                self.current = self.sum / Decimal::from(self.period as u64);
                self.initialized = true;
            } else {
                self.current = self.sum / Decimal::from(self.count as u64);
            }
        } else {
            self.current = (price - self.current) * self.multiplier + self.current;
        }
        self.current
    }

    /// Returns the current EMA value.
    pub fn value(&self) -> Decimal {
        self.current
    }

    /// Returns whether the EMA has received enough data to be valid.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Returns the configured period.
    pub fn period(&self) -> usize {
        self.period
    }
}
