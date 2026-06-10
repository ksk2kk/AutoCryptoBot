use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::Duration;

use chrono::Utc;
use rust_decimal::Decimal;

use cte_core::{
    Candle, Exchange, MarketEvent, OrderBook, OrderType, Side, SimPosition,
    Symbol, MarketType, Timeframe, Trade,
};
use cte_strategy::indicators::bollinger::BollingerBands;
use cte_strategy::indicators::ema::Ema;

use crate::panels;
use crate::{GuiCommand, PnlSummary};

/// Maximum number of candles to keep in the rolling window.
const MAX_CANDLES: usize = 500;
/// Maximum number of recent trades to display.
const MAX_RECENT_TRADES: usize = 100;
/// Maximum number of log messages to keep.
const MAX_LOG_MESSAGES: usize = 200;

/// The main application struct for the CTE GUI.
pub struct CteApp {
    // Market data (received from background)
    pub candles: VecDeque<Candle>,
    pub orderbook: OrderBook,
    pub recent_trades: VecDeque<Trade>,

    // Trading state
    pub positions: Vec<SimPosition>,
    pub total_pnl: PnlSummary,

    // UI state
    pub selected_exchange: Exchange,
    pub selected_symbol: String,
    pub selected_timeframe: Timeframe,
    pub show_bollinger: bool,
    pub show_ema: bool,
    pub available_symbols: Vec<String>,
    pub available_exchanges: Vec<Exchange>,
    pub no_auto_trade: bool,

    // Manual order input
    pub order_side: Side,
    pub order_size: String,
    pub order_type: OrderType,
    pub order_price: String,

    // Channels
    pub data_rx: mpsc::Receiver<MarketEvent>,
    pub pnl_rx: mpsc::Receiver<PnlSummary>,
    pub status_rx: mpsc::Receiver<Vec<(Exchange, bool)>>,
    pub cmd_tx: mpsc::Sender<GuiCommand>,

    // Status
    pub connected_exchanges: Vec<(Exchange, bool)>,
    pub log_messages: VecDeque<String>,

    // Indicators (computed locally from candle data for chart overlays)
    pub ema_fast: Ema,
    pub ema_slow: Ema,
    pub bollinger: BollingerBands,
    pub ema_fast_values: VecDeque<f64>,
    pub ema_slow_values: VecDeque<f64>,
    pub bollinger_upper: VecDeque<f64>,
    pub bollinger_middle: VecDeque<f64>,
    pub bollinger_lower: VecDeque<f64>,

    // Capital tracking
    pub initial_capital: Decimal,
}

impl CteApp {
    /// Create a new CteApp with the given channels and initial configuration.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        data_rx: mpsc::Receiver<MarketEvent>,
        pnl_rx: mpsc::Receiver<PnlSummary>,
        status_rx: mpsc::Receiver<Vec<(Exchange, bool)>>,
        cmd_tx: mpsc::Sender<GuiCommand>,
        primary_exchange: Exchange,
        primary_symbol: String,
        selected_timeframe: Timeframe,
        capital: Decimal,
        available_symbols: Vec<String>,
        available_exchanges: Vec<Exchange>,
        no_auto_trade: bool,
    ) -> Self {
        let empty_symbol = Symbol {
            base: String::new(),
            quote: "USDT".to_string(),
            market_type: MarketType::LinearPerpetual,
            exchange: primary_exchange,
            raw_symbol: String::new(),
        };

        Self {
            candles: VecDeque::with_capacity(MAX_CANDLES),
            orderbook: OrderBook {
                symbol: empty_symbol,
                timestamp: Utc::now(),
                bids: Vec::new(),
                asks: Vec::new(),
                sequence: 0,
            },
            recent_trades: VecDeque::with_capacity(MAX_RECENT_TRADES),
            positions: Vec::new(),
            total_pnl: PnlSummary {
                equity: capital,
                ..PnlSummary::default()
            },
            selected_exchange: primary_exchange,
            selected_symbol: primary_symbol,
            selected_timeframe,
            show_bollinger: false,
            show_ema: true,
            available_symbols,
            available_exchanges,
            no_auto_trade,
            order_side: Side::Long,
            order_size: "100".to_string(),
            order_type: OrderType::Market,
            order_price: String::new(),
            data_rx,
            pnl_rx,
            status_rx,
            cmd_tx,
            connected_exchanges: Vec::new(),
            log_messages: VecDeque::with_capacity(MAX_LOG_MESSAGES),
            ema_fast: Ema::new(9),
            ema_slow: Ema::new(21),
            bollinger: BollingerBands::default_params(),
            ema_fast_values: VecDeque::with_capacity(MAX_CANDLES),
            ema_slow_values: VecDeque::with_capacity(MAX_CANDLES),
            bollinger_upper: VecDeque::with_capacity(MAX_CANDLES),
            bollinger_middle: VecDeque::with_capacity(MAX_CANDLES),
            bollinger_lower: VecDeque::with_capacity(MAX_CANDLES),
            initial_capital: capital,
        }
    }

    /// Drain all pending messages from channels without blocking.
    fn drain_channels(&mut self) {
        // Drain market data events
        let mut candle_count = 0;
        while let Ok(event) = self.data_rx.try_recv() {
            match event {
                MarketEvent::CandleUpdate(candle) => {
                    // Only process candles for the selected symbol
                    if candle.symbol.raw_symbol == self.selected_symbol {
                        // Update indicators
                        let _close_f64 = decimal_to_f64(candle.close);
                        let ema_fast_val = decimal_to_f64(self.ema_fast.update(candle.close));
                        let ema_slow_val = decimal_to_f64(self.ema_slow.update(candle.close));
                        self.bollinger.update(candle.close);
                        let bb_upper = decimal_to_f64(self.bollinger.upper());
                        let bb_middle = decimal_to_f64(self.bollinger.middle());
                        let bb_lower = decimal_to_f64(self.bollinger.lower());

                        self.ema_fast_values.push_back(ema_fast_val);
                        self.ema_slow_values.push_back(ema_slow_val);
                        self.bollinger_upper.push_back(bb_upper);
                        self.bollinger_middle.push_back(bb_middle);
                        self.bollinger_lower.push_back(bb_lower);

                        // Trim indicator buffers
                        while self.ema_fast_values.len() > MAX_CANDLES {
                            self.ema_fast_values.pop_front();
                        }
                        while self.ema_slow_values.len() > MAX_CANDLES {
                            self.ema_slow_values.pop_front();
                        }
                        while self.bollinger_upper.len() > MAX_CANDLES {
                            self.bollinger_upper.pop_front();
                        }
                        while self.bollinger_middle.len() > MAX_CANDLES {
                            self.bollinger_middle.pop_front();
                        }
                        while self.bollinger_lower.len() > MAX_CANDLES {
                            self.bollinger_lower.pop_front();
                        }

                        self.candles.push_back(candle);
                        while self.candles.len() > MAX_CANDLES {
                            self.candles.pop_front();
                        }
                        candle_count += 1;
                    }
                }
                MarketEvent::TradeUpdate(trade) => {
                    if trade.symbol.raw_symbol == self.selected_symbol {
                        self.recent_trades.push_back(trade);
                        while self.recent_trades.len() > MAX_RECENT_TRADES {
                            self.recent_trades.pop_front();
                        }
                    }
                }
                MarketEvent::OrderBookUpdate(ob) => {
                    if ob.symbol.raw_symbol == self.selected_symbol {
                        self.orderbook = ob;
                    }
                }
            }
        }

        if candle_count > 0 {
            self.log_messages.push_back(format!(
                "[{}] Received {} candle update(s) for {}",
                Utc::now().format("%H:%M:%S"),
                candle_count,
                self.selected_symbol
            ));
            while self.log_messages.len() > MAX_LOG_MESSAGES {
                self.log_messages.pop_front();
            }
        }

        // Drain PnL updates (take the latest)
        let mut latest_pnl = None;
        while let Ok(pnl) = self.pnl_rx.try_recv() {
            latest_pnl = Some(pnl);
        }
        if let Some(pnl) = latest_pnl {
            self.total_pnl = pnl;
        }

        // Drain status updates
        while let Ok(status) = self.status_rx.try_recv() {
            self.connected_exchanges = status;
        }
    }
}

impl eframe::App for CteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain channels for latest data
        self.drain_channels();

        // Request periodic repaint for live updates
        ctx.request_repaint_after(Duration::from_millis(100));

        // === TOP BAR ===
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            panels::status::render_status_bar(ui, self);
        });

        // === BOTTOM PANEL: Manual order + log ===
        egui::TopBottomPanel::bottom("bottom_panel")
            .min_height(120.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.columns(2, |cols| {
                    panels::manual_order::render_manual_order(&mut cols[0], self);
                    panels::log::render_log(&mut cols[1], self);
                });
            });

        // === RIGHT SIDE PANEL: Order book + Recent trades ===
        egui::SidePanel::right("right_panel")
            .min_width(280.0)
            .default_width(320.0)
            .resizable(true)
            .show(ctx, |ui| {
                let available_height = ui.available_height();
                let half_height = available_height / 2.0;

                // Order book in top half
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), half_height),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        panels::orderbook::render_orderbook(ui, self);
                    },
                );

                ui.separator();

                // Recent trades in bottom half
                panels::trades::render_recent_trades(ui, self);
            });

        // === CENTRAL PANEL: Chart + Positions ===
        egui::CentralPanel::default().show(ctx, |ui| {
            let available_height = ui.available_height();

            // Top section: Chart (roughly 60% of height)
            let chart_height = available_height * 0.6;
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), chart_height),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // Chart controls
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.show_ema, "EMA (9/21)");
                        ui.checkbox(&mut self.show_bollinger, "Bollinger Bands");
                        ui.label(format!("Candles: {}", self.candles.len()));
                    });

                    panels::chart::render_candlestick_chart(ui, self);
                },
            );

            ui.separator();

            // PnL summary bar
            panels::pnl::render_pnl_summary(ui, self);

            ui.separator();

            // Bottom section: Positions table
            panels::positions::render_positions_table(ui, self);
        });
    }
}

/// Convert a Decimal to f64 for use in egui_plot.
pub fn decimal_to_f64(d: Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}
