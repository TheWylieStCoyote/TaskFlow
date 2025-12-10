//! Sorting tests.

use crate::app::model::Model;
use crate::domain::SortSpec;
use crate::domain::{Priority, SortField, SortOrder, Task, TaskStatus};

#[test]
fn test_sort_by_title() {
    let mut model = Model::new();

    let task_b = Task::new("Banana");
    let task_a = Task::new("Apple");
    let task_c = Task::new("Cherry");

    model.tasks.insert(task_b.id, task_b.clone());
    model.tasks.insert(task_a.id, task_a.clone());
    model.tasks.insert(task_c.id, task_c.clone());

    model.filtering.sort = SortSpec {
        field: SortField::Title,
        order: SortOrder::Ascending,
    };
    model.refresh_visible_tasks();

    assert_eq!(model.visible_tasks[0], task_a.id);
    assert_eq!(model.visible_tasks[1], task_b.id);
    assert_eq!(model.visible_tasks[2], task_c.id);
}

#[test]
fn test_sort_by_title_descending() {
    let mut model = Model::new();

    let task_b = Task::new("Banana");
    let task_a = Task::new("Apple");
    let task_c = Task::new("Cherry");

    model.tasks.insert(task_b.id, task_b.clone());
    model.tasks.insert(task_a.id, task_a.clone());
    model.tasks.insert(task_c.id, task_c.clone());

    model.filtering.sort = SortSpec {
        field: SortField::Title,
        order: SortOrder::Descending,
    };
    model.refresh_visible_tasks();

    assert_eq!(model.visible_tasks[0], task_c.id);
    assert_eq!(model.visible_tasks[1], task_b.id);
    assert_eq!(model.visible_tasks[2], task_a.id);
}

#[test]
fn test_sort_by_due_date() {
    let mut model = Model::new();

    let today = chrono::Utc::now().date_naive();
    let tomorrow = today + chrono::Duration::days(1);
    let next_week = today + chrono::Duration::days(7);

    let task_soon = Task::new("Soon").with_due_date(tomorrow);
    let task_later = Task::new("Later").with_due_date(next_week);
    let task_no_date = Task::new("No date");

    model.tasks.insert(task_later.id, task_later.clone());
    model.tasks.insert(task_soon.id, task_soon.clone());
    model.tasks.insert(task_no_date.id, task_no_date.clone());

    model.filtering.sort = SortSpec {
        field: SortField::DueDate,
        order: SortOrder::Ascending,
    };
    model.refresh_visible_tasks();

    // Tasks with dates come first, then tasks without dates
    assert_eq!(model.visible_tasks[0], task_soon.id);
    assert_eq!(model.visible_tasks[1], task_later.id);
    assert_eq!(model.visible_tasks[2], task_no_date.id);
}

#[test]
fn test_sort_by_status() {
    let mut model = Model::new();
    model.filtering.show_completed = true; // Show completed for this test

    let task_todo = Task::new("Todo").with_status(TaskStatus::Todo);
    let task_in_progress = Task::new("In Progress").with_status(TaskStatus::InProgress);
    let task_done = Task::new("Done").with_status(TaskStatus::Done);

    model.tasks.insert(task_done.id, task_done.clone());
    model.tasks.insert(task_todo.id, task_todo.clone());
    model
        .tasks
        .insert(task_in_progress.id, task_in_progress.clone());

    model.filtering.sort = SortSpec {
        field: SortField::Status,
        order: SortOrder::Ascending,
    };
    model.refresh_visible_tasks();

    // Order: InProgress, Todo, Blocked, Done, Cancelled
    assert_eq!(model.visible_tasks[0], task_in_progress.id);
    assert_eq!(model.visible_tasks[1], task_todo.id);
    assert_eq!(model.visible_tasks[2], task_done.id);
}

#[test]
fn test_sort_order_toggle() {
    let mut model = Model::new();

    let task_high = Task::new("High").with_priority(Priority::High);
    let task_low = Task::new("Low").with_priority(Priority::Low);

    model.tasks.insert(task_high.id, task_high.clone());
    model.tasks.insert(task_low.id, task_low.clone());

    // Ascending: High first (lower priority number)
    model.filtering.sort = SortSpec {
        field: SortField::Priority,
        order: SortOrder::Ascending,
    };
    model.refresh_visible_tasks();
    assert_eq!(model.visible_tasks[0], task_high.id);
    assert_eq!(model.visible_tasks[1], task_low.id);

    // Descending: Low first
    model.filtering.sort.order = SortOrder::Descending;
    model.refresh_visible_tasks();
    assert_eq!(model.visible_tasks[0], task_low.id);
    assert_eq!(model.visible_tasks[1], task_high.id);
}
