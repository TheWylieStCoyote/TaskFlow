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

    // Refresh after all data is inserted so caches are populated
    model.refresh_visible_tasks();

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

#[test]
fn add_entry_updates_task_total_and_cache() {
    let mut model = create_model_with_time_entries();
    let task_id = model.selected_task_id().unwrap();
    model.time_log.visible = true;
    assert_eq!(
        model.tasks[&task_id].actual_minutes, 0,
        "fixture never stopped a timer"
    );

    update(&mut model, Message::Ui(UiMessage::TimeLogAddEntry));

    assert_eq!(
        model.tasks[&task_id].actual_minutes, 150,
        "60 + 60 + the new 30"
    );
    assert_eq!(model.task_cache.time_sums.get(&task_id), Some(&150));
    assert_eq!(model.time_entries_for_task(&task_id).len(), 3);
}

#[test]
fn delete_entry_updates_task_total() {
    let mut model = create_model_with_time_entries();
    let task_id = model.selected_task_id().unwrap();
    model.recompute_task_time(&task_id);
    assert_eq!(model.tasks[&task_id].actual_minutes, 120);
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::ConfirmDelete;
    model.time_log.selected = 0;

    update(&mut model, Message::Ui(UiMessage::TimeLogDelete));

    assert_eq!(model.tasks[&task_id].actual_minutes, 60);
    assert_eq!(model.time_entries_for_task(&task_id).len(), 1);
}

#[test]
fn edit_start_is_interpreted_as_local_clock_time() {
    let mut model = create_model_with_time_entries();
    let task_id = model.selected_task_id().unwrap();
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::EditStart;
    model.time_log.selected = 0;
    model.time_log.buffer = "08:15".to_string();

    update(&mut model, Message::Ui(UiMessage::TimeLogSubmit));

    let entry = model.time_entries_for_task(&task_id)[0];
    let local = entry.started_at.with_timezone(&chrono::Local);
    assert_eq!(local.format("%H:%M").to_string(), "08:15");
    assert_eq!(
        model.tasks[&task_id].actual_minutes,
        model.total_time_for_task(&task_id)
    );
}

#[test]
fn edit_end_before_start_rolls_over_to_next_day() {
    let mut model = Model::new();
    let task = Task::new("Night shift");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.selected_index = 0;

    // Start at 23:00 local today, currently ending at 23:30.
    let local_start = chrono::Local::now()
        .date_naive()
        .and_hms_opt(23, 0, 0)
        .unwrap();
    let started_at = chrono::TimeZone::from_local_datetime(&chrono::Local, &local_start)
        .single()
        .unwrap()
        .with_timezone(&Utc);
    let mut entry = TimeEntry::start(task_id);
    entry.started_at = started_at;
    entry.ended_at = Some(started_at + Duration::minutes(30));
    entry.duration_minutes = Some(30);
    model.time_entries.insert(entry.id, entry);
    model.refresh_visible_tasks();

    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::EditEnd;
    model.time_log.selected = 0;
    model.time_log.buffer = "01:30".to_string();
    update(&mut model, Message::Ui(UiMessage::TimeLogSubmit));

    let entry = model.time_entries_for_task(&task_id)[0];
    assert_eq!(entry.duration_minutes, Some(150), "23:00 to 01:30 next day");
    assert!(entry.ended_at.unwrap() > entry.started_at);
    assert_eq!(model.tasks[&task_id].actual_minutes, 150);
}
