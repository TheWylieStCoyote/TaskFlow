//! Time tracking tests.

use crate::app::Model;
use crate::domain::{Task, TimeEntry};

#[test]
fn test_model_start_time_tracking() {
    let mut model = Model::new();

    let task = Task::new("Task");
    model.tasks.insert(task.id, task);

    model.start_time_tracking(model.tasks.keys().next().copied().unwrap());

    assert!(model.active_time_entry.is_some());
    assert!(model.time_entries.len() == 1);
    assert!(model.dirty);

    let entry = model.active_time_entry().unwrap();
    assert!(entry.is_running());
}

#[test]
fn test_model_stop_time_tracking() {
    let mut model = Model::new();

    let task = Task::new("Task");
    let task_id = task.id;
    model.tasks.insert(task.id, task);

    model.start_time_tracking(task_id);
    model.stop_time_tracking();

    assert!(model.active_time_entry.is_none());

    // Entry should still exist but be stopped
    let entry = model.time_entries.values().next().unwrap();
    assert!(!entry.is_running());
}

#[test]
fn test_model_start_stops_previous() {
    let mut model = Model::new();

    let task1 = Task::new("Task 1");
    let task2 = Task::new("Task 2");
    let task1_id = task1.id;
    let task2_id = task2.id;
    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);

    // Start tracking task1
    model.start_time_tracking(task1_id);
    let first_entry_id = model.active_time_entry.unwrap();

    // Start tracking task2 (should stop task1)
    model.start_time_tracking(task2_id);

    // Two entries total
    assert_eq!(model.time_entries.len(), 2);

    // First entry should be stopped
    let first_entry = model.time_entries.get(&first_entry_id).unwrap();
    assert!(!first_entry.is_running());

    // Active entry should be for task2
    let active = model.active_time_entry().unwrap();
    assert_eq!(active.task_id, task2_id);
}

#[test]
fn test_model_is_tracking_task() {
    let mut model = Model::new();

    let task1 = Task::new("Task 1");
    let task2 = Task::new("Task 2");
    let task1_id = task1.id;
    let task2_id = task2.id;
    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);

    // Not tracking anything initially
    assert!(!model.is_tracking_task(&task1_id));
    assert!(!model.is_tracking_task(&task2_id));

    // Start tracking task1
    model.start_time_tracking(task1_id);

    assert!(model.is_tracking_task(&task1_id));
    assert!(!model.is_tracking_task(&task2_id));
}

#[test]
fn test_model_total_time_for_task() {
    let mut model = Model::new();

    let task = Task::new("Task");
    let task_id = task.id;
    model.tasks.insert(task.id, task);

    // Add multiple completed time entries
    let mut entry1 = TimeEntry::start(task_id);
    entry1.duration_minutes = Some(30);
    entry1.ended_at = Some(chrono::Utc::now());

    let mut entry2 = TimeEntry::start(task_id);
    entry2.duration_minutes = Some(45);
    entry2.ended_at = Some(chrono::Utc::now());

    model.time_entries.insert(entry1.id, entry1);
    model.time_entries.insert(entry2.id, entry2);

    let total = model.total_time_for_task(&task_id);
    assert_eq!(total, 75); // 30 + 45
}
