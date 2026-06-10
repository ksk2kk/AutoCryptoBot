use std::collections::HashMap;

use chrono::Utc;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tracing::{debug, info, warn};
use uuid::Uuid;

use cte_core::{CteError, OrderStatus, OrderType, Result, Side, SimOrder, SimPosition, Symbol};

/// Simulated order execution engine for paper trading.
pub struct SimTrader {
    positions: Vec<SimPosition>,
    pending_orders: Vec<SimOrder>,
    closed_positions: Vec<SimPosition>,
    balance: Decimal,
    initial_balance: Decimal,
    total_realized_pnl: Decimal,
    max_positions: usize,
    max_position_size_usd: Decimal,
}

impl SimTrader {
    /// Create a new SimTrader with starting capital and position limits.
    pub fn new(capital: Decimal, max_positions: usize, max_position_size_usd: Decimal) -> Self {
        Self {
            positions: Vec::new(),
            pending_orders: Vec::new(),
            closed_positions: Vec::new(),
            balance: capital,
            initial_balance: capital,
            total_realized_pnl: Decimal::ZERO,
            max_positions,
            max_position_size_usd,
        }
    }

    /// Execute a market order immediately at the given price.
    pub fn market_order(
        &mut self,
        symbol: Symbol,
        side: Side,
        size_usd: Decimal,
        current_price: Decimal,
    ) -> Result<SimPosition> {
        // Validate position limits
        if self.positions.len() >= self.max_positions {
            return Err(CteError::PositionLimitReached {
                max: self.max_positions,
            });
        }

        // Clamp size to max
        let actual_size_usd = size_usd.min(self.max_position_size_usd);

        // Check balance
        if actual_size_usd > self.balance {
            return Err(CteError::InsufficientBalance {
                required: actual_size_usd,
                available: self.balance,
            });
        }

        if current_price.is_zero() {
            return Err(CteError::Strategy("Cannot open position at zero price".to_string()));
        }

        let quantity = actual_size_usd / current_price;

        let position = SimPosition {
            id: Uuid::new_v4(),
            symbol: symbol.clone(),
            side,
            entry_price: current_price,
            quantity,
            unrealized_pnl: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
            opened_at: Utc::now(),
            closed_at: None,
            usd_size: actual_size_usd,
        };

        self.balance -= actual_size_usd;
        self.positions.push(position.clone());

        info!(
            symbol = %symbol,
            side = %side,
            size_usd = %actual_size_usd,
            price = %current_price,
            qty = %quantity,
            "Market order filled"
        );

        Ok(position)
    }

    /// Place a limit order that will be checked on each tick.
    pub fn limit_order(&mut self, order: SimOrder) -> Result<()> {
        if self.positions.len() >= self.max_positions && self.pending_orders.len() >= self.max_positions {
            return Err(CteError::PositionLimitReached {
                max: self.max_positions,
            });
        }

        let order_size = order.price.unwrap_or(Decimal::ZERO) * order.quantity;
        if order_size > self.max_position_size_usd {
            return Err(CteError::Strategy(format!(
                "Order size {} exceeds max {}",
                order_size, self.max_position_size_usd
            )));
        }

        debug!(
            symbol = %order.symbol,
            side = %order.side,
            price = ?order.price,
            qty = %order.quantity,
            "Limit order placed"
        );

        self.pending_orders.push(order);
        Ok(())
    }

    /// Process a tick: check pending orders for fills, update unrealized PnL.
    pub fn tick(&mut self, symbol: &Symbol, current_price: Decimal) {
        // Update unrealized PnL for open positions
        for pos in self.positions.iter_mut() {
            if pos.symbol == *symbol {
                pos.update_pnl(current_price);
            }
        }

        // Check pending orders for fills
        let mut filled_indices = Vec::new();
        for (i, order) in self.pending_orders.iter().enumerate() {
            if order.symbol != *symbol || order.status != OrderStatus::Pending {
                continue;
            }

            let should_fill = match (order.side, order.order_type) {
                (Side::Long, OrderType::Limit) => {
                    // Buy limit fills when price drops to or below limit price
                    order.price.map_or(false, |p| current_price <= p)
                }
                (Side::Short, OrderType::Limit) => {
                    // Sell limit fills when price rises to or above limit price
                    order.price.map_or(false, |p| current_price >= p)
                }
                (_, OrderType::Market) => true,
            };

            if should_fill {
                filled_indices.push(i);
            }
        }

        // Fill orders (in reverse to preserve indices)
        for i in filled_indices.into_iter().rev() {
            let mut order = self.pending_orders.remove(i);
            order.status = OrderStatus::Filled;
            order.filled_at = Some(Utc::now());

            let fill_price = order.price.unwrap_or(current_price);
            let size_usd = fill_price * order.quantity;

            if self.positions.len() < self.max_positions && size_usd <= self.balance {
                let position = SimPosition {
                    id: Uuid::new_v4(),
                    symbol: order.symbol.clone(),
                    side: order.side,
                    entry_price: fill_price,
                    quantity: order.quantity,
                    unrealized_pnl: Decimal::ZERO,
                    realized_pnl: Decimal::ZERO,
                    opened_at: Utc::now(),
                    closed_at: None,
                    usd_size: size_usd,
                };

                self.balance -= size_usd;
                self.positions.push(position);

                info!(
                    symbol = %order.symbol,
                    side = %order.side,
                    price = %fill_price,
                    "Limit order filled"
                );
            } else {
                warn!(
                    symbol = %order.symbol,
                    "Could not fill order: position limit or insufficient balance"
                );
            }
        }
    }

    /// Close a position at the current price.
    pub fn close_position(
        &mut self,
        position_id: Uuid,
        current_price: Decimal,
    ) -> Result<SimPosition> {
        let pos_idx = self
            .positions
            .iter()
            .position(|p| p.id == position_id)
            .ok_or_else(|| {
                CteError::Strategy(format!("Position {} not found", position_id))
            })?;

        let mut position = self.positions.remove(pos_idx);
        position.update_pnl(current_price);
        position.realized_pnl = position.unrealized_pnl;
        position.unrealized_pnl = Decimal::ZERO;
        position.closed_at = Some(Utc::now());

        // Return capital + PnL to balance
        let returned = position.entry_price * position.quantity + position.realized_pnl;
        self.balance += returned;
        self.total_realized_pnl += position.realized_pnl;

        info!(
            id = %position.id,
            symbol = %position.symbol,
            pnl = %position.realized_pnl,
            "Position closed"
        );

        self.closed_positions.push(position.clone());
        Ok(position)
    }

    /// Get all open positions.
    pub fn open_positions(&self) -> &[SimPosition] {
        &self.positions
    }

    /// Get all closed positions.
    pub fn closed_positions(&self) -> &[SimPosition] {
        &self.closed_positions
    }

    /// Get pending orders.
    pub fn pending_orders(&self) -> &[SimOrder] {
        &self.pending_orders
    }

    /// Total unrealized PnL across all open positions.
    pub fn total_unrealized_pnl(&self) -> Decimal {
        self.positions.iter().map(|p| p.unrealized_pnl).sum()
    }

    /// Total realized PnL from all closed positions.
    pub fn total_realized_pnl(&self) -> Decimal {
        self.total_realized_pnl
    }

    /// Total portfolio value given current prices for each symbol.
    pub fn total_portfolio_value(&self, prices: &HashMap<String, Decimal>) -> Decimal {
        let positions_value: Decimal = self
            .positions
            .iter()
            .map(|p| {
                let current_price = prices
                    .get(&p.symbol.raw_symbol)
                    .copied()
                    .unwrap_or(p.entry_price);
                current_price * p.quantity
            })
            .sum();

        self.balance + positions_value
    }

    /// Current equity = balance + unrealized PnL.
    pub fn equity(&self) -> Decimal {
        self.balance + self.total_unrealized_pnl()
    }

    /// Current available balance.
    pub fn balance(&self) -> Decimal {
        self.balance
    }

    /// Initial balance at start.
    pub fn initial_balance(&self) -> Decimal {
        self.initial_balance
    }

    /// Max allowed positions.
    pub fn max_positions(&self) -> usize {
        self.max_positions
    }

    /// Max position size in USD.
    pub fn max_position_size_usd(&self) -> Decimal {
        self.max_position_size_usd
    }

    /// Current drawdown percentage from initial balance.
    pub fn drawdown_pct(&self) -> Decimal {
        if self.initial_balance.is_zero() {
            return Decimal::ZERO;
        }
        let equity = self.equity();
        if equity >= self.initial_balance {
            return Decimal::ZERO;
        }
        ((self.initial_balance - equity) / self.initial_balance) * dec!(100)
    }

    /// Number of consecutive losses from recent closed positions.
    pub fn consecutive_losses(&self) -> u32 {
        let mut count = 0u32;
        for pos in self.closed_positions.iter().rev() {
            if pos.realized_pnl < Decimal::ZERO {
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    /// Find open position by symbol and side.
    pub fn find_position(&self, symbol: &Symbol, side: Side) -> Option<&SimPosition> {
        self.positions
            .iter()
            .find(|p| p.symbol == *symbol && p.side == side)
    }

    /// Find open position ID by symbol and side.
    pub fn find_position_id(&self, symbol: &Symbol, side: Side) -> Option<Uuid> {
        self.find_position(symbol, side).map(|p| p.id)
    }

    /// Cancel a pending order.
    pub fn cancel_order(&mut self, order_id: Uuid) -> Result<()> {
        let idx = self
            .pending_orders
            .iter()
            .position(|o| o.id == order_id)
            .ok_or_else(|| CteError::Strategy(format!("Order {} not found", order_id)))?;

        self.pending_orders[idx].status = OrderStatus::Cancelled;
        self.pending_orders.remove(idx);
        Ok(())
    }
}
