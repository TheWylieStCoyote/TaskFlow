//! Tests for evening review component.

use super::*;
use crate::app::Model;
use crate::config::Theme;
use crate::domain::{Task, TaskStatus};
use chrono::{Duration, Utc};

fn create_model_with_tasks() -> Model {
    let mut model = Model::new();

    // Add a completed task from today
    let mut completed = Task::new("Completed task");
    completed.status = TaskStatus::Done;
    completed.completed_at = Some(Utc::now());
    model.tasks.insert(completed.id, completed);

    // Add an incomplete task due today
    let mut due_today = Task::new("Due today");
    due_today.due_date = Some(Utc::now().date_naive());
    model.tasks.insert(due_today.id, due_today);

    // Add an incomplete task scheduled for today
    let mut scheduled_today = Task::new("Scheduled today");
    scheduled_today.scheduled_date = Some(Utc::now().date_naive());
    model.tasks.insert(scheduled_today.id, scheduled_today);

    // Add a task for tomorrow
    let mut tomorrow_task = Task::new("Tomorrow task");
    tomorrow_task.due_date = Some(Utc::now().date_naive() + Duration::days(1));
    model.tasks.insert(tomorrow_task.id, tomorrow_task);

    model
}

#[test]
fn test_queries_completed_today() {
    let model = create_model_with_tasks();
    let theme = Theme::default();
    let review = EveningReview::new(&model, &theme);

    let completed = review.completed_today();
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].title, "Completed task");
}

#[test]
fn test_queries_incomplete_due_today() {
    let model = create_model_with_tasks();
    let theme = Theme::default();
    let review = EveningReview::new(&model, &theme);

    let incomplete = review.incomplete_due_today();
    assert_eq!(incomplete.len(), 1);
    assert_eq!(incomplete[0].title, "Due today");
}

#[test]
fn test_queries_incomplete_scheduled_today() {
    let model = create_model_with_tasks();
    let theme = Theme::default();
    let review = EveningReview::new(&model, &theme);

    let scheduled = review.incomplete_scheduled_today();
    assert_eq!(scheduled.len(), 1);
    assert_eq!(scheduled[0].title, "Scheduled today");
}

#[test]
fn test_queries_all_incomplete_today() {
    let model = create_model_with_tasks();
    let theme = Theme::default();
    let review = EveningReview::new(&model, &theme);

    let all_incomplete = review.all_incomplete_today();
    assert_eq!(all_incomplete.len(), 2);
}

#[test]
fn test_queries_tomorrow_tasks() {
    let model = create_model_with_tasks();
    let theme = Theme::default();
    let review = EveningReview::new(&model, &theme);

    let tomorrow = review.tomorrow_tasks();
    assert_eq!(tomorrow.len(), 1);
    assert_eq!(tomorrow[0].title, "Tomorrow task");
}

#[test]
fn test_completion_rate_calculation() {
    let model = create_model_with_tasks();
    let theme = Theme::default();
    let review = EveningReview::new(&model, &theme);

    // 1 completed, 2 incomplete = 33.3%
    let rate = review.today_completion_rate();
    assert!((rate - 33.33).abs() < 1.0);
}

#[test]
fn test_completion_rate_all_complete() {
    let mut model = Model::new();

    // Only completed tasks
    let mut task = Task::new("Done");
    task.status = TaskStatus::Done;
    task.completed_at = Some(Utc::now());
    task.due_date = Some(Utc::now().date_naive());
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let review = EveningReview::new(&model, &theme);

    // 1 completed, 0 incomplete = 100%
    let rate = review.today_completion_rate();
    assert!((rate - 100.0).abs() < 0.01);
}

#[test]
fn test_completion_rate_no_tasks() {
    let model = Model::new();
    let theme = Theme::default();
    let review = EveningReview::new(&model, &theme);

    // No tasks = 100% (nothing to do)
    let rate = review.today_completion_rate();
    assert!((rate - 100.0).abs() < 0.01);
}
