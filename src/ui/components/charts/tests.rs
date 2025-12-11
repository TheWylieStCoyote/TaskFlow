//! Tests for chart widgets.

use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

use super::{BarChart, BurndownChart, ProgressGauge, Sparkline, StatBox};

fn test_buffer(width: u16, height: u16) -> (Rect, Buffer) {
    let area = Rect::new(0, 0, width, height);
    let buffer = Buffer::empty(area);
    (area, buffer)
}

#[test]
fn test_bar_chart_creation() {
    let data = vec![("Item 1".to_string(), 10), ("Item 2".to_string(), 20)];
    let _chart = BarChart::new("Test", &data).bar_color(Color::Blue);
}

#[test]
fn test_bar_chart_render_empty() {
    let data: Vec<(String, u32)> = vec![];
    let chart = BarChart::new("Test", &data);
    let (area, mut buf) = test_buffer(50, 10);
    chart.render(area, &mut buf);
    // Should not panic
}

#[test]
fn test_bar_chart_render_with_data() {
    let data = vec![
        ("Monday".to_string(), 5),
        ("Tuesday".to_string(), 10),
        ("Wednesday".to_string(), 8),
    ];
    let chart = BarChart::new("Weekly Tasks", &data);
    let (area, mut buf) = test_buffer(60, 10);
    chart.render(area, &mut buf);
    // Should render without panic
}

#[test]
fn test_sparkline_creation() {
    let data = vec![1.0, 2.0, 3.0, 2.0, 1.0];
    let _spark = Sparkline::new("Test", &data).line_color(Color::Blue);
}

#[test]
fn test_sparkline_render_empty() {
    let data: Vec<f64> = vec![];
    let spark = Sparkline::new("Empty", &data);
    let (area, mut buf) = test_buffer(30, 3);
    spark.render(area, &mut buf);
}

#[test]
fn test_sparkline_render_with_data() {
    let data = vec![1.0, 3.0, 2.0, 4.0, 3.0, 5.0, 4.0, 6.0];
    let spark = Sparkline::new("Velocity", &data);
    let (area, mut buf) = test_buffer(30, 3);
    spark.render(area, &mut buf);
}

#[test]
fn test_burndown_chart_render() {
    let scope = vec![10.0, 10.0, 11.0, 11.0, 11.0];
    let completed = vec![0.0, 2.0, 4.0, 6.0, 8.0];
    let chart = BurndownChart::new("Sprint Burndown", &scope, &completed);
    let (area, mut buf) = test_buffer(40, 15);
    chart.render(area, &mut buf);
}

#[test]
fn test_progress_gauge_creation() {
    let _gauge = ProgressGauge::new("Progress", 0.75)
        .filled_color(Color::Blue)
        .empty_color(Color::DarkGray);
}

#[test]
fn test_progress_gauge_render() {
    let gauge = ProgressGauge::new("Done", 0.65);
    let (area, mut buf) = test_buffer(40, 1);
    gauge.render(area, &mut buf);
}

#[test]
fn test_progress_gauge_clamping() {
    // Test that values outside 0-1 are clamped
    let gauge_over = ProgressGauge::new("Over", 1.5);
    let (area, mut buf) = test_buffer(40, 1);
    gauge_over.render(area, &mut buf);

    let gauge_under = ProgressGauge::new("Under", -0.5);
    let (area, mut buf) = test_buffer(40, 1);
    gauge_under.render(area, &mut buf);
}

#[test]
fn test_stat_box_creation() {
    let _stat = StatBox::new("Tasks", "42").trend(5.0);
}

#[test]
fn test_stat_box_render() {
    let stat = StatBox::new("Completed", "156").trend(10.0);
    let (area, mut buf) = test_buffer(15, 4);
    stat.render(area, &mut buf);
}

#[test]
fn test_stat_box_negative_trend() {
    let stat = StatBox::new("Overdue", "3").trend(-2.0);
    let (area, mut buf) = test_buffer(15, 4);
    stat.render(area, &mut buf);
}
