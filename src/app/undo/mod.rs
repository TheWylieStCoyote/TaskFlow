//! Undo/redo system for reversible operations.
//!
//! This module implements a command pattern for undo/redo functionality.
//! Each [`UndoAction`] captures the state needed to reverse an operation,
//! enabling users to undo recent changes and redo them if needed.
//!
//! # History Management
//!
//! - Actions are stored in a bounded stack (max 50 items)
//! - Undo moves actions to the redo stack
//! - New actions clear the redo stack
//!
//! # Supported Operations
//!
//! - Task CRUD (create, update, delete, complete)
//! - Project CRUD
//! - Time entry management
//! - Work log entries
//! - Bulk operations

mod action;
mod stack;

pub use action::UndoAction;
pub use stack::UndoStack;

/// Maximum number of undo/redo actions to keep in history
pub const MAX_UNDO_HISTORY: usize = 50;

/// Maximum length for names/titles in descriptions
pub(crate) const DESC_MAX_LEN: usize = 20;

/// Generate a description for create/delete/modify actions with entity name
macro_rules! action_desc {
    (create $entity:literal, $name:expr) => {
        format!(
            concat!("Create ", $entity, " \"{}\""),
            $crate::app::undo::truncate($name, $crate::app::undo::DESC_MAX_LEN)
        )
    };
    (delete $entity:literal, $name:expr) => {
        format!(
            concat!("Delete ", $entity, " \"{}\""),
            $crate::app::undo::truncate($name, $crate::app::undo::DESC_MAX_LEN)
        )
    };
    (modify $entity:literal, $name:expr) => {
        format!(
            concat!("Modify ", $entity, " \"{}\""),
            $crate::app::undo::truncate($name, $crate::app::undo::DESC_MAX_LEN)
        )
    };
    // WorkLog uses different verbs
    (add $entity:literal, $name:expr) => {
        format!(
            concat!("Add ", $entity, " \"{}\""),
            $crate::app::undo::truncate($name, $crate::app::undo::DESC_MAX_LEN)
        )
    };
    (edit $entity:literal, $name:expr) => {
        format!(
            concat!("Edit ", $entity, " \"{}\""),
            $crate::app::undo::truncate($name, $crate::app::undo::DESC_MAX_LEN)
        )
    };
}
pub(crate) use action_desc;

/// Generate inverse for self-inverse actions (create/delete pairs that clone as-is)
macro_rules! inverse_clone {
    ($self:expr, $variant:ident) => {
        UndoAction::$variant($self.clone())
    };
    ($self:expr, $variant:ident { $($field:ident),+ }) => {
        UndoAction::$variant { $($field: $field.clone()),+ }
    };
}
pub(crate) use inverse_clone;

/// Generate inverse for before/after actions (swap and clone)
macro_rules! inverse_swap {
    ($variant:ident, $before:expr, $after:expr) => {
        UndoAction::$variant {
            before: $after.clone(),
            after: $before.clone(),
        }
    };
}
pub(crate) use inverse_swap;

/// Truncate a string with ellipsis if too long
pub(crate) fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests;
