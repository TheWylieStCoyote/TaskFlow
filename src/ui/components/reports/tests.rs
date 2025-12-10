//! Reports component tests

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use super::*;
use crate::app::Model;
use crate::config::Theme;

/// Helper to render a widget into a buffer
fn render_widget<W: Widget>(widget: W, width: u16, height: u16) -> Buffer {
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    widget.render(area, &mut buffer);
    buffer
}

/// Extract text content from buffer
fn buffer_content(buffer: &Buffer) -> String {
    let mut content = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            content.push(
                buffer
                    .cell((x, y))
                    .map_or(' ', |c| c.symbol().chars().next().unwrap_or(' ')),
            );
        }
        content.push('\n');
    }
    content
}

#[test]
fn test_report_panel_navigation() {
    let panel = ReportPanel::Overview;
    assert_eq!(panel.next(), ReportPanel::Velocity);
    assert_eq!(panel.prev(), ReportPanel::Estimation);
}

#[test]
fn test_report_panel_cycle() {
    let mut panel = ReportPanel::Overview;
    for _ in 0..7 {
        panel = panel.next();
    }
    assert_eq!(panel, ReportPanel::Overview);
}

#[test]
fn test_report_panel_index() {
    assert_eq!(ReportPanel::Overview.index(), 0);
    assert_eq!(ReportPanel::Velocity.index(), 1);
    assert_eq!(ReportPanel::Tags.index(), 2);
    assert_eq!(ReportPanel::Time.index(), 3);
    assert_eq!(ReportPanel::Focus.index(), 4);
    assert_eq!(ReportPanel::Insights.index(), 5);
    assert_eq!(ReportPanel::Estimation.index(), 6);
}

#[test]
fn test_report_panel_names() {
    let names = ReportPanel::names();
    assert_eq!(names.len(), 7);
    assert_eq!(names[0], "Overview");
    assert_eq!(names[4], "Focus");
    assert_eq!(names[6], "Estimation");
}

#[test]
fn test_format_duration_minutes_only() {
    assert_eq!(format_duration(45), "45m");
    assert_eq!(format_duration(0), "0m");
}

#[test]
fn test_format_duration_hours_minutes() {
    assert_eq!(format_duration(90), "1h 30m");
    assert_eq!(format_duration(120), "2h 0m");
    assert_eq!(format_duration(135), "2h 15m");
}

// ==================== Rendering Tests ====================

#[test]
fn test_reports_view_renders_title() {
    let model = Model::new();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Overview, &theme);
    let buffer = render_widget(view, 80, 30);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Reports"),
        "Reports title should be visible"
    );
}

#[test]
fn test_reports_view_renders_tabs() {
    let model = Model::new();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Overview, &theme);
    let buffer = render_widget(view, 100, 30);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Overview"),
        "Overview tab should be visible"
    );
    assert!(
        content.contains("Velocity"),
        "Velocity tab should be visible"
    );
    assert!(content.contains("Tags"), "Tags tab should be visible");
}

#[test]
fn test_reports_view_overview_panel() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Overview, &theme);
    let buffer = render_widget(view, 100, 40);
    let content = buffer_content(&buffer);

    // Overview shows stat boxes
    assert!(
        content.contains("Total") || content.contains("Done"),
        "Overview should show statistics"
    );
}

#[test]
fn test_reports_view_velocity_panel() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Velocity, &theme);
    let buffer = render_widget(view, 100, 40);
    let content = buffer_content(&buffer);

    // Velocity panel renders
    assert!(
        content.contains("Velocity") || content.contains("Tasks"),
        "Velocity panel should render"
    );
}

#[test]
fn test_reports_view_tags_panel() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Tags, &theme);
    let buffer = render_widget(view, 100, 40);
    let content = buffer_content(&buffer);

    // Tags panel shows tag statistics
    assert!(
        content.contains("Tag") || content.contains("unique"),
        "Tags panel should render"
    );
}

#[test]
fn test_reports_view_time_panel() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Time, &theme);
    let buffer = render_widget(view, 100, 40);
    let content = buffer_content(&buffer);

    // Time panel renders
    assert!(
        content.contains("Time") || content.contains("Hours"),
        "Time panel should render"
    );
}

#[test]
fn test_reports_view_focus_panel() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Focus, &theme);
    let buffer = render_widget(view, 100, 40);
    let content = buffer_content(&buffer);

    // Focus panel renders (may show various messages)
    assert!(
        content.contains("Focus") || content.contains("task") || content.contains("Tasks"),
        "Focus panel should render"
    );
}

#[test]
fn test_reports_view_insights_panel() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Insights, &theme);
    let buffer = render_widget(view, 100, 40);
    let content = buffer_content(&buffer);

    // Insights panel renders
    assert!(
        content.contains("Insight") || content.contains("Task"),
        "Insights panel should render"
    );
}

#[test]
fn test_reports_view_estimation_panel() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Estimation, &theme);
    let buffer = render_widget(view, 100, 40);
    let content = buffer_content(&buffer);

    // Estimation panel renders
    assert!(
        content.contains("Estimation") || content.contains("Accuracy") || content.contains("Task"),
        "Estimation panel should render"
    );
}

#[test]
fn test_reports_view_empty_model() {
    let model = Model::new();
    let theme = Theme::default();

    // Test all panels with empty model - should not panic
    for panel in [
        ReportPanel::Overview,
        ReportPanel::Velocity,
        ReportPanel::Tags,
        ReportPanel::Time,
        ReportPanel::Focus,
        ReportPanel::Insights,
        ReportPanel::Estimation,
    ] {
        let view = ReportsView::new(&model, panel, &theme);
        let _ = render_widget(view, 80, 30);
    }
}

#[test]
fn test_reports_view_small_area() {
    let model = Model::new();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Overview, &theme);

    // Very small area should not panic
    let _ = render_widget(view, 10, 5);
}
