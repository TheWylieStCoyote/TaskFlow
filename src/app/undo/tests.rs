//! Tests for the undo/redo system.

use super::*;
use crate::domain::{Project, Task};

#[test]
fn test_undo_stack_push_pop() {
    let mut stack = UndoStack::new();
    assert!(stack.is_empty());

    let task = Task::new("Test task");
    stack.push(UndoAction::TaskCreated(Box::new(task)));

    assert!(!stack.is_empty());
    assert_eq!(stack.len(), 1);

    let action = stack.pop().unwrap();
    assert!(matches!(action, UndoAction::TaskCreated(_)));
    assert!(stack.is_empty());
}

#[test]
fn test_undo_stack_max_history() {
    let mut stack = UndoStack::new();

    // Push more than MAX_UNDO_HISTORY actions
    for i in 0..MAX_UNDO_HISTORY + 10 {
        let task = Task::new(format!("Task {i}"));
        stack.push(UndoAction::TaskCreated(Box::new(task)));
    }

    // Should be capped at MAX_UNDO_HISTORY
    assert_eq!(stack.len(), MAX_UNDO_HISTORY);
}

#[test]
fn test_undo_stack_peek() {
    let mut stack = UndoStack::new();
    assert!(stack.peek().is_none());

    let task = Task::new("Test task");
    stack.push(UndoAction::TaskCreated(Box::new(task)));

    assert!(stack.peek().is_some());
    assert_eq!(stack.len(), 1); // Peek doesn't remove
}

#[test]
fn test_undo_action_description() {
    let task = Task::new("My test task");
    let action = UndoAction::TaskCreated(Box::new(task));
    assert!(action.description().contains("Create task"));

    let project = Project::new("My project");
    let action = UndoAction::ProjectCreated(Box::new(project));
    assert!(action.description().contains("Create project"));
}

#[test]
fn test_truncate() {
    assert_eq!(truncate("short", 10), "short");
    assert_eq!(truncate("this is a very long string", 10), "this is...");
}

#[test]
fn test_undo_stack_clear() {
    let mut stack = UndoStack::new();

    for i in 0..5 {
        let task = Task::new(format!("Task {i}"));
        stack.push(UndoAction::TaskCreated(Box::new(task)));
    }

    assert_eq!(stack.len(), 5);
    stack.clear();
    assert!(stack.is_empty());
}

// Redo tests
#[test]
fn test_redo_after_undo() {
    let mut stack = UndoStack::new();

    let task = Task::new("Test task");
    stack.push(UndoAction::TaskCreated(Box::new(task)));

    assert!(!stack.can_redo());
    assert_eq!(stack.len(), 1);

    // Undo the action
    let action = stack.pop_for_undo().unwrap();
    assert!(matches!(action, UndoAction::TaskCreated(_)));
    assert!(stack.is_empty());
    assert!(stack.can_redo());
    assert_eq!(stack.redo_len(), 1);

    // Redo the action
    let action = stack.pop_for_redo().unwrap();
    assert!(matches!(action, UndoAction::TaskCreated(_)));
    assert!(!stack.is_empty());
    assert!(!stack.can_redo());
}

#[test]
fn test_new_action_clears_redo() {
    let mut stack = UndoStack::new();

    let task1 = Task::new("Task 1");
    stack.push(UndoAction::TaskCreated(Box::new(task1)));

    // Undo to create redo entry
    stack.pop_for_undo();
    assert!(stack.can_redo());

    // New action should clear redo
    let task2 = Task::new("Task 2");
    stack.push(UndoAction::TaskCreated(Box::new(task2)));
    assert!(!stack.can_redo());
}

#[test]
fn test_multiple_undo_redo() {
    let mut stack = UndoStack::new();

    // Push 3 actions
    for i in 1..=3 {
        let task = Task::new(format!("Task {i}"));
        stack.push(UndoAction::TaskCreated(Box::new(task)));
    }
    assert_eq!(stack.len(), 3);

    // Undo all 3
    stack.pop_for_undo();
    stack.pop_for_undo();
    stack.pop_for_undo();
    assert!(stack.is_empty());
    assert_eq!(stack.redo_len(), 3);

    // Redo 2
    stack.pop_for_redo();
    stack.pop_for_redo();
    assert_eq!(stack.len(), 2);
    assert_eq!(stack.redo_len(), 1);
}

#[test]
fn test_redo_empty_stack() {
    let mut stack = UndoStack::new();
    assert!(!stack.can_redo());
    assert!(stack.pop_for_redo().is_none());
}

#[test]
fn test_peek_redo() {
    let mut stack = UndoStack::new();
    assert!(stack.peek_redo().is_none());

    let task = Task::new("Test task");
    stack.push(UndoAction::TaskCreated(Box::new(task)));
    stack.pop_for_undo();

    assert!(stack.peek_redo().is_some());
    assert_eq!(stack.redo_len(), 1); // Peek doesn't remove
}

#[test]
fn test_undo_action_inverse_modified() {
    let before = Task::new("Before");
    let after = Task::new("After");
    let action = UndoAction::TaskModified {
        before: Box::new(before.clone()),
        after: Box::new(after.clone()),
    };

    let inverse = action.inverse();
    let UndoAction::TaskModified {
        before: inv_before,
        after: inv_after,
    } = inverse
    else {
        panic!("Expected TaskModified, got {inverse:?}");
    };
    // Inverse swaps before and after
    assert_eq!(inv_before.title, "After");
    assert_eq!(inv_after.title, "Before");
}

#[test]
fn test_clear_clears_both_stacks() {
    let mut stack = UndoStack::new();

    let task = Task::new("Test");
    stack.push(UndoAction::TaskCreated(Box::new(task)));
    stack.pop_for_undo();

    assert!(!stack.is_empty() || stack.can_redo());
    stack.clear();
    assert!(stack.is_empty());
    assert!(!stack.can_redo());
}

#[test]
fn test_project_deleted_description() {
    let project = Project::new("My project");
    let action = UndoAction::ProjectDeleted(Box::new(project));
    assert!(action.description().contains("Delete project"));
}

#[test]
fn test_project_modified_description() {
    let before = Project::new("Before");
    let after = Project::new("After");
    let action = UndoAction::ProjectModified {
        before: Box::new(before),
        after: Box::new(after),
    };
    assert!(action.description().contains("Modify project"));
}

#[test]
fn test_project_deleted_inverse() {
    let project = Project::new("Test project");
    let action = UndoAction::ProjectDeleted(Box::new(project.clone()));
    let inverse = action.inverse();
    let UndoAction::ProjectDeleted(inv_project) = inverse else {
        panic!("Expected ProjectDeleted, got {inverse:?}");
    };
    assert_eq!(inv_project.name, "Test project");
}

#[test]
fn test_project_modified_inverse() {
    let before = Project::new("Before");
    let after = Project::new("After");
    let action = UndoAction::ProjectModified {
        before: Box::new(before),
        after: Box::new(after),
    };

    let inverse = action.inverse();
    let UndoAction::ProjectModified {
        before: inv_before,
        after: inv_after,
    } = inverse
    else {
        panic!("Expected ProjectModified, got {inverse:?}");
    };
    // Inverse swaps before and after
    assert_eq!(inv_before.name, "After");
    assert_eq!(inv_after.name, "Before");
}

// Time entry undo tests
#[test]
fn test_time_entry_started_description() {
    use crate::domain::{TaskId, TimeEntry};

    let task_id = TaskId::new();
    let entry = TimeEntry::start(task_id);
    let action = UndoAction::TimeEntryStarted(Box::new(entry));
    assert!(action.description().contains("Start time tracking"));
}

#[test]
fn test_time_entry_stopped_description() {
    use crate::domain::{TaskId, TimeEntry};

    let task_id = TaskId::new();
    let before = TimeEntry::start(task_id);
    let mut after = before.clone();
    after.stop();
    let action = UndoAction::TimeEntryStopped {
        before: Box::new(before),
        after: Box::new(after),
    };
    assert!(action.description().contains("Stop time tracking"));
}

#[test]
fn test_time_entry_deleted_description() {
    use crate::domain::{TaskId, TimeEntry};

    let task_id = TaskId::new();
    let entry = TimeEntry::start(task_id);
    let action = UndoAction::TimeEntryDeleted(Box::new(entry));
    assert!(action.description().contains("Delete time entry"));
}

#[test]
fn test_time_entry_started_inverse() {
    use crate::domain::{TaskId, TimeEntry};

    let task_id = TaskId::new();
    let entry = TimeEntry::start(task_id);
    let entry_id = entry.id;
    let action = UndoAction::TimeEntryStarted(Box::new(entry));
    let inverse = action.inverse();

    let UndoAction::TimeEntryStarted(inv_entry) = inverse else {
        panic!("Expected TimeEntryStarted, got {inverse:?}");
    };
    assert_eq!(inv_entry.id, entry_id);
}

#[test]
fn test_time_entry_stopped_inverse() {
    use crate::domain::{TaskId, TimeEntry};

    let task_id = TaskId::new();
    let before = TimeEntry::start(task_id);
    let mut after = before.clone();
    after.stop();

    let action = UndoAction::TimeEntryStopped {
        before: Box::new(before.clone()),
        after: Box::new(after.clone()),
    };

    let inverse = action.inverse();
    let UndoAction::TimeEntryStopped {
        before: inv_before,
        after: inv_after,
    } = inverse
    else {
        panic!("Expected TimeEntryStopped, got {inverse:?}");
    };
    // Inverse swaps before and after
    assert_eq!(inv_before.id, after.id);
    assert!(inv_before.ended_at.is_some());
    assert_eq!(inv_after.id, before.id);
    assert!(inv_after.ended_at.is_none());
}

#[test]
fn test_time_entry_deleted_inverse() {
    use crate::domain::{TaskId, TimeEntry};

    let task_id = TaskId::new();
    let entry = TimeEntry::start(task_id);
    let entry_id = entry.id;
    let action = UndoAction::TimeEntryDeleted(Box::new(entry));
    let inverse = action.inverse();

    let UndoAction::TimeEntryDeleted(inv_entry) = inverse else {
        panic!("Expected TimeEntryDeleted, got {inverse:?}");
    };
    assert_eq!(inv_entry.id, entry_id);
}

#[test]
fn test_undo_redo_time_entry_started() {
    use crate::domain::{TaskId, TimeEntry};

    let mut stack = UndoStack::new();
    let task_id = TaskId::new();
    let entry = TimeEntry::start(task_id);
    let entry_id = entry.id;

    stack.push(UndoAction::TimeEntryStarted(Box::new(entry)));
    assert_eq!(stack.len(), 1);
    assert!(!stack.can_redo());

    // Undo the action
    let action = stack.pop_for_undo().unwrap();
    let UndoAction::TimeEntryStarted(e) = action else {
        panic!("Expected TimeEntryStarted, got {action:?}");
    };
    assert_eq!(e.id, entry_id);
    assert!(stack.is_empty());
    assert!(stack.can_redo());

    // Redo the action
    let action = stack.pop_for_redo().unwrap();
    let UndoAction::TimeEntryStarted(e) = action else {
        panic!("Expected TimeEntryStarted, got {action:?}");
    };
    assert_eq!(e.id, entry_id);
    assert!(!stack.is_empty());
    assert!(!stack.can_redo());
}

#[test]
fn test_undo_redo_time_entry_stopped() {
    use crate::domain::{TaskId, TimeEntry};

    let mut stack = UndoStack::new();
    let task_id = TaskId::new();
    let before = TimeEntry::start(task_id);
    let mut after = before.clone();
    after.stop();

    stack.push(UndoAction::TimeEntryStopped {
        before: Box::new(before.clone()),
        after: Box::new(after.clone()),
    });

    // Undo the stop
    let action = stack.pop_for_undo().unwrap();
    let UndoAction::TimeEntryStopped {
        before: b,
        after: a,
    } = action
    else {
        panic!("Expected TimeEntryStopped, got {action:?}");
    };
    // After undo, the entry should be running again
    assert!(b.ended_at.is_none());
    assert!(a.ended_at.is_some());

    // Redo the stop
    let action = stack.pop_for_redo().unwrap();
    let UndoAction::TimeEntryStopped {
        before: b,
        after: _,
    } = action
    else {
        panic!("Expected TimeEntryStopped, got {action:?}");
    };
    // After redo, the entry should be stopped again
    assert!(b.ended_at.is_some());
}
