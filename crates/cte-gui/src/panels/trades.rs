use egui::{Color32, RichText};

use crate::app::{decimal_to_f64, CteApp};

/// Render the recent trades ticker panel.
pub fn render_recent_trades(ui: &mut egui::Ui, app: &CteApp) {
    ui.heading("Recent Trades");
    ui.separator();

    if app.recent_trades.is_empty() {
        ui.label("Waiting for trade data...");
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("trades_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("trades_grid")
                .num_columns(4)
                .spacing([8.0, 2.0])
                .striped(true)
                .show(ui, |ui| {
                    // Header
                    ui.label(RichText::new("Time").strong().small());
                    ui.label(RichText::new("Side").strong().small());
                    ui.label(RichText::new("Price").strong().small());
                    ui.label(RichText::new("Qty").strong().small());
                    ui.end_row();

                    // Display trades from most recent to oldest
                    for trade in app.recent_trades.iter().rev().take(50) {
                        let time_str = trade.timestamp.format("%H:%M:%S").to_string();

                        // is_buyer_maker == true means the trade was a sell (taker sold)
                        let is_buy = !trade.is_buyer_maker;
                        let side_color = if is_buy {
                            Color32::from_rgb(38, 166, 91)
                        } else {
                            Color32::from_rgb(214, 48, 49)
                        };
                        let side_text = if is_buy { "BUY" } else { "SELL" };

                        let price = decimal_to_f64(trade.price);
                        let qty = decimal_to_f64(trade.quantity);

                        ui.label(RichText::new(time_str).small().monospace());
                        ui.label(
                            RichText::new(side_text)
                                .color(side_color)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("{:.2}", price))
                                .color(side_color)
                                .monospace()
                                .small(),
                        );
                        ui.label(
                            RichText::new(format!("{:.4}", qty))
                                .monospace()
                                .small(),
                        );
                        ui.end_row();
                    }
                });
        });
}
