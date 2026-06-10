use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use cte_core::config::StrategyConfig;
use cte_core::traits::Strategy;
use cte_core::{Candle, OrderBook, SimPosition, Trade, TradingSignal};

use crate::position_manager::{PositionEvent, PositionManager};
use crate::sim_trader::SimTrader;
use crate::strategies::breakout::BreakoutStrategy;
use crate::strategies::mean_reversion::MeanReversionStrategy;
use crate::strategies::momentum::MomentumStrategy;
use crate::strategies::scalper::ScalperStrategy;

/// Summary of PnL for reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PnlSummary {
    pub total_realized_pnl: Decimal,
    pub total_unrealized_pnl: Decimal,
    pub equity: Decimal,
    pub balance: Decimal,
    pub open_positions: usize,
    pub closed_trades: usize,
    pub win_rate: Decimal,
    pub drawdown_pct: Decimal,
    pub consecutive_losses: u32,
}

/// The strategy execution engine that ties all strategies together.
pub struct StrategyEngine {
    strategies: Vec<Box<dyn Strategy>>,
    position_manager: PositionManager,
    all_signals: Vec<TradingSignal>,
    all_events: Vec<PositionEvent>,
    candles_processed: u64,
    trades_processed: u64,
}

impl StrategyEngine {
    /// Create a new StrategyEngine with the given config and trader.
    pub fn new(config: &StrategyConfig, trader: SimTrader) -> Self {
        let balance = trader.balance();
        let max_drawdown = dec!(10); // 10% max drawdown default

        let strategies: Vec<Box<dyn Strategy>> = match config.strategy_type.as_str() {
            "mean_reversion" => {
                vec![Box::new(MeanReversionStrategy::new(balance))]
            }
            "momentum" => {
                let fast = config.ema_fast.unwrap_or(9);
                let slow = config.ema_slow.unwrap_or(21);
                let rsi_period = config.rsi_period.unwrap_or(14);
                vec![Box::new(MomentumStrategy::with_params(
                    fast,
                    slow,
                    rsi_period,
                    balance,
                    dec!(0.02),
                ))]
            }
            "breakout" => {
                vec![Box::new(BreakoutStrategy::new(balance))]
            }
            "scalper" => {
                vec![Box::new(ScalperStrategy::new(balance))]
            }
            "combined" | _ => {
                // Run all strategies
                vec![
                    Box::new(MeanReversionStrategy::new(balance)),
                    Box::new(MomentumStrategy::new(balance)),
                    Box::new(BreakoutStrategy::new(balance)),
                ]
            }
        };

        let position_manager = PositionManager::new(trader, max_drawdown);

        Self {
            strategies,
            position_manager,
            all_signals: Vec::new(),
            all_events: Vec::new(),
            candles_processed: 0,
            trades_processed: 0,
        }
    }

    /// Create an engine with explicit strategies.
    pub fn with_strategies(
        strategies: Vec<Box<dyn Strategy>>,
        trader: SimTrader,
        max_drawdown_pct: Decimal,
    ) -> Self {
        let position_manager = PositionManager::new(trader, max_drawdown_pct);

        Self {
            strategies,
            position_manager,
            all_signals: Vec::new(),
            all_events: Vec::new(),
            candles_processed: 0,
            trades_processed: 0,
        }
    }

    /// Process a candle through all strategies, then handle resulting signals.
    pub fn process_candle(&mut self, candle: &Candle) {
        self.candles_processed += 1;

        // Update positions with current price
        self.position_manager.tick(&candle.symbol, candle.close);

        // Collect signals from all strategies
        let mut signals = Vec::new();
        for strategy in self.strategies.iter_mut() {
            let strat_signals = strategy.on_candle(candle);
            if !strat_signals.is_empty() {
                debug!(
                    strategy = strategy.name(),
                    signals = strat_signals.len(),
                    "Strategy produced signals"
                );
            }
            signals.extend(strat_signals);
        }

        // Process each signal through position manager
        for signal in signals {
            self.all_signals.push(signal.clone());
            let events = self.position_manager.process_signal(signal, candle.close);
            for event in &events {
                match event {
                    PositionEvent::Opened(pos) => {
                        info!(
                            symbol = %pos.symbol,
                            side = %pos.side,
                            size = %pos.usd_size,
                            "Engine: position opened"
                        );
                    }
                    PositionEvent::Closed(pos) => {
                        info!(
                            symbol = %pos.symbol,
                            pnl = %pos.realized_pnl,
                            "Engine: position closed"
                        );
                    }
                    PositionEvent::Rejected { signal, reason } => {
                        debug!(signal = signal, reason = reason, "Signal rejected");
                    }
                    PositionEvent::RiskHalt { reason } => {
                        info!(reason = reason, "Engine: trading halted");
                    }
                }
            }
            self.all_events.extend(events);
        }
    }

    /// Process a trade through all strategies.
    pub fn process_trade(&mut self, trade: &Trade) {
        self.trades_processed += 1;

        // Update positions with trade price
        self.position_manager.tick(&trade.symbol, trade.price);

        let mut signals = Vec::new();
        for strategy in self.strategies.iter_mut() {
            signals.extend(strategy.on_trade(trade));
        }

        for signal in signals {
            self.all_signals.push(signal.clone());
            let events = self.position_manager.process_signal(signal, trade.price);
            self.all_events.extend(events);
        }
    }

    /// Process an orderbook update through all strategies.
    pub fn process_orderbook(&mut self, book: &OrderBook) {
        let mid_price = book.mid_price().unwrap_or(Decimal::ZERO);

        let mut signals = Vec::new();
        for strategy in self.strategies.iter_mut() {
            signals.extend(strategy.on_orderbook(book));
        }

        for signal in signals {
            self.all_signals.push(signal.clone());
            let events = self.position_manager.process_signal(signal, mid_price);
            self.all_events.extend(events);
        }
    }

    /// Get all open positions.
    pub fn get_positions(&self) -> &[SimPosition] {
        self.position_manager.trader().open_positions()
    }

    /// Get all closed positions.
    pub fn get_closed_positions(&self) -> &[SimPosition] {
        self.position_manager.trader().closed_positions()
    }

    /// Get a PnL summary.
    pub fn get_pnl_summary(&self) -> PnlSummary {
        let trader = self.position_manager.trader();
        let closed = trader.closed_positions();

        let wins = closed.iter().filter(|p| p.realized_pnl > Decimal::ZERO).count();
        let total_closed = closed.len();
        let win_rate = if total_closed > 0 {
            Decimal::from(wins as u64) / Decimal::from(total_closed as u64) * dec!(100)
        } else {
            Decimal::ZERO
        };

        PnlSummary {
            total_realized_pnl: trader.total_realized_pnl(),
            total_unrealized_pnl: trader.total_unrealized_pnl(),
            equity: trader.equity(),
            balance: trader.balance(),
            open_positions: trader.open_positions().len(),
            closed_trades: total_closed,
            win_rate,
            drawdown_pct: trader.drawdown_pct(),
            consecutive_losses: trader.consecutive_losses(),
        }
    }

    /// Get all signals generated so far.
    pub fn signals(&self) -> &[TradingSignal] {
        &self.all_signals
    }

    /// Get all events generated so far.
    pub fn events(&self) -> &[PositionEvent] {
        &self.all_events
    }

    /// Get the number of candles processed.
    pub fn candles_processed(&self) -> u64 {
        self.candles_processed
    }

    /// Get the number of trades processed.
    pub fn trades_processed(&self) -> u64 {
        self.trades_processed
    }

    /// Check if the engine is halted.
    pub fn is_halted(&self) -> bool {
        self.position_manager.is_halted()
    }

    /// Resume trading after a halt.
    pub fn resume(&mut self) {
        self.position_manager.resume_trading();
    }

    /// Get a reference to the position manager.
    pub fn position_manager(&self) -> &PositionManager {
        &self.position_manager
    }

    /// Get a mutable reference to the position manager.
    pub fn position_manager_mut(&mut self) -> &mut PositionManager {
        &mut self.position_manager
    }

    /// Reset all strategies.
    pub fn reset_strategies(&mut self) {
        for strategy in self.strategies.iter_mut() {
            strategy.reset();
        }
    }

    /// Get the names of all loaded strategies.
    pub fn strategy_names(&self) -> Vec<&str> {
        self.strategies.iter().map(|s| s.name()).collect()
    }
}
