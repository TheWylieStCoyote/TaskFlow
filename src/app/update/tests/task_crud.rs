//! Task CRUD tests (create, read, update, delete).

use crate::app::{update::update, Message, Model, TaskMessage};
use crate::domain::{Priority, TaskStatus};

use super::create_test_model_with_tasks;

#[test]
fn test_task_toggle_complete() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Task should be Todo initially
    assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Todo);

    update(&mut model, Message::Task(TaskMessage::ToggleComplete));

    assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Done);
}

#[test]
fn test_task_set_status() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    update(
        &mut model,
        Message::Task(TaskMessage::SetStatus(task_id, TaskStatus::InProgress)),
    );

    assert_eq!(
        model.tasks.get(&task_id).unwrap().status,
        TaskStatus::InProgress
    );
}

#[test]
fn test_task_set_priority() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    update(
        &mut model,
        Message::Task(TaskMessage::SetPriority(task_id, Priority::Urgent)),
    );

    assert_eq!(
        model.tasks.get(&task_id).unwrap().priority,
        Priority::Urgent
    );
}

#[test]
fn test_task_create() {
    let mut model = Model::new();
    assert!(model.tasks.is_empty());

    update(
        &mut model,
        Message::Task(TaskMessage::Create("New task".to_string())),
    );

    assert_eq!(model.tasks.len(), 1);
    let task = model.tasks.values().next().unwrap();
    assert_eq!(task.title, "New task");
}

#[test]
fn test_task_delete() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];
    let initial_count = model.tasks.len();

    update(&mut model, Message::Task(TaskMessage::Delete(task_id)));

    assert_eq!(model.tasks.len(), initial_count - 1);
    assert!(!model.tasks.contains_key(&task_id));
}

#[test]
fn test_task_create_uses_default_priority() {
    let mut model = Model::new();
    model.default_priority = Priority::High;

    update(
        &mut model,
        Message::Task(TaskMessage::Create("High priority task".to_string())),
    );

    let task = model.tasks.values().next().unwrap();
    assert_eq!(task.title, "High priority task");
    assert_eq!(task.priority, Priority::High);
}

#[test]
fn test_cycle_priority() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Set initial priority to None
    model.tasks.get_mut(&task_id).unwrap().priority = Priority::None;

    // Cycle through priorities
    update(&mut model, Message::Task(TaskMessage::CyclePriority));
    assert_eq!(model.tasks.get(&task_id).unwrap().priority, Priority::Low);

    update(&mut model, Message::Task(TaskMessage::CyclePriority));
    assert_eq!(
        model.tasks.get(&task_id).unwrap().priority,
        Priority::Medium
    );

    update(&mut model, Message::Task(TaskMessage::CyclePriority));
    assert_eq!(model.tasks.get(&task_id).unwrap().priority, Priority::High);

    update(&mut model, Message::Task(TaskMessage::CyclePriority));
    assert_eq!(
        model.tasks.get(&task_id).unwrap().priority,
        Priority::Urgent
    );

    update(&mut model, Message::Task(TaskMessage::CyclePriority));
    assert_eq!(model.tasks.get(&task_id).unwrap().priority, Priority::None);
}
