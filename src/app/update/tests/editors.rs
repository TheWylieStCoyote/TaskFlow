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
    model.selected_index = 0;

    // Create work log entries
    let entry1 = WorkLogEntry::new(task_id, "First work log entry");
    model.work_logs.insert(entry1.id, entry1);

    let entry2 = WorkLogEntry::new(task_id, "Second work log entry");
    model.work_logs.insert(entry2.id, entry2);

    // Refresh after all data is inserted so caches are populated
    model.refresh_visible_tasks();

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
    model.work_log_editor.buffer = vec![String::new()];

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
    model.description_editor.buffer = vec![String::new()];

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

// ============================================================================
// Remaining work log editor arms and branches
// ============================================================================

mod work_log_remaining {
    use crate::app::{update::update, Message, Model, UiMessage};
    use crate::domain::{Task, WorkLogEntry};
    use crate::ui::WorkLogMode;

    use super::create_model_with_work_logs;

    fn ui(model: &mut Model, msg: UiMessage) {
        update(model, Message::Ui(msg));
    }

    fn editing_model(mode: WorkLogMode) -> Model {
        let mut model = create_model_with_work_logs();
        model.work_log_editor.visible = true;
        model.work_log_editor.mode = mode;
        model.work_log_editor.buffer = vec!["first line".to_string(), "second".to_string()];
        model.work_log_editor.cursor_line = 0;
        model.work_log_editor.cursor_col = 0;
        model
    }

    #[test]
    fn test_work_log_input_delete_removes_char_under_cursor() {
        let mut model = editing_model(WorkLogMode::Add);
        ui(&mut model, UiMessage::WorkLogInputDelete);
        assert_eq!(model.work_log_editor.buffer[0], "irst line");
        assert_eq!(model.work_log_editor.cursor_col, 0);
    }

    #[test]
    fn test_work_log_cursor_movement_in_edit_mode() {
        let mut model = editing_model(WorkLogMode::Edit);

        ui(&mut model, UiMessage::WorkLogCursorRight);
        ui(&mut model, UiMessage::WorkLogCursorRight);
        assert_eq!(model.work_log_editor.cursor_col, 2);

        ui(&mut model, UiMessage::WorkLogCursorLeft);
        assert_eq!(model.work_log_editor.cursor_col, 1);

        ui(&mut model, UiMessage::WorkLogCursorEnd);
        assert_eq!(model.work_log_editor.cursor_col, "first line".len());

        ui(&mut model, UiMessage::WorkLogCursorHome);
        assert_eq!(model.work_log_editor.cursor_col, 0);

        ui(&mut model, UiMessage::WorkLogCursorDown);
        assert_eq!(model.work_log_editor.cursor_line, 1);

        ui(&mut model, UiMessage::WorkLogCursorDown);
        assert_eq!(model.work_log_editor.cursor_line, 1, "stays on last line");

        ui(&mut model, UiMessage::WorkLogCursorUp);
        assert_eq!(model.work_log_editor.cursor_line, 0);

        ui(&mut model, UiMessage::WorkLogCursorUp);
        assert_eq!(model.work_log_editor.cursor_line, 0, "stays on first line");
    }

    #[test]
    fn test_work_log_editing_messages_ignored_outside_add_or_edit() {
        let mut model = editing_model(WorkLogMode::Browse);
        let before = model.work_log_editor.buffer.clone();

        for msg in [
            UiMessage::WorkLogInputChar('x'),
            UiMessage::WorkLogInputBackspace,
            UiMessage::WorkLogInputDelete,
            UiMessage::WorkLogCursorLeft,
            UiMessage::WorkLogCursorRight,
            UiMessage::WorkLogCursorUp,
            UiMessage::WorkLogCursorDown,
            UiMessage::WorkLogNewline,
            UiMessage::WorkLogCursorHome,
            UiMessage::WorkLogCursorEnd,
        ] {
            ui(&mut model, msg);
        }

        assert_eq!(model.work_log_editor.buffer, before);
        assert_eq!(model.work_log_editor.cursor_line, 0);
        assert_eq!(model.work_log_editor.cursor_col, 0);
    }

    #[test]
    fn test_work_log_search_input_ignored_outside_search_mode() {
        let mut model = editing_model(WorkLogMode::Browse);
        model.work_log_editor.search_query = "abc".to_string();

        ui(&mut model, UiMessage::WorkLogSearchChar('d'));
        ui(&mut model, UiMessage::WorkLogSearchBackspace);

        assert_eq!(model.work_log_editor.search_query, "abc");
    }

    #[test]
    fn test_work_log_submit_in_browse_mode_saves_nothing() {
        let mut model = editing_model(WorkLogMode::Browse);
        let count = model.work_logs.len();

        ui(&mut model, UiMessage::WorkLogSubmit);

        assert_eq!(model.work_logs.len(), count);
        assert!(model.alerts.status_message.is_none());
        assert_eq!(model.work_log_editor.mode, WorkLogMode::Browse);
        assert_eq!(model.work_log_editor.buffer, vec![String::new()]);
    }

    #[test]
    fn test_work_log_submit_edit_updates_selected_entry() {
        let mut model = create_model_with_work_logs();
        model.work_log_editor.visible = true;
        model.work_log_editor.selected = 1;
        let task_id = model.selected_task_id().unwrap();
        let target_id = model.work_logs_for_task(&task_id)[1].id;

        ui(&mut model, UiMessage::WorkLogEdit);
        assert_eq!(model.work_log_editor.mode, WorkLogMode::Edit);
        model.work_log_editor.buffer = vec!["Rewritten".to_string(), "entry".to_string()];

        ui(&mut model, UiMessage::WorkLogSubmit);

        assert_eq!(model.work_logs[&target_id].content, "Rewritten\nentry");
        assert_eq!(
            model.alerts.status_message.as_deref(),
            Some("Work log entry updated")
        );
        assert_eq!(model.work_log_editor.mode, WorkLogMode::Browse);
        assert_eq!(model.undo_stack.len(), 1);
    }

    #[test]
    fn test_work_log_submit_edit_with_stale_selection_is_noop() {
        let mut model = editing_model(WorkLogMode::Edit);
        model.work_log_editor.selected = 42;
        let before: Vec<_> = model
            .work_logs
            .values()
            .map(|e| e.content.clone())
            .collect();

        ui(&mut model, UiMessage::WorkLogSubmit);

        let after: Vec<_> = model
            .work_logs
            .values()
            .map(|e| e.content.clone())
            .collect();
        assert_eq!(before.len(), after.len());
        assert!(model.alerts.status_message.is_none());
        assert_eq!(model.work_log_editor.mode, WorkLogMode::Browse);
    }

    #[test]
    fn test_work_log_submit_without_selected_task_only_resets_mode() {
        let mut model = Model::new();
        model.work_log_editor.visible = true;
        model.work_log_editor.mode = WorkLogMode::Add;
        model.work_log_editor.buffer = vec!["orphan".to_string()];

        ui(&mut model, UiMessage::WorkLogSubmit);

        assert!(model.work_logs.is_empty());
        assert_eq!(model.work_log_editor.mode, WorkLogMode::Browse);
    }

    #[test]
    fn test_work_log_view_requires_entry_at_selection() {
        let mut model = Model::new();
        let task = Task::new("No logs");
        model.tasks.insert(task.id, task);
        model.refresh_visible_tasks();
        model.work_log_editor.visible = true;

        ui(&mut model, UiMessage::WorkLogView);

        assert_eq!(model.work_log_editor.mode, WorkLogMode::Browse);
    }

    #[test]
    fn test_work_log_view_in_add_mode_is_ignored() {
        let mut model = editing_model(WorkLogMode::Add);
        ui(&mut model, UiMessage::WorkLogView);
        assert_eq!(model.work_log_editor.mode, WorkLogMode::Add);
    }

    #[test]
    fn test_work_log_down_stops_at_last_entry() {
        let mut model = create_model_with_work_logs();
        model.work_log_editor.visible = true;
        model.work_log_editor.selected = 1;
        ui(&mut model, UiMessage::WorkLogDown);
        assert_eq!(model.work_log_editor.selected, 1);
    }

    #[test]
    fn test_work_log_up_stops_at_zero() {
        let mut model = create_model_with_work_logs();
        model.work_log_editor.visible = true;
        model.work_log_editor.selected = 0;
        ui(&mut model, UiMessage::WorkLogUp);
        assert_eq!(model.work_log_editor.selected, 0);
    }

    #[test]
    fn test_work_log_edit_with_stale_selection_stays_in_browse() {
        let mut model = create_model_with_work_logs();
        model.work_log_editor.visible = true;
        model.work_log_editor.selected = 10;
        ui(&mut model, UiMessage::WorkLogEdit);
        assert_eq!(model.work_log_editor.mode, WorkLogMode::Browse);
    }

    #[test]
    fn test_work_log_confirm_delete_from_view_mode() {
        let mut model = create_model_with_work_logs();
        model.work_log_editor.visible = true;
        model.work_log_editor.mode = WorkLogMode::View;
        ui(&mut model, UiMessage::WorkLogConfirmDelete);
        assert_eq!(model.work_log_editor.mode, WorkLogMode::ConfirmDelete);

        // Not allowed while editing
        model.work_log_editor.mode = WorkLogMode::Edit;
        ui(&mut model, UiMessage::WorkLogConfirmDelete);
        assert_eq!(model.work_log_editor.mode, WorkLogMode::Edit);
    }

    #[test]
    fn test_work_log_delete_last_entry_adjusts_selection() {
        let mut model = create_model_with_work_logs();
        model.work_log_editor.visible = true;
        model.work_log_editor.mode = WorkLogMode::ConfirmDelete;
        model.work_log_editor.selected = 1;

        ui(&mut model, UiMessage::WorkLogDelete);

        assert_eq!(model.work_logs.len(), 1);
        assert_eq!(model.work_log_editor.selected, 0);
        assert_eq!(
            model.alerts.status_message.as_deref(),
            Some("Work log entry deleted")
        );
    }

    #[test]
    fn test_work_log_delete_ignored_without_confirmation() {
        let mut model = create_model_with_work_logs();
        model.work_log_editor.visible = true;
        model.work_log_editor.mode = WorkLogMode::Browse;
        ui(&mut model, UiMessage::WorkLogDelete);
        assert_eq!(model.work_logs.len(), 2);
    }

    #[test]
    fn test_work_log_delete_with_stale_selection_returns_to_browse() {
        let mut model = create_model_with_work_logs();
        model.work_log_editor.visible = true;
        model.work_log_editor.mode = WorkLogMode::ConfirmDelete;
        model.work_log_editor.selected = 7;
        ui(&mut model, UiMessage::WorkLogDelete);
        assert_eq!(model.work_logs.len(), 2);
        assert_eq!(model.work_log_editor.mode, WorkLogMode::Browse);
    }

    #[test]
    fn test_work_log_add_entry_is_listed_first() {
        let mut model = create_model_with_work_logs();
        let task_id = model.selected_task_id().unwrap();
        model.work_log_editor.visible = true;
        model.work_log_editor.mode = WorkLogMode::Add;
        model.work_log_editor.selected = 1;
        model.work_log_editor.buffer = vec!["Newest".to_string()];

        ui(&mut model, UiMessage::WorkLogSubmit);

        assert_eq!(model.work_log_editor.selected, 0);
        assert_eq!(
            model.alerts.status_message.as_deref(),
            Some("Work log entry added")
        );
        let logs: Vec<_> = model
            .work_logs
            .values()
            .filter(|e| e.task_id == task_id)
            .collect();
        assert_eq!(logs.len(), 3);
        assert!(logs.iter().any(|e| e.content == "Newest"));
    }

    #[test]
    fn test_description_input_delete() {
        let mut model = create_model_with_work_logs();
        model.description_editor.visible = true;
        model.description_editor.buffer = vec!["hello".to_string()];
        model.description_editor.cursor_line = 0;
        model.description_editor.cursor_col = 1;

        ui(&mut model, UiMessage::DescriptionInputDelete);
        assert_eq!(model.description_editor.buffer[0], "hllo");

        model.description_editor.visible = false;
        ui(&mut model, UiMessage::DescriptionInputDelete);
        assert_eq!(
            model.description_editor.buffer[0], "hllo",
            "ignored when hidden"
        );
    }

    #[test]
    fn test_description_cursor_end_ignored_when_hidden() {
        let mut model = Model::new();
        model.description_editor.visible = false;
        model.description_editor.buffer = vec!["abc".to_string()];
        ui(&mut model, UiMessage::DescriptionCursorEnd);
        assert_eq!(model.description_editor.cursor_col, 0);
    }

    #[test]
    fn test_work_log_entries_for_other_task_are_not_edited() {
        let mut model = create_model_with_work_logs();
        let other = Task::new("Other task");
        let other_entry = WorkLogEntry::new(other.id, "Other content");
        let other_entry_id = other_entry.id;
        model.tasks.insert(other.id, other);
        model.work_logs.insert(other_entry_id, other_entry);
        model.refresh_visible_tasks();
        model.selected_index = model
            .visible_tasks
            .iter()
            .position(|id| model.tasks[id].title == "Test task")
            .unwrap();
        model.work_log_editor.visible = true;
        model.work_log_editor.mode = WorkLogMode::Edit;
        model.work_log_editor.selected = 0;
        model.work_log_editor.buffer = vec!["changed".to_string()];

        ui(&mut model, UiMessage::WorkLogSubmit);

        assert_eq!(model.work_logs[&other_entry_id].content, "Other content");
    }
}
