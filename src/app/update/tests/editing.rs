//! Task editing tests (title, due date, tags, description).

use chrono::NaiveDate;

use crate::app::{update::update, Message, SystemMessage, UiMessage};
use crate::ui::{InputMode, InputTarget};

use super::create_test_model_with_tasks;

// === Task Title Editing ===

#[test]
fn test_start_edit_task() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();
    let original_title = model.tasks.get(&task_id).unwrap().title.clone();

    update(&mut model, Message::Ui(UiMessage::StartEditTask));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert_eq!(model.input_target, InputTarget::EditTask(task_id));
    assert_eq!(model.input_buffer, original_title);
    assert_eq!(model.cursor_position, original_title.len());
}

#[test]
fn test_edit_task_title() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Start editing
    update(&mut model, Message::Ui(UiMessage::StartEditTask));

    // Clear and type new title
    model.input_buffer.clear();
    model.cursor_position = 0;
    for c in "Updated Title".chars() {
        update(&mut model, Message::Ui(UiMessage::InputChar(c)));
    }

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Title should be updated
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.title, "Updated Title");
    assert_eq!(model.input_mode, InputMode::Normal);
}

#[test]
fn test_cancel_edit_task() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();
    let original_title = model.tasks.get(&task_id).unwrap().title.clone();

    // Start editing
    update(&mut model, Message::Ui(UiMessage::StartEditTask));

    // Type something
    model.input_buffer = "Changed".to_string();

    // Cancel
    update(&mut model, Message::Ui(UiMessage::CancelInput));

    // Title should NOT be changed
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.title, original_title);
    assert_eq!(model.input_mode, InputMode::Normal);
}

// === Due Date Editing ===

#[test]
fn test_edit_due_date() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Start editing due date
    update(&mut model, Message::Ui(UiMessage::StartEditDueDate));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert!(matches!(model.input_target, InputTarget::EditDueDate(_)));

    // Type a date
    model.input_buffer = "2025-12-25".to_string();
    model.cursor_position = model.input_buffer.len();

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Due date should be set
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(
        task.due_date,
        Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap())
    );
}

#[test]
fn test_clear_due_date() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Set an initial due date
    model.tasks.get_mut(&task_id).unwrap().due_date =
        Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());

    // Start editing due date
    update(&mut model, Message::Ui(UiMessage::StartEditDueDate));

    // Clear the buffer
    model.input_buffer.clear();
    model.cursor_position = 0;

    // Submit empty
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Due date should be cleared
    let task = model.tasks.get(&task_id).unwrap();
    assert!(task.due_date.is_none());
}

#[test]
fn test_invalid_due_date_keeps_old() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Set an initial due date
    let original_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    model.tasks.get_mut(&task_id).unwrap().due_date = Some(original_date);

    // Start editing due date
    update(&mut model, Message::Ui(UiMessage::StartEditDueDate));

    // Type invalid date
    model.input_buffer = "not-a-date".to_string();
    model.cursor_position = model.input_buffer.len();

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Due date should be unchanged
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.due_date, Some(original_date));
}

// === Tag Editing ===

#[test]
fn test_start_edit_tags() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Add some initial tags
    model.tasks.get_mut(&task_id).unwrap().tags = vec!["work".to_string(), "urgent".to_string()];

    update(&mut model, Message::Ui(UiMessage::StartEditTags));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert!(matches!(model.input_target, InputTarget::EditTags(_)));
    assert_eq!(model.input_buffer, "work, urgent");
}

#[test]
fn test_edit_tags_add_new() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Task has no tags initially
    assert!(model.tasks.get(&task_id).unwrap().tags.is_empty());

    // Start editing tags
    update(&mut model, Message::Ui(UiMessage::StartEditTags));

    // Type new tags
    model.input_buffer = "feature, bug, priority".to_string();
    model.cursor_position = model.input_buffer.len();

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Tags should be set
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.tags, vec!["feature", "bug", "priority"]);
}

#[test]
fn test_edit_tags_clear() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Set initial tags
    model.tasks.get_mut(&task_id).unwrap().tags = vec!["work".to_string()];

    // Start editing tags
    update(&mut model, Message::Ui(UiMessage::StartEditTags));

    // Clear input
    model.input_buffer.clear();
    model.cursor_position = 0;

    // Submit empty
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Tags should be cleared
    let task = model.tasks.get(&task_id).unwrap();
    assert!(task.tags.is_empty());
}

#[test]
fn test_edit_tags_trims_whitespace() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Start editing tags
    update(&mut model, Message::Ui(UiMessage::StartEditTags));

    // Type tags with extra whitespace
    model.input_buffer = "  work  ,  play  , rest ".to_string();
    model.cursor_position = model.input_buffer.len();

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Tags should be trimmed
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.tags, vec!["work", "play", "rest"]);
}

#[test]
fn test_edit_tags_filters_empty() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Start editing tags
    update(&mut model, Message::Ui(UiMessage::StartEditTags));

    // Type tags with empty entries
    model.input_buffer = "work,,, ,play".to_string();
    model.cursor_position = model.input_buffer.len();

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Only non-empty tags should remain
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.tags, vec!["work", "play"]);
}

#[test]
fn test_cancel_edit_tags() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Set initial tags
    let original_tags = vec!["original".to_string()];
    model.tasks.get_mut(&task_id).unwrap().tags = original_tags.clone();

    // Start editing
    update(&mut model, Message::Ui(UiMessage::StartEditTags));

    // Type something different
    model.input_buffer = "new, tags, here".to_string();

    // Cancel
    update(&mut model, Message::Ui(UiMessage::CancelInput));

    // Tags should NOT be changed
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(task.tags, original_tags);
    assert_eq!(model.input_mode, InputMode::Normal);
}

// === Description Editing ===

#[test]
fn test_start_edit_description_enters_edit_mode() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Task starts with no description
    assert!(model.tasks.get(&task_id).unwrap().description.is_none());

    update(&mut model, Message::Ui(UiMessage::StartEditDescription));

    assert_eq!(model.input_mode, InputMode::Editing);
    assert!(matches!(
        model.input_target,
        InputTarget::EditDescription(_)
    ));
    assert!(model.input_buffer.is_empty());
}

#[test]
fn test_start_edit_description_prefills_existing() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Set existing description
    model.tasks.get_mut(&task_id).unwrap().description = Some("Existing notes here".to_string());

    update(&mut model, Message::Ui(UiMessage::StartEditDescription));

    assert_eq!(model.input_buffer, "Existing notes here");
}

#[test]
fn test_edit_description_add_new() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Start editing description
    update(&mut model, Message::Ui(UiMessage::StartEditDescription));

    // Type new description
    model.input_buffer = "This is a detailed task description".to_string();
    model.cursor_position = model.input_buffer.len();

    // Submit
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Description should be set
    let task = model.tasks.get(&task_id).unwrap();
    assert_eq!(
        task.description,
        Some("This is a detailed task description".to_string())
    );
}

#[test]
fn test_edit_description_clear() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Set initial description
    model.tasks.get_mut(&task_id).unwrap().description = Some("Old description".to_string());

    // Start editing
    update(&mut model, Message::Ui(UiMessage::StartEditDescription));

    // Clear input
    model.input_buffer.clear();
    model.cursor_position = 0;

    // Submit empty
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Description should be cleared
    let task = model.tasks.get(&task_id).unwrap();
    assert!(task.description.is_none());
}

#[test]
fn test_edit_description_undo() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0].clone();

    // Start with no description
    assert!(model.tasks.get(&task_id).unwrap().description.is_none());

    // Add a description
    update(&mut model, Message::Ui(UiMessage::StartEditDescription));
    model.input_buffer = "New description".to_string();
    model.cursor_position = model.input_buffer.len();
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Verify description was set
    assert_eq!(
        model.tasks.get(&task_id).unwrap().description,
        Some("New description".to_string())
    );

    // Undo
    update(&mut model, Message::System(SystemMessage::Undo));

    // Description should be gone
    assert!(model.tasks.get(&task_id).unwrap().description.is_none());
}
