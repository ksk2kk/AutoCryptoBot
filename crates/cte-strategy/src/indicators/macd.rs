use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::ema::Ema;

/// MACD (Moving Average Convergence Divergence) indicator.
/// Standard parameters: fast=12, slow=26, signal=9.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Macd {
    fast_ema: Ema,
    slow_ema: Ema,
    signal_ema: Ema,
    macd_line: Decimal,
    signal_line: Decimal,
    histogram: Decimal,
    initialized: bool,
}

impl Macd {
    /// Create a new MACD with given fast, slow, and signal periods.
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self {
            fast_ema: Ema::new(fast_period),
            slow_ema: Ema::new(slow_period),
            signal_ema: Ema::new(signal_period),
            macd_line: Decimal::ZERO,
            signal_line: Decimal::ZERO,
            histogram: Decimal::ZERO,
            initialized: false,
        }
    }

    /// Create with standard parameters (12, 26, 9).
    pub fn default_params() -> Self {
        Self::new(12, 26, 9)
    }

    /// Update with a new price.
    pub fn update(&mut self, price: Decimal) {
        let fast = self.fast_ema.update(price);
        let slow = self.slow_ema.update(price);

        // MACD line = fast EMA - slow EMA
        self.macd_line = fast - slow;

        // Signal line = EMA of MACD line
        if self.slow_ema.is_initialized() {
            self.signal_line = self.signal_ema.update(self.macd_line);
            if self.signal_ema.is_initialized() {
                self.initialized = true;
            }
        }

        // Histogram = MACD line - Signal line
        self.histogram = self.macd_line - self.signal_line;
    }

    /// Returns the MACD line value.
    pub fn macd_line(&self) -> Decimal {
        self.macd_line
    }

    /// Returns the signal line value.
    pub fn signal_line(&self) -> Decimal {
        self.signal_line
    }

    /// Returns the histogram value (MACD - Signal).
    pub fn histogram(&self) -> Decimal {
        self.histogram
    }

    /// Returns whether the indicator is fully initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}
