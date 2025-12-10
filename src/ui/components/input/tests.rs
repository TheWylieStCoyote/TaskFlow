//! Tests for input components.

use super::*;
use crate::domain::TaskId;
use crate::ui::test_utils::{buffer_content, render_widget, test_theme};

// InputMode tests
#[test]
fn test_input_mode_default_is_normal() {
    let mode = InputMode::default();
    assert_eq!(mode, InputMode::Normal);
}

#[test]
fn test_input_mode_equality() {
    assert_eq!(InputMode::Normal, InputMode::Normal);
    assert_eq!(InputMode::Editing, InputMode::Editing);
    assert_ne!(InputMode::Normal, InputMode::Editing);
}

// InputTarget tests
#[test]
fn test_input_target_default_is_task() {
    let target = InputTarget::default();
    assert_eq!(target, InputTarget::Task);
}

#[test]
fn test_input_target_variants() {
    let task_id = TaskId::new();

    // Test each variant can be created
    let _ = InputTarget::Task;
    let _ = InputTarget::Subtask(task_id);
    let _ = InputTarget::EditTask(task_id);
    let _ = InputTarget::EditDueDate(task_id);
    let _ = InputTarget::EditTags(task_id);
    let _ = InputTarget::EditDescription(task_id);
    let _ = InputTarget::Project;
    let _ = InputTarget::Search;
    let _ = InputTarget::MoveToProject(task_id);
    let _ = InputTarget::FilterByTag;
    let _ = InputTarget::BulkMoveToProject;
    let _ = InputTarget::BulkSetStatus;
    let _ = InputTarget::EditDependencies(task_id);
    let _ = InputTarget::EditRecurrence(task_id);
}

// InputDialog tests
#[test]
fn test_input_dialog_renders_title() {
    let theme = test_theme();
    let dialog = InputDialog::new("New Task", "", 0, &theme);
    let buffer = render_widget(dialog, 40, 5);
    let content = buffer_content(&buffer);

    assert!(content.contains("New Task"), "Title should be visible");
}

#[test]
fn test_input_dialog_renders_input_text() {
    let theme = test_theme();
    let dialog = InputDialog::new("Edit", "Hello World", 11, &theme);
    let buffer = render_widget(dialog, 40, 5);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Hello World"),
        "Input text should be visible"
    );
}

#[test]
fn test_input_dialog_shows_cursor() {
    let theme = test_theme();
    let dialog = InputDialog::new("Test", "abc", 3, &theme);
    let buffer = render_widget(dialog, 40, 5);
    let content = buffer_content(&buffer);

    // Cursor indicator should be present
    assert!(content.contains('▌'), "Cursor indicator should be visible");
}

#[test]
fn test_input_dialog_cursor_in_middle() {
    let theme = test_theme();
    let dialog = InputDialog::new("Test", "abcdef", 3, &theme);
    let buffer = render_widget(dialog, 40, 5);
    let content = buffer_content(&buffer);

    // With cursor in the middle, we should see text before and after
    assert!(
        content.contains("abc"),
        "Text before cursor should be visible"
    );
}

#[test]
fn test_input_dialog_empty_input() {
    let theme = test_theme();
    let dialog = InputDialog::new("New", "", 0, &theme);
    let buffer = render_widget(dialog, 40, 5);
    let content = buffer_content(&buffer);

    // Should still show cursor
    assert!(
        content.contains('▌'),
        "Cursor should be visible even with empty input"
    );
}

// ConfirmDialog tests
#[test]
fn test_confirm_dialog_renders_title() {
    let theme = test_theme();
    let dialog = ConfirmDialog::new("Confirm Delete", "Are you sure?", &theme);
    let buffer = render_widget(dialog, 40, 8);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Confirm Delete"),
        "Title should be visible"
    );
}

#[test]
fn test_confirm_dialog_renders_message() {
    let theme = test_theme();
    let dialog = ConfirmDialog::new("Delete", "Delete this task?", &theme);
    let buffer = render_widget(dialog, 40, 8);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Delete this task"),
        "Message should be visible"
    );
}

#[test]
fn test_confirm_dialog_shows_yes_no_options() {
    let theme = test_theme();
    let dialog = ConfirmDialog::new("Confirm", "Proceed?", &theme);
    let buffer = render_widget(dialog, 40, 8);
    let content = buffer_content(&buffer);

    assert!(content.contains("[y]es"), "Yes option should be visible");
    assert!(content.contains("[n]o"), "No option should be visible");
}

// QuickCaptureDialog tests
#[test]
fn test_quick_capture_dialog_renders() {
    let theme = test_theme();
    let dialog = QuickCaptureDialog::new("Buy groceries", 13, &theme);
    let buffer = render_widget(dialog, 80, 10);
    let content = buffer_content(&buffer);

    assert!(content.contains("Quick Capture"), "Title should be visible");
    assert!(
        content.contains("Buy groceries"),
        "Input text should be visible"
    );
}

#[test]
fn test_quick_capture_dialog_shows_hints() {
    let theme = test_theme();
    let dialog = QuickCaptureDialog::new("", 0, &theme);
    let buffer = render_widget(dialog, 80, 10);
    let content = buffer_content(&buffer);

    assert!(content.contains("#tag"), "Tag hint should be visible");
    assert!(
        content.contains("@project"),
        "Project hint should be visible"
    );
    assert!(
        content.contains("!priority"),
        "Priority hint should be visible"
    );
}

#[test]
fn test_quick_capture_dialog_shows_cursor() {
    let theme = test_theme();
    let dialog = QuickCaptureDialog::new("test", 2, &theme);
    let buffer = render_widget(dialog, 80, 10);
    let content = buffer_content(&buffer);

    assert!(content.contains('▌'), "Cursor indicator should be visible");
}

#[test]
fn test_quick_capture_dialog_empty_input() {
    let theme = test_theme();
    let dialog = QuickCaptureDialog::new("", 0, &theme);
    let buffer = render_widget(dialog, 80, 10);
    let content = buffer_content(&buffer);

    assert!(
        content.contains('▌'),
        "Cursor should be visible with empty input"
    );
}

// OverdueAlert tests
#[test]
fn test_overdue_alert_singular() {
    let theme = test_theme();
    let alert = OverdueAlert::new(1, vec!["Buy milk".to_string()], &theme);
    let buffer = render_widget(alert, 60, 15);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("1 overdue task"),
        "Should show singular form"
    );
    assert!(content.contains("Buy milk"), "Task title should be visible");
}

#[test]
fn test_overdue_alert_plural() {
    let theme = test_theme();
    let alert = OverdueAlert::new(
        3,
        vec![
            "Task 1".to_string(),
            "Task 2".to_string(),
            "Task 3".to_string(),
        ],
        &theme,
    );
    let buffer = render_widget(alert, 60, 15);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("3 overdue tasks"),
        "Should show plural form"
    );
}

#[test]
fn test_overdue_alert_truncates_long_list() {
    let theme = test_theme();
    let tasks: Vec<String> = (1..=10).map(|i| format!("Task {i}")).collect();
    let alert = OverdueAlert::new(10, tasks, &theme);
    let buffer = render_widget(alert, 60, 15);
    let content = buffer_content(&buffer);

    assert!(content.contains("and 5 more"), "Should show overflow count");
}

#[test]
fn test_overdue_alert_shows_dismiss_message() {
    let theme = test_theme();
    let alert = OverdueAlert::new(1, vec!["Task".to_string()], &theme);
    let buffer = render_widget(alert, 60, 15);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Press any key"),
        "Dismiss message should be visible"
    );
}

// StorageErrorAlert tests
#[test]
fn test_storage_error_alert_renders() {
    let theme = test_theme();
    let alert = StorageErrorAlert::new("File not found: tasks.json", &theme);
    let buffer = render_widget(alert, 60, 15);
    let content = buffer_content(&buffer);

    assert!(content.contains("Storage Error"), "Title should be visible");
    assert!(
        content.contains("File not found"),
        "Error message should be visible"
    );
}

#[test]
fn test_storage_error_alert_shows_sample_data_message() {
    let theme = test_theme();
    let alert = StorageErrorAlert::new("Error", &theme);
    let buffer = render_widget(alert, 70, 15);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("sample data"),
        "Sample data message should be visible"
    );
}

#[test]
fn test_storage_error_alert_shows_continue_message() {
    let theme = test_theme();
    let alert = StorageErrorAlert::new("Error", &theme);
    let buffer = render_widget(alert, 70, 15);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Press any key"),
        "Continue message should be visible"
    );
}

// Edge cases for cursor handling
#[test]
fn test_input_dialog_cursor_at_end() {
    let theme = test_theme();
    let dialog = InputDialog::new("Test", "hello", 5, &theme);
    let buffer = render_widget(dialog, 40, 5);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("hello"),
        "Text should be visible with cursor at end"
    );
}

#[test]
fn test_input_dialog_cursor_beyond_length() {
    let theme = test_theme();
    // Cursor position beyond string length should be clamped
    let dialog = InputDialog::new("Test", "abc", 100, &theme);
    let _ = render_widget(dialog, 40, 5);
    // Should not panic
}

#[test]
fn test_quick_capture_cursor_beyond_length() {
    let theme = test_theme();
    // Cursor position beyond string length should be clamped
    let dialog = QuickCaptureDialog::new("abc", 100, &theme);
    let _ = render_widget(dialog, 80, 10);
    // Should not panic
}

// Additional InputTarget tests
#[test]
fn test_input_target_scheduled_date() {
    let task_id = TaskId::new();
    let target = InputTarget::EditScheduledDate(task_id);
    assert!(matches!(target, InputTarget::EditScheduledDate(_)));
}

#[test]
fn test_input_target_import_format() {
    use crate::storage::ImportFormat;
    let target = InputTarget::ImportFilePath(ImportFormat::Csv);
    assert!(matches!(target, InputTarget::ImportFilePath(_)));
}

#[test]
fn test_input_target_snooze() {
    let task_id = TaskId::new();
    let target = InputTarget::SnoozeTask(task_id);
    assert!(matches!(target, InputTarget::SnoozeTask(_)));
}

#[test]
fn test_input_target_estimate() {
    let task_id = TaskId::new();
    let target = InputTarget::EditEstimate(task_id);
    assert!(matches!(target, InputTarget::EditEstimate(_)));
}

#[test]
fn test_input_target_new_habit() {
    let target = InputTarget::NewHabit;
    assert!(matches!(target, InputTarget::NewHabit));
}

#[test]
fn test_input_target_quick_capture() {
    let target = InputTarget::QuickCapture;
    assert!(matches!(target, InputTarget::QuickCapture));
}
