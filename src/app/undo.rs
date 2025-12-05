use crate::domain::{Project, Task};

/// Maximum number of undo/redo actions to keep in history
pub const MAX_UNDO_HISTORY: usize = 50;

/// Represents an action that can be undone/redone
#[derive(Debug, Clone)]
pub enum UndoAction {
    /// Task was created - undo by deleting it
    TaskCreated(Box<Task>),
    /// Task was deleted - undo by restoring it
    TaskDeleted(Box<Task>),
    /// Task was modified - undo by restoring previous state
    TaskModified { before: Box<Task>, after: Box<Task> },
    /// Project was created - undo by deleting it
    ProjectCreated(Box<Project>),
}

impl UndoAction {
    /// Get a human-readable description of the action
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::TaskCreated(task) => {
                format!("Create task \"{}\"", truncate(&task.title, 20))
            }
            Self::TaskDeleted(task) => {
                format!("Delete task \"{}\"", truncate(&task.title, 20))
            }
            Self::TaskModified { before, .. } => {
                format!("Modify task \"{}\"", truncate(&before.title, 20))
            }
            Self::ProjectCreated(project) => {
                format!("Create project \"{}\"", truncate(&project.name, 20))
            }
        }
    }

    /// Get the inverse action for redo
    #[must_use]
    pub fn inverse(&self) -> Self {
        match self {
            // Undo create = delete, so redo = create again
            Self::TaskCreated(task) => Self::TaskCreated(task.clone()),
            // Undo delete = restore, so redo = delete again
            Self::TaskDeleted(task) => Self::TaskDeleted(task.clone()),
            // Undo modify swaps before/after, so redo swaps them back
            Self::TaskModified { before, after } => Self::TaskModified {
                before: after.clone(),
                after: before.clone(),
            },
            // Undo project create = delete, so redo = create again
            Self::ProjectCreated(project) => Self::ProjectCreated(project.clone()),
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
    pub fn new() -> Self {
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
            let task = Task::new(format!("Task {}", i));
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
            let task = Task::new(format!("Task {}", i));
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
            let task = Task::new(format!("Task {}", i));
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
        if let UndoAction::TaskModified {
            before: inv_before,
            after: inv_after,
        } = inverse
        {
            // Inverse swaps before and after
            assert_eq!(inv_before.title, "After");
            assert_eq!(inv_after.title, "Before");
        } else {
            panic!("Expected TaskModified");
        }
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
}
