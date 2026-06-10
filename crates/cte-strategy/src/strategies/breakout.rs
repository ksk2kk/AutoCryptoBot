use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::VecDeque;

use cte_core::traits::Strategy;
use cte_core::{Candle, OrderBook, Symbol, Trade, TradingSignal};

use crate::indicators::atr::Atr;
use crate::risk;

/// Breakout strategy tracking N-period highs and lows.
///
/// Entry logic:
/// - Long when: price breaks above N-period high with increased volume
/// - Short when: price breaks below N-period low with increased volume
///
/// Exit logic:
/// - Opposite signal or ATR trailing stop
pub struct BreakoutStrategy {
    period: usize,
    highs: VecDeque<Decimal>,
    lows: VecDeque<Decimal>,
    volumes: VecDeque<Decimal>,
    atr: Atr,
    in_long: bool,
    in_short: bool,
    entry_price: Decimal,
    trailing_stop: Decimal,
    atr_stop_multiplier: Decimal,
    current_symbol: Option<Symbol>,
    balance: Decimal,
    risk_per_trade: Decimal,
}

impl BreakoutStrategy {
    pub fn new(balance: Decimal) -> Self {
        Self {
            period: 20,
            highs: VecDeque::with_capacity(21),
            lows: VecDeque::with_capacity(21),
            volumes: VecDeque::with_capacity(21),
            atr: Atr::new(14),
            in_long: false,
            in_short: false,
            entry_price: Decimal::ZERO,
            trailing_stop: Decimal::ZERO,
            atr_stop_multiplier: dec!(2),
            current_symbol: None,
            balance,
            risk_per_trade: dec!(0.02),
        }
    }

    pub fn with_params(
        period: usize,
        atr_period: usize,
        atr_stop_multiplier: Decimal,
        balance: Decimal,
        risk_per_trade: Decimal,
    ) -> Self {
        Self {
            period,
            highs: VecDeque::with_capacity(period + 1),
            lows: VecDeque::with_capacity(period + 1),
            volumes: VecDeque::with_capacity(period + 1),
            atr: Atr::new(atr_period),
            in_long: false,
            in_short: false,
            entry_price: Decimal::ZERO,
            trailing_stop: Decimal::ZERO,
            atr_stop_multiplier,
            current_symbol: None,
            balance,
            risk_per_trade,
        }
    }

    fn n_period_high(&self) -> Option<Decimal> {
        self.highs.iter().copied().max()
    }

    fn n_period_low(&self) -> Option<Decimal> {
        self.lows.iter().copied().min()
    }

    fn average_volume(&self) -> Decimal {
        if self.volumes.is_empty() {
            return Decimal::ZERO;
        }
        let sum: Decimal = self.volumes.iter().copied().sum();
        sum / Decimal::from(self.volumes.len() as u64)
    }

    fn update_trailing_stop(&mut self, close: Decimal, atr: Decimal) {
        if self.in_long {
            let new_stop = close - self.atr_stop_multiplier * atr;
            if new_stop > self.trailing_stop {
                self.trailing_stop = new_stop;
            }
        } else if self.in_short {
            let new_stop = close + self.atr_stop_multiplier * atr;
            if self.trailing_stop.is_zero() || new_stop < self.trailing_stop {
                self.trailing_stop = new_stop;
            }
        }
    }
}

impl Strategy for BreakoutStrategy {
    fn name(&self) -> &str {
        "breakout"
    }

    fn on_candle(&mut self, candle: &Candle) -> Vec<TradingSignal> {
        let mut signals = Vec::new();

        let close = candle.close;
        let high = candle.high;
        let low = candle.low;
        let volume = candle.volume;

        let atr = self.atr.update(high, low, close);

        // Track historical data before adding current candle
        let prev_high = self.n_period_high();
        let prev_low = self.n_period_low();
        let avg_vol = self.average_volume();

        // Add current data to history
        self.highs.push_back(high);
        self.lows.push_back(low);
        self.volumes.push_back(volume);

        if self.highs.len() > self.period {
            self.highs.pop_front();
        }
        if self.lows.len() > self.period {
            self.lows.pop_front();
        }
        if self.volumes.len() > self.period {
            self.volumes.pop_front();
        }

        self.current_symbol = Some(candle.symbol.clone());

        // Need enough data
        if self.highs.len() < self.period || !self.atr.is_initialized() {
            return signals;
        }

        let volume_increased = volume > avg_vol * dec!(1.2);

        // Exit logic: trailing stop
        if self.in_long {
            self.update_trailing_stop(close, atr);
            if close < self.trailing_stop {
                signals.push(TradingSignal::CloseLong {
                    symbol: candle.symbol.clone(),
                    reason: format!(
                        "Breakout trailing stop hit: price={}, stop={}",
                        close, self.trailing_stop
                    ),
                });
                self.in_long = false;
                self.trailing_stop = Decimal::ZERO;
            }
        }

        if self.in_short {
            self.update_trailing_stop(close, atr);
            if close > self.trailing_stop {
                signals.push(TradingSignal::CloseShort {
                    symbol: candle.symbol.clone(),
                    reason: format!(
                        "Breakout trailing stop hit: price={}, stop={}",
                        close, self.trailing_stop
                    ),
                });
                self.in_short = false;
                self.trailing_stop = Decimal::ZERO;
            }
        }

        // Entry logic
        if !self.in_long && !self.in_short {
            if let Some(period_high) = prev_high {
                if close > period_high && volume_increased {
                    let size_usd = risk::calculate_position_size(atr, self.balance, self.risk_per_trade);
                    signals.push(TradingSignal::OpenLong {
                        symbol: candle.symbol.clone(),
                        size_usd,
                        reason: format!(
                            "Breakout long: price={} > {}-period high={}, vol_inc={}",
                            close, self.period, period_high, volume_increased
                        ),
                    });
                    self.in_long = true;
                    self.entry_price = close;
                    self.trailing_stop = close - self.atr_stop_multiplier * atr;
                }
            }

            if let Some(period_low) = prev_low {
                if close < period_low && volume_increased && !self.in_long {
                    let size_usd = risk::calculate_position_size(atr, self.balance, self.risk_per_trade);
                    signals.push(TradingSignal::OpenShort {
                        symbol: candle.symbol.clone(),
                        size_usd,
                        reason: format!(
                            "Breakout short: price={} < {}-period low={}, vol_inc={}",
                            close, self.period, period_low, volume_increased
                        ),
                    });
                    self.in_short = true;
                    self.entry_price = close;
                    self.trailing_stop = close + self.atr_stop_multiplier * atr;
                }
            }
        }

        signals
    }

    fn on_trade(&mut self, _trade: &Trade) -> Vec<TradingSignal> {
        Vec::new()
    }

    fn on_orderbook(&mut self, _book: &OrderBook) -> Vec<TradingSignal> {
        Vec::new()
    }

    fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.volumes.clear();
        self.atr = Atr::new(self.atr.period());
        self.in_long = false;
        self.in_short = false;
        self.entry_price = Decimal::ZERO;
        self.trailing_stop = Decimal::ZERO;
        self.current_symbol = None;
    }
}
