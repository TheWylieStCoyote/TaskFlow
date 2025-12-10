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
    assert_eq!(data.total, 0);
    assert_eq!(data.completed, 0);
    assert_eq!(data.remaining, 0);
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

    assert_eq!(data.total, 4);
    assert_eq!(data.completed, 2);
    assert_eq!(data.remaining, 2);
}

#[test]
fn test_burndown_daily_completions_length() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let burndown = Burndown::new(&model, &theme);
    let data = burndown.get_burndown_data(None);

    // Should have 14 days of completion history
    assert_eq!(data.daily_completions.len(), 14);
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

    assert_eq!(data.total, 4);
    assert_eq!(data.completed, 2);
    assert_eq!(data.remaining, 2);
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

    assert_eq!(data.total, 2);
    assert_eq!(data.completed, 1);
    assert_eq!(data.remaining, 1);
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

    assert_eq!(data.total, 3);
    assert_eq!(data.completed, 2);
    assert!(!data.daily_completions.is_empty());
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

    assert_eq!(data.total, 2);
    assert_eq!(data.completed, 2);
    assert_eq!(data.remaining, 0);
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
