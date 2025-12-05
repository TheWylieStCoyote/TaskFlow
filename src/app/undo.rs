use crate::domain::{Project, Task};

/// Maximum number of undo actions to keep in history
pub const MAX_UNDO_HISTORY: usize = 50;

/// Represents an action that can be undone
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
    pub fn description(&self) -> String {
        match self {
            UndoAction::TaskCreated(task) => {
                format!("Create task \"{}\"", truncate(&task.title, 20))
            }
            UndoAction::TaskDeleted(task) => {
                format!("Delete task \"{}\"", truncate(&task.title, 20))
            }
            UndoAction::TaskModified { before, .. } => {
                format!("Modify task \"{}\"", truncate(&before.title, 20))
            }
            UndoAction::ProjectCreated(project) => {
                format!("Create project \"{}\"", truncate(&project.name, 20))
            }
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

/// Undo history stack
#[derive(Debug, Default)]
pub struct UndoStack {
    actions: Vec<UndoAction>,
}

impl UndoStack {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    /// Push an action onto the undo stack
    pub fn push(&mut self, action: UndoAction) {
        self.actions.push(action);
        // Limit history size
        if self.actions.len() > MAX_UNDO_HISTORY {
            self.actions.remove(0);
        }
    }

    /// Pop the most recent action from the stack
    pub fn pop(&mut self) -> Option<UndoAction> {
        self.actions.pop()
    }

    /// Check if there are any actions to undo
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Get the number of actions in the stack
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Peek at the most recent action without removing it
    pub fn peek(&self) -> Option<&UndoAction> {
        self.actions.last()
    }

    /// Clear all undo history
    pub fn clear(&mut self) {
        self.actions.clear();
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
}
