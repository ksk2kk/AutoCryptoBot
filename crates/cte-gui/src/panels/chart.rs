use egui::Color32;
use egui_plot::{
    Bar, BarChart, BoxElem, BoxPlot, BoxSpread, Line, Plot, PlotPoints,
};

use crate::app::{decimal_to_f64, CteApp};

/// Render the candlestick chart with volume bars and optional indicator overlays.
pub fn render_candlestick_chart(ui: &mut egui::Ui, app: &CteApp) {
    if app.candles.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() / 3.0);
            ui.label("Waiting for candle data...");
        });
        return;
    }

    let candle_count = app.candles.len();

    // Build candle box elements
    let mut bullish_boxes: Vec<BoxElem> = Vec::with_capacity(candle_count);
    let mut bearish_boxes: Vec<BoxElem> = Vec::with_capacity(candle_count);
    let mut volume_bars: Vec<Bar> = Vec::with_capacity(candle_count);

    // Track price and volume ranges for scaling
    let mut max_volume: f64 = 0.0;

    for (i, candle) in app.candles.iter().enumerate() {
        let x = i as f64;
        let open = decimal_to_f64(candle.open);
        let high = decimal_to_f64(candle.high);
        let low = decimal_to_f64(candle.low);
        let close = decimal_to_f64(candle.close);
        let volume = decimal_to_f64(candle.volume);

        if volume > max_volume {
            max_volume = volume;
        }

        let is_bullish = close >= open;

        // BoxElem uses BoxSpread: whisker_low, quarter1, median, quarter3, whisker_high
        // For candlesticks:
        //   whisker_low = low (wick low)
        //   quarter1 = min(open, close) (body bottom)
        //   median = (open + close) / 2.0 (middle of body, used for rendering)
        //   quarter3 = max(open, close) (body top)
        //   whisker_high = high (wick high)
        let body_low = open.min(close);
        let body_high = open.max(close);
        let median = (open + close) / 2.0;

        let box_elem = BoxElem::new(x, BoxSpread::new(low, body_low, median, body_high, high))
            .box_width(0.7)
            .whisker_width(0.1);

        if is_bullish {
            bullish_boxes.push(box_elem.fill(Color32::from_rgb(38, 166, 91)).stroke(egui::Stroke::new(1.0, Color32::from_rgb(38, 166, 91))));
        } else {
            bearish_boxes.push(box_elem.fill(Color32::from_rgb(214, 48, 49)).stroke(egui::Stroke::new(1.0, Color32::from_rgb(214, 48, 49))));
        }

        // Volume bar (will be plotted in a separate sub-plot area conceptually,
        // but we use the same plot with a scaled-down height)
        let vol_color = if is_bullish {
            Color32::from_rgba_premultiplied(38, 166, 91, 80)
        } else {
            Color32::from_rgba_premultiplied(214, 48, 49, 80)
        };
        volume_bars.push(Bar::new(x, volume).width(0.7).fill(vol_color));
    }

    // Create the box plots for candles
    let bullish_plot = BoxPlot::new(bullish_boxes).name("Bullish");
    let bearish_plot = BoxPlot::new(bearish_boxes).name("Bearish");

    // Main candlestick chart
    Plot::new("candlestick_chart")
        .height(ui.available_height() * 0.75)
        .allow_drag(true)
        .allow_zoom(true)
        .allow_scroll(true)
        .x_axis_label("Candle Index")
        .y_axis_label("Price")
        .show_axes([true, true])
        .show(ui, |plot_ui| {
            plot_ui.box_plot(bullish_plot);
            plot_ui.box_plot(bearish_plot);

            // EMA overlay lines
            if app.show_ema && !app.ema_fast_values.is_empty() {
                let offset = candle_count.saturating_sub(app.ema_fast_values.len());

                let fast_points: PlotPoints = PlotPoints::new(
                    app.ema_fast_values
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| [(i + offset) as f64, v])
                        .collect(),
                );
                let slow_points: PlotPoints = PlotPoints::new(
                    app.ema_slow_values
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| [(i + offset) as f64, v])
                        .collect(),
                );

                plot_ui.line(
                    Line::new(fast_points)
                        .name("EMA 9")
                        .color(Color32::from_rgb(52, 152, 219))
                        .width(1.5),
                );
                plot_ui.line(
                    Line::new(slow_points)
                        .name("EMA 21")
                        .color(Color32::from_rgb(243, 156, 18))
                        .width(1.5),
                );
            }

            // Bollinger Bands overlay
            if app.show_bollinger && !app.bollinger_upper.is_empty() {
                let offset = candle_count.saturating_sub(app.bollinger_upper.len());

                let upper_points: PlotPoints = PlotPoints::new(
                    app.bollinger_upper
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| [(i + offset) as f64, v])
                        .collect(),
                );
                let middle_points: PlotPoints = PlotPoints::new(
                    app.bollinger_middle
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| [(i + offset) as f64, v])
                        .collect(),
                );
                let lower_points: PlotPoints = PlotPoints::new(
                    app.bollinger_lower
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| [(i + offset) as f64, v])
                        .collect(),
                );

                plot_ui.line(
                    Line::new(upper_points)
                        .name("BB Upper")
                        .color(Color32::from_rgb(155, 89, 182))
                        .width(1.0)
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
                plot_ui.line(
                    Line::new(middle_points)
                        .name("BB Middle")
                        .color(Color32::from_rgb(155, 89, 182))
                        .width(1.0),
                );
                plot_ui.line(
                    Line::new(lower_points)
                        .name("BB Lower")
                        .color(Color32::from_rgb(155, 89, 182))
                        .width(1.0)
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
            }
        });

    // Volume sub-chart
    if !volume_bars.is_empty() {
        let volume_chart = BarChart::new(volume_bars).name("Volume");

        Plot::new("volume_chart")
            .height(ui.available_height().max(60.0))
            .allow_drag(true)
            .allow_zoom(true)
            .allow_scroll(true)
            .show_axes([true, true])
            .y_axis_label("Volume")
            .show(ui, |plot_ui| {
                plot_ui.bar_chart(volume_chart);
            });
    }
}
