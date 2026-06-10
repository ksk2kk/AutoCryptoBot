use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Bollinger Bands indicator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BollingerBands {
    period: usize,
    std_dev_multiplier: Decimal,
    prices: VecDeque<Decimal>,
    upper: Decimal,
    middle: Decimal,
    lower: Decimal,
    initialized: bool,
}

impl BollingerBands {
    /// Create new Bollinger Bands with the given period and standard deviation multiplier.
    /// Defaults: period=20, std_dev=2.0
    pub fn new(period: usize, std_dev_multiplier: Decimal) -> Self {
        Self {
            period,
            std_dev_multiplier,
            prices: VecDeque::with_capacity(period + 1),
            upper: Decimal::ZERO,
            middle: Decimal::ZERO,
            lower: Decimal::ZERO,
            initialized: false,
        }
    }

    /// Create with standard defaults (20 period, 2 std dev).
    pub fn default_params() -> Self {
        Self::new(20, dec!(2))
    }

    /// Update with a new price.
    pub fn update(&mut self, price: Decimal) {
        self.prices.push_back(price);
        if self.prices.len() > self.period {
            self.prices.pop_front();
        }

        if self.prices.len() >= self.period {
            self.initialized = true;
            let period_dec = Decimal::from(self.period as u64);

            // Calculate SMA (middle band)
            let sum: Decimal = self.prices.iter().copied().sum();
            self.middle = sum / period_dec;

            // Calculate standard deviation
            let variance: Decimal = self.prices.iter()
                .map(|p| {
                    let diff = *p - self.middle;
                    diff * diff
                })
                .sum::<Decimal>() / period_dec;

            let std_dev = Self::decimal_sqrt(variance);

            self.upper = self.middle + self.std_dev_multiplier * std_dev;
            self.lower = self.middle - self.std_dev_multiplier * std_dev;
        } else {
            // Not enough data yet; use what we have
            let count = Decimal::from(self.prices.len() as u64);
            let sum: Decimal = self.prices.iter().copied().sum();
            self.middle = sum / count;
            self.upper = self.middle;
            self.lower = self.middle;
        }
    }

    /// Compute an approximate square root for a Decimal using Newton's method.
    fn decimal_sqrt(value: Decimal) -> Decimal {
        if value <= Decimal::ZERO {
            return Decimal::ZERO;
        }
        // Newton's method for square root
        let mut guess = value / dec!(2);
        if guess.is_zero() {
            guess = dec!(0.0001);
        }
        for _ in 0..20 {
            let next = (guess + value / guess) / dec!(2);
            if (next - guess).abs() < dec!(0.00000001) {
                return next;
            }
            guess = next;
        }
        guess
    }

    /// Returns the upper band.
    pub fn upper(&self) -> Decimal {
        self.upper
    }

    /// Returns the middle band (SMA).
    pub fn middle(&self) -> Decimal {
        self.middle
    }

    /// Returns the lower band.
    pub fn lower(&self) -> Decimal {
        self.lower
    }

    /// Returns whether the indicator has enough data.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Returns the configured period.
    pub fn period(&self) -> usize {
        self.period
    }
}
