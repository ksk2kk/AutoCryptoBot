use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use cte_core::traits::Strategy;
use cte_core::{Candle, OrderBook, OrderBookLevel, Symbol, Trade, TradingSignal};

use crate::indicators::atr::Atr;
use crate::indicators::vwap::Vwap;
use crate::risk;

/// Scalper strategy using order-flow imbalance + VWAP deviation.
///
/// Entry logic:
/// - Long when: price < VWAP - 0.5*ATR AND bid volume > ask volume
/// - Short when: price > VWAP + 0.5*ATR AND ask volume > bid volume
///
/// Exit logic:
/// - Quick exit: small target (0.3*ATR) or VWAP cross
pub struct ScalperStrategy {
    vwap: Vwap,
    atr: Atr,
    in_long: bool,
    in_short: bool,
    entry_price: Decimal,
    current_atr: Decimal,
    current_vwap: Decimal,
    bid_volume: Decimal,
    ask_volume: Decimal,
    current_symbol: Option<Symbol>,
    balance: Decimal,
    risk_per_trade: Decimal,
}

impl ScalperStrategy {
    pub fn new(balance: Decimal) -> Self {
        Self {
            vwap: Vwap::new(),
            atr: Atr::new(14),
            in_long: false,
            in_short: false,
            entry_price: Decimal::ZERO,
            current_atr: Decimal::ZERO,
            current_vwap: Decimal::ZERO,
            bid_volume: Decimal::ZERO,
            ask_volume: Decimal::ZERO,
            current_symbol: None,
            balance,
            risk_per_trade: dec!(0.01), // 1% risk for scalping
        }
    }

    fn total_volume(levels: &[OrderBookLevel]) -> Decimal {
        levels.iter().map(|l| l.quantity).sum()
    }
}

impl Strategy for ScalperStrategy {
    fn name(&self) -> &str {
        "scalper"
    }

    fn on_candle(&mut self, candle: &Candle) -> Vec<TradingSignal> {
        let mut signals = Vec::new();

        let close = candle.close;
        let volume = candle.volume;

        self.current_atr = self.atr.update(candle.high, candle.low, close);
        self.current_vwap = self.vwap.update_with_time(close, volume, candle.close_time);
        self.current_symbol = Some(candle.symbol.clone());

        // Wait for ATR to initialize
        if !self.atr.is_initialized() || !self.vwap.is_initialized() {
            return signals;
        }

        let atr = self.current_atr;
        let vwap = self.current_vwap;

        // Exit logic: target or VWAP cross
        if self.in_long {
            let target = self.entry_price + dec!(0.3) * atr;
            let vwap_cross = close > vwap;
            if close >= target || vwap_cross {
                signals.push(TradingSignal::CloseLong {
                    symbol: candle.symbol.clone(),
                    reason: format!(
                        "Scalper exit long: price={}, target={}, vwap_cross={}",
                        close, target, vwap_cross
                    ),
                });
                self.in_long = false;
                self.entry_price = Decimal::ZERO;
            }
        }

        if self.in_short {
            let target = self.entry_price - dec!(0.3) * atr;
            let vwap_cross = close < vwap;
            if close <= target || vwap_cross {
                signals.push(TradingSignal::CloseShort {
                    symbol: candle.symbol.clone(),
                    reason: format!(
                        "Scalper exit short: price={}, target={}, vwap_cross={}",
                        close, target, vwap_cross
                    ),
                });
                self.in_short = false;
                self.entry_price = Decimal::ZERO;
            }
        }

        // Entry logic: VWAP deviation + order flow imbalance
        if !self.in_long && !self.in_short {
            let vwap_lower = vwap - dec!(0.5) * atr;
            let vwap_upper = vwap + dec!(0.5) * atr;

            if close < vwap_lower && self.bid_volume > self.ask_volume {
                let size_usd = risk::calculate_position_size(atr, self.balance, self.risk_per_trade);
                signals.push(TradingSignal::OpenLong {
                    symbol: candle.symbol.clone(),
                    size_usd,
                    reason: format!(
                        "Scalper long: price={} < VWAP-0.5*ATR={}, bid_vol={} > ask_vol={}",
                        close, vwap_lower, self.bid_volume, self.ask_volume
                    ),
                });
                self.in_long = true;
                self.entry_price = close;
            } else if close > vwap_upper && self.ask_volume > self.bid_volume {
                let size_usd = risk::calculate_position_size(atr, self.balance, self.risk_per_trade);
                signals.push(TradingSignal::OpenShort {
                    symbol: candle.symbol.clone(),
                    size_usd,
                    reason: format!(
                        "Scalper short: price={} > VWAP+0.5*ATR={}, ask_vol={} > bid_vol={}",
                        close, vwap_upper, self.ask_volume, self.bid_volume
                    ),
                });
                self.in_short = true;
                self.entry_price = close;
            }
        }

        signals
    }

    fn on_trade(&mut self, trade: &Trade) -> Vec<TradingSignal> {
        // Update order flow: track cumulative bid/ask volume from trades
        if trade.is_buyer_maker {
            // Seller is the taker (aggressive sell)
            self.ask_volume += trade.quantity;
        } else {
            // Buyer is the taker (aggressive buy)
            self.bid_volume += trade.quantity;
        }
        Vec::new()
    }

    fn on_orderbook(&mut self, book: &OrderBook) -> Vec<TradingSignal> {
        // Update bid/ask volume from orderbook snapshot
        self.bid_volume = Self::total_volume(&book.bids);
        self.ask_volume = Self::total_volume(&book.asks);
        Vec::new()
    }

    fn reset(&mut self) {
        self.vwap.reset();
        self.atr = Atr::new(self.atr.period());
        self.in_long = false;
        self.in_short = false;
        self.entry_price = Decimal::ZERO;
        self.current_atr = Decimal::ZERO;
        self.current_vwap = Decimal::ZERO;
        self.bid_volume = Decimal::ZERO;
        self.ask_volume = Decimal::ZERO;
        self.current_symbol = None;
    }
}
