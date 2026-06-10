use egui::{Color32, RichText};
use rust_decimal::Decimal;

use cte_core::{OrderType, Side};

use crate::app::CteApp;
use crate::GuiCommand;

/// Render the manual order input form.
pub fn render_manual_order(ui: &mut egui::Ui, app: &mut CteApp) {
    ui.heading("Manual Order");
    ui.separator();

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;

        // Side selector
        ui.label(RichText::new("Side:").strong());
        let long_color = if app.order_side == Side::Long {
            Color32::from_rgb(38, 166, 91)
        } else {
            Color32::GRAY
        };
        let short_color = if app.order_side == Side::Short {
            Color32::from_rgb(214, 48, 49)
        } else {
            Color32::GRAY
        };

        if ui
            .button(RichText::new("LONG").color(long_color).strong())
            .clicked()
        {
            app.order_side = Side::Long;
        }
        if ui
            .button(RichText::new("SHORT").color(short_color).strong())
            .clicked()
        {
            app.order_side = Side::Short;
        }

        ui.separator();

        // Order type selector
        ui.label(RichText::new("Type:").strong());
        egui::ComboBox::from_id_salt("order_type_selector")
            .selected_text(match app.order_type {
                OrderType::Market => "Market",
                OrderType::Limit => "Limit",
            })
            .width(80.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.order_type, OrderType::Market, "Market");
                ui.selectable_value(&mut app.order_type, OrderType::Limit, "Limit");
            });

        ui.separator();

        // Size input (USD)
        ui.label(RichText::new("Size $:").strong());
        ui.add(
            egui::TextEdit::singleline(&mut app.order_size)
                .desired_width(80.0)
                .hint_text("USD"),
        );

        // Price input (for limit orders)
        if app.order_type == OrderType::Limit {
            ui.separator();
            ui.label(RichText::new("Price:").strong());
            ui.add(
                egui::TextEdit::singleline(&mut app.order_price)
                    .desired_width(100.0)
                    .hint_text("Limit price"),
            );
        }

        ui.separator();

        // Submit button
        let submit_color = match app.order_side {
            Side::Long => Color32::from_rgb(38, 166, 91),
            Side::Short => Color32::from_rgb(214, 48, 49),
        };
        let submit_text = match app.order_side {
            Side::Long => "BUY / LONG",
            Side::Short => "SELL / SHORT",
        };

        let submit_button = egui::Button::new(
            RichText::new(submit_text).color(Color32::WHITE).strong(),
        )
        .fill(submit_color);

        if ui.add(submit_button).clicked() {
            // Parse the size
            if let Ok(size_f64) = app.order_size.parse::<f64>() {
                let size_usd = Decimal::from_f64_retain(size_f64).unwrap_or(Decimal::ZERO);
                if size_usd > Decimal::ZERO {
                    // Parse price (for limit orders or as reference for market)
                    let price = if app.order_type == OrderType::Limit {
                        app.order_price
                            .parse::<f64>()
                            .ok()
                            .and_then(|p| Decimal::from_f64_retain(p))
                    } else {
                        // For market orders, use the best bid/ask from orderbook
                        match app.order_side {
                            Side::Long => app.orderbook.best_ask().map(|l| l.price),
                            Side::Short => app.orderbook.best_bid().map(|l| l.price),
                        }
                    };

                    let cmd = GuiCommand::ManualOrder {
                        symbol: app.selected_symbol.clone(),
                        side: app.order_side,
                        order_type: app.order_type,
                        size_usd,
                        price,
                    };

                    if app.cmd_tx.send(cmd).is_ok() {
                        app.log_messages.push_back(format!(
                            "[{}] Order submitted: {} {} ${} {}",
                            chrono::Utc::now().format("%H:%M:%S"),
                            app.order_side,
                            app.selected_symbol,
                            size_f64,
                            match app.order_type {
                                OrderType::Market => "MARKET".to_string(),
                                OrderType::Limit => format!("LIMIT @ {}", app.order_price),
                            },
                        ));
                    }
                }
            }
        }

        // Quick size buttons
        ui.separator();
        for size in &["50", "100", "250", "500", "1000"] {
            if ui.small_button(*size).clicked() {
                app.order_size = size.to_string();
            }
        }
    });
}
