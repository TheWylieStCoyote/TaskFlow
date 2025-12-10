//! UI tests (input, toggles, help).

use crate::app::{update::update, Message, Model, UiMessage};
use crate::domain::Priority;
use crate::ui::InputMode;

#[test]
fn test_ui_toggle_show_completed() {
    let mut model = Model::new();
    assert!(!model.filtering.show_completed);

    update(&mut model, Message::Ui(UiMessage::ToggleShowCompleted));

    assert!(model.filtering.show_completed);

    update(&mut model, Message::Ui(UiMessage::ToggleShowCompleted));

    assert!(!model.filtering.show_completed);
}

#[test]
fn test_ui_toggle_sidebar() {
    let mut model = Model::new();
    assert!(model.show_sidebar);

    update(&mut model, Message::Ui(UiMessage::ToggleSidebar));

    assert!(!model.show_sidebar);
}

#[test]
fn test_ui_input_char() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;

    update(&mut model, Message::Ui(UiMessage::InputChar('H')));
    update(&mut model, Message::Ui(UiMessage::InputChar('i')));

    assert_eq!(model.input.buffer, "Hi");
    assert_eq!(model.input.cursor, 2);
}

#[test]
fn test_ui_input_backspace() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.buffer = "Hello".to_string();
    model.input.cursor = 5;

    update(&mut model, Message::Ui(UiMessage::InputBackspace));

    assert_eq!(model.input.buffer, "Hell");
    assert_eq!(model.input.cursor, 4);
}

#[test]
fn test_ui_input_cursor_movement() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.buffer = "Hello".to_string();
    model.input.cursor = 3;

    update(&mut model, Message::Ui(UiMessage::InputCursorLeft));
    assert_eq!(model.input.cursor, 2);

    update(&mut model, Message::Ui(UiMessage::InputCursorRight));
    assert_eq!(model.input.cursor, 3);

    update(&mut model, Message::Ui(UiMessage::InputCursorStart));
    assert_eq!(model.input.cursor, 0);

    update(&mut model, Message::Ui(UiMessage::InputCursorEnd));
    assert_eq!(model.input.cursor, 5);
}

#[test]
fn test_ui_submit_input_creates_task() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.buffer = "New task from input".to_string();

    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    assert_eq!(model.input.mode, InputMode::Normal);
    assert!(model.input.buffer.is_empty());
    assert_eq!(model.tasks.len(), 1);
    let task = model.tasks.values().next().unwrap();
    assert_eq!(task.title, "New task from input");
}

#[test]
fn test_ui_submit_input_empty_ignored() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.buffer = "   ".to_string(); // whitespace only

    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    assert_eq!(model.input.mode, InputMode::Normal);
    assert!(model.tasks.is_empty()); // no task created
}

#[test]
fn test_ui_cancel_input() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.buffer = "Some text".to_string();
    model.input.cursor = 5;

    update(&mut model, Message::Ui(UiMessage::CancelInput));

    assert_eq!(model.input.mode, InputMode::Normal);
    assert!(model.input.buffer.is_empty());
    assert_eq!(model.input.cursor, 0);
}

#[test]
fn test_show_help() {
    let mut model = Model::new();
    assert!(!model.show_help);

    update(&mut model, Message::Ui(UiMessage::ShowHelp));

    assert!(model.show_help);

    update(&mut model, Message::Ui(UiMessage::HideHelp));

    assert!(!model.show_help);
}

#[test]
fn test_submit_input_uses_default_priority() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.buffer = "Task via input".to_string();
    model.default_priority = Priority::Urgent;

    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    let task = model.tasks.values().next().unwrap();
    assert_eq!(task.title, "Task via input");
    assert_eq!(task.priority, Priority::Urgent);
}
