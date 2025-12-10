//! Core types for the application model.

use std::collections::HashSet;
use std::path::PathBuf;

use chrono::{Datelike, Duration, NaiveDate, Utc};

use crate::domain::{Filter, SortSpec, TaskId};
use crate::storage::{ImportResult, StorageBackend};
use crate::ui::{InputMode, InputTarget};

/// State for the calendar view.
///
/// Tracks the currently displayed month and selected day.
///
/// # Examples
///
/// ```
/// use taskflow::app::CalendarState;
///
/// // Default is current month with today selected
/// let state = CalendarState::default();
/// assert!(state.selected_day.is_some());
/// ```
#[derive(Debug, Clone)]
pub struct CalendarState {
    /// The year being displayed
    pub year: i32,
    /// The month being displayed (1-12)
    pub month: u32,
    /// The selected day within the month (if any)
    pub selected_day: Option<u32>,
    /// Whether focus is on the task list (true) or calendar grid (false)
    pub focus_task_list: bool,
}

impl Default for CalendarState {
    fn default() -> Self {
        let today = Utc::now().date_naive();
        Self {
            year: today.year(),
            month: today.month(),
            selected_day: Some(today.day()),
            focus_task_list: false,
        }
    }
}

impl CalendarState {
    /// Returns the number of days in the current month.
    #[must_use]
    pub fn days_in_month(&self) -> u32 {
        // Get first day of next month, subtract one day
        let (next_year, next_month) = if self.month == 12 {
            (self.year + 1, 1)
        } else {
            (self.year, self.month + 1)
        };
        NaiveDate::from_ymd_opt(next_year, next_month, 1)
            .and_then(|d| d.checked_sub_signed(Duration::days(1)))
            .map_or(31, |d| d.day())
    }

    /// Validates and clamps the selected day to be within the valid range for the month.
    ///
    /// Returns the clamped day value (1 to days_in_month).
    #[must_use]
    pub fn validated_day(&self) -> Option<u32> {
        self.selected_day
            .map(|day| day.clamp(1, self.days_in_month()))
    }

    /// Returns true if the current state represents a valid calendar date.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        if !(1..=12).contains(&self.month) {
            return false;
        }
        if let Some(day) = self.selected_day {
            if day < 1 || day > self.days_in_month() {
                return false;
            }
        }
        true
    }

    /// Creates a NaiveDate from the current state, if valid.
    #[must_use]
    pub fn to_date(&self) -> Option<NaiveDate> {
        self.selected_day
            .and_then(|day| NaiveDate::from_ymd_opt(self.year, self.month, day))
    }
}

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

/// Zoom level for the timeline view.
///
/// Controls how time is displayed on the horizontal axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimelineZoom {
    /// Each column represents one day
    #[default]
    Day,
    /// Each column represents one week
    Week,
}

/// State for the timeline/Gantt view.
///
/// Tracks the viewport position, selection, and display options.
#[derive(Debug, Clone)]
pub struct TimelineState {
    /// Leftmost visible date in the viewport
    pub viewport_start: NaiveDate,
    /// Number of days visible in the viewport
    pub viewport_days: u32,
    /// Index of the selected task in the timeline list
    pub selected_task_index: usize,
    /// Whether to show dependency lines between tasks
    pub show_dependencies: bool,
    /// Current zoom level
    pub zoom_level: TimelineZoom,
    /// Vertical scroll offset for task list
    pub task_scroll_offset: usize,
}

impl Default for TimelineState {
    fn default() -> Self {
        let today = Utc::now().date_naive();
        Self {
            // Start viewport 7 days before today
            viewport_start: today - Duration::days(7),
            viewport_days: 21,
            selected_task_index: 0,
            show_dependencies: false,
            zoom_level: TimelineZoom::default(),
            task_scroll_offset: 0,
        }
    }
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

/// State for daily review mode.
#[derive(Debug, Clone, Default)]
pub struct DailyReviewState {
    /// Whether daily review mode is active
    pub visible: bool,
    /// Current phase of the daily review
    pub phase: crate::ui::DailyReviewPhase,
    /// Selected index within current review phase
    pub selected: usize,
}

/// State for weekly review mode.
#[derive(Debug, Clone, Default)]
pub struct WeeklyReviewState {
    /// Whether weekly review mode is active
    pub visible: bool,
    /// Current phase of the weekly review
    pub phase: crate::ui::WeeklyReviewPhase,
    /// Selected index within current review phase
    pub selected: usize,
}

/// State for template picker modal.
#[derive(Debug, Clone, Default)]
pub struct TemplatePickerState {
    /// Whether template picker is visible
    pub visible: bool,
    /// Index of selected template in picker
    pub selected: usize,
}

/// State for saved filter picker modal.
#[derive(Debug, Clone, Default)]
pub struct SavedFilterPickerState {
    /// Whether saved filter picker is visible
    pub visible: bool,
    /// Selected index in saved filter picker
    pub selected: usize,
}

/// State for keybindings editor modal.
#[derive(Debug, Clone, Default)]
pub struct KeybindingsEditorState {
    /// Whether keybindings editor is visible
    pub visible: bool,
    /// Selected keybinding index in editor
    pub selected: usize,
    /// Whether currently capturing a new key
    pub capturing: bool,
}

/// State for time log editor modal.
#[derive(Debug, Clone, Default)]
pub struct TimeLogEditorState {
    /// Whether time log editor is visible
    pub visible: bool,
    /// Selected time entry index in log
    pub selected: usize,
    /// Current mode in time log editor
    pub mode: crate::ui::TimeLogMode,
    /// Text buffer for editing time entries
    pub buffer: String,
}

/// State for multi-line description editor.
#[derive(Debug, Clone)]
pub struct DescriptionEditorState {
    /// Whether description editor is visible
    pub visible: bool,
    /// Text buffer for editing description (multi-line)
    pub buffer: Vec<String>,
    /// Cursor line position in description buffer
    pub cursor_line: usize,
    /// Cursor column position in description buffer
    pub cursor_col: usize,
}

impl Default for DescriptionEditorState {
    fn default() -> Self {
        Self {
            visible: false,
            buffer: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
        }
    }
}

impl super::MultilineEditor for DescriptionEditorState {
    fn buffer(&self) -> &[String] {
        &self.buffer
    }

    fn buffer_mut(&mut self) -> &mut Vec<String> {
        &mut self.buffer
    }

    fn cursor_line(&self) -> usize {
        self.cursor_line
    }

    fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    fn set_cursor(&mut self, line: usize, col: usize) {
        self.cursor_line = line;
        self.cursor_col = col;
    }
}

/// State for work log editor modal.
#[derive(Debug, Clone)]
pub struct WorkLogEditorState {
    /// Whether work log editor is visible
    pub visible: bool,
    /// Selected work log entry index
    pub selected: usize,
    /// Current mode in work log editor
    pub mode: crate::ui::WorkLogMode,
    /// Text buffer for editing work log entries (multi-line)
    pub buffer: Vec<String>,
    /// Cursor line position in work log buffer
    pub cursor_line: usize,
    /// Cursor column position in work log buffer
    pub cursor_col: usize,
    /// Search query for filtering work log entries
    pub search_query: String,
}

impl Default for WorkLogEditorState {
    fn default() -> Self {
        Self {
            visible: false,
            selected: 0,
            mode: crate::ui::WorkLogMode::default(),
            buffer: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            search_query: String::new(),
        }
    }
}

impl super::MultilineEditor for WorkLogEditorState {
    fn buffer(&self) -> &[String] {
        &self.buffer
    }

    fn buffer_mut(&mut self) -> &mut Vec<String> {
        &mut self.buffer
    }

    fn cursor_line(&self) -> usize {
        self.cursor_line
    }

    fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    fn set_cursor(&mut self, line: usize, col: usize) {
        self.cursor_line = line;
        self.cursor_col = col;
    }
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

/// State for the Pomodoro timer.
///
/// Groups all Pomodoro-related fields including the active session,
/// configuration, and statistics.
#[derive(Debug, Clone, Default)]
pub struct PomodoroState {
    /// Active Pomodoro session (if any)
    pub session: Option<crate::domain::PomodoroSession>,
    /// Pomodoro timer configuration (work/break durations)
    pub config: crate::domain::PomodoroConfig,
    /// Pomodoro statistics (completed sessions, total time)
    pub stats: crate::domain::PomodoroStats,
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

/// State for storage backend and persistence.
///
/// Groups fields related to data persistence and storage backend.
#[derive(Default)]
pub struct StorageState {
    /// Active storage backend (if configured)
    pub(crate) backend: Option<Box<dyn StorageBackend>>,
    /// Path to data file/directory
    pub data_path: Option<PathBuf>,
    /// Whether there are unsaved changes
    pub dirty: bool,
}

impl std::fmt::Debug for StorageState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageState")
            .field(
                "backend",
                &self.backend.as_ref().map(|_| "<StorageBackend>"),
            )
            .field("data_path", &self.data_path)
            .field("dirty", &self.dirty)
            .finish()
    }
}

impl Clone for StorageState {
    fn clone(&self) -> Self {
        // Storage backend cannot be cloned, so we create without backend
        Self {
            backend: None,
            data_path: self.data_path.clone(),
            dirty: self.dirty,
        }
    }
}

/// State for import operations.
///
/// Groups fields related to importing data from external sources.
#[derive(Debug, Clone, Default)]
pub struct ImportState {
    /// Pending import result awaiting confirmation
    pub pending: Option<ImportResult>,
    /// Whether import preview dialog is showing
    pub show_preview: bool,
}

impl ImportState {
    /// Returns true if there's a pending import.
    #[inline]
    #[must_use]
    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Clears the import state.
    pub fn clear(&mut self) {
        self.pending = None;
        self.show_preview = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calendar_state_days_in_month() {
        // Test various months
        let state = CalendarState {
            year: 2024,
            month: 1, // January
            ..Default::default()
        };
        assert_eq!(state.days_in_month(), 31);

        let state = CalendarState {
            year: 2024,
            month: 2, // February (leap year 2024)
            ..Default::default()
        };
        assert_eq!(state.days_in_month(), 29);

        let state = CalendarState {
            year: 2023,
            month: 2, // February (non-leap year)
            ..Default::default()
        };
        assert_eq!(state.days_in_month(), 28);

        let state = CalendarState {
            year: 2023,
            month: 4, // April
            ..Default::default()
        };
        assert_eq!(state.days_in_month(), 30);
    }

    #[test]
    fn test_calendar_state_is_valid() {
        let mut state = CalendarState::default();

        // Valid default state
        assert!(state.is_valid());

        // Invalid month
        state.month = 0;
        assert!(!state.is_valid());

        state.month = 13;
        assert!(!state.is_valid());

        // Invalid day for month
        state.month = 2;
        state.year = 2023;
        state.selected_day = Some(30); // Feb doesn't have 30 days
        assert!(!state.is_valid());

        // Valid day
        state.selected_day = Some(28);
        assert!(state.is_valid());

        // No day selected is valid
        state.selected_day = None;
        assert!(state.is_valid());
    }

    #[test]
    fn test_calendar_state_validated_day() {
        // Day within range
        let state = CalendarState {
            year: 2023,
            month: 2, // February with 28 days
            selected_day: Some(15),
            ..Default::default()
        };
        assert_eq!(state.validated_day(), Some(15));

        // Day too high gets clamped
        let state = CalendarState {
            year: 2023,
            month: 2,
            selected_day: Some(31),
            ..Default::default()
        };
        assert_eq!(state.validated_day(), Some(28));

        // Day too low gets clamped
        let state = CalendarState {
            year: 2023,
            month: 2,
            selected_day: Some(0),
            ..Default::default()
        };
        assert_eq!(state.validated_day(), Some(1));

        // None stays None
        let state = CalendarState {
            year: 2023,
            month: 2,
            selected_day: None,
            ..Default::default()
        };
        assert_eq!(state.validated_day(), None);
    }

    #[test]
    fn test_calendar_state_to_date() {
        let state = CalendarState {
            year: 2024,
            month: 3,
            selected_day: Some(15),
            ..Default::default()
        };

        let date = state.to_date();
        assert!(date.is_some());
        let date = date.unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 3);
        assert_eq!(date.day(), 15);

        // Invalid date returns None
        let state = CalendarState {
            year: 2024,
            month: 3,
            selected_day: Some(32),
            ..Default::default()
        };
        assert!(state.to_date().is_none());

        // No day returns None
        let state = CalendarState {
            year: 2024,
            month: 3,
            selected_day: None,
            ..Default::default()
        };
        assert!(state.to_date().is_none());
    }

    // ==================== RunningState Tests ====================

    #[test]
    fn test_running_state_default() {
        let state = RunningState::default();
        assert_eq!(state, RunningState::Running);
    }

    #[test]
    fn test_running_state_variants() {
        assert_ne!(RunningState::Running, RunningState::Quitting);
    }

    // ==================== TimelineZoom Tests ====================

    #[test]
    fn test_timeline_zoom_default() {
        let zoom = TimelineZoom::default();
        assert_eq!(zoom, TimelineZoom::Day);
    }

    // ==================== TimelineState Tests ====================

    #[test]
    fn test_timeline_state_default() {
        let state = TimelineState::default();
        assert_eq!(state.viewport_days, 21);
        assert_eq!(state.selected_task_index, 0);
        assert!(!state.show_dependencies);
        assert_eq!(state.zoom_level, TimelineZoom::Day);
    }

    // ==================== AlertState Tests ====================

    #[test]
    fn test_alert_state_default() {
        let state = AlertState::default();
        assert!(!state.show_overdue);
        assert!(state.storage_error.is_none());
        assert!(!state.show_storage_error);
        assert!(state.error_message.is_none());
        assert!(state.status_message.is_none());
    }

    // ==================== ViewSelectionState Tests ====================

    #[test]
    fn test_view_selection_state_default() {
        let state = ViewSelectionState::default();
        assert_eq!(state.kanban_column, 0);
        assert_eq!(state.kanban_task_index, 0);
        assert_eq!(state.eisenhower_quadrant, 0);
        assert_eq!(state.weekly_planner_day, 0);
    }

    // ==================== InputState Tests ====================

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

    // ==================== MultiSelectState Tests ====================

    #[test]
    fn test_multi_select_state_is_active() {
        let mut state = MultiSelectState::default();
        assert!(!state.is_active());

        state.mode = true;
        assert!(state.is_active());
    }

    #[test]
    fn test_multi_select_state_has_selection() {
        use crate::domain::TaskId;

        let mut state = MultiSelectState::default();
        assert!(!state.has_selection());

        state.selected.insert(TaskId::new());
        assert!(state.has_selection());
    }

    #[test]
    fn test_multi_select_state_clear() {
        use crate::domain::TaskId;

        let mut state = MultiSelectState {
            mode: true,
            selected: HashSet::from([TaskId::new(), TaskId::new()]),
        };

        state.clear();

        assert!(!state.mode);
        assert!(state.selected.is_empty());
    }

    // ==================== ImportState Tests ====================

    #[test]
    fn test_import_state_has_pending() {
        use crate::storage::ImportResult;

        let mut state = ImportState::default();
        assert!(!state.has_pending());

        state.pending = Some(ImportResult {
            imported: Vec::new(),
            skipped: Vec::new(),
            errors: Vec::new(),
        });
        assert!(state.has_pending());
    }

    #[test]
    fn test_import_state_clear() {
        use crate::storage::ImportResult;

        let mut state = ImportState {
            pending: Some(ImportResult {
                imported: Vec::new(),
                skipped: Vec::new(),
                errors: Vec::new(),
            }),
            show_preview: true,
        };

        state.clear();

        assert!(state.pending.is_none());
        assert!(!state.show_preview);
    }

    // ==================== DescriptionEditorState Tests ====================

    #[test]
    fn test_description_editor_state_default() {
        let state = DescriptionEditorState::default();
        assert!(!state.visible);
        assert_eq!(state.buffer, vec![String::new()]);
        assert_eq!(state.cursor_line, 0);
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_description_editor_implements_multiline_editor() {
        use super::super::MultilineEditor;

        let mut state = DescriptionEditorState::default();
        state.insert_char('a');
        state.insert_char('b');
        state.insert_char('c');

        assert_eq!(state.content(), "abc");
    }

    // ==================== WorkLogEditorState Tests ====================

    #[test]
    fn test_work_log_editor_state_default() {
        let state = WorkLogEditorState::default();
        assert!(!state.visible);
        assert_eq!(state.selected, 0);
        assert_eq!(state.buffer, vec![String::new()]);
        assert!(state.search_query.is_empty());
    }

    #[test]
    fn test_work_log_editor_implements_multiline_editor() {
        use super::super::MultilineEditor;

        let mut state = WorkLogEditorState::default();
        state.set_content("line 1\nline 2");

        assert_eq!(state.buffer.len(), 2);
        assert_eq!(state.content(), "line 1\nline 2");
    }

    // ==================== StorageState Tests ====================

    #[test]
    fn test_storage_state_debug() {
        let state = StorageState::default();
        let debug_str = format!("{state:?}");
        assert!(debug_str.contains("StorageState"));
    }

    #[test]
    fn test_storage_state_clone() {
        use std::path::PathBuf;

        let state = StorageState {
            backend: None,
            data_path: Some(PathBuf::from("/tmp/test")),
            dirty: true,
        };

        let cloned = state.clone();
        assert!(cloned.backend.is_none()); // Backend doesn't clone
        assert_eq!(cloned.data_path, Some(PathBuf::from("/tmp/test")));
        assert!(cloned.dirty);
    }
}
