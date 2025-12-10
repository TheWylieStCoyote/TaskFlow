//! System message tests (quit, resize).

use crate::app::{update::update, Message, Model, RunningState, SystemMessage, TimeMessage};

use super::create_test_model_with_tasks;

#[test]
fn test_system_quit_preserves_timer() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];
    model.start_time_tracking(task_id);

    assert!(model.active_time_entry.is_some());

    update(&mut model, Message::System(SystemMessage::Quit));

    // Timer should persist across app restarts (not stopped on quit)
    assert!(model.active_time_entry.is_some());
    assert_eq!(model.running, RunningState::Quitting);
}

#[test]
fn test_system_resize() {
    let mut model = Model::new();

    update(
        &mut model,
        Message::System(SystemMessage::Resize {
            width: 120,
            height: 40,
        }),
    );

    assert_eq!(model.terminal_size, (120, 40));
}

#[test]
fn test_time_toggle_tracking_start() {
    let mut model = create_test_model_with_tasks();
    assert!(model.active_time_entry.is_none());

    update(&mut model, Message::Time(TimeMessage::ToggleTracking));

    assert!(model.active_time_entry.is_some());
}

#[test]
fn test_time_toggle_tracking_stop() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];
    model.start_time_tracking(task_id);

    update(&mut model, Message::Time(TimeMessage::ToggleTracking));

    assert!(model.active_time_entry.is_none());
}

// ============================================================================
// Undo/Redo Tests
// ============================================================================

#[test]
fn test_undo_task_created() {
    use crate::app::TaskMessage;

    let mut model = Model::new();
    assert_eq!(model.tasks.len(), 0);

    // Create a task
    update(&mut model, Message::Task(TaskMessage::Create("Test task".into())));
    assert_eq!(model.tasks.len(), 1);

    // Undo should remove the task
    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(model.tasks.len(), 0);
    assert!(model.alerts.status_message.as_ref().is_some_and(|m| m.contains("Undone")));

    // Redo should restore it
    update(&mut model, Message::System(SystemMessage::Redo));
    assert_eq!(model.tasks.len(), 1);
    assert!(model.alerts.status_message.as_ref().is_some_and(|m| m.contains("Redone")));
}

#[test]
fn test_undo_task_deleted() {
    use crate::app::TaskMessage;

    let mut model = create_test_model_with_tasks();
    let initial_count = model.tasks.len();
    let task_id = model.visible_tasks[0];

    // Delete a task
    update(&mut model, Message::Task(TaskMessage::Delete(task_id)));
    assert_eq!(model.tasks.len(), initial_count - 1);

    // Undo should restore it
    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(model.tasks.len(), initial_count);
    assert!(model.tasks.contains_key(&task_id));
}

#[test]
fn test_undo_task_modified() {
    use crate::app::TaskMessage;
    use crate::domain::Priority;

    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];
    let original_priority = model.tasks.get(&task_id).unwrap().priority;

    // Modify the task priority
    update(
        &mut model,
        Message::Task(TaskMessage::SetPriority(task_id, Priority::High)),
    );
    assert_eq!(model.tasks.get(&task_id).unwrap().priority, Priority::High);

    // Undo should restore original priority
    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(model.tasks.get(&task_id).unwrap().priority, original_priority);
}

#[test]
fn test_undo_empty_stack() {
    let mut model = Model::new();
    let initial_status = model.alerts.status_message.clone();

    // Undo on empty stack should do nothing (no panic)
    update(&mut model, Message::System(SystemMessage::Undo));

    // No "Undone" message should appear
    assert_eq!(model.alerts.status_message, initial_status);
}

#[test]
fn test_redo_empty_stack() {
    let mut model = Model::new();
    let initial_status = model.alerts.status_message.clone();

    // Redo on empty stack should do nothing (no panic)
    update(&mut model, Message::System(SystemMessage::Redo));

    // No "Redone" message should appear
    assert_eq!(model.alerts.status_message, initial_status);
}

// ============================================================================
// Tick Tests
// ============================================================================

#[test]
fn test_tick_clears_old_status_message() {
    use std::time::{Duration, Instant};

    let mut model = Model::new();
    model.alerts.status_message = Some("Old message".to_string());
    // Simulate message set 4 seconds ago
    model.alerts.status_message_set_at = Instant::now().checked_sub(Duration::from_secs(4));

    update(&mut model, Message::System(SystemMessage::Tick));

    // Message should be cleared after 3 second timeout
    assert!(model.alerts.status_message.is_none());
    assert!(model.alerts.status_message_set_at.is_none());
}

#[test]
fn test_tick_keeps_recent_status_message() {
    use std::time::Instant;

    let mut model = Model::new();
    model.alerts.status_message = Some("Recent message".to_string());
    model.alerts.status_message_set_at = Some(Instant::now());

    update(&mut model, Message::System(SystemMessage::Tick));

    // Message should still be there (less than 3 seconds old)
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_tick_sets_timestamp_if_missing() {
    let mut model = Model::new();
    model.alerts.status_message = Some("Message without timestamp".to_string());
    model.alerts.status_message_set_at = None;

    update(&mut model, Message::System(SystemMessage::Tick));

    // Timestamp should be set
    assert!(model.alerts.status_message_set_at.is_some());
    // Message should still be there
    assert!(model.alerts.status_message.is_some());
}

// ============================================================================
// RefreshStorage Tests
// ============================================================================

#[test]
fn test_refresh_storage_no_changes() {
    let mut model = Model::new();

    update(&mut model, Message::System(SystemMessage::RefreshStorage));

    // Should show "no changes" message
    assert!(model
        .alerts
        .status_message
        .as_ref()
        .is_some_and(|m| m.contains("No external changes")));
}

// ============================================================================
// Save Tests
// ============================================================================

#[test]
fn test_save_does_not_panic() {
    let mut model = Model::new();

    // Save should not panic even without a storage backend
    update(&mut model, Message::System(SystemMessage::Save));
    // No assertion needed - just verify no panic
}
