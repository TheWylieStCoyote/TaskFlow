//! Tests for work log editor widget.

use super::*;
use crate::config::Theme;
use crate::domain::TaskId;
use crate::ui::test_utils::{buffer_content, render_widget};

#[test]
fn test_work_log_editor_empty() {
    let theme = Theme::default();
    let empty_buffer = vec![String::new()];
    let editor = WorkLogEditor::new(
        vec![],
        0,
        WorkLogMode::Browse,
        &empty_buffer,
        0,
        0,
        "",
        &theme,
    );
    let buffer = render_widget(editor, 70, 10);
    let content = buffer_content(&buffer);

    assert!(content.contains("Work Log"));
    assert!(content.contains("No work log entries"));
}

#[test]
fn test_work_log_editor_with_entry() {
    let theme = Theme::default();
    let task_id = TaskId::new();
    let entry = WorkLogEntry::new(task_id, "Test entry content");
    let empty_buffer = vec![String::new()];

    let editor = WorkLogEditor::new(
        vec![&entry],
        0,
        WorkLogMode::Browse,
        &empty_buffer,
        0,
        0,
        "",
        &theme,
    );
    let buffer = render_widget(editor, 80, 10);
    let content = buffer_content(&buffer);

    assert!(content.contains("Work Log"));
    assert!(content.contains("Test entry"));
}

#[test]
fn test_work_log_editor_view_mode() {
    let theme = Theme::default();
    let task_id = TaskId::new();
    let entry = WorkLogEntry::new(task_id, "Full content\nwith multiple\nlines");
    let empty_buffer = vec![String::new()];

    let editor = WorkLogEditor::new(
        vec![&entry],
        0,
        WorkLogMode::View,
        &empty_buffer,
        0,
        0,
        "",
        &theme,
    );
    let buffer = render_widget(editor, 80, 15);
    let content = buffer_content(&buffer);

    assert!(content.contains("View Entry"));
    assert!(content.contains("Full content"));
}

#[test]
fn test_work_log_editor_add_mode() {
    let theme = Theme::default();
    let buffer_content_vec = vec!["First line".to_string(), "Second line".to_string()];

    let editor = WorkLogEditor::new(
        vec![],
        0,
        WorkLogMode::Add,
        &buffer_content_vec,
        0,
        5,
        "",
        &theme,
    );
    let buffer = render_widget(editor, 80, 15);
    let content = buffer_content(&buffer);

    assert!(content.contains("Add Entry"));
    assert!(content.contains("First line"));
}

#[test]
fn test_work_log_editor_confirm_delete() {
    let theme = Theme::default();
    let task_id = TaskId::new();
    let entry = WorkLogEntry::new(task_id, "Entry to delete");
    let empty_buffer = vec![String::new()];

    let editor = WorkLogEditor::new(
        vec![&entry],
        0,
        WorkLogMode::ConfirmDelete,
        &empty_buffer,
        0,
        0,
        "",
        &theme,
    );
    let buffer = render_widget(editor, 80, 10);
    let content = buffer_content(&buffer);

    assert!(content.contains("Confirm Delete"));
    assert!(content.contains("Entry to delete"));
}

#[test]
fn test_work_log_search_mode() {
    let theme = Theme::default();
    let task_id = TaskId::new();
    let entry1 = WorkLogEntry::new(task_id, "Meeting notes from Monday");
    let entry2 = WorkLogEntry::new(task_id, "Bug fix details");
    let empty_buffer = vec![String::new()];

    let editor = WorkLogEditor::new(
        vec![&entry1, &entry2],
        0,
        WorkLogMode::Search,
        &empty_buffer,
        0,
        0,
        "meeting",
        &theme,
    );
    let buffer = render_widget(editor, 80, 15);
    let content = buffer_content(&buffer);

    assert!(content.contains("Search Work Log"));
    assert!(content.contains("2 matches")); // Shows count of filtered entries passed in
}

#[test]
fn test_truncate_string() {
    assert_eq!(truncate_string("short", 10), "short");
    assert_eq!(truncate_string("very long string here", 10), "very lo...");
}
