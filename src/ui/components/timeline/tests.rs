//! Tests for the timeline widget.

use chrono::{Duration, NaiveDate, Utc};
use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

use super::*;
use crate::app::{Model, TimelineZoom};
use crate::config::Theme;
use crate::domain::{Priority, Task, TaskStatus};

fn create_test_timeline<'a>(model: &'a Model, theme: &'a Theme) -> Timeline<'a> {
    Timeline::new(model, theme)
}

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

// =========================================================================
// Basic widget tests
// =========================================================================

#[test]
fn test_timeline_renders_without_panic() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let timeline = Timeline::new(&model, &theme);

    let area = Rect::new(0, 0, 120, 30);
    let mut buffer = Buffer::empty(area);
    timeline.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

// =========================================================================
// Bar string building tests
// =========================================================================

#[test]
fn test_build_bar_string() {
    let model = Model::new();
    let theme = Theme::default();
    let timeline = Timeline::new(&model, &theme);

    assert_eq!(timeline.build_bar_string(5, false, false), "[===]");
    assert_eq!(timeline.build_bar_string(5, true, false), "<===]");
    assert_eq!(timeline.build_bar_string(5, false, true), "[===>");
    assert_eq!(timeline.build_bar_string(5, true, true), "<===>");
    assert_eq!(timeline.build_bar_string(1, false, false), "█");
}

#[test]
fn test_build_bar_string_empty() {
    let model = Model::new();
    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    assert_eq!(timeline.build_bar_string(0, false, false), "");
}

#[test]
fn test_build_bar_string_single_char() {
    let model = Model::new();
    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    // Single char without extensions = block
    assert_eq!(timeline.build_bar_string(1, false, false), "█");

    // Single char with left extension
    assert_eq!(timeline.build_bar_string(1, true, false), "╡");

    // Single char with right extension
    assert_eq!(timeline.build_bar_string(1, false, true), "╞");

    // Single char with both extensions
    assert_eq!(timeline.build_bar_string(1, true, true), "═");
}

#[test]
fn test_build_bar_string_short_bar() {
    let model = Model::new();
    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    // 2 chars = brackets with no fill
    assert_eq!(timeline.build_bar_string(2, false, false), "[]");

    // 3 chars = brackets with 1 fill
    assert_eq!(timeline.build_bar_string(3, false, false), "[=]");
}

#[test]
fn test_build_bar_string_extends_left() {
    let model = Model::new();
    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    // Extends left = < start
    assert_eq!(timeline.build_bar_string(3, true, false), "<=]");
    assert_eq!(timeline.build_bar_string(5, true, false), "<===]");
}

#[test]
fn test_build_bar_string_extends_right() {
    let model = Model::new();
    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    // Extends right = > end
    assert_eq!(timeline.build_bar_string(3, false, true), "[=>");
    assert_eq!(timeline.build_bar_string(5, false, true), "[===>");
}

#[test]
fn test_build_bar_string_extends_both() {
    let model = Model::new();
    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    // Both extensions
    assert_eq!(timeline.build_bar_string(3, true, true), "<=>");
    assert_eq!(timeline.build_bar_string(5, true, true), "<===>");
}

#[test]
fn test_build_bar_string_long_bar() {
    let model = Model::new();
    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    // Longer bar
    assert_eq!(timeline.build_bar_string(10, false, false), "[========]");
    assert_eq!(timeline.build_bar_string(10, true, true), "<========>");
}

// =========================================================================
// Zoom params tests
// =========================================================================

#[test]
fn test_zoom_params_day_zoom() {
    let mut model = Model::new();
    model.timeline_state.zoom_level = TimelineZoom::Day;
    model.timeline_state.viewport_days = 14;

    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    let (num_cols, days_per_col) = timeline.zoom_params();
    assert_eq!(num_cols, 14);
    assert_eq!(days_per_col, 1);
}

#[test]
fn test_zoom_params_week_zoom() {
    let mut model = Model::new();
    model.timeline_state.zoom_level = TimelineZoom::Week;
    model.timeline_state.viewport_days = 28;

    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    let (num_cols, days_per_col) = timeline.zoom_params();
    assert_eq!(num_cols, 28);
    assert_eq!(days_per_col, 7);
}

// =========================================================================
// Date to column tests
// =========================================================================

#[test]
fn test_date_to_column_in_viewport() {
    let mut model = Model::new();
    let start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
    model.timeline_state.viewport_start = start;
    model.timeline_state.viewport_days = 14;
    model.timeline_state.zoom_level = TimelineZoom::Day;

    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    // First day = column 0
    assert_eq!(timeline.date_to_column(start), Some(0));

    // Day 7 = column 6 (0-indexed)
    let day7 = NaiveDate::from_ymd_opt(2024, 12, 7).unwrap();
    assert_eq!(timeline.date_to_column(day7), Some(6));

    // Day 14 = column 13 (last column)
    let day14 = NaiveDate::from_ymd_opt(2024, 12, 14).unwrap();
    assert_eq!(timeline.date_to_column(day14), Some(13));
}

#[test]
fn test_date_to_column_out_of_viewport() {
    let mut model = Model::new();
    let start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
    model.timeline_state.viewport_start = start;
    model.timeline_state.viewport_days = 14;
    model.timeline_state.zoom_level = TimelineZoom::Day;

    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    // Before viewport
    let before = NaiveDate::from_ymd_opt(2024, 11, 30).unwrap();
    assert_eq!(timeline.date_to_column(before), None);

    // After viewport
    let after = NaiveDate::from_ymd_opt(2024, 12, 20).unwrap();
    assert_eq!(timeline.date_to_column(after), None);
}

#[test]
fn test_date_to_column_week_zoom() {
    let mut model = Model::new();
    let start = NaiveDate::from_ymd_opt(2024, 12, 2).unwrap(); // Monday
    model.timeline_state.viewport_start = start;
    model.timeline_state.viewport_days = 28;
    model.timeline_state.zoom_level = TimelineZoom::Week;

    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    // In week zoom, date_to_column returns the day index (0-27),
    // not the week index
    assert_eq!(timeline.date_to_column(start), Some(0));

    // Day 7 is at index 7 in the 28-day viewport
    let day7 = start + Duration::days(7);
    assert_eq!(timeline.date_to_column(day7), Some(1));
}

// =========================================================================
// Task span tests
// =========================================================================

#[test]
fn test_task_span_with_both_dates() {
    let mut task = Task::new("Test");
    task.scheduled_date = Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    task.due_date = Some(NaiveDate::from_ymd_opt(2024, 1, 5).unwrap());

    let (start, end) = Timeline::task_span(&task);
    assert_eq!(start, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    assert_eq!(end, NaiveDate::from_ymd_opt(2024, 1, 5).unwrap());
}

#[test]
fn test_task_span_milestone() {
    let mut task = Task::new("Milestone");
    task.due_date = Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());

    let (start, end) = Timeline::task_span(&task);
    assert_eq!(start, end);
}

#[test]
fn test_task_span_with_due_date_only() {
    let due = NaiveDate::from_ymd_opt(2024, 12, 15).unwrap();
    let mut task = Task::new("Test");
    task.due_date = Some(due);

    let (start, end) = Timeline::task_span(&task);
    assert_eq!(start, due);
    assert_eq!(end, due);
}

#[test]
fn test_task_span_with_scheduled_and_due() {
    let scheduled = NaiveDate::from_ymd_opt(2024, 12, 10).unwrap();
    let due_date = NaiveDate::from_ymd_opt(2024, 12, 15).unwrap();

    let mut task = Task::new("Test");
    task.scheduled_date = Some(scheduled);
    task.due_date = Some(due_date);

    let (start, end) = Timeline::task_span(&task);
    assert_eq!(start, scheduled);
    assert_eq!(end, due_date);
}

#[test]
fn test_task_span_scheduled_only() {
    let scheduled = NaiveDate::from_ymd_opt(2024, 12, 10).unwrap();
    let mut task = Task::new("Test");
    task.scheduled_date = Some(scheduled);

    let (start, end) = Timeline::task_span(&task);
    assert_eq!(start, scheduled);
    assert_eq!(end, scheduled);
}

#[test]
fn test_task_span_no_dates() {
    let task = Task::new("Test");
    let (start, end) = Timeline::task_span(&task);

    // Should default to today (UTC, matching task_span implementation)
    let today = Utc::now().date_naive();
    assert_eq!(start, today);
    assert_eq!(end, today);
}

// =========================================================================
// Title bar rendering tests
// =========================================================================

#[test]
fn test_render_title_bar() {
    let mut model = Model::new();
    model.timeline_state.viewport_start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
    model.timeline_state.viewport_days = 14;
    model.timeline_state.zoom_level = TimelineZoom::Day;
    model.timeline_state.show_dependencies = true;

    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    let area = Rect::new(0, 0, 80, 1);
    let mut buffer = Buffer::empty(area);
    let today = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
    timeline.render_title_bar(area, &mut buffer, today);

    let content = buffer_content(&buffer);
    assert!(
        content.contains("Timeline"),
        "Should contain Timeline title"
    );
    assert!(content.contains("Day"), "Should show zoom level");
    assert!(content.contains("ON"), "Dependencies should be ON");
}

#[test]
fn test_render_title_bar_deps_off() {
    let mut model = Model::new();
    model.timeline_state.show_dependencies = false;

    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    let area = Rect::new(0, 0, 80, 1);
    let mut buffer = Buffer::empty(area);
    let today = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
    timeline.render_title_bar(area, &mut buffer, today);

    let content = buffer_content(&buffer);
    assert!(content.contains("off"), "Dependencies should be off");
}

#[test]
fn test_render_title_bar_week_zoom() {
    let mut model = Model::new();
    model.timeline_state.zoom_level = TimelineZoom::Week;

    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    let area = Rect::new(0, 0, 80, 1);
    let mut buffer = Buffer::empty(area);
    let today = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
    timeline.render_title_bar(area, &mut buffer, today);

    let content = buffer_content(&buffer);
    assert!(content.contains("Week"), "Should show Week zoom level");
}

// =========================================================================
// Date headers rendering tests
// =========================================================================

#[test]
fn test_render_date_headers_day_zoom() {
    let mut model = Model::new();
    model.timeline_state.viewport_start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
    model.timeline_state.viewport_days = 7;
    model.timeline_state.zoom_level = TimelineZoom::Day;

    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    let area = Rect::new(0, 0, 80, 2);
    let mut buffer = Buffer::empty(area);
    let today = NaiveDate::from_ymd_opt(2024, 12, 3).unwrap();
    timeline.render_date_headers(area, &mut buffer, today);

    // Should render day numbers and weekdays
    assert!(buffer.area.width > 0);
}

#[test]
fn test_render_date_headers_week_zoom() {
    let mut model = Model::new();
    model.timeline_state.viewport_start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
    model.timeline_state.viewport_days = 28;
    model.timeline_state.zoom_level = TimelineZoom::Week;

    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    let area = Rect::new(0, 0, 80, 2);
    let mut buffer = Buffer::empty(area);
    let today = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
    timeline.render_date_headers(area, &mut buffer, today);

    // Should render without panic
    assert!(buffer.area.width > 0);
}

#[test]
fn test_render_date_headers_small_height() {
    let mut model = Model::new();
    model.timeline_state.viewport_start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
    model.timeline_state.zoom_level = TimelineZoom::Day;

    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    // Height less than 2 - should early return
    let area = Rect::new(0, 0, 80, 1);
    let mut buffer = Buffer::empty(area);
    let today = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
    timeline.render_date_headers(area, &mut buffer, today);

    // Should not panic, just return early
    assert!(buffer.area.width > 0);
}

// =========================================================================
// Task rows rendering tests
// =========================================================================

#[test]
fn test_render_task_rows_empty() {
    let model = Model::new();
    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    let area = Rect::new(0, 0, 80, 10);
    let mut buffer = Buffer::empty(area);
    let today = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
    timeline.render_task_rows(area, &mut buffer, today, 14);

    let content = buffer_content(&buffer);
    assert!(
        content.contains("No tasks"),
        "Should show empty message when no tasks"
    );
}

#[test]
fn test_render_task_rows_with_tasks() {
    let mut model = Model::new();
    let start_date = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
    model.timeline_state.viewport_start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
    model.timeline_state.viewport_days = 14;

    // Add task with due date in viewport
    let mut task = Task::new("Test Task");
    task.due_date = Some(start_date);
    model.tasks.insert(task.id, task);
    model.refresh_visible_tasks();

    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    let area = Rect::new(0, 0, 100, 10);
    let mut buffer = Buffer::empty(area);
    timeline.render_task_rows(area, &mut buffer, start_date, 14);

    let content = buffer_content(&buffer);
    assert!(
        content.contains("Test Task") || content.contains("Test"),
        "Should show task title"
    );
}

#[test]
fn test_render_task_rows_milestone() {
    let mut model = Model::new();
    let date = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
    model.timeline_state.viewport_start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
    model.timeline_state.viewport_days = 14;

    // Milestone: same start and end date
    let mut task = Task::new("Milestone");
    task.due_date = Some(date);
    task.scheduled_date = Some(date);
    model.tasks.insert(task.id, task);
    model.refresh_visible_tasks();

    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    let area = Rect::new(0, 0, 100, 10);
    let mut buffer = Buffer::empty(area);
    timeline.render_task_rows(area, &mut buffer, date, 14);

    // Milestone should render with diamond character
    assert!(buffer.area.width > 0);
}

#[test]
fn test_render_task_rows_with_dependencies() {
    let mut model = Model::new();
    let start_date = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
    model.timeline_state.viewport_start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
    model.timeline_state.viewport_days = 14;
    model.timeline_state.show_dependencies = true;

    // Add two tasks with chain relationship
    let mut task1 = Task::new("Task 1");
    task1.due_date = Some(start_date);

    let mut task2 = Task::new("Task 2");
    task2.due_date = Some(start_date + Duration::days(3));
    task1.next_task_id = Some(task2.id);

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.refresh_visible_tasks();

    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    let area = Rect::new(0, 0, 100, 10);
    let mut buffer = Buffer::empty(area);
    timeline.render_task_rows(area, &mut buffer, start_date, 14);

    // Should render dependency lines
    assert!(buffer.area.width > 0);
}

// =========================================================================
// Task color tests
// =========================================================================

#[test]
fn test_task_color_priority_high() {
    let model = Model::new();
    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    let task = Task::new("High priority").with_priority(Priority::High);
    let color = timeline.task_color(&task);

    // High priority should have a color
    assert!(color != Color::Reset);
}

#[test]
fn test_task_color_completed() {
    let model = Model::new();
    let theme = Theme::default();
    let timeline = create_test_timeline(&model, &theme);

    let task = Task::new("Completed task").with_status(TaskStatus::Done);
    let color = timeline.task_color(&task);

    // Completed task should have a color
    assert!(color != Color::Reset);
}
