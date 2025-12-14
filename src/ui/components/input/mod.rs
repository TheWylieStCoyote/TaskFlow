//! Text input component and input state management.
//!
//! Provides the input field widget for task creation, editing, and search.
//! Handles different input modes (normal vs editing) and input targets
//! (task, project, tag, etc.).
//!
//! # Input Modes
//!
//! - **Normal**: Regular navigation, keypresses trigger actions
//! - **Editing**: Text input mode, keypresses insert characters
//!
//! # Input Targets
//!
//! The input field can target different entity types: tasks, subtasks,
//! projects, tags, due dates, and more.

mod alerts;
mod confirm;
mod dialog;
mod quick_capture;

#[cfg(test)]
mod tests;

pub use alerts::{OverdueAlert, StorageErrorAlert};
pub use confirm::ConfirmDialog;
pub use dialog::InputDialog;
pub use quick_capture::QuickCaptureDialog;

use crate::domain::{GoalId, HabitId, ProjectId, TaskId};
use crate::storage::ImportFormat;

/// Input mode for the application
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
}

/// What type of item is being created/edited
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InputTarget {
    #[default]
    Task,
    Subtask(TaskId), // Creating a subtask under the given parent
    EditTask(TaskId),
    EditDueDate(TaskId),
    EditScheduledDate(TaskId),
    EditScheduledTime(TaskId), // Time block (e.g., "9:00-11:00", "9am-11am")
    EditTags(TaskId),
    EditDescription(TaskId),
    Project,
    EditProject(ProjectId), // Renaming an existing project
    Search,
    MoveToProject(TaskId),
    FilterByTag,
    BulkMoveToProject,
    BulkSetStatus,
    EditDependencies(TaskId),
    EditRecurrence(TaskId),
    LinkTask(TaskId),             // Linking current task to next task in chain
    ImportFilePath(ImportFormat), // File path input for import
    SavedFilterName,              // Name for a new saved filter
    SnoozeTask(TaskId),           // Snooze date for a task
    EditEstimate(TaskId),         // Time estimate for a task (e.g., "30m", "2h", "1h30m")
    NewHabit,                     // Creating a new habit
    EditHabit(HabitId),           // Editing an existing habit's name
    QuickCapture,                 // Quick capture mode with syntax hints
    GoalName,                     // Creating a new goal
    EditGoalName(GoalId),         // Editing an existing goal's name
    KeyResultName(GoalId),        // Creating a key result for a goal
}
