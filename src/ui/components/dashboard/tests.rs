//! Dashboard component tests

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::app::Model;
use crate::config::Theme;
use crate::domain::{Task, TaskStatus};

use super::stats::{format_duration, DashboardStats};
use super::Dashboard;

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
fn test_dashboard_renders_completion_panel() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Completion"),
        "Completion panel should be visible"
    );
}

#[test]
fn test_dashboard_renders_time_tracking_panel() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Time Tracking"),
        "Time Tracking panel should be visible"
    );
}

#[test]
fn test_dashboard_renders_projects_panel() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Projects"),
        "Projects panel should be visible"
    );
}

#[test]
fn test_dashboard_renders_status_distribution_panel() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Status Distribution"),
        "Status Distribution panel should be visible"
    );
}

#[test]
fn test_dashboard_renders_this_week_panel() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("This Week"),
        "This Week panel should be visible"
    );
}

#[test]
fn test_dashboard_shows_overall_completion_rate() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Overall"),
        "Overall completion rate should be visible"
    );
}

#[test]
fn test_dashboard_shows_overdue_count() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Overdue"),
        "Overdue count should be visible"
    );
}

#[test]
fn test_dashboard_shows_tracking_status() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    // Should show Tracking: with either Active or Idle
    assert!(
        content.contains("Tracking"),
        "Tracking status should be visible"
    );
}

#[test]
fn test_dashboard_shows_status_types() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    // Status distribution should show task statuses
    assert!(
        content.contains("Todo") || content.contains("Done"),
        "Status types should be visible"
    );
}

#[test]
fn test_dashboard_shows_no_projects_when_empty() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("No projects"),
        "Should show 'No projects' when empty"
    );
}

#[test]
fn test_dashboard_with_sample_data() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);

    // Should render without panic
    let _ = buffer_content(&buffer);
}

#[test]
fn test_dashboard_completion_rate_calculation() {
    let mut model = Model::new();

    // Add 4 tasks, 2 done, 2 not done
    let task1 = Task::new("Task 1").with_status(TaskStatus::Done);
    let task2 = Task::new("Task 2").with_status(TaskStatus::Done);
    let task3 = Task::new("Task 3").with_status(TaskStatus::Todo);
    let task4 = Task::new("Task 4").with_status(TaskStatus::Todo);

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.tasks.insert(task3.id, task3);
    model.tasks.insert(task4.id, task4);

    let stats = DashboardStats::new(&model);

    // Completion rate should be 50%
    assert!((stats.completion_rate() - 50.0).abs() < 0.1);
}

#[test]
fn test_dashboard_completion_rate_empty() {
    let model = Model::new();
    let stats = DashboardStats::new(&model);

    // No tasks = 0% completion
    assert_eq!(stats.completion_rate(), 0.0);
}

#[test]
fn test_dashboard_status_counts() {
    let mut model = Model::new();

    let task1 = Task::new("Task 1").with_status(TaskStatus::Todo);
    let task2 = Task::new("Task 2").with_status(TaskStatus::InProgress);
    let task3 = Task::new("Task 3").with_status(TaskStatus::Done);

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.tasks.insert(task3.id, task3);

    let stats = DashboardStats::new(&model);

    let (todo, in_progress, blocked, done, cancelled) = stats.status_counts();
    assert_eq!(todo, 1);
    assert_eq!(in_progress, 1);
    assert_eq!(blocked, 0);
    assert_eq!(done, 1);
    assert_eq!(cancelled, 0);
}

#[test]
fn test_dashboard_format_duration() {
    assert_eq!(format_duration(30), "30m");
    assert_eq!(format_duration(60), "1h 0m");
    assert_eq!(format_duration(90), "1h 30m");
    assert_eq!(format_duration(125), "2h 5m");
}

#[test]
fn test_dashboard_renders_focus_sessions_panel() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Focus Sessions"),
        "Focus Sessions panel should be visible"
    );
}

#[test]
fn test_dashboard_shows_focus_stats() {
    let mut model = Model::new();
    // Record a pomodoro cycle
    model.pomodoro_stats.record_cycle(25);

    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    // Should show the stats
    assert!(
        content.contains("Today") || content.contains("Streak"),
        "Focus stats should be visible"
    );
}
