use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

/// Relative Strength Index indicator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rsi {
    period: usize,
    avg_gain: Decimal,
    avg_loss: Decimal,
    prev_price: Option<Decimal>,
    count: usize,
    initialized: bool,
    gains: Vec<Decimal>,
    losses: Vec<Decimal>,
    current_rsi: Decimal,
}

impl Rsi {
    /// Create a new RSI with the given period (default 14).
    pub fn new(period: usize) -> Self {
        Self {
            period,
            avg_gain: Decimal::ZERO,
            avg_loss: Decimal::ZERO,
            prev_price: None,
            count: 0,
            initialized: false,
            gains: Vec::with_capacity(period),
            losses: Vec::with_capacity(period),
            current_rsi: dec!(50),
        }
    }

    /// Create RSI with the standard 14-period.
    pub fn default_period() -> Self {
        Self::new(14)
    }

    /// Update with a new price and return the RSI value (0-100).
    pub fn update(&mut self, price: Decimal) -> Decimal {
        if let Some(prev) = self.prev_price {
            let change = price - prev;
            let gain = if change > Decimal::ZERO { change } else { Decimal::ZERO };
            let loss = if change < Decimal::ZERO { -change } else { Decimal::ZERO };

            if !self.initialized {
                self.gains.push(gain);
                self.losses.push(loss);
                self.count += 1;

                if self.count >= self.period {
                    // Calculate initial averages using simple average
                    let period_dec = Decimal::from(self.period as u64);
                    self.avg_gain = self.gains.iter().copied().sum::<Decimal>() / period_dec;
                    self.avg_loss = self.losses.iter().copied().sum::<Decimal>() / period_dec;
                    self.initialized = true;
                    self.current_rsi = self.calculate_rsi();
                }
            } else {
                // Wilder's smoothing method
                let period_dec = Decimal::from(self.period as u64);
                self.avg_gain = (self.avg_gain * (period_dec - dec!(1)) + gain) / period_dec;
                self.avg_loss = (self.avg_loss * (period_dec - dec!(1)) + loss) / period_dec;
                self.current_rsi = self.calculate_rsi();
            }
        }
        self.prev_price = Some(price);
        self.current_rsi
    }

    fn calculate_rsi(&self) -> Decimal {
        if self.avg_loss.is_zero() {
            return dec!(100);
        }
        let rs = self.avg_gain / self.avg_loss;
        dec!(100) - (dec!(100) / (dec!(1) + rs))
    }

    /// Returns the current RSI value.
    pub fn value(&self) -> Decimal {
        self.current_rsi
    }

    /// Returns whether the RSI has enough data to be valid.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Returns the configured period.
    pub fn period(&self) -> usize {
        self.period
    }
}
