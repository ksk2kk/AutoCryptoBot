use egui::RichText;

use crate::app::CteApp;

/// Render the strategy log and status messages panel.
pub fn render_log(ui: &mut egui::Ui, app: &CteApp) {
    ui.heading("Log");
    ui.separator();

    if app.log_messages.is_empty() {
        ui.label("No log messages yet.");
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("log_scroll")
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            for msg in app.log_messages.iter().rev().take(50) {
                ui.label(RichText::new(msg).monospace().small());
            }
        });
}
