//! Time entry management and cleanup tests.

use crate::app::{
    update::update, Message, SystemMessage, TaskMessage, TimeMessage, UiMessage, UndoAction,
};
use crate::domain::{Task, TaskId, TimeEntry};

use super::create_test_model_with_tasks;

// === Time Entry Cleanup on Task Deletion ===

#[test]
fn test_delete_task_removes_time_entries() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Add time entries for this task
    let entry1 = TimeEntry::start(task_id);
    let entry2 = TimeEntry::start(task_id);
    model.time_entries.insert(entry1.id, entry1);
    model.time_entries.insert(entry2.id, entry2);
    assert_eq!(model.time_entries.len(), 2);

    // Delete the task
    update(&mut model, Message::Task(TaskMessage::Delete(task_id)));

    // Time entries should be removed
    assert!(model.time_entries.is_empty());
}

#[test]
fn test_delete_task_clears_active_time_entry() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Start time tracking on this task
    update(&mut model, Message::Time(TimeMessage::ToggleTracking));
    assert!(model.active_time_entry.is_some());
    assert_eq!(model.time_entries.len(), 1);

    // Delete the task
    update(&mut model, Message::Task(TaskMessage::Delete(task_id)));

    // Active entry should be cleared and time entries removed
    assert!(model.active_time_entry.is_none());
    assert!(model.time_entries.is_empty());
}

#[test]
fn test_undo_delete_task_restores_time_entries() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Add time entries for this task
    let entry1 = TimeEntry::start(task_id);
    let entry1_id = entry1.id;
    model.time_entries.insert(entry1.id, entry1);
    assert_eq!(model.time_entries.len(), 1);

    // Delete the task
    let initial_task_count = model.tasks.len();
    update(&mut model, Message::Task(TaskMessage::Delete(task_id)));
    assert_eq!(model.tasks.len(), initial_task_count - 1);
    assert!(model.time_entries.is_empty());

    // Undo the delete
    update(&mut model, Message::System(SystemMessage::Undo));

    // Task and time entries should be restored
    assert_eq!(model.tasks.len(), initial_task_count);
    assert!(model.tasks.contains_key(&task_id));
    assert_eq!(model.time_entries.len(), 1);
    assert!(model.time_entries.contains_key(&entry1_id));
}

#[test]
fn test_redo_delete_task_removes_time_entries_again() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Add time entries for this task
    let entry1 = TimeEntry::start(task_id);
    model.time_entries.insert(entry1.id, entry1);

    // Delete, undo, then redo
    update(&mut model, Message::Task(TaskMessage::Delete(task_id)));
    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(model.time_entries.len(), 1);

    update(&mut model, Message::System(SystemMessage::Redo));

    // Task and time entries should be removed again
    assert!(!model.tasks.contains_key(&task_id));
    assert!(model.time_entries.is_empty());
}

#[test]
fn test_bulk_delete_removes_time_entries() {
    let mut model = create_test_model_with_tasks();
    let task1_id = model.visible_tasks[0];
    let task2_id = model.visible_tasks[1];

    // Add time entries for both tasks
    let entry1 = TimeEntry::start(task1_id);
    let entry2 = TimeEntry::start(task2_id);
    model.time_entries.insert(entry1.id, entry1);
    model.time_entries.insert(entry2.id, entry2);
    assert_eq!(model.time_entries.len(), 2);

    // Set up multi-select
    model.multi_select.mode = true;
    model.multi_select.selected.insert(task1_id);
    model.multi_select.selected.insert(task2_id);

    // Bulk delete
    update(&mut model, Message::Ui(UiMessage::BulkDelete));

    // All time entries should be removed
    assert!(model.time_entries.is_empty());
}

#[test]
fn test_bulk_delete_clears_active_time_entry() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Start time tracking
    update(&mut model, Message::Time(TimeMessage::ToggleTracking));
    assert!(model.active_time_entry.is_some());

    // Set up multi-select and delete
    model.multi_select.mode = true;
    model.multi_select.selected.insert(task_id);
    update(&mut model, Message::Ui(UiMessage::BulkDelete));

    // Active entry should be cleared
    assert!(model.active_time_entry.is_none());
}

#[test]
fn test_undo_bulk_delete_restores_time_entries() {
    let mut model = create_test_model_with_tasks();
    let task1_id = model.visible_tasks[0];
    let task2_id = model.visible_tasks[1];

    // Add time entries
    let entry1 = TimeEntry::start(task1_id);
    let entry2 = TimeEntry::start(task2_id);
    let entry1_id = entry1.id;
    let entry2_id = entry2.id;
    model.time_entries.insert(entry1.id, entry1);
    model.time_entries.insert(entry2.id, entry2);

    // Bulk delete
    model.multi_select.mode = true;
    model.multi_select.selected.insert(task1_id);
    model.multi_select.selected.insert(task2_id);
    update(&mut model, Message::Ui(UiMessage::BulkDelete));
    assert!(model.time_entries.is_empty());

    // Undo both deletes (one at a time)
    update(&mut model, Message::System(SystemMessage::Undo));
    update(&mut model, Message::System(SystemMessage::Undo));

    // Both tasks and time entries should be restored
    assert!(model.tasks.contains_key(&task1_id));
    assert!(model.tasks.contains_key(&task2_id));
    assert!(model.time_entries.contains_key(&entry1_id));
    assert!(model.time_entries.contains_key(&entry2_id));
}

#[test]
fn test_confirm_delete_removes_time_entries() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 0;
    let task_id = model.visible_tasks[0];

    // Add time entry
    let entry = TimeEntry::start(task_id);
    model.time_entries.insert(entry.id, entry);

    // Confirm delete
    model.show_confirm_delete = true;
    update(&mut model, Message::Ui(UiMessage::ConfirmDelete));

    // Time entry should be removed
    assert!(model.time_entries.is_empty());
}

#[test]
fn test_undo_restores_running_time_entry_as_active() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Start time tracking (creates a running entry)
    update(&mut model, Message::Time(TimeMessage::ToggleTracking));
    assert!(model.active_time_entry.is_some());
    let entry_id = model.active_time_entry.unwrap();

    // Delete the task (which should clear the active entry)
    update(&mut model, Message::Task(TaskMessage::Delete(task_id)));
    assert!(model.active_time_entry.is_none());

    // Undo should restore the running entry as active
    update(&mut model, Message::System(SystemMessage::Undo));
    assert!(model.active_time_entry.is_some());
    assert_eq!(model.active_time_entry.unwrap(), entry_id);
}

#[test]
fn test_task_deleted_undo_action_contains_time_entries() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Add time entries
    let entry1 = TimeEntry::start(task_id);
    let mut entry2 = TimeEntry::start(task_id);
    entry2.stop();
    model.time_entries.insert(entry1.id, entry1);
    model.time_entries.insert(entry2.id, entry2);

    // Delete task
    update(&mut model, Message::Task(TaskMessage::Delete(task_id)));

    // Check the undo action contains time entries
    let action = model.undo_stack.peek().unwrap();
    if let UndoAction::TaskDeleted {
        task: _,
        time_entries,
    } = action
    {
        assert_eq!(time_entries.len(), 2);
    } else {
        panic!("Expected TaskDeleted undo action");
    }
}

// === Advanced Time Tracking Undo Tests ===

#[test]
fn test_timer_switch_single_undo() {
    let mut model = create_test_model_with_tasks();
    let task1_id = model.visible_tasks[0];
    let task2_id = model.visible_tasks[1];

    // Start tracking task 1
    model.selected_index = 0;
    update(&mut model, Message::Time(TimeMessage::ToggleTracking));
    assert!(model.active_time_entry.is_some());
    let initial_undo_count = model.undo_stack.len();

    // Switch to task 2 (should create TimerSwitched composite action)
    model.selected_index = 1;
    update(&mut model, Message::Time(TimeMessage::ToggleTracking));

    // Should have added only ONE undo action (TimerSwitched)
    assert_eq!(model.undo_stack.len(), initial_undo_count + 1);

    // Verify we're now tracking task 2
    let active = model.active_time_entry().unwrap();
    assert_eq!(active.task_id, task2_id);

    // Single undo should restore tracking to task 1
    update(&mut model, Message::System(SystemMessage::Undo));

    // Should be tracking task 1 again
    let active = model.active_time_entry().unwrap();
    assert_eq!(active.task_id, task1_id);
}

#[test]
fn test_timer_switched_redo() {
    let mut model = create_test_model_with_tasks();
    let task1_id = model.visible_tasks[0];
    let task2_id = model.visible_tasks[1];

    // Start tracking task 1
    model.selected_index = 0;
    update(&mut model, Message::Time(TimeMessage::ToggleTracking));

    // Switch to task 2
    model.selected_index = 1;
    update(&mut model, Message::Time(TimeMessage::ToggleTracking));
    assert_eq!(model.active_time_entry().unwrap().task_id, task2_id);

    // Undo the switch
    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(model.active_time_entry().unwrap().task_id, task1_id);

    // Redo the switch
    update(&mut model, Message::System(SystemMessage::Redo));
    assert_eq!(model.active_time_entry().unwrap().task_id, task2_id);
}

#[test]
fn test_restore_time_entry_checks_task_exists() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];

    // Create a time entry for a task that will be deleted
    let entry = TimeEntry::start(task_id);
    let entry_id = entry.id;

    // Delete the task (without going through normal flow)
    model.tasks.remove(&task_id);

    // Try to restore the entry - it should be added but NOT become active
    // because the task doesn't exist
    model.restore_time_entry(entry);

    // Entry should exist but not be active
    assert!(model.time_entries.contains_key(&entry_id));
    assert!(model.active_time_entry.is_none());
}

#[test]
fn test_restore_time_entry_doesnt_overwrite_active() {
    let mut model = create_test_model_with_tasks();
    let _task1_id = model.visible_tasks[0];
    let task2_id = model.visible_tasks[1];

    // Start tracking task 1
    model.selected_index = 0;
    update(&mut model, Message::Time(TimeMessage::ToggleTracking));
    let task1_entry_id = model.active_time_entry.unwrap();

    // Create a running entry for task 2 (simulating what would be restored)
    let task2_entry = TimeEntry::start(task2_id);
    let task2_entry_id = task2_entry.id;

    // Restore task 2's entry - should NOT become active since task 1 is tracking
    model.restore_time_entry(task2_entry);

    // Task 1 should still be the active entry
    assert_eq!(model.active_time_entry, Some(task1_entry_id));

    // Task 2's entry should exist but not be active
    assert!(model.time_entries.contains_key(&task2_entry_id));
}

#[test]
fn test_undo_delete_while_tracking_different_task() {
    let mut model = create_test_model_with_tasks();
    let task1_id = model.visible_tasks[0];
    let _task2_id = model.visible_tasks[1];

    // Start tracking task 1
    model.selected_index = 0;
    update(&mut model, Message::Time(TimeMessage::ToggleTracking));
    let task1_entry_id = model.active_time_entry.unwrap();

    // Delete task 1 (clears active_time_entry but entry stored as running)
    update(&mut model, Message::Task(TaskMessage::Delete(task1_id)));
    assert!(model.active_time_entry.is_none());
    assert!(!model.tasks.contains_key(&task1_id));

    // Start tracking task 2
    model.selected_index = 0; // task2 is now at index 0
    update(&mut model, Message::Time(TimeMessage::ToggleTracking));
    let task2_entry_id = model.active_time_entry.unwrap();
    assert_ne!(task1_entry_id, task2_entry_id);

    // First undo: undoes task2 time entry start
    update(&mut model, Message::System(SystemMessage::Undo));
    assert!(model.active_time_entry.is_none());

    // Second undo: undoes task1 deletion
    update(&mut model, Message::System(SystemMessage::Undo));

    // Task 1 should be restored
    assert!(model.tasks.contains_key(&task1_id));

    // The restored task1 entry SHOULD become active because:
    // 1. Entry was stored in running state (no ended_at)
    // 2. Task exists
    // 3. No other active entry
    assert_eq!(model.active_time_entry, Some(task1_entry_id));

    // The time entry should exist
    assert!(model.time_entries.contains_key(&task1_entry_id));
}

#[test]
fn test_restore_doesnt_steal_active_timer() {
    let mut model = create_test_model_with_tasks();
    let task1_id = model.visible_tasks[0];
    let _task2_id = model.visible_tasks[1];

    // Start tracking task 1
    model.selected_index = 0;
    update(&mut model, Message::Time(TimeMessage::ToggleTracking));
    let _task1_entry_id = model.active_time_entry.unwrap();

    // Delete task 1 (entry stored as running in undo action)
    update(&mut model, Message::Task(TaskMessage::Delete(task1_id)));
    assert!(model.active_time_entry.is_none());

    // Start tracking task 2
    model.selected_index = 0;
    update(&mut model, Message::Time(TimeMessage::ToggleTracking));
    let task2_entry_id = model.active_time_entry.unwrap();

    // Now manually try to restore task1 without undoing task2's tracking
    let task1 = Task::new("Restored Task".to_string());
    let running_entry = TimeEntry::start(task1_id);

    // Insert task first
    model.tasks.insert(task1_id, task1);

    // Restore entry - should NOT steal active since task2 is tracking
    model.restore_time_entry(running_entry);

    // Task 2 should STILL be the active tracking (not stolen)
    assert_eq!(model.active_time_entry, Some(task2_entry_id));
}

#[test]
fn test_time_entry_modified_undo_action() {
    // Test that TimeEntryModified description and inverse work correctly
    let task_id = TaskId::new();
    let mut before = TimeEntry::start(task_id);
    before.description = Some("Original".to_string());

    let mut after = before.clone();
    after.description = Some("Modified".to_string());

    let action = UndoAction::TimeEntryModified {
        before: Box::new(before.clone()),
        after: Box::new(after.clone()),
    };

    // Check description
    assert_eq!(action.description(), "Modify time entry");

    // Check inverse swaps before/after
    let inverse = action.inverse();
    if let UndoAction::TimeEntryModified {
        before: inv_before,
        after: inv_after,
    } = inverse
    {
        assert_eq!(inv_before.description, Some("Modified".to_string()));
        assert_eq!(inv_after.description, Some("Original".to_string()));
    } else {
        panic!("Expected TimeEntryModified");
    }
}

#[test]
fn test_timer_switched_undo_action() {
    // Test that TimerSwitched description works correctly
    let task1_id = TaskId::new();
    let task2_id = TaskId::new();

    let stopped_before = TimeEntry::start(task1_id);
    let mut stopped_after = stopped_before.clone();
    stopped_after.stop();
    let started = TimeEntry::start(task2_id);

    let action = UndoAction::TimerSwitched {
        stopped_entry_before: Box::new(stopped_before),
        stopped_entry_after: Box::new(stopped_after),
        started_entry: Box::new(started),
    };

    // Check description
    assert_eq!(action.description(), "Switch timer");
}

#[test]
fn test_fresh_start_uses_simple_undo_action() {
    let mut model = create_test_model_with_tasks();

    // Start tracking with no existing timer
    model.selected_index = 0;
    update(&mut model, Message::Time(TimeMessage::ToggleTracking));

    // Should use TimeEntryStarted (not TimerSwitched)
    let action = model.undo_stack.peek().unwrap();
    assert!(
        matches!(action, UndoAction::TimeEntryStarted(_)),
        "Fresh start should use TimeEntryStarted"
    );
}
