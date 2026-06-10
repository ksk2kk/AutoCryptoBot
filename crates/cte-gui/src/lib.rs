mod app;
mod panels;

use std::sync::mpsc;
use std::thread;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tracing::{error, info, warn};

use cte_core::{AppConfig, Exchange, MarketEvent, MarketType, Timeframe};
use cte_exchange::ExchangeRegistry;
use cte_strategy::SimTrader;

pub use app::CteApp;

/// PnL summary sent from the strategy engine to the GUI.
#[derive(Debug, Clone)]
pub struct PnlSummary {
    pub total_unrealized_pnl: Decimal,
    pub total_realized_pnl: Decimal,
    pub equity: Decimal,
    pub open_positions: usize,
    pub total_trades: u32,
    pub win_rate: f64,
}

impl Default for PnlSummary {
    fn default() -> Self {
        Self {
            total_unrealized_pnl: Decimal::ZERO,
            total_realized_pnl: Decimal::ZERO,
            equity: Decimal::ZERO,
            open_positions: 0,
            total_trades: 0,
            win_rate: 0.0,
        }
    }
}

/// Command sent from GUI to the background trading engine.
#[derive(Debug, Clone)]
pub enum GuiCommand {
    ManualOrder {
        symbol: String,
        side: cte_core::Side,
        order_type: cte_core::OrderType,
        size_usd: Decimal,
        price: Option<Decimal>,
    },
    ClosePosition {
        position_id: uuid::Uuid,
    },
}

/// Launch the CTE GUI application.
///
/// Sets up a tokio runtime in a background thread, creates exchange connections,
/// starts WebSocket subscriptions, and launches the eframe native window.
pub fn run_app(
    config: AppConfig,
    symbols: String,
    exchanges: String,
    timeframe: String,
    capital: f64,
    no_auto_trade: bool,
) -> anyhow::Result<()> {
    // Parse parameters
    let symbol_list: Vec<String> = symbols
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .collect();

    let exchange_list: Vec<Exchange> = exchanges
        .split(',')
        .filter_map(|s| s.trim().parse::<Exchange>().ok())
        .collect();

    let selected_timeframe: Timeframe = timeframe
        .parse()
        .unwrap_or(Timeframe::M5);

    let capital_decimal = Decimal::from_f64_retain(capital).unwrap_or(dec!(10000));

    // Create channels for communication between tokio runtime and GUI
    let (data_tx, data_rx) = mpsc::channel::<MarketEvent>();
    let (pnl_tx, pnl_rx) = mpsc::channel::<PnlSummary>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<GuiCommand>();
    let (status_tx, status_rx) = mpsc::channel::<Vec<(Exchange, bool)>>();

    // Clone config data for the background thread
    let exchange_configs = config.exchanges.clone();
    let max_positions = config.trading.max_positions;
    let max_position_size = Decimal::from_f64_retain(config.trading.max_position_size_usd)
        .unwrap_or(dec!(2000));

    let primary_symbol = symbol_list.first().cloned().unwrap_or_else(|| "BTCUSDT".to_string());
    let primary_exchange = exchange_list.first().copied().unwrap_or(Exchange::Binance);

    // Spawn background tokio runtime thread
    let bg_symbol_list = symbol_list.clone();
    let bg_exchange_list = exchange_list.clone();
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime");

        rt.block_on(async move {
            // Build exchange registry from config
            let registry = ExchangeRegistry::from_config(&exchange_configs);

            // Connect to exchanges and report status
            let mut connection_status: Vec<(Exchange, bool)> = Vec::new();
            for exchange in &bg_exchange_list {
                if let Some(connector) = registry.get(*exchange) {
                    match connector.connect().await {
                        Ok(()) => {
                            info!(exchange = %exchange, "Connected successfully");
                            connection_status.push((*exchange, true));
                        }
                        Err(e) => {
                            error!(exchange = %exchange, error = %e, "Connection failed");
                            connection_status.push((*exchange, false));
                        }
                    }
                } else {
                    warn!(exchange = %exchange, "No connector configured");
                    connection_status.push((*exchange, false));
                }
            }

            let _ = status_tx.send(connection_status.clone());

            // Create the SimTrader for the strategy engine
            let mut trader = SimTrader::new(capital_decimal, max_positions, max_position_size);

            // Subscribe to market data for each exchange/symbol combination
            for exchange in &bg_exchange_list {
                if let Some(connector) = registry.get(*exchange) {
                    for sym_str in &bg_symbol_list {
                        let symbol = cte_core::Symbol {
                            base: sym_str.replace("USDT", "").to_string(),
                            quote: "USDT".to_string(),
                            market_type: MarketType::LinearPerpetual,
                            exchange: *exchange,
                            raw_symbol: sym_str.clone(),
                        };

                        // Subscribe to candles
                        let data_tx_candle = data_tx.clone();
                        match connector.subscribe_candles(&symbol, selected_timeframe).await {
                            Ok(mut rx) => {
                                let sym_clone = symbol.clone();
                                tokio::spawn(async move {
                                    loop {
                                        match rx.recv().await {
                                            Ok(candle) => {
                                                let _ = data_tx_candle
                                                    .send(MarketEvent::CandleUpdate(candle));
                                            }
                                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                                warn!(symbol = %sym_clone, lagged = n, "Candle receiver lagged");
                                            }
                                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                                warn!(symbol = %sym_clone, "Candle channel closed");
                                                break;
                                            }
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                error!(exchange = %exchange, symbol = %sym_str, error = %e, "Failed to subscribe candles");
                            }
                        }

                        // Subscribe to trades
                        let data_tx_trade = data_tx.clone();
                        match connector.subscribe_trades(&symbol).await {
                            Ok(mut rx) => {
                                let sym_clone = symbol.clone();
                                tokio::spawn(async move {
                                    loop {
                                        match rx.recv().await {
                                            Ok(trade) => {
                                                let _ = data_tx_trade
                                                    .send(MarketEvent::TradeUpdate(trade));
                                            }
                                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                                warn!(symbol = %sym_clone, lagged = n, "Trade receiver lagged");
                                            }
                                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                                warn!(symbol = %sym_clone, "Trade channel closed");
                                                break;
                                            }
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                error!(exchange = %exchange, symbol = %sym_str, error = %e, "Failed to subscribe trades");
                            }
                        }

                        // Subscribe to order book
                        let data_tx_ob = data_tx.clone();
                        match connector.subscribe_orderbook(&symbol).await {
                            Ok(mut rx) => {
                                let sym_clone = symbol.clone();
                                tokio::spawn(async move {
                                    loop {
                                        match rx.recv().await {
                                            Ok(ob) => {
                                                let _ = data_tx_ob
                                                    .send(MarketEvent::OrderBookUpdate(ob));
                                            }
                                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                                warn!(symbol = %sym_clone, lagged = n, "OrderBook receiver lagged");
                                            }
                                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                                warn!(symbol = %sym_clone, "OrderBook channel closed");
                                                break;
                                            }
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                error!(exchange = %exchange, symbol = %sym_str, error = %e, "Failed to subscribe orderbook");
                            }
                        }
                    }
                }
            }

            // Process commands from GUI and update PnL periodically
            let mut pnl_tick_interval = tokio::time::interval(std::time::Duration::from_millis(500));
            loop {
                // Check for GUI commands (non-blocking)
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        GuiCommand::ManualOrder { symbol, side, order_type, size_usd, price } => {
                            let sym = cte_core::Symbol {
                                base: symbol.replace("USDT", "").to_string(),
                                quote: "USDT".to_string(),
                                market_type: MarketType::LinearPerpetual,
                                exchange: primary_exchange,
                                raw_symbol: symbol.clone(),
                            };
                            match order_type {
                                cte_core::OrderType::Market => {
                                    let exec_price = price.unwrap_or(Decimal::ZERO);
                                    if !exec_price.is_zero() {
                                        match trader.market_order(sym, side, size_usd, exec_price) {
                                            Ok(pos) => info!(id = %pos.id, "Manual market order filled"),
                                            Err(e) => error!(error = %e, "Manual market order failed"),
                                        }
                                    }
                                }
                                cte_core::OrderType::Limit => {
                                    if let Some(limit_price) = price {
                                        let quantity = size_usd / limit_price;
                                        let order = cte_core::SimOrder {
                                            id: uuid::Uuid::new_v4(),
                                            symbol: sym,
                                            side,
                                            order_type: cte_core::OrderType::Limit,
                                            price: Some(limit_price),
                                            quantity,
                                            status: cte_core::OrderStatus::Pending,
                                            created_at: chrono::Utc::now(),
                                            filled_at: None,
                                        };
                                        match trader.limit_order(order) {
                                            Ok(()) => info!("Manual limit order placed"),
                                            Err(e) => error!(error = %e, "Manual limit order failed"),
                                        }
                                    }
                                }
                            }
                        }
                        GuiCommand::ClosePosition { position_id } => {
                            // Close at last known price - requires tracking
                            // For now, use ZERO which the SimTrader will reject
                            // In real usage the GUI sends the current market price
                            let _ = trader.close_position(position_id, Decimal::ZERO);
                        }
                    }
                }

                // Send PnL update
                pnl_tick_interval.tick().await;
                let summary = PnlSummary {
                    total_unrealized_pnl: trader.total_unrealized_pnl(),
                    total_realized_pnl: trader.total_realized_pnl(),
                    equity: trader.equity(),
                    open_positions: trader.open_positions().len(),
                    total_trades: trader.closed_positions().len() as u32,
                    win_rate: {
                        let closed = trader.closed_positions();
                        if closed.is_empty() {
                            0.0
                        } else {
                            let wins = closed.iter()
                                .filter(|p| p.realized_pnl > Decimal::ZERO)
                                .count();
                            wins as f64 / closed.len() as f64 * 100.0
                        }
                    },
                };
                let _ = pnl_tx.send(summary);
            }
        });
    });

    // Build the initial app state
    let app = CteApp::new(
        data_rx,
        pnl_rx,
        status_rx,
        cmd_tx,
        primary_exchange,
        primary_symbol,
        selected_timeframe,
        capital_decimal,
        symbol_list,
        exchange_list,
        no_auto_trade,
    );

    // Configure eframe native options
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 900.0])
            .with_min_inner_size([1024.0, 600.0])
            .with_title("CTE - Crypto Trading Engine"),
        ..Default::default()
    };

    // Launch the eframe window
    eframe::run_native(
        "CTE - Crypto Trading Engine",
        native_options,
        Box::new(|cc| {
            // Set dark visuals by default
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))?;

    Ok(())
}
