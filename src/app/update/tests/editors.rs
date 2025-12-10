//! Work log and description editor tests.

use crate::app::{update::update, Message, Model, UiMessage};
use crate::domain::{Task, WorkLogEntry};
use crate::ui::WorkLogMode;

fn create_model_with_work_logs() -> Model {
    let mut model = Model::new();

    // Create a task
    let task = Task::new("Test task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    // Create work log entries
    let entry1 = WorkLogEntry::new(task_id, "First work log entry");
    model.work_logs.insert(entry1.id, entry1);

    let entry2 = WorkLogEntry::new(task_id, "Second work log entry");
    model.work_logs.insert(entry2.id, entry2);

    model
}

// Work log tests
#[test]
fn test_show_work_log() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = false;

    update(&mut model, Message::Ui(UiMessage::ShowWorkLog));

    assert!(model.work_log_editor.visible);
    assert_eq!(model.work_log_editor.selected, 0);
    assert_eq!(model.work_log_editor.mode, WorkLogMode::Browse);
}

#[test]
fn test_show_work_log_no_task_selected() {
    let mut model = Model::new();

    update(&mut model, Message::Ui(UiMessage::ShowWorkLog));

    assert!(!model.work_log_editor.visible);
}

#[test]
fn test_hide_work_log() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.mode = WorkLogMode::Edit;

    update(&mut model, Message::Ui(UiMessage::HideWorkLog));

    assert!(!model.work_log_editor.visible);
    assert_eq!(model.work_log_editor.mode, WorkLogMode::Browse);
}

#[test]
fn test_work_log_up() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.selected = 1;

    update(&mut model, Message::Ui(UiMessage::WorkLogUp));

    assert_eq!(model.work_log_editor.selected, 0);
}

#[test]
fn test_work_log_down() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.selected = 0;

    update(&mut model, Message::Ui(UiMessage::WorkLogDown));

    assert_eq!(model.work_log_editor.selected, 1);
}

#[test]
fn test_work_log_view() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.mode = WorkLogMode::Browse;

    update(&mut model, Message::Ui(UiMessage::WorkLogView));

    assert_eq!(model.work_log_editor.mode, WorkLogMode::View);
}

#[test]
fn test_work_log_view_toggle_back() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.mode = WorkLogMode::View;

    update(&mut model, Message::Ui(UiMessage::WorkLogView));

    assert_eq!(model.work_log_editor.mode, WorkLogMode::Browse);
}

#[test]
fn test_work_log_add() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;

    update(&mut model, Message::Ui(UiMessage::WorkLogAdd));

    assert_eq!(model.work_log_editor.mode, WorkLogMode::Add);
}

#[test]
fn test_work_log_edit() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.selected = 0;

    update(&mut model, Message::Ui(UiMessage::WorkLogEdit));

    assert_eq!(model.work_log_editor.mode, WorkLogMode::Edit);
}

#[test]
fn test_work_log_confirm_delete() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.mode = WorkLogMode::Browse;

    update(&mut model, Message::Ui(UiMessage::WorkLogConfirmDelete));

    assert_eq!(model.work_log_editor.mode, WorkLogMode::ConfirmDelete);
}

#[test]
fn test_work_log_cancel() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.mode = WorkLogMode::Edit;

    update(&mut model, Message::Ui(UiMessage::WorkLogCancel));

    assert_eq!(model.work_log_editor.mode, WorkLogMode::Browse);
}

#[test]
fn test_work_log_submit_add() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.mode = WorkLogMode::Add;
    // Set buffer content
    model.work_log_editor.buffer = vec!["New work log entry".to_string()];

    let initial_count = model.work_logs.len();

    update(&mut model, Message::Ui(UiMessage::WorkLogSubmit));

    assert_eq!(model.work_logs.len(), initial_count + 1);
    assert_eq!(model.work_log_editor.mode, WorkLogMode::Browse);
}

#[test]
fn test_work_log_submit_empty() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.mode = WorkLogMode::Add;
    model.work_log_editor.buffer = vec!["".to_string()];

    let initial_count = model.work_logs.len();

    update(&mut model, Message::Ui(UiMessage::WorkLogSubmit));

    // Should not add empty entry
    assert_eq!(model.work_logs.len(), initial_count);
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_work_log_delete() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.mode = WorkLogMode::ConfirmDelete;
    model.work_log_editor.selected = 0;

    let initial_count = model.work_logs.len();

    update(&mut model, Message::Ui(UiMessage::WorkLogDelete));

    assert_eq!(model.work_logs.len(), initial_count - 1);
    assert_eq!(model.work_log_editor.mode, WorkLogMode::Browse);
}

#[test]
fn test_work_log_input_char() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.mode = WorkLogMode::Add;
    model.work_log_editor.buffer = vec!["Test".to_string()];
    model.work_log_editor.cursor_col = 4;

    update(&mut model, Message::Ui(UiMessage::WorkLogInputChar('!')));

    assert_eq!(model.work_log_editor.buffer[0], "Test!");
}

#[test]
fn test_work_log_backspace() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.mode = WorkLogMode::Add;
    model.work_log_editor.buffer = vec!["Test".to_string()];
    model.work_log_editor.cursor_col = 4;

    update(&mut model, Message::Ui(UiMessage::WorkLogInputBackspace));

    assert_eq!(model.work_log_editor.buffer[0], "Tes");
}

#[test]
fn test_work_log_newline() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.mode = WorkLogMode::Add;
    model.work_log_editor.buffer = vec!["Line1".to_string()];
    model.work_log_editor.cursor_col = 5;

    update(&mut model, Message::Ui(UiMessage::WorkLogNewline));

    assert_eq!(model.work_log_editor.buffer.len(), 2);
}

#[test]
fn test_work_log_search_start() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.mode = WorkLogMode::Browse;

    update(&mut model, Message::Ui(UiMessage::WorkLogSearchStart));

    assert_eq!(model.work_log_editor.mode, WorkLogMode::Search);
}

#[test]
fn test_work_log_search_cancel() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.mode = WorkLogMode::Search;
    model.work_log_editor.search_query = "test".to_string();

    update(&mut model, Message::Ui(UiMessage::WorkLogSearchCancel));

    assert_eq!(model.work_log_editor.mode, WorkLogMode::Browse);
    assert!(model.work_log_editor.search_query.is_empty());
}

#[test]
fn test_work_log_search_apply() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.mode = WorkLogMode::Search;
    model.work_log_editor.selected = 1;

    update(&mut model, Message::Ui(UiMessage::WorkLogSearchApply));

    assert_eq!(model.work_log_editor.mode, WorkLogMode::Browse);
    assert_eq!(model.work_log_editor.selected, 0);
}

#[test]
fn test_work_log_search_clear() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.search_query = "test".to_string();

    update(&mut model, Message::Ui(UiMessage::WorkLogSearchClear));

    assert!(model.work_log_editor.search_query.is_empty());
}

#[test]
fn test_work_log_search_char() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.mode = WorkLogMode::Search;

    update(&mut model, Message::Ui(UiMessage::WorkLogSearchChar('a')));

    assert_eq!(model.work_log_editor.search_query, "a");
}

#[test]
fn test_work_log_search_backspace() {
    let mut model = create_model_with_work_logs();
    model.work_log_editor.visible = true;
    model.work_log_editor.mode = WorkLogMode::Search;
    model.work_log_editor.search_query = "test".to_string();

    update(&mut model, Message::Ui(UiMessage::WorkLogSearchBackspace));

    assert_eq!(model.work_log_editor.search_query, "tes");
}

// Description editor tests
#[test]
fn test_start_edit_description_multiline() {
    let mut model = Model::new();
    let mut task = Task::new("Test");
    task.description = Some("Test description\nLine 2".to_string());
    model.tasks.insert(task.id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    update(
        &mut model,
        Message::Ui(UiMessage::StartEditDescriptionMultiline),
    );

    assert!(model.description_editor.visible);
}

#[test]
fn test_hide_description_editor() {
    let mut model = Model::new();
    model.description_editor.visible = true;

    update(&mut model, Message::Ui(UiMessage::HideDescriptionEditor));

    assert!(!model.description_editor.visible);
}

#[test]
fn test_description_submit() {
    let mut model = Model::new();
    let task = Task::new("Test");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;
    model.description_editor.visible = true;
    model.description_editor.buffer = vec!["New description".to_string()];

    update(&mut model, Message::Ui(UiMessage::DescriptionSubmit));

    assert!(!model.description_editor.visible);
    assert_eq!(
        model.tasks.get(&task_id).unwrap().description,
        Some("New description".to_string())
    );
}

#[test]
fn test_description_submit_empty() {
    let mut model = Model::new();
    let mut task = Task::new("Test");
    task.description = Some("Existing".to_string());
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;
    model.description_editor.visible = true;
    model.description_editor.buffer = vec!["".to_string()];

    update(&mut model, Message::Ui(UiMessage::DescriptionSubmit));

    // Empty description should be set to None
    assert!(model.tasks.get(&task_id).unwrap().description.is_none());
}

#[test]
fn test_description_input_char() {
    let mut model = Model::new();
    model.description_editor.visible = true;
    model.description_editor.buffer = vec!["Test".to_string()];
    model.description_editor.cursor_col = 4;

    update(
        &mut model,
        Message::Ui(UiMessage::DescriptionInputChar('!')),
    );

    assert_eq!(model.description_editor.buffer[0], "Test!");
}

#[test]
fn test_description_cursor_movements() {
    let mut model = Model::new();
    model.description_editor.visible = true;
    model.description_editor.buffer = vec!["Line 1".to_string(), "Line 2".to_string()];
    model.description_editor.cursor_line = 0;
    model.description_editor.cursor_col = 3;

    // Test cursor left
    update(&mut model, Message::Ui(UiMessage::DescriptionCursorLeft));
    assert_eq!(model.description_editor.cursor_col, 2);

    // Test cursor right
    update(&mut model, Message::Ui(UiMessage::DescriptionCursorRight));
    assert_eq!(model.description_editor.cursor_col, 3);

    // Test cursor down
    update(&mut model, Message::Ui(UiMessage::DescriptionCursorDown));
    assert_eq!(model.description_editor.cursor_line, 1);

    // Test cursor up
    update(&mut model, Message::Ui(UiMessage::DescriptionCursorUp));
    assert_eq!(model.description_editor.cursor_line, 0);

    // Test home
    update(&mut model, Message::Ui(UiMessage::DescriptionCursorHome));
    assert_eq!(model.description_editor.cursor_col, 0);

    // Test end
    update(&mut model, Message::Ui(UiMessage::DescriptionCursorEnd));
    assert_eq!(model.description_editor.cursor_col, 6);
}

#[test]
fn test_description_newline() {
    let mut model = Model::new();
    model.description_editor.visible = true;
    model.description_editor.buffer = vec!["Line1".to_string()];
    model.description_editor.cursor_col = 5;

    update(&mut model, Message::Ui(UiMessage::DescriptionNewline));

    assert_eq!(model.description_editor.buffer.len(), 2);
}
