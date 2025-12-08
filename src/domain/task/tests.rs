//! Tests for task module.

use super::*;
use chrono::NaiveDate;

#[test]
fn test_task_new_creates_unique_id() {
    let task1 = Task::new("Task 1");
    let task2 = Task::new("Task 2");
    assert_ne!(task1.id, task2.id);
}

#[test]
fn test_task_new_sets_defaults() {
    let task = Task::new("Test task");
    assert_eq!(task.title, "Test task");
    assert_eq!(task.status, TaskStatus::Todo);
    assert_eq!(task.priority, Priority::None);
    assert!(task.description.is_none());
    assert!(task.due_date.is_none());
    assert!(task.completed_at.is_none());
    assert!(task.tags.is_empty());
}

#[test]
fn test_task_with_priority() {
    let task = Task::new("Test").with_priority(Priority::High);
    assert_eq!(task.priority, Priority::High);
}

#[test]
fn test_task_with_status_sets_completion() {
    let task = Task::new("Test").with_status(TaskStatus::Done);
    assert_eq!(task.status, TaskStatus::Done);
    assert!(task.completed_at.is_some());
}

#[test]
fn test_task_with_due_date() {
    let date = NaiveDate::from_ymd_opt(2025, 12, 25).unwrap();
    let task = Task::new("Test").with_due_date(date);
    assert_eq!(task.due_date, Some(date));
}

#[test]
fn test_task_toggle_complete_todo_to_done() {
    let mut task = Task::new("Test");
    assert_eq!(task.status, TaskStatus::Todo);
    assert!(task.completed_at.is_none());

    task.toggle_complete();

    assert_eq!(task.status, TaskStatus::Done);
    assert!(task.completed_at.is_some());
}

#[test]
fn test_task_toggle_complete_done_to_todo() {
    let mut task = Task::new("Test").with_status(TaskStatus::Done);
    assert_eq!(task.status, TaskStatus::Done);
    assert!(task.completed_at.is_some());

    task.toggle_complete();

    assert_eq!(task.status, TaskStatus::Todo);
    assert!(task.completed_at.is_none());
}

#[test]
fn test_task_is_overdue_no_due_date() {
    let task = Task::new("Test");
    assert!(!task.is_overdue());
}

#[test]
fn test_task_is_overdue_past_date() {
    let past_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let task = Task::new("Test").with_due_date(past_date);
    assert!(task.is_overdue());
}

#[test]
fn test_task_is_overdue_completed() {
    let past_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let task = Task::new("Test")
        .with_due_date(past_date)
        .with_status(TaskStatus::Done);
    assert!(!task.is_overdue());
}

#[test]
fn test_task_is_due_today() {
    let today = Utc::now().date_naive();
    let task = Task::new("Test").with_due_date(today);
    assert!(task.is_due_today());

    let yesterday = today - chrono::Duration::days(1);
    let task2 = Task::new("Test").with_due_date(yesterday);
    assert!(!task2.is_due_today());
}

#[test]
fn test_priority_as_str() {
    assert_eq!(Priority::None.as_str(), "none");
    assert_eq!(Priority::Low.as_str(), "low");
    assert_eq!(Priority::Medium.as_str(), "medium");
    assert_eq!(Priority::High.as_str(), "high");
    assert_eq!(Priority::Urgent.as_str(), "urgent");
}

#[test]
fn test_priority_symbol() {
    assert_eq!(Priority::None.symbol(), " ");
    assert_eq!(Priority::Low.symbol(), "!");
    assert_eq!(Priority::Medium.symbol(), "!!");
    assert_eq!(Priority::High.symbol(), "!!!");
    assert_eq!(Priority::Urgent.symbol(), "!!!!");
}

#[test]
fn test_priority_parse() {
    assert_eq!(Priority::parse("none"), Some(Priority::None));
    assert_eq!(Priority::parse("low"), Some(Priority::Low));
    assert_eq!(Priority::parse("medium"), Some(Priority::Medium));
    assert_eq!(Priority::parse("med"), Some(Priority::Medium));
    assert_eq!(Priority::parse("high"), Some(Priority::High));
    assert_eq!(Priority::parse("urgent"), Some(Priority::Urgent));
    // Case insensitive
    assert_eq!(Priority::parse("HIGH"), Some(Priority::High));
    assert_eq!(Priority::parse("Low"), Some(Priority::Low));
    // Invalid
    assert_eq!(Priority::parse("invalid"), None);
    assert_eq!(Priority::parse(""), None);
}

#[test]
fn test_task_status_as_str() {
    assert_eq!(TaskStatus::Todo.as_str(), "todo");
    assert_eq!(TaskStatus::InProgress.as_str(), "in_progress");
    assert_eq!(TaskStatus::Blocked.as_str(), "blocked");
    assert_eq!(TaskStatus::Done.as_str(), "done");
    assert_eq!(TaskStatus::Cancelled.as_str(), "cancelled");
}

#[test]
fn test_task_status_symbol() {
    assert_eq!(TaskStatus::Todo.symbol(), "[ ]");
    assert_eq!(TaskStatus::InProgress.symbol(), "[~]");
    assert_eq!(TaskStatus::Blocked.symbol(), "[!]");
    assert_eq!(TaskStatus::Done.symbol(), "[x]");
    assert_eq!(TaskStatus::Cancelled.symbol(), "[-]");
}

#[test]
fn test_task_status_is_complete() {
    assert!(!TaskStatus::Todo.is_complete());
    assert!(!TaskStatus::InProgress.is_complete());
    assert!(!TaskStatus::Blocked.is_complete());
    assert!(TaskStatus::Done.is_complete());
    assert!(TaskStatus::Cancelled.is_complete());
}

#[test]
fn test_time_variance_over_estimate() {
    let mut task = Task::new("Test");
    task.estimated_minutes = Some(60);
    task.actual_minutes = 90;
    assert_eq!(task.time_variance(), Some(30));
    assert_eq!(task.time_variance_display(), Some("+30m over".to_string()));
}

#[test]
fn test_time_variance_under_estimate() {
    let mut task = Task::new("Test");
    task.estimated_minutes = Some(60);
    task.actual_minutes = 45;
    assert_eq!(task.time_variance(), Some(-15));
    assert_eq!(task.time_variance_display(), Some("-15m under".to_string()));
}

#[test]
fn test_time_variance_on_target() {
    let mut task = Task::new("Test");
    task.estimated_minutes = Some(60);
    task.actual_minutes = 60;
    assert_eq!(task.time_variance(), Some(0));
    assert_eq!(task.time_variance_display(), Some("on target".to_string()));
}

#[test]
fn test_time_variance_no_estimate() {
    let task = Task::new("Test");
    assert_eq!(task.time_variance(), None);
    assert_eq!(task.time_variance_display(), None);
}

#[test]
fn test_time_variance_display_hours() {
    let mut task = Task::new("Test");
    task.estimated_minutes = Some(60);
    task.actual_minutes = 150; // 90 minutes over
    assert_eq!(
        task.time_variance_display(),
        Some("+1h 30m over".to_string())
    );

    task.actual_minutes = 0; // 60 minutes under
    assert_eq!(
        task.time_variance_display(),
        Some("-1h 0m under".to_string())
    );
}

#[test]
fn test_estimation_accuracy() {
    let mut task = Task::new("Test");
    task.estimated_minutes = Some(100);
    task.actual_minutes = 100;
    assert!((task.estimation_accuracy().unwrap() - 100.0).abs() < 0.01);

    task.actual_minutes = 50;
    assert!((task.estimation_accuracy().unwrap() - 50.0).abs() < 0.01);

    task.actual_minutes = 150;
    assert!((task.estimation_accuracy().unwrap() - 150.0).abs() < 0.01);
}

#[test]
fn test_estimation_accuracy_zero_estimate() {
    let mut task = Task::new("Test");
    task.estimated_minutes = Some(0);
    task.actual_minutes = 100;
    assert_eq!(task.estimation_accuracy(), None);
}

#[test]
fn test_estimation_accuracy_no_estimate() {
    let task = Task::new("Test");
    assert_eq!(task.estimation_accuracy(), None);
}

// Snooze functionality tests

#[test]
fn test_task_not_snoozed_by_default() {
    let task = Task::new("Test");
    assert!(!task.is_snoozed());
    assert!(task.snooze_until.is_none());
}

#[test]
fn test_task_snooze_until_future_date() {
    let mut task = Task::new("Test");
    let future = Utc::now().date_naive() + chrono::Duration::days(7);
    task.snooze_until_date(future);

    assert!(task.is_snoozed());
    assert_eq!(task.snooze_until, Some(future));
}

#[test]
fn test_task_snooze_until_past_date_not_snoozed() {
    let mut task = Task::new("Test");
    let past = Utc::now().date_naive() - chrono::Duration::days(1);
    task.snooze_until_date(past);

    // Snooze date is set, but is_snoozed returns false for past dates
    assert!(!task.is_snoozed());
    assert_eq!(task.snooze_until, Some(past));
}

#[test]
fn test_task_snooze_until_today_not_snoozed() {
    let mut task = Task::new("Test");
    let today = Utc::now().date_naive();
    task.snooze_until_date(today);

    // Today is not > today, so not snoozed
    assert!(!task.is_snoozed());
}

#[test]
fn test_task_clear_snooze() {
    let mut task = Task::new("Test");
    let future = Utc::now().date_naive() + chrono::Duration::days(7);
    task.snooze_until_date(future);
    assert!(task.is_snoozed());

    task.clear_snooze();
    assert!(!task.is_snoozed());
    assert!(task.snooze_until.is_none());
}

#[test]
fn test_snooze_updates_updated_at() {
    let mut task = Task::new("Test");
    let original_updated = task.updated_at;

    // Small delay to ensure time difference
    std::thread::sleep(std::time::Duration::from_millis(10));

    let future = Utc::now().date_naive() + chrono::Duration::days(1);
    task.snooze_until_date(future);

    assert!(task.updated_at > original_updated);
}

#[test]
fn test_clear_snooze_updates_updated_at() {
    let mut task = Task::new("Test");
    let future = Utc::now().date_naive() + chrono::Duration::days(1);
    task.snooze_until_date(future);
    let after_snooze = task.updated_at;

    std::thread::sleep(std::time::Duration::from_millis(10));

    task.clear_snooze();
    assert!(task.updated_at > after_snooze);
}

// Scheduled date tests

#[test]
fn test_task_scheduled_date() {
    let date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
    let mut task = Task::new("Test");
    task.scheduled_date = Some(date);

    assert_eq!(task.scheduled_date, Some(date));
}

#[test]
fn test_task_scheduled_vs_due_dates_independent() {
    let scheduled = NaiveDate::from_ymd_opt(2025, 6, 10).unwrap();
    let due = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    let task = Task::new("Test").with_due_date(due);
    let mut task = task;
    task.scheduled_date = Some(scheduled);

    assert_eq!(task.scheduled_date, Some(scheduled));
    assert_eq!(task.due_date, Some(due));
}

// Custom fields tests

#[test]
fn test_task_custom_fields_empty_by_default() {
    let task = Task::new("Test");
    assert!(task.custom_fields.is_empty());
}

#[test]
fn test_task_custom_fields_string() {
    let mut task = Task::new("Test");
    task.custom_fields
        .insert("client".to_string(), serde_json::json!("Acme Corp"));

    assert_eq!(
        task.custom_fields.get("client"),
        Some(&serde_json::json!("Acme Corp"))
    );
}

#[test]
fn test_task_custom_fields_number() {
    let mut task = Task::new("Test");
    task.custom_fields
        .insert("story_points".to_string(), serde_json::json!(5));

    assert_eq!(
        task.custom_fields.get("story_points"),
        Some(&serde_json::json!(5))
    );
}

#[test]
fn test_task_custom_fields_complex() {
    let mut task = Task::new("Test");
    task.custom_fields.insert(
        "metadata".to_string(),
        serde_json::json!({"reviewed": true, "reviewer": "alice"}),
    );

    let metadata = task.custom_fields.get("metadata").unwrap();
    assert_eq!(metadata["reviewed"], serde_json::json!(true));
    assert_eq!(metadata["reviewer"], serde_json::json!("alice"));
}

// Dependencies tests

#[test]
fn test_task_dependencies_empty_by_default() {
    let task = Task::new("Test");
    assert!(task.dependencies.is_empty());
}

#[test]
fn test_task_dependencies() {
    let task1 = Task::new("Blocker");
    let mut task2 = Task::new("Blocked task");
    task2.dependencies.push(task1.id);

    assert_eq!(task2.dependencies.len(), 1);
    assert_eq!(task2.dependencies[0], task1.id);
}

// Task chain tests

#[test]
fn test_task_chain_next_task() {
    let task1 = Task::new("First");
    let mut task2 = Task::new("Second");
    task2.next_task_id = Some(task1.id);

    assert_eq!(task2.next_task_id, Some(task1.id));
}

#[test]
fn test_task_chain_none_by_default() {
    let task = Task::new("Test");
    assert!(task.next_task_id.is_none());
}

// Sort order tests

#[test]
fn test_task_sort_order() {
    let mut task = Task::new("Test");
    task.sort_order = Some(100);

    assert_eq!(task.sort_order, Some(100));
}

#[test]
fn test_task_sort_order_negative() {
    let mut task = Task::new("Test");
    task.sort_order = Some(-50);

    assert_eq!(task.sort_order, Some(-50));
}

// Serialization tests

#[test]
fn test_task_serialization_roundtrip() {
    let task = Task::new("Roundtrip test")
        .with_priority(Priority::High)
        .with_status(TaskStatus::InProgress)
        .with_tags(vec!["tag1".to_string(), "tag2".to_string()])
        .with_description("A description");

    let json = serde_json::to_string(&task).expect("Failed to serialize");
    let restored: Task = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(restored.id, task.id);
    assert_eq!(restored.title, task.title);
    assert_eq!(restored.priority, task.priority);
    assert_eq!(restored.status, task.status);
    assert_eq!(restored.tags, task.tags);
    assert_eq!(restored.description, task.description);
}

#[test]
fn test_task_serialization_with_snooze() {
    let mut task = Task::new("Snoozed task");
    let future = NaiveDate::from_ymd_opt(2025, 12, 25).unwrap();
    task.snooze_until = Some(future);

    let json = serde_json::to_string(&task).expect("Failed to serialize");
    let restored: Task = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(restored.snooze_until, Some(future));
}

#[test]
fn test_task_serialization_with_custom_fields() {
    let mut task = Task::new("Task with fields");
    task.custom_fields
        .insert("key".to_string(), serde_json::json!("value"));

    let json = serde_json::to_string(&task).expect("Failed to serialize");
    let restored: Task = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(
        restored.custom_fields.get("key"),
        Some(&serde_json::json!("value"))
    );
}
