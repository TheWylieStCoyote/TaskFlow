//! Core UI state types.

use std::collections::HashSet;

use crate::domain::{Filter, SortSpec, TaskId};
use crate::ui::{InputMode, InputTarget};

/// Application lifecycle state.
///
/// Indicates whether the application is running or in the process
/// of shutting down.
///
/// # Examples
///
/// ```
/// use taskflow::app::RunningState;
///
/// let state = RunningState::default();
/// assert_eq!(state, RunningState::Running);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunningState {
    /// Application is running normally
    #[default]
    Running,
    /// Application is shutting down
    Quitting,
}

/// State for alert dialogs and status messages.
///
/// Groups related fields for managing alert visibility, error messages,
/// and transient status messages displayed to the user.
#[derive(Debug, Clone, Default)]
pub struct AlertState {
    /// Whether overdue tasks alert is visible (shown at startup)
    pub show_overdue: bool,
    /// Storage load error message (if any)
    pub storage_error: Option<String>,
    /// Whether storage error alert is visible
    pub show_storage_error: bool,
    /// Error message to display in footer (shown in red)
    pub error_message: Option<String>,
    /// Temporary status message to display to user (success/info)
    pub status_message: Option<String>,
    /// When the status message was set (for auto-clear after timeout)
    pub status_message_set_at: Option<std::time::Instant>,
}

/// State for view-specific selections.
///
/// Tracks selection state in specialized views (Kanban, Eisenhower, etc.).
#[derive(Debug, Clone, Default)]
pub struct ViewSelectionState {
    /// Selected column in Kanban view (0-3: Todo, InProgress, Blocked, Done)
    pub kanban_column: usize,
    /// Selected task index within the current Kanban column
    pub kanban_task_index: usize,
    /// Selected quadrant in Eisenhower view (0-3: TL, TR, BL, BR)
    pub eisenhower_quadrant: usize,
    /// Selected task index within the current Eisenhower quadrant
    pub eisenhower_task_index: usize,
    /// Selected day in WeeklyPlanner view (0-6: Mon-Sun)
    pub weekly_planner_day: usize,
    /// Selected task index within the current WeeklyPlanner day
    pub weekly_planner_task_index: usize,
    /// Selected task index in Network view
    pub network_task_index: usize,
}

/// State for text input (task creation, editing, search).
///
/// Groups all input-related fields including the current mode,
/// target entity, text buffer, and cursor position.
#[derive(Debug, Clone, Default)]
pub struct InputState {
    /// Current input mode (Normal or Editing)
    pub mode: InputMode,
    /// What the input is targeting (new task, edit, search, etc.)
    pub target: InputTarget,
    /// Current text in the input field
    pub buffer: String,
    /// Cursor position within input buffer (character index)
    pub cursor: usize,
}

impl InputState {
    /// Returns true if the input is in editing mode.
    #[inline]
    #[must_use]
    pub fn is_editing(&self) -> bool {
        self.mode == InputMode::Editing
    }

    /// Clears the input buffer and resets to normal mode.
    pub fn clear(&mut self) {
        self.mode = InputMode::Normal;
        self.target = InputTarget::default();
        self.buffer.clear();
        self.cursor = 0;
    }
}

/// State for multi-select mode (bulk operations).
///
/// Groups fields for selecting multiple tasks for bulk operations.
#[derive(Debug, Clone, Default)]
pub struct MultiSelectState {
    /// Whether multi-select mode is active
    pub mode: bool,
    /// Set of selected task IDs for bulk operations
    pub selected: HashSet<TaskId>,
}

impl MultiSelectState {
    /// Returns true if multi-select mode is active.
    #[inline]
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.mode
    }

    /// Returns true if there are any selected tasks.
    #[inline]
    #[must_use]
    pub fn has_selection(&self) -> bool {
        !self.selected.is_empty()
    }

    /// Clears all selections and exits multi-select mode.
    pub fn clear(&mut self) {
        self.mode = false;
        self.selected.clear();
    }
}

/// State for filtering and sorting tasks.
///
/// Groups fields related to task filtering and display options.
#[derive(Debug, Clone, Default)]
pub struct FilterState {
    /// Current filter settings
    pub filter: Filter,
    /// Current sort settings
    pub sort: SortSpec,
    /// Whether to show completed tasks
    pub show_completed: bool,
}

/// State for habit tracking view.
#[derive(Debug, Clone, Default)]
pub struct HabitViewState {
    /// Index of selected habit in list
    pub selected: usize,
    /// Whether habit analytics popup is visible
    pub show_analytics: bool,
    /// Whether to show archived habits
    pub show_archived: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_running_state_default() {
        let state = RunningState::default();
        assert_eq!(state, RunningState::Running);
    }

    #[test]
    fn test_running_state_variants() {
        assert_ne!(RunningState::Running, RunningState::Quitting);
    }

    #[test]
    fn test_alert_state_default() {
        let state = AlertState::default();
        assert!(!state.show_overdue);
        assert!(state.storage_error.is_none());
        assert!(!state.show_storage_error);
        assert!(state.error_message.is_none());
        assert!(state.status_message.is_none());
    }

    #[test]
    fn test_view_selection_state_default() {
        let state = ViewSelectionState::default();
        assert_eq!(state.kanban_column, 0);
        assert_eq!(state.kanban_task_index, 0);
        assert_eq!(state.eisenhower_quadrant, 0);
        assert_eq!(state.weekly_planner_day, 0);
    }

    #[test]
    fn test_input_state_is_editing() {
        let mut state = InputState::default();
        assert!(!state.is_editing());

        state.mode = InputMode::Editing;
        assert!(state.is_editing());
    }

    #[test]
    fn test_input_state_clear() {
        let mut state = InputState {
            mode: InputMode::Editing,
            target: InputTarget::Search,
            buffer: "test search".to_string(),
            cursor: 5,
        };

        state.clear();

        assert_eq!(state.mode, InputMode::Normal);
        assert_eq!(state.target, InputTarget::default());
        assert!(state.buffer.is_empty());
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn test_multi_select_state_is_active() {
        let mut state = MultiSelectState::default();
        assert!(!state.is_active());

        state.mode = true;
        assert!(state.is_active());
    }

    #[test]
    fn test_multi_select_state_has_selection() {
        let mut state = MultiSelectState::default();
        assert!(!state.has_selection());

        state.selected.insert(TaskId::new());
        assert!(state.has_selection());
    }

    #[test]
    fn test_multi_select_state_clear() {
        let mut state = MultiSelectState {
            mode: true,
            selected: HashSet::from([TaskId::new(), TaskId::new()]),
        };

        state.clear();

        assert!(!state.mode);
        assert!(state.selected.is_empty());
    }
}
