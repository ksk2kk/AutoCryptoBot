use chrono::{DateTime, Datelike, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Volume Weighted Average Price indicator.
/// Resets daily.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vwap {
    cumulative_volume: Decimal,
    cumulative_tp_volume: Decimal,
    current_vwap: Decimal,
    current_day: Option<u32>,
}

impl Vwap {
    /// Create a new VWAP indicator.
    pub fn new() -> Self {
        Self {
            cumulative_volume: Decimal::ZERO,
            cumulative_tp_volume: Decimal::ZERO,
            current_vwap: Decimal::ZERO,
            current_day: None,
        }
    }

    /// Update the VWAP with price and volume. Returns the current VWAP.
    pub fn update(&mut self, price: Decimal, volume: Decimal) -> Decimal {
        self.cumulative_volume += volume;
        self.cumulative_tp_volume += price * volume;

        if !self.cumulative_volume.is_zero() {
            self.current_vwap = self.cumulative_tp_volume / self.cumulative_volume;
        }

        self.current_vwap
    }

    /// Update VWAP with timestamp awareness for daily reset.
    pub fn update_with_time(
        &mut self,
        price: Decimal,
        volume: Decimal,
        timestamp: DateTime<Utc>,
    ) -> Decimal {
        let day = timestamp.ordinal();

        // Reset if new day
        if let Some(prev_day) = self.current_day {
            if day != prev_day {
                self.reset();
            }
        }
        self.current_day = Some(day);

        self.update(price, volume)
    }

    /// Reset the VWAP (typically at start of new day).
    pub fn reset(&mut self) {
        self.cumulative_volume = Decimal::ZERO;
        self.cumulative_tp_volume = Decimal::ZERO;
        self.current_vwap = Decimal::ZERO;
        self.current_day = None;
    }

    /// Returns the current VWAP value.
    pub fn value(&self) -> Decimal {
        self.current_vwap
    }

    /// Returns whether we have any data.
    pub fn is_initialized(&self) -> bool {
        !self.cumulative_volume.is_zero()
    }
}

impl Default for Vwap {
    fn default() -> Self {
        Self::new()
    }
}
