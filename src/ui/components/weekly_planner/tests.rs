//! Tests for weekly planner component.

use super::*;
use crate::domain::{Priority, Task};
use chrono::Weekday;

#[test]
fn test_weekly_planner_renders_without_panic() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let planner = WeeklyPlanner::new(&model, &theme);

    let area = Rect::new(0, 0, 140, 30);
    let mut buffer = Buffer::empty(area);
    planner.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_week_start_is_monday() {
    let week_start = WeeklyPlanner::week_start();
    assert_eq!(week_start.weekday(), Weekday::Mon);
}

#[test]
fn test_weekly_planner_empty_model() {
    let model = Model::new();
    let theme = Theme::default();
    let planner = WeeklyPlanner::new(&model, &theme);

    let area = Rect::new(0, 0, 140, 30);
    let mut buffer = Buffer::empty(area);
    planner.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_weekly_planner_tasks_for_today() {
    let mut model = Model::new();
    let today = Utc::now().date_naive();

    // Create task due today
    let mut task = Task::new("Due Today");
    task.due_date = Some(today);
    model.tasks.insert(task.id, task);
    model.refresh_visible_tasks();

    let theme = Theme::default();
    let planner = WeeklyPlanner::new(&model, &theme);

    let tasks = planner.tasks_for_date(today);
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "Due Today");
}

#[test]
fn test_weekly_planner_scheduled_tasks() {
    let mut model = Model::new();
    let today = Utc::now().date_naive();

    // Create task scheduled for today (not due)
    let mut task = Task::new("Scheduled Today");
    task.scheduled_date = Some(today);
    model.tasks.insert(task.id, task);
    model.refresh_visible_tasks();

    let theme = Theme::default();
    let planner = WeeklyPlanner::new(&model, &theme);

    let tasks = planner.tasks_for_date(today);
    assert_eq!(tasks.len(), 1);
}

#[test]
fn test_weekly_planner_no_tasks_other_day() {
    let mut model = Model::new();
    let today = Utc::now().date_naive();
    let next_week = today + chrono::Duration::days(10);

    // Create task for next week (outside this week's view)
    let mut task = Task::new("Next Week");
    task.due_date = Some(next_week);
    model.tasks.insert(task.id, task);
    model.refresh_visible_tasks();

    let theme = Theme::default();
    let planner = WeeklyPlanner::new(&model, &theme);

    // No tasks for today
    let tasks = planner.tasks_for_date(today);
    assert!(tasks.is_empty());
}

#[test]
fn test_weekly_planner_completed_task() {
    let mut model = Model::new();
    let today = Utc::now().date_naive();

    let mut task = Task::new("Done Task");
    task.due_date = Some(today);
    task.toggle_complete();
    model.tasks.insert(task.id, task);
    model.refresh_visible_tasks();

    let theme = Theme::default();
    let planner = WeeklyPlanner::new(&model, &theme);

    let area = Rect::new(0, 0, 140, 30);
    let mut buffer = Buffer::empty(area);
    planner.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_weekly_planner_high_priority_task() {
    let mut model = Model::new();
    let today = Utc::now().date_naive();

    let mut task = Task::new("Urgent Task");
    task.due_date = Some(today);
    task.priority = Priority::Urgent;
    model.tasks.insert(task.id, task);
    model.refresh_visible_tasks();

    let theme = Theme::default();
    let planner = WeeklyPlanner::new(&model, &theme);

    let area = Rect::new(0, 0, 140, 30);
    let mut buffer = Buffer::empty(area);
    planner.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_weekly_planner_narrow_area() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let planner = WeeklyPlanner::new(&model, &theme);

    // Very narrow area
    let area = Rect::new(0, 0, 50, 10);
    let mut buffer = Buffer::empty(area);
    planner.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_weekly_planner_day_selection() {
    let mut model = Model::new();
    model.view_selection.weekly_planner_day = 3; // Thursday

    let theme = Theme::default();
    let planner = WeeklyPlanner::new(&model, &theme);

    let area = Rect::new(0, 0, 140, 30);
    let mut buffer = Buffer::empty(area);
    planner.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_weekly_planner_task_selection() {
    let mut model = Model::new();
    let today = Utc::now().date_naive();
    let week_start = WeeklyPlanner::week_start();
    let day_offset = (today - week_start).num_days() as usize;

    // Add multiple tasks for today
    for i in 0..3 {
        let mut task = Task::new(format!("Task {}", i + 1));
        task.due_date = Some(today);
        model.tasks.insert(task.id, task);
    }
    model.refresh_visible_tasks();

    // Select today's column and second task
    model.view_selection.weekly_planner_day = day_offset.min(6);
    model.view_selection.weekly_planner_task_index = 1;

    let theme = Theme::default();
    let planner = WeeklyPlanner::new(&model, &theme);

    let area = Rect::new(0, 0, 140, 30);
    let mut buffer = Buffer::empty(area);
    planner.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_weekly_planner_long_task_title() {
    let mut model = Model::new();
    let today = Utc::now().date_naive();

    let mut task = Task::new(
        "This is a very long task title that should be truncated to fit the column width",
    );
    task.due_date = Some(today);
    model.tasks.insert(task.id, task);
    model.refresh_visible_tasks();

    let theme = Theme::default();
    let planner = WeeklyPlanner::new(&model, &theme);

    let area = Rect::new(0, 0, 140, 30);
    let mut buffer = Buffer::empty(area);
    planner.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_weekly_planner_overdue_task() {
    let mut model = Model::new();
    let yesterday = Utc::now().date_naive() - chrono::Duration::days(1);
    let week_start = WeeklyPlanner::week_start();

    // Only test if yesterday is still in this week
    if yesterday >= week_start {
        let mut task = Task::new("Overdue Task");
        task.due_date = Some(yesterday);
        model.tasks.insert(task.id, task);
        model.refresh_visible_tasks();

        let theme = Theme::default();
        let planner = WeeklyPlanner::new(&model, &theme);

        let area = Rect::new(0, 0, 140, 30);
        let mut buffer = Buffer::empty(area);
        planner.render(area, &mut buffer);

        assert!(buffer.area.width > 0);
    }
}

#[test]
fn test_weekly_planner_both_scheduled_and_due() {
    let mut model = Model::new();
    let today = Utc::now().date_naive();

    // Task with both scheduled and due date on same day
    let mut task = Task::new("Scheduled and Due");
    task.due_date = Some(today);
    task.scheduled_date = Some(today);
    model.tasks.insert(task.id, task);
    model.refresh_visible_tasks();

    let theme = Theme::default();
    let planner = WeeklyPlanner::new(&model, &theme);

    // Should only appear once
    let tasks = planner.tasks_for_date(today);
    assert_eq!(tasks.len(), 1);
}
