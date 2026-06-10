use rust_decimal::Decimal;
use tracing::{debug, info, warn};

use cte_core::{Side, SimPosition, Symbol, TradingSignal};

use crate::risk;
use crate::sim_trader::SimTrader;

/// Events produced by the position manager.
#[derive(Debug, Clone)]
pub enum PositionEvent {
    Opened(SimPosition),
    Closed(SimPosition),
    Rejected { signal: String, reason: String },
    RiskHalt { reason: String },
}

/// Position manager that wraps SimTrader with signal handling,
/// deduplication, and risk checks.
pub struct PositionManager {
    trader: SimTrader,
    max_drawdown_pct: Decimal,
    halted: bool,
}

impl PositionManager {
    /// Create a new PositionManager wrapping the given SimTrader.
    pub fn new(trader: SimTrader, max_drawdown_pct: Decimal) -> Self {
        Self {
            trader,
            max_drawdown_pct,
            halted: false,
        }
    }

    /// Process a trading signal and return resulting position events.
    pub fn process_signal(
        &mut self,
        signal: TradingSignal,
        current_price: Decimal,
    ) -> Vec<PositionEvent> {
        let mut events = Vec::new();

        // Check if trading is halted
        if self.halted {
            events.push(PositionEvent::Rejected {
                signal: format!("{:?}", signal),
                reason: "Trading halted due to risk limits".to_string(),
            });
            return events;
        }

        // Check risk before processing
        let drawdown = self.trader.drawdown_pct();
        let consecutive_losses = self.trader.consecutive_losses();

        if risk::should_stop_trading(consecutive_losses, self.max_drawdown_pct, drawdown) {
            self.halted = true;
            warn!(
                drawdown = %drawdown,
                consecutive_losses = consecutive_losses,
                "Trading halted by risk manager"
            );
            events.push(PositionEvent::RiskHalt {
                reason: format!(
                    "Drawdown: {:.2}%, Consecutive losses: {}",
                    drawdown, consecutive_losses
                ),
            });
            return events;
        }

        match signal {
            TradingSignal::OpenLong {
                ref symbol,
                size_usd,
                ref reason,
            } => {
                events.extend(self.handle_open(symbol, Side::Long, size_usd, current_price, reason));
            }
            TradingSignal::OpenShort {
                ref symbol,
                size_usd,
                ref reason,
            } => {
                events.extend(self.handle_open(symbol, Side::Short, size_usd, current_price, reason));
            }
            TradingSignal::CloseLong {
                ref symbol,
                ref reason,
            } => {
                events.extend(self.handle_close(symbol, Side::Long, current_price, reason));
            }
            TradingSignal::CloseShort {
                ref symbol,
                ref reason,
            } => {
                events.extend(self.handle_close(symbol, Side::Short, current_price, reason));
            }
        }

        events
    }

    fn handle_open(
        &mut self,
        symbol: &Symbol,
        side: Side,
        size_usd: Decimal,
        current_price: Decimal,
        reason: &str,
    ) -> Vec<PositionEvent> {
        let mut events = Vec::new();

        // Deduplication: don't open same-side position on same symbol
        if self.trader.find_position(symbol, side).is_some() {
            debug!(
                symbol = %symbol,
                side = %side,
                "Duplicate signal ignored: already have position"
            );
            events.push(PositionEvent::Rejected {
                signal: format!("Open {:?} {}", side, symbol),
                reason: "Duplicate: position already exists".to_string(),
            });
            return events;
        }

        // Check exposure
        if !risk::check_exposure(&self.trader) {
            events.push(PositionEvent::Rejected {
                signal: format!("Open {:?} {}", side, symbol),
                reason: "Exposure limit reached".to_string(),
            });
            return events;
        }

        // Execute the order
        match self.trader.market_order(symbol.clone(), side, size_usd, current_price) {
            Ok(position) => {
                info!(
                    symbol = %symbol,
                    side = %side,
                    size_usd = %size_usd,
                    reason = reason,
                    "Position opened"
                );
                events.push(PositionEvent::Opened(position));
            }
            Err(e) => {
                warn!(
                    symbol = %symbol,
                    side = %side,
                    error = %e,
                    "Failed to open position"
                );
                events.push(PositionEvent::Rejected {
                    signal: format!("Open {:?} {}", side, symbol),
                    reason: e.to_string(),
                });
            }
        }

        events
    }

    fn handle_close(
        &mut self,
        symbol: &Symbol,
        side: Side,
        current_price: Decimal,
        reason: &str,
    ) -> Vec<PositionEvent> {
        let mut events = Vec::new();

        let position_id = match self.trader.find_position_id(symbol, side) {
            Some(id) => id,
            None => {
                debug!(
                    symbol = %symbol,
                    side = %side,
                    "No position to close"
                );
                return events;
            }
        };

        match self.trader.close_position(position_id, current_price) {
            Ok(closed) => {
                info!(
                    symbol = %symbol,
                    side = %side,
                    pnl = %closed.realized_pnl,
                    reason = reason,
                    "Position closed"
                );
                events.push(PositionEvent::Closed(closed));
            }
            Err(e) => {
                warn!(
                    symbol = %symbol,
                    error = %e,
                    "Failed to close position"
                );
                events.push(PositionEvent::Rejected {
                    signal: format!("Close {:?} {}", side, symbol),
                    reason: e.to_string(),
                });
            }
        }

        events
    }

    /// Get a reference to the underlying trader.
    pub fn trader(&self) -> &SimTrader {
        &self.trader
    }

    /// Get a mutable reference to the underlying trader.
    pub fn trader_mut(&mut self) -> &mut SimTrader {
        &mut self.trader
    }

    /// Check if trading is halted.
    pub fn is_halted(&self) -> bool {
        self.halted
    }

    /// Resume trading (reset halt).
    pub fn resume_trading(&mut self) {
        self.halted = false;
        info!("Trading resumed");
    }

    /// Update tick on the trader for a symbol.
    pub fn tick(&mut self, symbol: &Symbol, current_price: Decimal) {
        self.trader.tick(symbol, current_price);
    }
}
