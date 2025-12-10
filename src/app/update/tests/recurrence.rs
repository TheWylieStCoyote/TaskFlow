//! Recurrence tests.

use chrono::NaiveDate;

use crate::app::{update::update, Message, SystemMessage, TaskMessage, UiMessage};
use crate::domain::Recurrence;
use crate::ui::InputMode;

use super::create_test_model_with_tasks;

#[test]
fn test_set_recurrence_daily() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Start editing recurrence
    update(&mut model, Message::Ui(UiMessage::StartEditRecurrence));
    assert_eq!(model.input.mode, InputMode::Editing);

    // Set to daily
    model.input.buffer = "d".to_string();
    model.input.cursor = 1;
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    let task = model.tasks.get(&task_id).unwrap();
    assert!(matches!(task.recurrence, Some(Recurrence::Daily)));
}

#[test]
fn test_set_recurrence_weekly() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    update(&mut model, Message::Ui(UiMessage::StartEditRecurrence));
    model.input.buffer = "w".to_string();
    model.input.cursor = 1;
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    let task = model.tasks.get(&task_id).unwrap();
    assert!(matches!(task.recurrence, Some(Recurrence::Weekly { .. })));
}

#[test]
fn test_clear_recurrence() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // First set recurrence
    if let Some(task) = model.tasks.get_mut(&task_id) {
        task.recurrence = Some(Recurrence::Daily);
    }

    // Now clear it
    update(&mut model, Message::Ui(UiMessage::StartEditRecurrence));
    model.input.buffer = "0".to_string();
    model.input.cursor = 1;
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    let task = model.tasks.get(&task_id).unwrap();
    assert!(task.recurrence.is_none());
}

#[test]
fn test_completing_recurring_task_creates_next() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];
    let initial_count = model.tasks.len();

    // Set task as recurring with a due date
    if let Some(task) = model.tasks.get_mut(&task_id) {
        task.recurrence = Some(Recurrence::Daily);
        task.due_date = Some(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap());
    }
    model.refresh_visible_tasks();

    // Complete the task
    update(&mut model, Message::Task(TaskMessage::ToggleComplete));

    // Should have created a new task
    assert_eq!(model.tasks.len(), initial_count + 1);

    // The new task should have the same title and be recurring
    let new_tasks: Vec<_> = model
        .tasks
        .values()
        .filter(|t| t.id != task_id && t.recurrence.is_some())
        .collect();
    assert_eq!(new_tasks.len(), 1);
    let new_task = new_tasks[0];
    assert!(new_task.recurrence.is_some());
    assert!(new_task.due_date.is_some());
}

#[test]
fn test_completing_non_recurring_task_no_new_task() {
    let mut model = create_test_model_with_tasks();
    let initial_count = model.tasks.len();

    // Complete a non-recurring task
    update(&mut model, Message::Task(TaskMessage::ToggleComplete));

    // Should NOT create a new task
    assert_eq!(model.tasks.len(), initial_count);
}

#[test]
fn test_recurrence_undo() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Set recurrence
    update(&mut model, Message::Ui(UiMessage::StartEditRecurrence));
    model.input.buffer = "d".to_string();
    model.input.cursor = 1;
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    assert!(model.tasks.get(&task_id).unwrap().recurrence.is_some());

    // Undo
    update(&mut model, Message::System(SystemMessage::Undo));

    assert!(model.tasks.get(&task_id).unwrap().recurrence.is_none());
}
