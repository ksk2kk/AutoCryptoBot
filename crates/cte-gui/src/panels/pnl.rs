use egui::{Color32, RichText};

use crate::app::{decimal_to_f64, CteApp};

/// Render the PnL summary panel as a horizontal bar.
pub fn render_pnl_summary(ui: &mut egui::Ui, app: &CteApp) {
    let pnl = &app.total_pnl;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 20.0;

        // Equity
        let equity = decimal_to_f64(pnl.equity);
        ui.label(RichText::new("Equity:").strong());
        ui.label(
            RichText::new(format!("${:.2}", equity))
                .color(Color32::WHITE)
                .strong()
                .monospace(),
        );

        ui.separator();

        // Unrealized PnL
        let unrealized = decimal_to_f64(pnl.total_unrealized_pnl);
        let unrealized_color = if unrealized >= 0.0 {
            Color32::from_rgb(38, 166, 91)
        } else {
            Color32::from_rgb(214, 48, 49)
        };
        ui.label(RichText::new("Unrealized:").strong());
        ui.label(
            RichText::new(format!("{:+.2}", unrealized))
                .color(unrealized_color)
                .monospace(),
        );

        ui.separator();

        // Realized PnL
        let realized = decimal_to_f64(pnl.total_realized_pnl);
        let realized_color = if realized >= 0.0 {
            Color32::from_rgb(38, 166, 91)
        } else {
            Color32::from_rgb(214, 48, 49)
        };
        ui.label(RichText::new("Realized:").strong());
        ui.label(
            RichText::new(format!("{:+.2}", realized))
                .color(realized_color)
                .monospace(),
        );

        ui.separator();

        // Open positions count
        ui.label(RichText::new("Positions:").strong());
        ui.label(
            RichText::new(format!("{}", pnl.open_positions))
                .monospace(),
        );

        ui.separator();

        // Total trades
        ui.label(RichText::new("Trades:").strong());
        ui.label(
            RichText::new(format!("{}", pnl.total_trades))
                .monospace(),
        );

        ui.separator();

        // Win rate
        ui.label(RichText::new("Win Rate:").strong());
        let wr_color = if pnl.win_rate >= 50.0 {
            Color32::from_rgb(38, 166, 91)
        } else if pnl.total_trades > 0 {
            Color32::from_rgb(214, 48, 49)
        } else {
            Color32::GRAY
        };
        ui.label(
            RichText::new(format!("{:.1}%", pnl.win_rate))
                .color(wr_color)
                .monospace(),
        );

        ui.separator();

        // ROI from initial capital
        let initial = decimal_to_f64(app.initial_capital);
        let roi = if initial > 0.0 {
            (equity - initial) / initial * 100.0
        } else {
            0.0
        };
        let roi_color = if roi >= 0.0 {
            Color32::from_rgb(38, 166, 91)
        } else {
            Color32::from_rgb(214, 48, 49)
        };
        ui.label(RichText::new("ROI:").strong());
        ui.label(
            RichText::new(format!("{:+.2}%", roi))
                .color(roi_color)
                .monospace(),
        );
    });
}
