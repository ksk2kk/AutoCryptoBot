use egui::{Color32, RichText};

use crate::app::{decimal_to_f64, CteApp};

/// Render the order book panel with bids and asks.
pub fn render_orderbook(ui: &mut egui::Ui, app: &CteApp) {
    ui.heading("Order Book");
    ui.separator();

    let max_levels = 15;
    let ob = &app.orderbook;

    // Calculate spread
    let spread_text = if let Some(spread) = ob.spread() {
        let mid = ob.mid_price().unwrap_or(rust_decimal::Decimal::ZERO);
        let spread_pct = if !mid.is_zero() {
            decimal_to_f64(spread) / decimal_to_f64(mid) * 100.0
        } else {
            0.0
        };
        format!("Spread: {:.4} ({:.3}%)", decimal_to_f64(spread), spread_pct)
    } else {
        "Spread: --".to_string()
    };

    // Asks (displayed top-to-bottom, highest ask first, so we reverse a limited slice)
    let asks_display: Vec<_> = ob.asks.iter().take(max_levels).collect();

    ui.label(RichText::new("ASKS").color(Color32::from_rgb(214, 48, 49)).strong());

    egui::ScrollArea::vertical()
        .id_salt("asks_scroll")
        .max_height(ui.available_height() * 0.4)
        .show(ui, |ui| {
            egui::Grid::new("asks_grid")
                .num_columns(3)
                .spacing([12.0, 2.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label(RichText::new("Price").strong().small());
                    ui.label(RichText::new("Qty").strong().small());
                    ui.label(RichText::new("Total").strong().small());
                    ui.end_row();

                    // Display asks from highest to lowest (reversed)
                    for level in asks_display.iter().rev() {
                        let price = decimal_to_f64(level.price);
                        let qty = decimal_to_f64(level.quantity);
                        let total = price * qty;

                        ui.label(
                            RichText::new(format!("{:.2}", price))
                                .color(Color32::from_rgb(214, 48, 49))
                                .monospace(),
                        );
                        ui.label(RichText::new(format!("{:.4}", qty)).monospace());
                        ui.label(RichText::new(format!("{:.2}", total)).monospace());
                        ui.end_row();
                    }
                });
        });

    // Spread in the middle
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(RichText::new(spread_text).strong().color(Color32::WHITE));
    });
    ui.separator();

    // Bids
    ui.label(RichText::new("BIDS").color(Color32::from_rgb(38, 166, 91)).strong());

    egui::ScrollArea::vertical()
        .id_salt("bids_scroll")
        .max_height(ui.available_height())
        .show(ui, |ui| {
            egui::Grid::new("bids_grid")
                .num_columns(3)
                .spacing([12.0, 2.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label(RichText::new("Price").strong().small());
                    ui.label(RichText::new("Qty").strong().small());
                    ui.label(RichText::new("Total").strong().small());
                    ui.end_row();

                    for level in ob.bids.iter().take(max_levels) {
                        let price = decimal_to_f64(level.price);
                        let qty = decimal_to_f64(level.quantity);
                        let total = price * qty;

                        ui.label(
                            RichText::new(format!("{:.2}", price))
                                .color(Color32::from_rgb(38, 166, 91))
                                .monospace(),
                        );
                        ui.label(RichText::new(format!("{:.4}", qty)).monospace());
                        ui.label(RichText::new(format!("{:.2}", total)).monospace());
                        ui.end_row();
                    }
                });
        });
}
