use crate::domain::{Project, Task, TimeEntry, WorkLogEntry};

/// Maximum number of undo/redo actions to keep in history
pub const MAX_UNDO_HISTORY: usize = 50;

/// Maximum length for names/titles in descriptions
const DESC_MAX_LEN: usize = 20;

/// Generate a description for create/delete/modify actions with entity name
macro_rules! action_desc {
    (create $entity:literal, $name:expr) => {
        format!(
            concat!("Create ", $entity, " \"{}\""),
            truncate($name, DESC_MAX_LEN)
        )
    };
    (delete $entity:literal, $name:expr) => {
        format!(
            concat!("Delete ", $entity, " \"{}\""),
            truncate($name, DESC_MAX_LEN)
        )
    };
    (modify $entity:literal, $name:expr) => {
        format!(
            concat!("Modify ", $entity, " \"{}\""),
            truncate($name, DESC_MAX_LEN)
        )
    };
    // WorkLog uses different verbs
    (add $entity:literal, $name:expr) => {
        format!(
            concat!("Add ", $entity, " \"{}\""),
            truncate($name, DESC_MAX_LEN)
        )
    };
    (edit $entity:literal, $name:expr) => {
        format!(
            concat!("Edit ", $entity, " \"{}\""),
            truncate($name, DESC_MAX_LEN)
        )
    };
}

/// Generate inverse for self-inverse actions (create/delete pairs that clone as-is)
macro_rules! inverse_clone {
    ($self:expr, $variant:ident) => {
        Self::$variant($self.clone())
    };
    ($self:expr, $variant:ident { $($field:ident),+ }) => {
        Self::$variant { $($field: $field.clone()),+ }
    };
}

/// Generate inverse for before/after actions (swap and clone)
macro_rules! inverse_swap {
    ($variant:ident, $before:expr, $after:expr) => {
        Self::$variant {
            before: $after.clone(),
            after: $before.clone(),
        }
    };
}

/// Represents an action that can be undone/redone
#[derive(Debug, Clone)]
pub enum UndoAction {
    /// Task was created - undo by deleting it
    TaskCreated(Box<Task>),
    /// Task was deleted - undo by restoring it (includes associated time entries)
    TaskDeleted {
        task: Box<Task>,
        time_entries: Vec<TimeEntry>,
    },
    /// Task was modified - undo by restoring previous state
    TaskModified { before: Box<Task>, after: Box<Task> },
    /// Project was created - undo by deleting it
    ProjectCreated(Box<Project>),
    /// Project was deleted - undo by restoring it
    ProjectDeleted(Box<Project>),
    /// Project was modified - undo by restoring previous state
    ProjectModified {
        before: Box<Project>,
        after: Box<Project>,
    },
    /// Time entry was created (started) - undo by deleting it
    TimeEntryStarted(Box<TimeEntry>),
    /// Time entry was stopped - undo by restoring running state
    TimeEntryStopped {
        before: Box<TimeEntry>,
        after: Box<TimeEntry>,
    },
    /// Time entry was deleted - undo by restoring it
    TimeEntryDeleted(Box<TimeEntry>),
    /// Time entry was modified - undo by restoring previous state
    TimeEntryModified {
        before: Box<TimeEntry>,
        after: Box<TimeEntry>,
    },
    /// Timer was switched from one task to another - undo both in one operation
    TimerSwitched {
        stopped_entry_before: Box<TimeEntry>,
        stopped_entry_after: Box<TimeEntry>,
        started_entry: Box<TimeEntry>,
    },
    /// Work log entry was created - undo by deleting it
    WorkLogCreated(Box<WorkLogEntry>),
    /// Work log entry was deleted - undo by restoring it
    WorkLogDeleted(Box<WorkLogEntry>),
    /// Work log entry was modified - undo by restoring previous state
    WorkLogModified {
        before: Box<WorkLogEntry>,
        after: Box<WorkLogEntry>,
    },
}

impl UndoAction {
    /// Get a human-readable description of the action
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            // Task actions
            Self::TaskCreated(task) => action_desc!(create "task", &task.title),
            Self::TaskDeleted { task, .. } => action_desc!(delete "task", &task.title),
            Self::TaskModified { before, .. } => action_desc!(modify "task", &before.title),

            // Project actions
            Self::ProjectCreated(project) => action_desc!(create "project", &project.name),
            Self::ProjectDeleted(project) => action_desc!(delete "project", &project.name),
            Self::ProjectModified { before, .. } => action_desc!(modify "project", &before.name),

            // Time entry actions (no entity name - actions are self-explanatory)
            Self::TimeEntryStarted(_) => "Start time tracking".to_string(),
            Self::TimeEntryStopped { .. } => "Stop time tracking".to_string(),
            Self::TimeEntryDeleted(_) => "Delete time entry".to_string(),
            Self::TimeEntryModified { .. } => "Modify time entry".to_string(),
            Self::TimerSwitched { .. } => "Switch timer".to_string(),

            // Work log actions (use add/edit verbs for consistency with UI)
            Self::WorkLogCreated(entry) => action_desc!(add "work log", entry.summary()),
            Self::WorkLogDeleted(entry) => action_desc!(delete "work log", entry.summary()),
            Self::WorkLogModified { before, .. } => action_desc!(edit "work log", before.summary()),
        }
    }

    /// Get the inverse action for redo
    #[must_use]
    pub fn inverse(&self) -> Self {
        match self {
            // Self-inverse actions: clone as-is (create/delete pairs)
            Self::TaskCreated(task) => inverse_clone!(task, TaskCreated),
            Self::TaskDeleted { task, time_entries } => {
                inverse_clone!(self, TaskDeleted { task, time_entries })
            }
            Self::ProjectCreated(project) => inverse_clone!(project, ProjectCreated),
            Self::ProjectDeleted(project) => inverse_clone!(project, ProjectDeleted),
            Self::TimeEntryStarted(entry) => inverse_clone!(entry, TimeEntryStarted),
            Self::TimeEntryDeleted(entry) => inverse_clone!(entry, TimeEntryDeleted),
            Self::WorkLogCreated(entry) => inverse_clone!(entry, WorkLogCreated),
            Self::WorkLogDeleted(entry) => inverse_clone!(entry, WorkLogDeleted),

            // Swap-inverse actions: swap before/after (modify actions)
            Self::TaskModified { before, after } => inverse_swap!(TaskModified, before, after),
            Self::ProjectModified { before, after } => {
                inverse_swap!(ProjectModified, before, after)
            }
            Self::TimeEntryStopped { before, after } => {
                inverse_swap!(TimeEntryStopped, before, after)
            }
            Self::TimeEntryModified { before, after } => {
                inverse_swap!(TimeEntryModified, before, after)
            }
            Self::WorkLogModified { before, after } => {
                inverse_swap!(WorkLogModified, before, after)
            }

            // Special case: TimerSwitched has 3 fields, clone all as-is
            Self::TimerSwitched {
                stopped_entry_before,
                stopped_entry_after,
                started_entry,
            } => Self::TimerSwitched {
                stopped_entry_before: stopped_entry_before.clone(),
                stopped_entry_after: stopped_entry_after.clone(),
                started_entry: started_entry.clone(),
            },
        }
    }
}

/// Truncate a string with ellipsis if too long
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Undo/Redo history stack
#[derive(Debug, Default)]
pub struct UndoStack {
    undo_actions: Vec<UndoAction>,
    redo_actions: Vec<UndoAction>,
}

impl UndoStack {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            undo_actions: Vec::new(),
            redo_actions: Vec::new(),
        }
    }

    /// Push an action onto the undo stack (clears redo stack)
    pub fn push(&mut self, action: UndoAction) {
        self.undo_actions.push(action);
        // Clear redo stack when a new action is performed
        self.redo_actions.clear();
        // Limit history size
        if self.undo_actions.len() > MAX_UNDO_HISTORY {
            self.undo_actions.remove(0);
        }
    }

    /// Pop the most recent action from the undo stack (legacy, doesn't affect redo)
    pub fn pop(&mut self) -> Option<UndoAction> {
        self.undo_actions.pop()
    }

    /// Pop and move to redo stack (call this when undoing)
    pub fn pop_for_undo(&mut self) -> Option<UndoAction> {
        if let Some(action) = self.undo_actions.pop() {
            // Push the inverse to redo stack
            self.redo_actions.push(action.inverse());
            // Limit redo history size
            if self.redo_actions.len() > MAX_UNDO_HISTORY {
                self.redo_actions.remove(0);
            }
            Some(action)
        } else {
            None
        }
    }

    /// Pop from redo stack and move back to undo stack
    pub fn pop_for_redo(&mut self) -> Option<UndoAction> {
        if let Some(action) = self.redo_actions.pop() {
            // Push the inverse back to undo stack
            self.undo_actions.push(action.inverse());
            Some(action)
        } else {
            None
        }
    }

    /// Check if there are any actions to undo
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.undo_actions.is_empty()
    }

    /// Check if there are any actions to redo
    #[must_use]
    pub const fn can_redo(&self) -> bool {
        !self.redo_actions.is_empty()
    }

    /// Get the number of undo actions in the stack
    #[must_use]
    pub const fn len(&self) -> usize {
        self.undo_actions.len()
    }

    /// Get the number of redo actions in the stack
    #[must_use]
    pub const fn redo_len(&self) -> usize {
        self.redo_actions.len()
    }

    /// Peek at the most recent undo action without removing it
    #[must_use]
    pub fn peek(&self) -> Option<&UndoAction> {
        self.undo_actions.last()
    }

    /// Peek at the most recent redo action without removing it
    #[must_use]
    pub fn peek_redo(&self) -> Option<&UndoAction> {
        self.redo_actions.last()
    }

    /// Clear all undo and redo history
    pub fn clear(&mut self) {
        self.undo_actions.clear();
        self.redo_actions.clear();
    }
}

#[cfg(test)]
mod tests {
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
}
