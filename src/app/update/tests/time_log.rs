//! Time log editor tests.

use chrono::{Duration, Utc};

use crate::app::{update::update, Message, Model, UiMessage};
use crate::domain::{Task, TimeEntry};
use crate::ui::TimeLogMode;

fn create_model_with_time_entries() -> Model {
    let mut model = Model::new();

    // Create a task
    let task = Task::new("Test task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    // Create some time entries
    let now = Utc::now();
    let entry1 = {
        let mut e = TimeEntry::start(task_id);
        e.started_at = now - Duration::hours(3);
        e.ended_at = Some(now - Duration::hours(2));
        e.duration_minutes = Some(60);
        e
    };
    model.time_entries.insert(entry1.id, entry1);

    let entry2 = {
        let mut e = TimeEntry::start(task_id);
        e.started_at = now - Duration::hours(1);
        e.ended_at = Some(now);
        e.duration_minutes = Some(60);
        e
    };
    model.time_entries.insert(entry2.id, entry2);

    model
}

#[test]
fn test_show_time_log() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = false;

    update(&mut model, Message::Ui(UiMessage::ShowTimeLog));

    assert!(model.time_log.visible);
    assert_eq!(model.time_log.selected, 0);
    assert_eq!(model.time_log.mode, TimeLogMode::Browse);
}

#[test]
fn test_show_time_log_no_task_selected() {
    let mut model = Model::new();
    model.time_log.visible = false;

    update(&mut model, Message::Ui(UiMessage::ShowTimeLog));

    // Should not show if no task selected
    assert!(!model.time_log.visible);
}

#[test]
fn test_hide_time_log() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::EditStart;
    model.time_log.buffer = "test".to_string();

    update(&mut model, Message::Ui(UiMessage::HideTimeLog));

    assert!(!model.time_log.visible);
    assert_eq!(model.time_log.mode, TimeLogMode::Browse);
    assert!(model.time_log.buffer.is_empty());
}

#[test]
fn test_time_log_up() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.selected = 1;

    update(&mut model, Message::Ui(UiMessage::TimeLogUp));

    assert_eq!(model.time_log.selected, 0);
}

#[test]
fn test_time_log_up_at_zero() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.selected = 0;

    update(&mut model, Message::Ui(UiMessage::TimeLogUp));

    assert_eq!(model.time_log.selected, 0);
}

#[test]
fn test_time_log_down() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.selected = 0;

    update(&mut model, Message::Ui(UiMessage::TimeLogDown));

    assert_eq!(model.time_log.selected, 1);
}

#[test]
fn test_time_log_down_at_end() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.selected = 1;

    update(&mut model, Message::Ui(UiMessage::TimeLogDown));

    // Should stay at end
    assert_eq!(model.time_log.selected, 1);
}

#[test]
fn test_time_log_edit_start() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.selected = 0;

    update(&mut model, Message::Ui(UiMessage::TimeLogEditStart));

    assert_eq!(model.time_log.mode, TimeLogMode::EditStart);
    assert!(!model.time_log.buffer.is_empty());
}

#[test]
fn test_time_log_edit_end() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.selected = 0;

    update(&mut model, Message::Ui(UiMessage::TimeLogEditEnd));

    assert_eq!(model.time_log.mode, TimeLogMode::EditEnd);
}

#[test]
fn test_time_log_confirm_delete() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::Browse;

    update(&mut model, Message::Ui(UiMessage::TimeLogConfirmDelete));

    assert_eq!(model.time_log.mode, TimeLogMode::ConfirmDelete);
}

#[test]
fn test_time_log_cancel() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::EditStart;
    model.time_log.buffer = "12:30".to_string();

    update(&mut model, Message::Ui(UiMessage::TimeLogCancel));

    assert_eq!(model.time_log.mode, TimeLogMode::Browse);
    assert!(model.time_log.buffer.is_empty());
}

#[test]
fn test_time_log_add_entry() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    let initial_count = model.time_entries.len();

    update(&mut model, Message::Ui(UiMessage::TimeLogAddEntry));

    assert_eq!(model.time_entries.len(), initial_count + 1);
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_time_log_delete() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::ConfirmDelete;
    model.time_log.selected = 0;
    let initial_count = model.time_entries.len();

    update(&mut model, Message::Ui(UiMessage::TimeLogDelete));

    assert_eq!(model.time_entries.len(), initial_count - 1);
    assert_eq!(model.time_log.mode, TimeLogMode::Browse);
}

#[test]
fn test_time_log_submit_edit_start() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::EditStart;
    model.time_log.selected = 0;
    model.time_log.buffer = "10:30".to_string();

    update(&mut model, Message::Ui(UiMessage::TimeLogSubmit));

    assert_eq!(model.time_log.mode, TimeLogMode::Browse);
    assert!(model.time_log.buffer.is_empty());
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_time_log_submit_edit_end() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::EditEnd;
    model.time_log.selected = 0;
    model.time_log.buffer = "11:30".to_string();

    update(&mut model, Message::Ui(UiMessage::TimeLogSubmit));

    assert_eq!(model.time_log.mode, TimeLogMode::Browse);
    assert!(model.time_log.buffer.is_empty());
}

#[test]
fn test_time_log_submit_invalid_format() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::EditStart;
    model.time_log.selected = 0;
    model.time_log.buffer = "invalid".to_string();

    update(&mut model, Message::Ui(UiMessage::TimeLogSubmit));

    // Should show error message
    assert!(model.alerts.status_message.is_some());
    assert!(model
        .alerts
        .status_message
        .as_ref()
        .unwrap()
        .contains("Invalid"));
}
