use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use cte_core::traits::Strategy;
use cte_core::{Candle, OrderBook, Symbol, Trade, TradingSignal};

use crate::indicators::ema::Ema;
use crate::indicators::rsi::Rsi;
use crate::risk;

/// Momentum strategy using EMA crossover with RSI filter.
///
/// Entry logic:
/// - Long when: fast EMA crosses above slow AND RSI > 50 AND RSI < 70
/// - Short when: fast EMA crosses below slow AND RSI < 50 AND RSI > 30
///
/// Exit logic:
/// - Close long: fast EMA crosses below slow OR RSI > 80
/// - Close short: fast EMA crosses above slow OR RSI < 20
pub struct MomentumStrategy {
    fast_ema: Ema,
    slow_ema: Ema,
    rsi: Rsi,
    prev_fast: Option<Decimal>,
    prev_slow: Option<Decimal>,
    in_long: bool,
    in_short: bool,
    current_symbol: Option<Symbol>,
    balance: Decimal,
    risk_per_trade: Decimal,
}

impl MomentumStrategy {
    pub fn new(balance: Decimal) -> Self {
        Self {
            fast_ema: Ema::new(9),
            slow_ema: Ema::new(21),
            rsi: Rsi::default_period(),
            prev_fast: None,
            prev_slow: None,
            in_long: false,
            in_short: false,
            current_symbol: None,
            balance,
            risk_per_trade: dec!(0.02),
        }
    }

    pub fn with_params(
        fast_period: usize,
        slow_period: usize,
        rsi_period: usize,
        balance: Decimal,
        risk_per_trade: Decimal,
    ) -> Self {
        Self {
            fast_ema: Ema::new(fast_period),
            slow_ema: Ema::new(slow_period),
            rsi: Rsi::new(rsi_period),
            prev_fast: None,
            prev_slow: None,
            in_long: false,
            in_short: false,
            current_symbol: None,
            balance,
            risk_per_trade,
        }
    }

    fn is_bullish_crossover(&self, fast: Decimal, slow: Decimal) -> bool {
        if let (Some(prev_f), Some(prev_s)) = (self.prev_fast, self.prev_slow) {
            prev_f <= prev_s && fast > slow
        } else {
            false
        }
    }

    fn is_bearish_crossover(&self, fast: Decimal, slow: Decimal) -> bool {
        if let (Some(prev_f), Some(prev_s)) = (self.prev_fast, self.prev_slow) {
            prev_f >= prev_s && fast < slow
        } else {
            false
        }
    }
}

impl Strategy for MomentumStrategy {
    fn name(&self) -> &str {
        "momentum"
    }

    fn on_candle(&mut self, candle: &Candle) -> Vec<TradingSignal> {
        let mut signals = Vec::new();

        let close = candle.close;
        let fast = self.fast_ema.update(close);
        let slow = self.slow_ema.update(close);
        let rsi = self.rsi.update(close);
        self.current_symbol = Some(candle.symbol.clone());

        // Wait until indicators are initialized
        if !self.fast_ema.is_initialized() || !self.slow_ema.is_initialized() || !self.rsi.is_initialized() {
            self.prev_fast = Some(fast);
            self.prev_slow = Some(slow);
            return signals;
        }

        let bullish_cross = self.is_bullish_crossover(fast, slow);
        let bearish_cross = self.is_bearish_crossover(fast, slow);

        // Exit logic first
        if self.in_long {
            if bearish_cross || rsi > dec!(80) {
                signals.push(TradingSignal::CloseLong {
                    symbol: candle.symbol.clone(),
                    reason: format!(
                        "Momentum exit long: bearish_cross={}, RSI={:.1}",
                        bearish_cross, rsi
                    ),
                });
                self.in_long = false;
            }
        }

        if self.in_short {
            if bullish_cross || rsi < dec!(20) {
                signals.push(TradingSignal::CloseShort {
                    symbol: candle.symbol.clone(),
                    reason: format!(
                        "Momentum exit short: bullish_cross={}, RSI={:.1}",
                        bullish_cross, rsi
                    ),
                });
                self.in_short = false;
            }
        }

        // Entry logic
        if !self.in_long && !self.in_short {
            if bullish_cross && rsi > dec!(50) && rsi < dec!(70) {
                // Use a default position size based on balance and risk
                let size_usd = risk::calculate_position_size(
                    close * dec!(0.02), // approximate ATR as 2% of price
                    self.balance,
                    self.risk_per_trade,
                );
                signals.push(TradingSignal::OpenLong {
                    symbol: candle.symbol.clone(),
                    size_usd,
                    reason: format!(
                        "Momentum long: EMA({}) crossed above EMA({}), RSI={:.1}",
                        self.fast_ema.period(),
                        self.slow_ema.period(),
                        rsi
                    ),
                });
                self.in_long = true;
            } else if bearish_cross && rsi < dec!(50) && rsi > dec!(30) {
                let size_usd = risk::calculate_position_size(
                    close * dec!(0.02),
                    self.balance,
                    self.risk_per_trade,
                );
                signals.push(TradingSignal::OpenShort {
                    symbol: candle.symbol.clone(),
                    size_usd,
                    reason: format!(
                        "Momentum short: EMA({}) crossed below EMA({}), RSI={:.1}",
                        self.fast_ema.period(),
                        self.slow_ema.period(),
                        rsi
                    ),
                });
                self.in_short = true;
            }
        }

        self.prev_fast = Some(fast);
        self.prev_slow = Some(slow);

        signals
    }

    fn on_trade(&mut self, _trade: &Trade) -> Vec<TradingSignal> {
        Vec::new()
    }

    fn on_orderbook(&mut self, _book: &OrderBook) -> Vec<TradingSignal> {
        Vec::new()
    }

    fn reset(&mut self) {
        self.fast_ema = Ema::new(self.fast_ema.period());
        self.slow_ema = Ema::new(self.slow_ema.period());
        self.rsi = Rsi::new(self.rsi.period());
        self.prev_fast = None;
        self.prev_slow = None;
        self.in_long = false;
        self.in_short = false;
        self.current_symbol = None;
    }
}
