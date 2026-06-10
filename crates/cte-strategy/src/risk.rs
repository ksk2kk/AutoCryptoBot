use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::sim_trader::SimTrader;

/// Check if total exposure is within acceptable limits.
/// Returns true if trading can continue, false if exposure is too high.
pub fn check_exposure(trader: &SimTrader) -> bool {
    let equity = trader.equity();
    if equity.is_zero() {
        return false;
    }

    // Total position value should not exceed 80% of equity
    let total_position_value: Decimal = trader
        .open_positions()
        .iter()
        .map(|p| p.usd_size)
        .sum();

    let exposure_ratio = total_position_value / equity;
    exposure_ratio < dec!(0.8)
}

/// Calculate position size based on ATR-based risk management.
///
/// Formula: size_usd = (balance * risk_per_trade) / (2 * ATR) * ATR = balance * risk_per_trade / 2
/// Adjusted by volatility: higher ATR = smaller position.
///
/// The ATR represents the expected move, so we size the position such that
/// a 1-ATR move against us equals our max risk amount.
pub fn calculate_position_size(atr: Decimal, balance: Decimal, risk_per_trade: Decimal) -> Decimal {
    if atr.is_zero() || balance.is_zero() {
        return Decimal::ZERO;
    }

    // Volatility-adjusted position sizing:
    // Lower volatility = larger position, higher volatility = smaller position
    let volatility_factor = dec!(1) / (dec!(1) + atr * dec!(10));

    let size = balance * risk_per_trade * dec!(10) * volatility_factor;

    // Cap at 20% of balance
    let max_size = balance * dec!(0.2);
    size.min(max_size).max(dec!(10)) // minimum $10
}

/// Determine if trading should be halted based on risk parameters.
///
/// - `consecutive_losses`: number of consecutive losing trades
/// - `max_drawdown_pct`: maximum allowed drawdown percentage (e.g., 10.0 for 10%)
/// - `current_drawdown`: current drawdown percentage
pub fn should_stop_trading(
    consecutive_losses: u32,
    max_drawdown_pct: Decimal,
    current_drawdown: Decimal,
) -> bool {
    // Stop if we've hit max drawdown
    if current_drawdown >= max_drawdown_pct {
        return true;
    }

    // Stop if we have too many consecutive losses (circuit breaker)
    if consecutive_losses >= 5 {
        return true;
    }

    // Progressive risk reduction: if 3+ consecutive losses and drawdown > half of max
    if consecutive_losses >= 3 && current_drawdown > max_drawdown_pct / dec!(2) {
        return true;
    }

    false
}

/// Calculate the maximum number of positions allowed given current drawdown.
pub fn adjusted_max_positions(base_max: usize, drawdown_pct: Decimal) -> usize {
    if drawdown_pct >= dec!(10) {
        return 1;
    }
    if drawdown_pct >= dec!(5) {
        return (base_max / 2).max(1);
    }
    base_max
}

/// Calculate risk-adjusted return (Sharpe-like ratio).
pub fn risk_reward_ratio(avg_win: Decimal, avg_loss: Decimal) -> Decimal {
    if avg_loss.is_zero() {
        return Decimal::ZERO;
    }
    avg_win / avg_loss.abs()
}
