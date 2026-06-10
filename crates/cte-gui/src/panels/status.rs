use egui::{Color32, RichText};

use crate::app::{decimal_to_f64, CteApp};
use cte_core::Timeframe;

/// Render the top status bar with exchange selector, symbol, timeframe, and connection status.
pub fn render_status_bar(ui: &mut egui::Ui, app: &mut CteApp) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;

        // Exchange selector
        ui.label(RichText::new("Exchange:").strong());
        egui::ComboBox::from_id_salt("exchange_selector")
            .selected_text(format!("{}", app.selected_exchange))
            .show_ui(ui, |ui| {
                for exchange in &app.available_exchanges {
                    let text = format!("{}", exchange);
                    ui.selectable_value(&mut app.selected_exchange, *exchange, text);
                }
            });

        // Symbol selector
        ui.label(RichText::new("Symbol:").strong());
        egui::ComboBox::from_id_salt("symbol_selector")
            .selected_text(&app.selected_symbol)
            .show_ui(ui, |ui| {
                for sym in &app.available_symbols.clone() {
                    ui.selectable_value(&mut app.selected_symbol, sym.clone(), sym);
                }
            });

        // Timeframe selector
        ui.label(RichText::new("Timeframe:").strong());
        egui::ComboBox::from_id_salt("timeframe_selector")
            .selected_text(format!("{}", app.selected_timeframe))
            .show_ui(ui, |ui| {
                let timeframes = [
                    Timeframe::S1,
                    Timeframe::M1,
                    Timeframe::M3,
                    Timeframe::M5,
                    Timeframe::M15,
                    Timeframe::M30,
                    Timeframe::H1,
                    Timeframe::H4,
                    Timeframe::D1,
                    Timeframe::W1,
                ];
                for tf in &timeframes {
                    let text = format!("{}", tf);
                    ui.selectable_value(&mut app.selected_timeframe, *tf, text);
                }
            });

        ui.separator();

        // Connection status indicators
        ui.label(RichText::new("Exchanges:").strong());
        for (exchange, connected) in &app.connected_exchanges {
            let (icon, color) = if *connected {
                ("●", Color32::from_rgb(38, 166, 91))
            } else {
                ("●", Color32::from_rgb(214, 48, 49))
            };
            ui.label(
                RichText::new(format!("{} {}", icon, exchange))
                    .color(color)
                    .small(),
            );
        }

        // If no exchanges connected yet, show placeholder
        if app.connected_exchanges.is_empty() {
            ui.label(RichText::new("● connecting...").color(Color32::YELLOW).small());
        }

        ui.separator();

        // Quick PnL display
        let total_pnl = decimal_to_f64(app.total_pnl.total_unrealized_pnl)
            + decimal_to_f64(app.total_pnl.total_realized_pnl);
        let pnl_color = if total_pnl >= 0.0 {
            Color32::from_rgb(38, 166, 91)
        } else {
            Color32::from_rgb(214, 48, 49)
        };
        ui.label(RichText::new("PnL:").strong());
        ui.label(
            RichText::new(format!("{:+.2}", total_pnl))
                .color(pnl_color)
                .strong()
                .monospace(),
        );

        // Auto trade indicator
        ui.separator();
        if app.no_auto_trade {
            ui.label(RichText::new("AUTO: OFF").color(Color32::GRAY).small());
        } else {
            ui.label(RichText::new("AUTO: ON").color(Color32::from_rgb(38, 166, 91)).small());
        }

        // Current time (right-aligned)
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let now = chrono::Utc::now();
            ui.label(
                RichText::new(now.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .monospace()
                    .small(),
            );
        });
    });
}
