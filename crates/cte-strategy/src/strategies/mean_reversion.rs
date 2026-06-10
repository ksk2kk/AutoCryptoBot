use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use cte_core::traits::Strategy;
use cte_core::{Candle, OrderBook, Symbol, Trade, TradingSignal};

use crate::indicators::atr::Atr;
use crate::indicators::bollinger::BollingerBands;
use crate::indicators::rsi::Rsi;
use crate::risk;

/// Mean Reversion strategy using Bollinger Bands + RSI.
///
/// Entry logic:
/// - Long when: price < lower band AND RSI < 30
/// - Short when: price > upper band AND RSI > 70
///
/// Exit logic:
/// - Close long when: price > middle band OR RSI > 60
/// - Close short when: price < middle band OR RSI < 40
///
/// Position sizing: ATR-based.
pub struct MeanReversionStrategy {
    bollinger: BollingerBands,
    rsi: Rsi,
    atr: Atr,
    in_long: bool,
    in_short: bool,
    current_symbol: Option<Symbol>,
    balance: Decimal,
    risk_per_trade: Decimal,
}

impl MeanReversionStrategy {
    pub fn new(balance: Decimal) -> Self {
        Self {
            bollinger: BollingerBands::default_params(),
            rsi: Rsi::default_period(),
            atr: Atr::new(14),
            in_long: false,
            in_short: false,
            current_symbol: None,
            balance,
            risk_per_trade: dec!(0.02), // 2% risk per trade
        }
    }

    pub fn with_params(
        bb_period: usize,
        bb_std_dev: Decimal,
        rsi_period: usize,
        atr_period: usize,
        balance: Decimal,
        risk_per_trade: Decimal,
    ) -> Self {
        Self {
            bollinger: BollingerBands::new(bb_period, bb_std_dev),
            rsi: Rsi::new(rsi_period),
            atr: Atr::new(atr_period),
            in_long: false,
            in_short: false,
            current_symbol: None,
            balance,
            risk_per_trade,
        }
    }
}

impl Strategy for MeanReversionStrategy {
    fn name(&self) -> &str {
        "mean_reversion"
    }

    fn on_candle(&mut self, candle: &Candle) -> Vec<TradingSignal> {
        let mut signals = Vec::new();

        // Update indicators
        let close = candle.close;
        self.bollinger.update(close);
        let rsi = self.rsi.update(close);
        let atr = self.atr.update(candle.high, candle.low, close);
        self.current_symbol = Some(candle.symbol.clone());

        // Wait for indicators to initialize
        if !self.bollinger.is_initialized() || !self.rsi.is_initialized() || !self.atr.is_initialized() {
            return signals;
        }

        let upper = self.bollinger.upper();
        let middle = self.bollinger.middle();
        let lower = self.bollinger.lower();

        // Exit logic first
        if self.in_long {
            if close > middle || rsi > dec!(60) {
                signals.push(TradingSignal::CloseLong {
                    symbol: candle.symbol.clone(),
                    reason: format!(
                        "Mean reversion exit: price={}, middle={}, RSI={:.1}",
                        close, middle, rsi
                    ),
                });
                self.in_long = false;
            }
        }

        if self.in_short {
            if close < middle || rsi < dec!(40) {
                signals.push(TradingSignal::CloseShort {
                    symbol: candle.symbol.clone(),
                    reason: format!(
                        "Mean reversion exit: price={}, middle={}, RSI={:.1}",
                        close, middle, rsi
                    ),
                });
                self.in_short = false;
            }
        }

        // Entry logic
        if !self.in_long && !self.in_short {
            if close < lower && rsi < dec!(30) {
                let size_usd = risk::calculate_position_size(atr, self.balance, self.risk_per_trade);
                signals.push(TradingSignal::OpenLong {
                    symbol: candle.symbol.clone(),
                    size_usd,
                    reason: format!(
                        "Mean reversion long: price={} < lower_bb={}, RSI={:.1}",
                        close, lower, rsi
                    ),
                });
                self.in_long = true;
            } else if close > upper && rsi > dec!(70) {
                let size_usd = risk::calculate_position_size(atr, self.balance, self.risk_per_trade);
                signals.push(TradingSignal::OpenShort {
                    symbol: candle.symbol.clone(),
                    size_usd,
                    reason: format!(
                        "Mean reversion short: price={} > upper_bb={}, RSI={:.1}",
                        close, upper, rsi
                    ),
                });
                self.in_short = true;
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
        self.bollinger = BollingerBands::default_params();
        self.rsi = Rsi::default_period();
        self.atr = Atr::new(14);
        self.in_long = false;
        self.in_short = false;
        self.current_symbol = None;
    }
}
