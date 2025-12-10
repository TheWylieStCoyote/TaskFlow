//! Tests for calendar view component.

use super::*;
use crate::app::Model;
use crate::config::Theme;
use crate::domain::Task;
use crate::ui::test_utils::{buffer_content, render_widget};
use chrono::Utc;

#[test]
fn test_calendar_renders_month_name() {
    let model = Model::new();
    let theme = Theme::default();
    let calendar = Calendar::new(&model, &theme);
    let buffer = render_widget(calendar, 60, 20);
    let content = buffer_content(&buffer);

    // Should show current month name
    let current_month = model.calendar_state.month;
    let month_name = match current_month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    };
    assert!(content.contains(month_name), "Month name should be visible");
}

#[test]
fn test_calendar_renders_year() {
    let model = Model::new();
    let theme = Theme::default();
    let calendar = Calendar::new(&model, &theme);
    let buffer = render_widget(calendar, 60, 20);
    let content = buffer_content(&buffer);

    let year = model.calendar_state.year.to_string();
    assert!(content.contains(&year), "Year should be visible");
}

#[test]
fn test_calendar_renders_day_headers() {
    let model = Model::new();
    let theme = Theme::default();
    let calendar = Calendar::new(&model, &theme);
    let buffer = render_widget(calendar, 60, 20);
    let content = buffer_content(&buffer);

    // Day headers: Mo Tu We Th Fr Sa Su
    assert!(content.contains("Mo"), "Monday header should be visible");
    assert!(content.contains("Tu"), "Tuesday header should be visible");
    assert!(content.contains("We"), "Wednesday header should be visible");
    assert!(content.contains("Th"), "Thursday header should be visible");
    assert!(content.contains("Fr"), "Friday header should be visible");
    assert!(content.contains("Sa"), "Saturday header should be visible");
    assert!(content.contains("Su"), "Sunday header should be visible");
}

#[test]
fn test_calendar_renders_day_numbers() {
    let model = Model::new();
    let theme = Theme::default();
    let calendar = Calendar::new(&model, &theme);
    let buffer = render_widget(calendar, 60, 20);
    let content = buffer_content(&buffer);

    // Should render day numbers
    assert!(content.contains(" 1"), "Day 1 should be visible");
    assert!(content.contains("15"), "Day 15 should be visible");
}

#[test]
fn test_calendar_renders_navigation_hint() {
    let model = Model::new();
    let theme = Theme::default();
    let calendar = Calendar::new(&model, &theme);
    let buffer = render_widget(calendar, 60, 20);
    let content = buffer_content(&buffer);

    // Navigation hints should be visible
    assert!(
        content.contains("day") || content.contains("week") || content.contains("month"),
        "Navigation hints should be visible"
    );
}

#[test]
fn test_calendar_renders_task_panel() {
    let model = Model::new();
    let theme = Theme::default();
    let calendar = Calendar::new(&model, &theme);
    let buffer = render_widget(calendar, 60, 20);
    let content = buffer_content(&buffer);

    // Task panel title "Tasks" or "Tasks for"
    assert!(content.contains("Tasks"), "Tasks panel should be visible");
}

#[test]
fn test_calendar_shows_no_tasks_message() {
    let model = Model::new();
    let theme = Theme::default();
    let calendar = Calendar::new(&model, &theme);
    let buffer = render_widget(calendar, 60, 20);
    let content = buffer_content(&buffer);

    // When no tasks, should show appropriate message
    assert!(
        content.contains("No tasks due") || content.contains("Select a day"),
        "Should show message when no tasks"
    );
}

#[test]
fn test_calendar_renders_tasks_for_selected_day() {
    use chrono::Datelike;

    let mut model = Model::new();
    let today = Utc::now().date_naive();

    // Add a task due today
    let task = Task::new("Today's Task").with_due_date(today);
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.calendar_state.selected_day = Some(today.day());
    model.calendar_state.year = today.year();
    model.calendar_state.month = today.month();
    model.current_view = crate::app::ViewId::Calendar;
    model.refresh_visible_tasks();

    let theme = Theme::default();
    let calendar = Calendar::new(&model, &theme);
    let buffer = render_widget(calendar, 80, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Today's Task"),
        "Task title should be visible in calendar task list"
    );
}

#[test]
fn test_calendar_with_sample_data() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let calendar = Calendar::new(&model, &theme);
    let buffer = render_widget(calendar, 80, 20);

    // Should render without panic
    let _ = buffer_content(&buffer);
}

#[test]
fn test_calendar_handles_small_area() {
    let model = Model::new();
    let theme = Theme::default();
    let calendar = Calendar::new(&model, &theme);
    // Very small area - should handle gracefully
    let buffer = render_widget(calendar, 20, 5);

    // Should render without panic
    let _ = buffer_content(&buffer);
}

#[test]
fn test_calendar_month_name_method() {
    let mut model = Model::new();
    let theme = Theme::default();

    // Test each month
    for month in 1..=12 {
        model.calendar_state.month = month;
        let calendar = Calendar::new(&model, &theme);
        let name = calendar.month_name();
        assert!(!name.is_empty(), "Month {month} should have a name");
    }
}

#[test]
fn test_calendar_days_in_month() {
    let mut model = Model::new();
    let theme = Theme::default();

    // Test typical months
    model.calendar_state.year = 2024;
    model.calendar_state.month = 1; // January
    let calendar = Calendar::new(&model, &theme);
    assert_eq!(calendar.days_in_month(), 31);

    model.calendar_state.month = 2; // February (leap year)
    let calendar = Calendar::new(&model, &theme);
    assert_eq!(calendar.days_in_month(), 29);

    model.calendar_state.month = 4; // April
    let calendar = Calendar::new(&model, &theme);
    assert_eq!(calendar.days_in_month(), 30);
}
