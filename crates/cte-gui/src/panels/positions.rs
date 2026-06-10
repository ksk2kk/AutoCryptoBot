use egui::{Color32, RichText};

use cte_core::Side;

use crate::app::{decimal_to_f64, CteApp};
use crate::GuiCommand;

/// Render the open positions table with PnL information.
pub fn render_positions_table(ui: &mut egui::Ui, app: &mut CteApp) {
    ui.heading("Open Positions");

    if app.positions.is_empty() && app.total_pnl.open_positions == 0 {
        ui.label("No open positions.");
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("positions_scroll")
        .show(ui, |ui| {
            egui::Grid::new("positions_grid")
                .num_columns(8)
                .spacing([10.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    // Header row
                    ui.label(RichText::new("Symbol").strong());
                    ui.label(RichText::new("Side").strong());
                    ui.label(RichText::new("Entry").strong());
                    ui.label(RichText::new("Size USD").strong());
                    ui.label(RichText::new("PnL %").strong());
                    ui.label(RichText::new("PnL $").strong());
                    ui.label(RichText::new("Duration").strong());
                    ui.label(RichText::new("Action").strong());
                    ui.end_row();

                    // Position rows
                    let positions_snapshot: Vec<_> = app.positions.clone();
                    for pos in &positions_snapshot {
                        // Symbol
                        ui.label(
                            RichText::new(&pos.symbol.raw_symbol).monospace(),
                        );

                        // Side
                        let side_color = match pos.side {
                            Side::Long => Color32::from_rgb(38, 166, 91),
                            Side::Short => Color32::from_rgb(214, 48, 49),
                        };
                        ui.label(
                            RichText::new(format!("{}", pos.side))
                                .color(side_color)
                                .strong(),
                        );

                        // Entry price
                        ui.label(
                            RichText::new(format!("{:.2}", decimal_to_f64(pos.entry_price)))
                                .monospace(),
                        );

                        // Size USD
                        ui.label(
                            RichText::new(format!("${:.2}", decimal_to_f64(pos.usd_size)))
                                .monospace(),
                        );

                        // PnL %
                        let pnl_pct = decimal_to_f64(pos.pnl_percent());
                        let pnl_color = if pnl_pct >= 0.0 {
                            Color32::from_rgb(38, 166, 91)
                        } else {
                            Color32::from_rgb(214, 48, 49)
                        };
                        ui.label(
                            RichText::new(format!("{:+.2}%", pnl_pct))
                                .color(pnl_color)
                                .monospace(),
                        );

                        // PnL $
                        let pnl_usd = decimal_to_f64(pos.unrealized_pnl);
                        ui.label(
                            RichText::new(format!("{:+.2}", pnl_usd))
                                .color(pnl_color)
                                .monospace(),
                        );

                        // Duration
                        let duration = chrono::Utc::now() - pos.opened_at;
                        let duration_str = if duration.num_hours() > 0 {
                            format!("{}h {}m", duration.num_hours(), duration.num_minutes() % 60)
                        } else {
                            format!("{}m", duration.num_minutes())
                        };
                        ui.label(RichText::new(duration_str).small());

                        // Close button
                        if ui.button(RichText::new("Close").color(Color32::WHITE)).clicked() {
                            let _ = app.cmd_tx.send(GuiCommand::ClosePosition {
                                position_id: pos.id,
                            });
                        }

                        ui.end_row();
                    }
                });
        });
}
