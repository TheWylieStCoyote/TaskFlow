//! Undo/Redo history stack implementation.

use super::{UndoAction, MAX_UNDO_HISTORY};

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
