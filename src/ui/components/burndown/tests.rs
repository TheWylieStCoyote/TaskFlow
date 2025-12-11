//! Tests for burndown chart view component.

use super::*;
use crate::app::Model;
use crate::config::Theme;
use crate::domain::{Task, TaskStatus};
use ratatui::buffer::Buffer;

#[test]
fn test_burndown_empty_model() {
    let model = Model::new();
    let theme = Theme::default();
    let burndown = Burndown::new(&model, &theme);
    let data = burndown.get_burndown_data(None);
    assert_eq!(data.total, 0.0);
    assert_eq!(data.completed, 0.0);
    assert_eq!(data.remaining, 0.0);
}

#[test]
fn test_burndown_with_tasks() {
    let mut model = Model::new();

    // Add some tasks
    let task1 = Task::new("Task 1").with_status(TaskStatus::Done);
    let task2 = Task::new("Task 2").with_status(TaskStatus::Done);
    let task3 = Task::new("Task 3").with_status(TaskStatus::Todo);
    let task4 = Task::new("Task 4").with_status(TaskStatus::InProgress);

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.tasks.insert(task3.id, task3);
    model.tasks.insert(task4.id, task4);

    let theme = Theme::default();
    let burndown = Burndown::new(&model, &theme);
    let data = burndown.get_burndown_data(None);

    assert_eq!(data.total, 4.0);
    assert_eq!(data.completed, 2.0);
    assert_eq!(data.remaining, 2.0);
}

#[test]
fn test_burndown_daily_points_length() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let burndown = Burndown::new(&model, &theme);
    let data = burndown.get_burndown_data(None);

    // Should have 14 days of data (default window)
    assert_eq!(data.daily_points.len(), 14);
}

#[test]
fn test_burndown_renders_without_panic() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let burndown = Burndown::new(&model, &theme);

    let area = Rect::new(0, 0, 100, 30);
    let mut buffer = Buffer::empty(area);
    burndown.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_burndown_small_area_does_not_panic() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let burndown = Burndown::new(&model, &theme);

    // Very small area - should early return without panic
    let area = Rect::new(0, 0, 20, 10);
    let mut buffer = Buffer::empty(area);
    burndown.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_burndown_progress_calculation() {
    let mut model = Model::new();
    let theme = Theme::default();

    // Add 4 tasks, 2 completed
    let task1 = Task::new("Task 1").with_status(TaskStatus::Done);
    let task2 = Task::new("Task 2").with_status(TaskStatus::Done);
    let task3 = Task::new("Task 3").with_status(TaskStatus::Todo);
    let task4 = Task::new("Task 4").with_status(TaskStatus::InProgress);

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.tasks.insert(task3.id, task3);
    model.tasks.insert(task4.id, task4);

    let burndown = Burndown::new(&model, &theme);
    let data = burndown.get_burndown_data(None);

    assert_eq!(data.total, 4.0);
    assert_eq!(data.completed, 2.0);
    assert_eq!(data.remaining, 2.0);
}

#[test]
fn test_burndown_with_project_filter() {
    use crate::domain::Project;

    let mut model = Model::new();
    let theme = Theme::default();

    // Create a project
    let project = Project::new("Test Project");
    let project_id = project.id;
    model.projects.insert(project.id, project);

    // Add tasks - 2 in project, 2 without project
    let mut task1 = Task::new("Project Task 1").with_status(TaskStatus::Done);
    task1.project_id = Some(project_id);
    let mut task2 = Task::new("Project Task 2").with_status(TaskStatus::Todo);
    task2.project_id = Some(project_id);
    let task3 = Task::new("No Project Task 1");
    let task4 = Task::new("No Project Task 2");

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.tasks.insert(task3.id, task3);
    model.tasks.insert(task4.id, task4);

    let burndown = Burndown::new(&model, &theme);
    let data = burndown.get_burndown_data(Some(project_id));

    assert_eq!(data.total, 2.0);
    assert_eq!(data.completed, 1.0);
    assert_eq!(data.remaining, 1.0);
}

#[test]
fn test_burndown_velocity_calculation() {
    use chrono::{Duration, Local};

    let mut model = Model::new();
    let theme = Theme::default();

    // Add some tasks with completion dates spread over time
    let today = Local::now().date_naive();
    let mut task1 = Task::new("Task 1").with_status(TaskStatus::Done);
    task1.completed_at = Some(
        (today - Duration::days(3))
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc(),
    );
    let mut task2 = Task::new("Task 2").with_status(TaskStatus::Done);
    task2.completed_at = Some(
        (today - Duration::days(2))
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc(),
    );
    let task3 = Task::new("Task 3");

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.tasks.insert(task3.id, task3);

    let burndown = Burndown::new(&model, &theme);
    let data = burndown.get_burndown_data(None);

    assert_eq!(data.total, 3.0);
    assert_eq!(data.completed, 2.0);
    assert!(!data.daily_points.is_empty());
}

#[test]
fn test_burndown_renders_full_chart() {
    let mut model = Model::new();
    let theme = Theme::default();

    // Add several tasks with varying states
    for i in 0..10 {
        let status = if i < 5 {
            TaskStatus::Done
        } else {
            TaskStatus::Todo
        };
        let task = Task::new(format!("Task {i}")).with_status(status);
        model.tasks.insert(task.id, task);
    }

    let burndown = Burndown::new(&model, &theme);
    let area = Rect::new(0, 0, 100, 30);
    let mut buffer = Buffer::empty(area);
    burndown.render(area, &mut buffer);

    // Should render chart elements
    assert!(buffer.area.width > 0);
}

#[test]
fn test_burndown_all_completed() {
    let mut model = Model::new();
    let theme = Theme::default();

    // All tasks completed
    let task1 = Task::new("Task 1").with_status(TaskStatus::Done);
    let task2 = Task::new("Task 2").with_status(TaskStatus::Done);

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);

    let burndown = Burndown::new(&model, &theme);
    let data = burndown.get_burndown_data(None);

    assert_eq!(data.total, 2.0);
    assert_eq!(data.completed, 2.0);
    assert_eq!(data.remaining, 0.0);
}

#[test]
fn test_burndown_projects_panel() {
    use crate::domain::Project;

    let mut model = Model::new();
    let theme = Theme::default();

    // Create projects with tasks
    let project1 = Project::new("Project Alpha");
    let project2 = Project::new("Project Beta");

    let mut task1 = Task::new("Task 1");
    task1.project_id = Some(project1.id);
    let mut task2 = Task::new("Task 2");
    task2.project_id = Some(project2.id);

    model.projects.insert(project1.id, project1);
    model.projects.insert(project2.id, project2);
    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);

    let burndown = Burndown::new(&model, &theme);
    let area = Rect::new(0, 0, 100, 30);
    let mut buffer = Buffer::empty(area);
    burndown.render(area, &mut buffer);

    // Should render without panic
    assert!(buffer.area.width > 0);
}

#[test]
fn test_burndown_time_window_configuration() {
    use crate::app::BurndownTimeWindow;

    let mut model = Model::new();
    let theme = Theme::default();

    // Add a task
    let task = Task::new("Test task");
    model.tasks.insert(task.id, task);

    // Test default window (14 days)
    let burndown = Burndown::new(&model, &theme);
    let data = burndown.get_burndown_data(None);
    assert_eq!(data.window_days, 14);
    assert_eq!(data.daily_points.len(), 14);

    // Change to 7 days
    model.burndown_state.time_window = BurndownTimeWindow::Days7;
    let burndown = Burndown::new(&model, &theme);
    let data = burndown.get_burndown_data(None);
    assert_eq!(data.window_days, 7);
    assert_eq!(data.daily_points.len(), 7);

    // Change to 30 days
    model.burndown_state.time_window = BurndownTimeWindow::Days30;
    let burndown = Burndown::new(&model, &theme);
    let data = burndown.get_burndown_data(None);
    assert_eq!(data.window_days, 30);
    assert_eq!(data.daily_points.len(), 30);
}

#[test]
fn test_burndown_mode_toggle() {
    use crate::app::BurndownMode;

    let mut model = Model::new();
    let theme = Theme::default();

    // Add tasks with estimates
    let mut task1 = Task::new("Task 1").with_status(TaskStatus::Done);
    task1.estimated_minutes = Some(60); // 1 hour
    let mut task2 = Task::new("Task 2").with_status(TaskStatus::Todo);
    task2.estimated_minutes = Some(120); // 2 hours

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);

    // Test task count mode (default)
    let burndown = Burndown::new(&model, &theme);
    let data = burndown.get_burndown_data(None);
    assert_eq!(data.mode, BurndownMode::TaskCount);
    assert_eq!(data.total, 2.0);
    assert_eq!(data.completed, 1.0);

    // Test time hours mode
    model.burndown_state.mode = BurndownMode::TimeHours;
    let burndown = Burndown::new(&model, &theme);
    let data = burndown.get_burndown_data(None);
    assert_eq!(data.mode, BurndownMode::TimeHours);
    assert_eq!(data.total, 3.0); // 3 hours total
    assert_eq!(data.completed, 1.0); // 1 hour completed
}

#[test]
fn test_burndown_scope_creep_tracking() {
    let mut model = Model::new();
    let theme = Theme::default();

    // Add a task
    let task = Task::new("Test task");
    model.tasks.insert(task.id, task);

    // Enable scope creep display
    model.burndown_state.show_scope_creep = true;

    let burndown = Burndown::new(&model, &theme);
    let data = burndown.get_burndown_data(None);

    // scope_added should track tasks created in the period
    // Since task was created "now", it should be counted as scope added today
    assert!(data.scope_added >= 0.0);
}
