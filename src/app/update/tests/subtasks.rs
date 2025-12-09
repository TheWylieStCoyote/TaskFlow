//! Subtask creation tests.

use crate::app::{update::update, Message, Model, SystemMessage, UiMessage};
use crate::domain::Priority;
use crate::ui::{InputMode, InputTarget};

use super::create_test_model_with_tasks;

#[test]
fn test_start_create_subtask() {
    let mut model = create_test_model_with_tasks();
    let _parent_id = model.visible_tasks[0];

    update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert!(matches!(model.input_target, InputTarget::Subtask(_)));
    assert!(model.input_buffer.is_empty());
}

#[test]
fn test_start_create_subtask_no_selection() {
    let mut model = Model::new();
    // No tasks, so no selection

    update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));

    // Should remain in normal mode since there's no parent task
    assert_eq!(model.input_mode, InputMode::Normal);
}

#[test]
fn test_submit_subtask_creates_with_parent() {
    let mut model = create_test_model_with_tasks();
    let parent_id = model.visible_tasks[0];
    let initial_count = model.tasks.len();

    // Start creating subtask
    update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));

    // Type subtask name
    model.input_buffer = "My subtask".to_string();
    model.cursor_position = model.input_buffer.len();

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Should have one more task
    assert_eq!(model.tasks.len(), initial_count + 1);

    // Find the new subtask
    let subtask = model
        .tasks
        .values()
        .find(|t| t.title == "My subtask")
        .expect("Subtask should exist");

    // Should have parent_task_id set
    assert_eq!(subtask.parent_task_id, Some(parent_id));
}

#[test]
fn test_subtask_inherits_default_priority() {
    let mut model = create_test_model_with_tasks();
    model.default_priority = Priority::High;

    // Start creating subtask
    update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));
    model.input_buffer = "Priority subtask".to_string();
    model.cursor_position = model.input_buffer.len();
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    let subtask = model
        .tasks
        .values()
        .find(|t| t.title == "Priority subtask")
        .expect("Subtask should exist");

    assert_eq!(subtask.priority, Priority::High);
}

#[test]
fn test_cancel_subtask_creation() {
    let mut model = create_test_model_with_tasks();
    let initial_count = model.tasks.len();

    // Start creating subtask
    update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));

    // Type something
    model.input_buffer = "Will be cancelled".to_string();

    // Cancel
    update(&mut model, Message::Ui(UiMessage::CancelInput));

    // No new task should be created
    assert_eq!(model.tasks.len(), initial_count);
    assert_eq!(model.input_mode, InputMode::Normal);
}

#[test]
fn test_subtask_empty_name_not_created() {
    let mut model = create_test_model_with_tasks();
    let initial_count = model.tasks.len();

    // Start creating subtask
    update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));

    // Submit with empty name
    model.input_buffer = "   ".to_string();
    model.cursor_position = model.input_buffer.len();
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // No new task should be created
    assert_eq!(model.tasks.len(), initial_count);
    assert_eq!(model.input_mode, InputMode::Normal);
}

#[test]
fn test_subtask_undo() {
    let mut model = create_test_model_with_tasks();
    let initial_count = model.tasks.len();

    // Create subtask
    update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));
    model.input_buffer = "Subtask to undo".to_string();
    model.cursor_position = model.input_buffer.len();
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    assert_eq!(model.tasks.len(), initial_count + 1);

    // Undo
    update(&mut model, Message::System(SystemMessage::Undo));

    assert_eq!(model.tasks.len(), initial_count);
    assert!(!model.tasks.values().any(|t| t.title == "Subtask to undo"));
}
