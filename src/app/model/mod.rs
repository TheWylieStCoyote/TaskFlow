//! Application state model.
//!
//! The [`Model`] struct holds the complete application state following
//! The Elm Architecture (TEA) pattern.
//!
//! ## State Categories
//!
//! The model organizes state into several categories:
//!
//! - **Data**: Tasks, projects, time entries
//! - **Navigation**: Current view, selection, focus
//! - **UI State**: Input mode, dialogs, sidebar
//! - **Storage**: Backend connection and dirty flag
//! - **History**: Undo/redo stack
//!
//! ## Examples
//!
//! ### Basic Usage
//!
//! ```
//! use taskflow::app::Model;
//! use taskflow::domain::Task;
//!
//! // Create a new model
//! let mut model = Model::new();
//!
//! // Add a task directly
//! let task = Task::new("My task");
//! model.tasks.insert(task.id.clone(), task);
//! model.refresh_visible_tasks();
//!
//! // Check the visible tasks
//! assert_eq!(model.visible_tasks.len(), 1);
//! ```
//!
//! ### With Sample Data
//!
//! ```
//! use taskflow::app::Model;
//!
//! // Create model with sample data for testing
//! let model = Model::new().with_sample_data();
//!
//! assert!(!model.tasks.is_empty());
//! assert!(!model.projects.is_empty());
//! ```

mod cache;
mod filtering;
mod hierarchy;
mod sample_data;
mod storage;
mod time_tracking;
mod types;

pub use cache::{FooterStats, TaskCache};
pub use types::{CalendarState, RunningState, TimelineState, TimelineZoom};

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use chrono::{NaiveDate, Utc};

use crate::domain::{
    Filter, Habit, HabitId, Priority, Project, ProjectId, SavedFilter, SavedFilterId, SortSpec,
    Task, TaskId, TimeEntry, TimeEntryId, WorkLogEntry, WorkLogEntryId,
};
use crate::storage::StorageBackend;
use crate::ui::{InputMode, InputTarget};

use super::{FocusPane, MacroState, TemplateManager, UndoStack, ViewId};

// ============================================================================
// Sidebar Layout
// ============================================================================
// The sidebar layout is defined by SIDEBAR_VIEWS array. When adding/removing
// views, update the array and all indices will adjust automatically.
//
// Layout:
//   [0..SIDEBAR_VIEW_COUNT-1]     = View items (from SIDEBAR_VIEWS array)
//   SIDEBAR_SEPARATOR_INDEX       = Separator line
//   SIDEBAR_PROJECTS_HEADER_INDEX = "Projects" header
//   SIDEBAR_FIRST_PROJECT_INDEX+  = Individual projects

/// Ordered list of views shown in the sidebar.
/// This is the single source of truth for sidebar view order.
/// When adding a new view:
/// 1. Add the ViewId variant to message.rs
/// 2. Add it to this array in the desired position
/// 3. Add rendering in sidebar.rs (must match this order!)
pub const SIDEBAR_VIEWS: &[ViewId] = &[
    ViewId::TaskList,         // 0: All Tasks
    ViewId::Today,            // 1: Today
    ViewId::Upcoming,         // 2: Upcoming
    ViewId::Overdue,          // 3: Overdue
    ViewId::Scheduled,        // 4: Scheduled
    ViewId::Calendar,         // 5: Calendar
    ViewId::Dashboard,        // 6: Dashboard
    ViewId::Reports,          // 7: Reports
    ViewId::Habits,           // 8: Habits
    ViewId::Blocked,          // 9: Blocked
    ViewId::Untagged,         // 10: Untagged
    ViewId::NoProject,        // 11: No Project
    ViewId::RecentlyModified, // 12: Recent
    ViewId::Kanban,           // 13: Kanban
    ViewId::Eisenhower,       // 14: Eisenhower
    ViewId::WeeklyPlanner,    // 15: Weekly Planner
    ViewId::Timeline,         // 16: Timeline
    ViewId::Snoozed,          // 17: Snoozed
];

/// Number of view items in the sidebar (before the separator).
/// Derived from SIDEBAR_VIEWS array length.
pub const SIDEBAR_VIEW_COUNT: usize = SIDEBAR_VIEWS.len();

/// Index of the separator line in the sidebar.
pub const SIDEBAR_SEPARATOR_INDEX: usize = SIDEBAR_VIEW_COUNT;

/// Index of the "Projects" header in the sidebar.
pub const SIDEBAR_PROJECTS_HEADER_INDEX: usize = SIDEBAR_SEPARATOR_INDEX + 1;

/// Index where individual projects start in the sidebar.
pub const SIDEBAR_FIRST_PROJECT_INDEX: usize = SIDEBAR_PROJECTS_HEADER_INDEX + 1;

/// The complete application state (Model in TEA).
///
/// This struct holds all application state in a single location,
/// following The Elm Architecture pattern. State is modified only
/// through the [`super::update()`] function in response to messages.
///
/// # State Organization
///
/// | Category | Fields |
/// |----------|--------|
/// | Lifecycle | `running` |
/// | Data | `tasks`, `projects`, `time_entries`, `active_time_entry` |
/// | Navigation | `current_view`, `selected_index`, `focus_pane` |
/// | Filtering | `filter`, `sort`, `show_completed`, `visible_tasks` |
/// | UI State | `show_sidebar`, `show_help`, `input_mode`, etc. |
/// | Storage | `storage`, `data_path`, `dirty` |
/// | History | `undo_stack` |
///
/// # Examples
///
/// ```
/// use taskflow::app::{Model, ViewId};
/// use taskflow::domain::Task;
///
/// // Create a new model with defaults
/// let mut model = Model::new();
/// assert_eq!(model.current_view, ViewId::TaskList);
/// assert!(model.tasks.is_empty());
///
/// // Add a task
/// let task = Task::new("Learn Rust");
/// model.tasks.insert(task.id.clone(), task);
/// model.refresh_visible_tasks();
///
/// // Check visible tasks
/// assert_eq!(model.visible_tasks.len(), 1);
/// ```
#[allow(clippy::struct_excessive_bools)]
pub struct Model {
    // Running state
    /// Application lifecycle state
    pub running: RunningState,

    // Data
    /// All tasks indexed by ID
    pub tasks: HashMap<TaskId, Task>,
    /// All projects indexed by ID
    pub projects: HashMap<ProjectId, Project>,
    /// All time entries indexed by ID
    pub time_entries: HashMap<TimeEntryId, TimeEntry>,
    /// Currently active time tracking entry (if any)
    pub active_time_entry: Option<TimeEntryId>,

    // Navigation
    /// Currently displayed view
    pub current_view: ViewId,
    /// Index of selected item in the task list
    pub selected_index: usize,

    // Visible items (filtered and sorted)
    /// Task IDs visible in current view after filtering and sorting
    pub visible_tasks: Vec<TaskId>,

    // Filter/Sort
    /// Current filter settings
    pub filter: Filter,
    /// Current sort settings
    pub sort: SortSpec,
    /// Whether to show completed tasks
    pub show_completed: bool,

    // UI state
    /// Whether sidebar is visible
    pub show_sidebar: bool,
    /// Whether help overlay is visible
    pub show_help: bool,
    /// Whether focus mode is active (single-task view)
    pub focus_mode: bool,
    /// Current terminal dimensions (width, height)
    pub terminal_size: (u16, u16),
    /// Which pane currently has keyboard focus
    pub focus_pane: FocusPane,
    /// Index of selected item in sidebar
    pub sidebar_selected: usize,
    /// Currently selected project for filtering (if any)
    pub selected_project: Option<ProjectId>,

    // Input state
    /// Current input mode (Normal or Editing)
    pub input_mode: InputMode,
    /// What the input is targeting (new task, edit, search, etc.)
    pub input_target: InputTarget,
    /// Current text in the input field
    pub input_buffer: String,
    /// Cursor position within input buffer
    pub cursor_position: usize,
    /// Whether delete confirmation dialog is showing
    pub show_confirm_delete: bool,

    // Multi-select state for bulk operations
    /// Set of selected task IDs for bulk operations
    pub selected_tasks: std::collections::HashSet<TaskId>,
    /// Whether multi-select mode is active
    pub multi_select_mode: bool,

    // Storage
    /// Active storage backend (if configured)
    pub(crate) storage: Option<Box<dyn StorageBackend>>,
    /// Path to data file/directory
    pub data_path: Option<PathBuf>,
    /// Whether there are unsaved changes
    pub dirty: bool,

    // Configuration
    /// Default priority for new tasks
    pub default_priority: Priority,

    // Undo history
    /// Undo/redo action stack
    pub undo_stack: UndoStack,

    // Calendar state
    /// State for the calendar view
    pub calendar_state: CalendarState,

    // Status message for user feedback
    /// Temporary status message to display to user
    pub status_message: Option<String>,
    /// When the status message was set (for auto-clear after timeout)
    pub status_message_set_at: Option<Instant>,

    // Macro recording/playback state
    /// Keyboard macro recording and playback state
    pub macro_state: MacroState,
    /// Pending macro slot when starting recording
    pub pending_macro_slot: Option<usize>,

    // Task templates
    /// Task template manager
    pub template_manager: TemplateManager,
    /// Whether template picker is visible
    pub show_templates: bool,
    /// Index of selected template in picker
    pub template_selected: usize,

    // Pomodoro timer
    /// Active Pomodoro session (if any)
    pub pomodoro_session: Option<crate::domain::PomodoroSession>,
    /// Pomodoro timer configuration
    pub pomodoro_config: crate::domain::PomodoroConfig,
    /// Pomodoro statistics
    pub pomodoro_stats: crate::domain::PomodoroStats,

    // Keybindings editor
    /// Whether keybindings editor is visible
    pub show_keybindings_editor: bool,
    /// Selected keybinding index in editor
    pub keybinding_selected: usize,
    /// Whether currently capturing a new key
    pub keybinding_capturing: bool,
    /// Keybindings configuration (mutable for editing)
    pub keybindings: crate::config::Keybindings,

    // Reports state
    /// Selected panel in the reports view
    pub report_panel: crate::ui::ReportPanel,

    // Import state
    /// Pending import result awaiting confirmation
    pub pending_import: Option<crate::storage::ImportResult>,
    /// Whether import preview dialog is showing
    pub show_import_preview: bool,

    // Overdue alert state
    /// Whether overdue tasks alert is visible (shown at startup)
    pub show_overdue_alert: bool,

    // Storage error alert state
    /// Storage load error message (if any)
    pub storage_load_error: Option<String>,
    /// Whether storage error alert is visible
    pub show_storage_error_alert: bool,
    /// Error message to display in footer (shown in red)
    pub error_message: Option<String>,

    // Time log editor state
    /// Whether time log editor is visible
    pub show_time_log: bool,
    /// Selected time entry index in log
    pub time_log_selected: usize,
    /// Current mode in time log editor
    pub time_log_mode: crate::ui::TimeLogMode,
    /// Text buffer for editing time entries
    pub time_log_buffer: String,

    // Work log state
    /// All work log entries indexed by ID
    pub work_logs: HashMap<WorkLogEntryId, WorkLogEntry>,
    /// Whether work log editor is visible
    pub show_work_log: bool,
    /// Selected work log entry index
    pub work_log_selected: usize,
    /// Current mode in work log editor
    pub work_log_mode: crate::ui::WorkLogMode,
    /// Text buffer for editing work log entries (multi-line)
    pub work_log_buffer: Vec<String>,
    /// Cursor line position in work log buffer
    pub work_log_cursor_line: usize,
    /// Cursor column position in work log buffer
    pub work_log_cursor_col: usize,
    /// Search query for filtering work log entries
    pub work_log_search_query: String,

    // Description editor state (multi-line)
    /// Whether description editor is visible
    pub show_description_editor: bool,
    /// Text buffer for editing description (multi-line)
    pub description_buffer: Vec<String>,
    /// Cursor line position in description buffer
    pub description_cursor_line: usize,
    /// Cursor column position in description buffer
    pub description_cursor_col: usize,

    // Saved filters
    /// User-defined saved filters (smart lists)
    pub saved_filters: HashMap<SavedFilterId, SavedFilter>,
    /// Currently active saved filter (if any)
    pub active_saved_filter: Option<SavedFilterId>,
    /// Whether saved filter picker is visible
    pub show_saved_filter_picker: bool,
    /// Selected index in saved filter picker
    pub saved_filter_selected: usize,

    // Daily review mode
    /// Whether daily review mode is active
    pub show_daily_review: bool,
    /// Current phase of the daily review
    pub daily_review_phase: crate::ui::DailyReviewPhase,
    /// Selected index within current review phase
    pub daily_review_selected: usize,

    // Weekly review mode
    /// Whether weekly review mode is active
    pub show_weekly_review: bool,
    /// Current phase of the weekly review
    pub weekly_review_phase: crate::ui::WeeklyReviewPhase,
    /// Selected index within current review phase
    pub weekly_review_selected: usize,

    // Timeline state
    /// State for the timeline/Gantt view
    pub timeline_state: TimelineState,

    // View-specific selection state
    /// Selected column in Kanban view (0-3: Todo, InProgress, Blocked, Done)
    pub kanban_selected_column: usize,
    /// Selected quadrant in Eisenhower view (0-3: TL, TR, BL, BR)
    pub eisenhower_selected_quadrant: usize,
    /// Selected day in WeeklyPlanner view (0-6: Mon-Sun)
    pub weekly_planner_selected_day: usize,

    // Habit tracking
    /// All habits indexed by ID
    pub habits: HashMap<HabitId, Habit>,
    /// Visible habit IDs (filtered by archived status)
    pub visible_habits: Vec<HabitId>,
    /// Index of selected habit in list
    pub habit_selected: usize,
    /// Whether habit analytics popup is visible
    pub show_habit_analytics: bool,
    /// Whether to show archived habits
    pub show_archived_habits: bool,

    // Performance caches
    /// Cached footer statistics (completed, overdue, due today counts)
    pub footer_stats: FooterStats,
    /// Cached per-task metadata (time sums, depths, subtask progress)
    pub task_cache: TaskCache,
}

impl Model {
    /// Creates a new Model with default settings.
    ///
    /// The model starts with:
    /// - Empty task and project collections
    /// - `TaskList` view selected
    /// - Sidebar visible
    /// - No active filters
    ///
    /// # Examples
    ///
    /// ```
    /// use taskflow::app::Model;
    ///
    /// let model = Model::new();
    /// assert!(model.tasks.is_empty());
    /// assert!(model.show_sidebar);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            running: RunningState::default(),
            tasks: HashMap::new(),
            projects: HashMap::new(),
            time_entries: HashMap::new(),
            active_time_entry: None,
            current_view: ViewId::default(),
            selected_index: 0,
            visible_tasks: Vec::new(),
            filter: Filter::default(),
            sort: SortSpec::default(),
            show_completed: false,
            show_sidebar: true,
            show_help: false,
            focus_mode: false,
            terminal_size: (80, 24),
            focus_pane: FocusPane::default(),
            sidebar_selected: 0,
            selected_project: None,
            input_mode: InputMode::Normal,
            input_target: InputTarget::default(),
            input_buffer: String::new(),
            cursor_position: 0,
            show_confirm_delete: false,
            selected_tasks: std::collections::HashSet::new(),
            multi_select_mode: false,
            storage: None,
            data_path: None,
            dirty: false,
            default_priority: Priority::default(),
            undo_stack: UndoStack::new(),
            calendar_state: CalendarState::default(),
            status_message: None,
            status_message_set_at: None,
            macro_state: MacroState::new(),
            pending_macro_slot: None,
            template_manager: TemplateManager::new(),
            show_templates: false,
            template_selected: 0,
            pomodoro_session: None,
            pomodoro_config: crate::domain::PomodoroConfig::default(),
            pomodoro_stats: crate::domain::PomodoroStats::default(),
            show_keybindings_editor: false,
            keybinding_selected: 0,
            keybinding_capturing: false,
            keybindings: crate::config::Keybindings::load(),
            report_panel: crate::ui::ReportPanel::default(),
            pending_import: None,
            show_import_preview: false,
            show_overdue_alert: false,
            storage_load_error: None,
            show_storage_error_alert: false,
            error_message: None,
            show_time_log: false,
            time_log_selected: 0,
            time_log_mode: crate::ui::TimeLogMode::default(),
            time_log_buffer: String::new(),
            work_logs: HashMap::new(),
            show_work_log: false,
            work_log_selected: 0,
            work_log_mode: crate::ui::WorkLogMode::default(),
            work_log_buffer: vec![String::new()],
            work_log_cursor_line: 0,
            work_log_cursor_col: 0,
            work_log_search_query: String::new(),
            show_description_editor: false,
            description_buffer: vec![String::new()],
            description_cursor_line: 0,
            description_cursor_col: 0,
            saved_filters: HashMap::new(),
            active_saved_filter: None,
            show_saved_filter_picker: false,
            saved_filter_selected: 0,
            show_daily_review: false,
            daily_review_phase: crate::ui::DailyReviewPhase::default(),
            daily_review_selected: 0,
            show_weekly_review: false,
            weekly_review_phase: crate::ui::WeeklyReviewPhase::default(),
            weekly_review_selected: 0,
            timeline_state: TimelineState::default(),
            kanban_selected_column: 0,
            eisenhower_selected_quadrant: 0,
            weekly_planner_selected_day: 0,
            habits: HashMap::new(),
            visible_habits: Vec::new(),
            habit_selected: 0,
            show_habit_analytics: false,
            show_archived_habits: false,
            footer_stats: FooterStats::default(),
            task_cache: TaskCache::new(),
        }
    }

    /// Returns all tasks as a vector for export.
    ///
    /// This collects all tasks regardless of filter or view settings.
    /// Useful for exporting to CSV or ICS format.
    #[must_use]
    pub fn tasks_for_export(&self) -> Vec<Task> {
        self.tasks.values().cloned().collect()
    }

    /// Returns the total number of items in the sidebar.
    ///
    /// Uses [`SIDEBAR_FIRST_PROJECT_INDEX`] as the base count, plus projects.
    #[must_use]
    pub fn sidebar_item_count(&self) -> usize {
        // Base items (views + separator + Projects header) + project count
        SIDEBAR_FIRST_PROJECT_INDEX + self.projects.len().max(1)
    }

    /// Returns all tasks due on a specific day.
    ///
    /// Used by the calendar view to display tasks for a selected date.
    #[must_use]
    pub fn tasks_for_day(&self, date: NaiveDate) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|t| t.due_date == Some(date))
            .collect()
    }

    /// Returns all tasks due on the currently selected calendar day.
    ///
    /// Returns an empty vector if no day is selected.
    #[must_use]
    pub fn tasks_for_selected_day(&self) -> Vec<&Task> {
        if let Some(day) = self.calendar_state.selected_day {
            if let Some(date) =
                NaiveDate::from_ymd_opt(self.calendar_state.year, self.calendar_state.month, day)
            {
                return self.tasks_for_day(date);
            }
        }
        Vec::new()
    }

    /// Returns the count of visible tasks for a specific day.
    ///
    /// Respects the `show_completed` setting.
    #[must_use]
    pub fn task_count_for_day(&self, date: NaiveDate) -> usize {
        self.tasks
            .values()
            .filter(|t| {
                t.due_date == Some(date) && (self.show_completed || !t.status.is_complete())
            })
            .count()
    }

    /// Returns true if any incomplete task on the given day is overdue.
    #[must_use]
    pub fn has_overdue_on_day(&self, date: NaiveDate) -> bool {
        let today = Utc::now().date_naive();
        date < today
            && self
                .tasks
                .values()
                .any(|t| t.due_date == Some(date) && !t.status.is_complete())
    }

    /// Returns work log entries for a specific task, ordered by creation time (newest first).
    #[must_use]
    pub fn work_logs_for_task(&self, task_id: &TaskId) -> Vec<&WorkLogEntry> {
        let mut logs: Vec<_> = self
            .work_logs
            .values()
            .filter(|e| &e.task_id == task_id)
            .collect();
        logs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        logs
    }

    /// Returns the count of work log entries for a specific task.
    #[must_use]
    pub fn work_log_count_for_task(&self, task_id: &TaskId) -> usize {
        self.work_logs
            .values()
            .filter(|e| &e.task_id == task_id)
            .count()
    }

    /// Refresh the visible habits list based on archive status filter.
    pub fn refresh_visible_habits(&mut self) {
        self.visible_habits = self
            .habits
            .values()
            .filter(|h| self.show_archived_habits || !h.archived)
            .map(|h| h.id)
            .collect();
        // Sort by name
        self.visible_habits.sort_by(|a, b| {
            let habit_a = self.habits.get(a);
            let habit_b = self.habits.get(b);
            match (habit_a, habit_b) {
                (Some(a), Some(b)) => a.name.cmp(&b.name),
                _ => std::cmp::Ordering::Equal,
            }
        });
        // Clamp selection
        if !self.visible_habits.is_empty() {
            self.habit_selected = self.habit_selected.min(self.visible_habits.len() - 1);
        } else {
            self.habit_selected = 0;
        }
    }

    /// Returns the currently selected habit (if any).
    #[must_use]
    pub fn selected_habit(&self) -> Option<&Habit> {
        self.visible_habits
            .get(self.habit_selected)
            .and_then(|id| self.habits.get(id))
    }

    /// Returns all habits as a vector for export.
    #[must_use]
    pub fn habits_for_export(&self) -> Vec<Habit> {
        self.habits.values().cloned().collect()
    }
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export UndoAction for use in submodules
pub(crate) use super::UndoAction;

#[cfg(test)]
mod tests;
