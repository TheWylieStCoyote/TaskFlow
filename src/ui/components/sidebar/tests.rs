//! Tests for sidebar navigation component.

use super::*;
use crate::ui::test_utils::{buffer_content, render_widget};

#[test]
fn test_sidebar_renders_navigation_title() {
    let model = Model::new();
    let theme = Theme::default();
    let sidebar = Sidebar::new(&model, &theme);
    let buffer = render_widget(sidebar, 30, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Navigation"),
        "Navigation title should be visible"
    );
}

#[test]
fn test_sidebar_renders_all_tasks_view() {
    let model = Model::new();
    let theme = Theme::default();
    let sidebar = Sidebar::new(&model, &theme);
    let buffer = render_widget(sidebar, 30, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("All Tasks"),
        "All Tasks view should be visible"
    );
}

#[test]
fn test_sidebar_renders_today_view() {
    let model = Model::new();
    let theme = Theme::default();
    let sidebar = Sidebar::new(&model, &theme);
    let buffer = render_widget(sidebar, 30, 20);
    let content = buffer_content(&buffer);

    assert!(content.contains("Today"), "Today view should be visible");
}

#[test]
fn test_sidebar_renders_upcoming_view() {
    let model = Model::new();
    let theme = Theme::default();
    let sidebar = Sidebar::new(&model, &theme);
    let buffer = render_widget(sidebar, 30, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Upcoming"),
        "Upcoming view should be visible"
    );
}

#[test]
fn test_sidebar_renders_overdue_view() {
    let model = Model::new();
    let theme = Theme::default();
    let sidebar = Sidebar::new(&model, &theme);
    let buffer = render_widget(sidebar, 30, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Overdue"),
        "Overdue view should be visible"
    );
}

#[test]
fn test_sidebar_renders_calendar_view() {
    let model = Model::new();
    let theme = Theme::default();
    let sidebar = Sidebar::new(&model, &theme);
    let buffer = render_widget(sidebar, 30, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Calendar"),
        "Calendar view should be visible"
    );
}

#[test]
fn test_sidebar_renders_dashboard_view() {
    let model = Model::new();
    let theme = Theme::default();
    let sidebar = Sidebar::new(&model, &theme);
    let buffer = render_widget(sidebar, 30, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Dashboard"),
        "Dashboard view should be visible"
    );
}

#[test]
fn test_sidebar_renders_projects_section() {
    let model = Model::new();
    let theme = Theme::default();
    let sidebar = Sidebar::new(&model, &theme);
    // Height 30 to accommodate all views including Heatmap, Forecast, Network, Burndown
    let buffer = render_widget(sidebar, 30, 30);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Projects"),
        "Projects section should be visible"
    );
}

#[test]
fn test_sidebar_shows_no_projects_when_empty() {
    let model = Model::new();
    let theme = Theme::default();
    let sidebar = Sidebar::new(&model, &theme);
    // Height 30 to accommodate all views including analytics views
    let buffer = render_widget(sidebar, 30, 30);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("No projects"),
        "Should show 'No projects' when empty"
    );
}

#[test]
fn test_sidebar_renders_projects_with_task_counts() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let sidebar = Sidebar::new(&model, &theme);
    // Height 50 to accommodate all views (20+) plus 10 projects
    let buffer = render_widget(sidebar, 30, 50);
    let content = buffer_content(&buffer);

    // Sample data has 10 projects; at least one should be visible
    assert!(
        content.contains("Backend")
            || content.contains("Frontend")
            || content.contains("Doc")
            || content.contains("DevOps")
            || content.contains("Mobile")
            || content.contains("Personal"),
        "Project names should be visible"
    );
}

#[test]
fn test_sidebar_uses_focused_border_when_focused() {
    let mut model = Model::new();
    model.focus_pane = FocusPane::Sidebar;
    let theme = Theme::default();
    let sidebar = Sidebar::new(&model, &theme);

    // Just ensure it renders without panic when focused
    let buffer = render_widget(sidebar, 30, 20);
    let _ = buffer_content(&buffer);
}

#[test]
fn test_sidebar_renders_separator() {
    let model = Model::new();
    let theme = Theme::default();
    let sidebar = Sidebar::new(&model, &theme);
    let buffer = render_widget(sidebar, 30, 20);
    let content = buffer_content(&buffer);

    // There should be a separator line between views and projects
    assert!(content.contains('─'), "Separator should be visible");
}
