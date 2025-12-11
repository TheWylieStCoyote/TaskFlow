//! Tests for focus view component.

use std::time::Duration;

use super::*;
use crate::app::Model;
use crate::config::Theme;
use crate::ui::test_utils::{buffer_content, render_widget};

#[test]
fn test_focus_view_renders_focus_mode_title() {
    let mut model = Model::new().with_sample_data();
    model.refresh_visible_tasks();
    let theme = Theme::default();
    let focus_view = FocusView::new(&model, &theme);
    let buffer = render_widget(focus_view, 60, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("FOCUS MODE"),
        "Focus mode title should be visible"
    );
}

#[test]
fn test_focus_view_shows_task_title() {
    let mut model = Model::new().with_sample_data();
    model.refresh_visible_tasks();
    let theme = Theme::default();
    let focus_view = FocusView::new(&model, &theme);
    let buffer = render_widget(focus_view, 80, 20);
    let content = buffer_content(&buffer);

    // Should show a task checkbox
    assert!(
        content.contains("[ ]") || content.contains("[x]"),
        "Task status indicator should be visible"
    );
}

#[test]
fn test_focus_view_shows_timer() {
    let mut model = Model::new().with_sample_data();
    model.refresh_visible_tasks();
    let theme = Theme::default();
    let focus_view = FocusView::new(&model, &theme);
    let buffer = render_widget(focus_view, 60, 20);
    let content = buffer_content(&buffer);

    // Timer shows time in format like 00:00:00
    assert!(
        content.contains(':'),
        "Timer should show colon-separated time"
    );
}

#[test]
fn test_focus_view_shows_help_text() {
    let mut model = Model::new().with_sample_data();
    model.refresh_visible_tasks();
    let theme = Theme::default();
    let focus_view = FocusView::new(&model, &theme);
    let buffer = render_widget(focus_view, 80, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Exit") || content.contains("Esc"),
        "Help text should mention exiting focus mode"
    );
}

#[test]
fn test_focus_view_no_task_selected() {
    let model = Model::new(); // No tasks, nothing selected
    let theme = Theme::default();
    let focus_view = FocusView::new(&model, &theme);
    let buffer = render_widget(focus_view, 60, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("No task selected"),
        "Should show message when no task selected"
    );
}

#[test]
fn test_format_duration() {
    assert_eq!(
        FocusView::format_duration(Duration::from_secs(0)),
        "00:00:00"
    );
    assert_eq!(
        FocusView::format_duration(Duration::from_secs(59)),
        "00:00:59"
    );
    assert_eq!(
        FocusView::format_duration(Duration::from_secs(60)),
        "00:01:00"
    );
    assert_eq!(
        FocusView::format_duration(Duration::from_secs(3661)),
        "01:01:01"
    );
    assert_eq!(
        FocusView::format_duration(Duration::from_secs(7200)),
        "02:00:00"
    );
}

#[test]
fn test_focus_view_with_high_priority_task() {
    use crate::domain::{Priority, Task};

    let mut model = Model::new();
    let theme = Theme::default();

    let mut task = Task::new("High priority task");
    task.priority = Priority::High;
    let task_id = task.id;
    model.tasks.insert(task.id, task);
    model.visible_tasks = vec![task_id];
    model.selected_index = 0;

    let focus_view = FocusView::new(&model, &theme);
    let buffer = render_widget(focus_view, 80, 24);
    let content = buffer_content(&buffer);

    assert!(content.contains("High priority task"));
    assert!(content.contains("High")); // Priority label
}

#[test]
fn test_focus_view_with_due_date() {
    use crate::domain::Task;
    use chrono::{Duration as ChronoDuration, Utc};

    let mut model = Model::new();
    let theme = Theme::default();

    let mut task = Task::new("Task with due date");
    task.due_date = Some(Utc::now().date_naive() + ChronoDuration::days(5));
    let task_id = task.id;
    model.tasks.insert(task.id, task);
    model.visible_tasks = vec![task_id];
    model.selected_index = 0;

    let focus_view = FocusView::new(&model, &theme);
    let buffer = render_widget(focus_view, 80, 24);
    let content = buffer_content(&buffer);

    assert!(content.contains("Due:"));
}

#[test]
fn test_focus_view_with_scheduled_date() {
    use crate::domain::Task;
    use chrono::Utc;

    let mut model = Model::new();
    let theme = Theme::default();

    let mut task = Task::new("Task with scheduled date");
    task.scheduled_date = Some(Utc::now().date_naive());
    let task_id = task.id;
    model.tasks.insert(task.id, task);
    model.visible_tasks = vec![task_id];
    model.selected_index = 0;

    let focus_view = FocusView::new(&model, &theme);
    let buffer = render_widget(focus_view, 80, 24);
    let content = buffer_content(&buffer);

    assert!(content.contains("Scheduled:"));
}

#[test]
fn test_focus_view_with_description() {
    use crate::domain::Task;

    let mut model = Model::new();
    let theme = Theme::default();

    let mut task = Task::new("Task with description");
    task.description = Some("This is a detailed description of the task.".to_string());
    let task_id = task.id;
    model.tasks.insert(task.id, task);
    model.visible_tasks = vec![task_id];
    model.selected_index = 0;

    let focus_view = FocusView::new(&model, &theme);
    let buffer = render_widget(focus_view, 80, 24);
    let content = buffer_content(&buffer);

    assert!(content.contains("detailed description"));
}

#[test]
fn test_focus_view_completed_task() {
    use crate::domain::{Task, TaskStatus};

    let mut model = Model::new();
    let theme = Theme::default();

    let task = Task::new("Completed task").with_status(TaskStatus::Done);
    let task_id = task.id;
    model.tasks.insert(task.id, task);
    model.visible_tasks = vec![task_id];
    model.selected_index = 0;

    let focus_view = FocusView::new(&model, &theme);
    let buffer = render_widget(focus_view, 80, 24);
    let content = buffer_content(&buffer);

    assert!(content.contains("[x]")); // Completed checkbox
}

#[test]
fn test_focus_view_with_chain() {
    use crate::domain::Task;

    let mut model = Model::new();
    let theme = Theme::default();

    // Create a chain of tasks
    let mut task1 = Task::new("First in chain");
    let task2 = Task::new("Second in chain");

    task1.next_task_id = Some(task2.id);
    let task2_id = task2.id;

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.visible_tasks = vec![task2_id];
    model.selected_index = 0;

    let focus_view = FocusView::new(&model, &theme);
    let buffer = render_widget(focus_view, 80, 24);
    let content = buffer_content(&buffer);

    // Should show chain info - second task is pointed to by first
    assert!(content.contains("Chain") || content.contains("CURRENT"));
}

#[test]
fn test_focus_view_with_next_in_chain() {
    use crate::domain::Task;

    let mut model = Model::new();
    let theme = Theme::default();

    // Create a chain of tasks
    let mut task1 = Task::new("First in chain");
    let task2 = Task::new("Second in chain");

    task1.next_task_id = Some(task2.id);
    let task1_id = task1.id;

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.visible_tasks = vec![task1_id];
    model.selected_index = 0;

    let focus_view = FocusView::new(&model, &theme);
    let buffer = render_widget(focus_view, 80, 24);
    let content = buffer_content(&buffer);

    // Should show chain info - first task has next
    assert!(content.contains("Chain") || content.contains("CURRENT") || content.contains("Second"));
}

#[test]
fn test_focus_view_timer_start_stop_hint() {
    use crate::domain::Task;

    let mut model = Model::new();
    let theme = Theme::default();

    let task = Task::new("Task for timer test");
    let task_id = task.id;
    model.tasks.insert(task.id, task);
    model.visible_tasks = vec![task_id];
    model.selected_index = 0;

    // When not tracking
    let focus_view = FocusView::new(&model, &theme);
    let buffer = render_widget(focus_view, 80, 24);
    let content = buffer_content(&buffer);
    assert!(content.contains("Start Timer") || content.contains("[t]"));
}

#[test]
fn test_focus_view_with_active_timer() {
    use crate::domain::{Task, TimeEntry};

    let mut model = Model::new();
    let theme = Theme::default();

    let task = Task::new("Task being tracked");
    let task_id = task.id;
    model.tasks.insert(task.id, task);
    model.visible_tasks = vec![task_id];
    model.selected_index = 0;

    // Start tracking
    let entry = TimeEntry::start(task_id);
    let entry_id = entry.id;
    model.time_entries.insert(entry.id, entry);
    model.active_time_entry = Some(entry_id);

    let focus_view = FocusView::new(&model, &theme);
    let buffer = render_widget(focus_view, 80, 24);
    let content = buffer_content(&buffer);

    // Should show timer and stop hint
    assert!(content.contains("Stop Timer") || content.contains("[t]"));
}

#[test]
fn test_focus_view_time_tracked_from_entries() {
    use crate::domain::{Task, TimeEntry};
    use chrono::{Duration as ChronoDuration, Utc};

    let mut model = Model::new();
    let theme = Theme::default();

    let task = Task::new("Task with time entries");
    let task_id = task.id;
    model.tasks.insert(task.id, task);
    model.visible_tasks = vec![task_id];
    model.selected_index = 0;

    // Add a completed time entry (30 minutes)
    let mut entry = TimeEntry::start(task_id);
    entry.started_at = Utc::now() - ChronoDuration::minutes(30);
    entry.stop();
    model.time_entries.insert(entry.id, entry);

    let focus_view = FocusView::new(&model, &theme);
    let buffer = render_widget(focus_view, 80, 24);

    // Should render with time tracked (contains colon from time format)
    let content = buffer_content(&buffer);
    assert!(content.contains(':'));
}
