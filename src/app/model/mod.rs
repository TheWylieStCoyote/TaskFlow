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
mod editor;
mod filtering;
mod hierarchy;
mod layout_cache;
mod sample_data;
mod storage;
mod time_tracking;
mod types;
mod view_queries;

pub use editor::MultilineEditor;

pub use cache::{FooterStats, ReportCache, TaskCache};
pub use layout_cache::LayoutCache;
pub use types::{
    AlertState, BurndownMode, BurndownState, BurndownTimeWindow, CalendarState,
    CommandPaletteState, DailyReviewState, DescriptionEditorState, DuplicatesViewState,
    EveningReviewState, FilterState, GoalViewState, HabitViewState, ImportState, InputState,
    KeybindingsEditorState, MultiSelectState, PomodoroState, RunningState, SavedFilterPickerState,
    StorageState, TaskDetailState, TemplatePickerState, TimeLogEditorState, TimelineState,
    TimelineZoom, ViewSelectionState, WeeklyReviewState, WorkLogEditorState,
};
pub use view_queries::extract_git_location;

use std::collections::HashMap;

use chrono::{NaiveDate, Utc};

use crate::domain::{
    CalendarEvent, CalendarEventId, Goal, GoalId, Habit, HabitId, KeyResult, KeyResultId, Priority,
    Project, ProjectId, SavedFilter, SavedFilterId, Task, TaskId, TimeEntry, TimeEntryId,
    WorkLogEntry, WorkLogEntryId,
};

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
    ViewId::Goals,            // 9: Goals
    ViewId::Blocked,          // 10: Blocked
    ViewId::Untagged,         // 11: Untagged
    ViewId::NoProject,        // 12: No Project
    ViewId::RecentlyModified, // 13: Recent
    ViewId::Kanban,           // 14: Kanban
    ViewId::Eisenhower,       // 15: Eisenhower
    ViewId::WeeklyPlanner,    // 16: Weekly Planner
    ViewId::Timeline,         // 17: Timeline
    ViewId::Snoozed,          // 18: Snoozed
    ViewId::Heatmap,          // 19: Heatmap
    ViewId::Forecast,         // 20: Forecast
    ViewId::Network,          // 21: Network
    ViewId::Burndown,         // 22: Burndown
    ViewId::Duplicates,       // 23: Duplicates
    ViewId::GitTodos,         // 24: Git TODOs
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
    /// Imported calendar events indexed by ID
    pub calendar_events: HashMap<CalendarEventId, CalendarEvent>,

    // Navigation
    /// Currently displayed view
    pub current_view: ViewId,
    /// Index of selected item in the task list
    pub selected_index: usize,

    // Visible items (filtered and sorted)
    /// Task IDs visible in current view after filtering and sorting
    pub visible_tasks: Vec<TaskId>,

    // Filter/Sort
    /// Filter and sort state (filter, sort, show_completed)
    pub filtering: FilterState,

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
    /// Text input state (mode, target, buffer, cursor)
    pub input: InputState,
    /// Whether delete confirmation dialog is showing
    pub show_confirm_delete: bool,

    // Multi-select state for bulk operations
    /// Multi-select state (mode, selected tasks)
    pub multi_select: MultiSelectState,

    // Storage
    /// Storage state (backend, data_path, dirty)
    pub storage: StorageState,

    // Configuration
    /// Default priority for new tasks
    pub default_priority: Priority,

    // Undo history
    /// Undo/redo action stack
    pub undo_stack: UndoStack,

    // Calendar state
    /// State for the calendar view
    pub calendar_state: CalendarState,

    // Macro recording/playback state
    /// Keyboard macro recording and playback state
    pub macro_state: MacroState,

    // Task templates
    /// Task template manager
    pub template_manager: TemplateManager,
    /// Template picker state
    pub template_picker: TemplatePickerState,

    // Pomodoro timer
    /// Pomodoro timer state (session, config, stats)
    pub pomodoro: PomodoroState,

    // Keybindings editor
    /// Keybindings editor state
    pub keybindings_editor: KeybindingsEditorState,
    /// Keybindings configuration (mutable for editing)
    pub keybindings: crate::config::Keybindings,

    // Reports state
    /// Selected panel in the reports view
    pub report_panel: crate::ui::ReportPanel,

    // Import state
    /// Import state (pending, show_preview)
    pub import: ImportState,

    // Alert state
    /// Consolidated alert and error state
    pub alerts: AlertState,

    // Time log editor state
    /// Time log editor state
    pub time_log: TimeLogEditorState,

    // Work log state
    /// All work log entries indexed by ID
    pub work_logs: HashMap<WorkLogEntryId, WorkLogEntry>,
    /// Work log editor state
    pub work_log_editor: WorkLogEditorState,

    // Description editor state (multi-line)
    /// Description editor state
    pub description_editor: DescriptionEditorState,

    // Saved filters
    /// User-defined saved filters (smart lists)
    pub saved_filters: HashMap<SavedFilterId, SavedFilter>,
    /// Currently active saved filter (if any)
    pub active_saved_filter: Option<SavedFilterId>,
    /// Saved filter picker state
    pub saved_filter_picker: SavedFilterPickerState,

    // Daily review mode
    /// Daily review state
    pub daily_review: DailyReviewState,

    // Weekly review mode
    /// Weekly review state
    pub weekly_review: WeeklyReviewState,

    // Evening review mode
    /// Evening review state (end-of-day reflection)
    pub evening_review: EveningReviewState,

    // Timeline state
    /// State for the timeline/Gantt view
    pub timeline_state: TimelineState,

    // View-specific selection state
    /// Selection state for specialized views (Kanban, Eisenhower, WeeklyPlanner)
    pub view_selection: ViewSelectionState,

    // Habit tracking
    /// All habits indexed by ID
    pub habits: HashMap<HabitId, Habit>,
    /// Visible habit IDs (filtered by archived status)
    pub visible_habits: Vec<HabitId>,
    /// Habit view state (selection, analytics, archive filter)
    pub habit_view: HabitViewState,

    // Goal/OKR tracking
    /// All goals indexed by ID
    pub goals: HashMap<GoalId, Goal>,
    /// All key results indexed by ID
    pub key_results: HashMap<KeyResultId, KeyResult>,
    /// Visible goal IDs (filtered by archived/quarter status)
    pub visible_goals: Vec<GoalId>,
    /// Goal view state (selection, expansion, filters)
    pub goal_view: GoalViewState,

    // Duplicate detection
    /// Duplicates view state (selection, pairs, threshold)
    pub duplicates_view: DuplicatesViewState,

    // Performance caches
    /// Cached footer statistics (completed, overdue, due today counts)
    pub footer_stats: FooterStats,
    /// Cached per-task metadata (time sums, depths, subtask progress)
    pub task_cache: TaskCache,
    /// Cached layout rectangles for mouse hit-testing
    pub layout_cache: LayoutCache,
    /// Cached analytics reports for different time windows
    pub report_cache: ReportCache,

    // Burndown chart state
    /// Burndown chart configuration (time window, mode, scope creep)
    pub burndown_state: BurndownState,
    // External command execution
    /// Pending editor command to execute (editor, file, line)
    pub pending_editor_command: Option<(String, String, String)>,

    // Task detail modal
    /// Task detail modal state (visibility, scroll)
    pub task_detail: TaskDetailState,

    // Command palette
    /// Command palette state (searchable action launcher)
    pub command_palette: CommandPaletteState,

    // Task list scroll state
    /// Ratatui ListState for task list scrolling (persists scroll offset).
    /// Uses RefCell for interior mutability during rendering.
    pub task_list_state: std::cell::RefCell<ratatui::widgets::ListState>,

    // Sidebar scroll state
    /// Ratatui ListState for sidebar scrolling (persists scroll offset).
    /// Uses RefCell for interior mutability during rendering.
    pub sidebar_list_state: std::cell::RefCell<ratatui::widgets::ListState>,
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
            calendar_events: HashMap::new(),
            current_view: ViewId::default(),
            selected_index: 0,
            visible_tasks: Vec::new(),
            filtering: FilterState::default(),
            show_sidebar: true,
            show_help: false,
            focus_mode: false,
            terminal_size: (80, 24),
            focus_pane: FocusPane::default(),
            sidebar_selected: 0,
            selected_project: None,
            input: InputState::default(),
            show_confirm_delete: false,
            multi_select: MultiSelectState::default(),
            storage: StorageState::default(),
            default_priority: Priority::default(),
            undo_stack: UndoStack::new(),
            calendar_state: CalendarState::default(),
            macro_state: MacroState::new(),
            template_manager: TemplateManager::new(),
            template_picker: TemplatePickerState::default(),
            pomodoro: PomodoroState::default(),
            keybindings_editor: KeybindingsEditorState::default(),
            keybindings: crate::config::Keybindings::load(),
            report_panel: crate::ui::ReportPanel::default(),
            import: ImportState::default(),
            alerts: AlertState::default(),
            time_log: TimeLogEditorState::default(),
            work_logs: HashMap::new(),
            work_log_editor: WorkLogEditorState::default(),
            description_editor: DescriptionEditorState::default(),
            saved_filters: HashMap::new(),
            active_saved_filter: None,
            saved_filter_picker: SavedFilterPickerState::default(),
            daily_review: DailyReviewState::default(),
            weekly_review: WeeklyReviewState::default(),
            evening_review: EveningReviewState::default(),
            timeline_state: TimelineState::default(),
            view_selection: ViewSelectionState::default(),
            habits: HashMap::new(),
            visible_habits: Vec::new(),
            habit_view: HabitViewState::default(),
            goals: HashMap::new(),
            key_results: HashMap::new(),
            visible_goals: Vec::new(),
            goal_view: GoalViewState::default(),
            duplicates_view: DuplicatesViewState::default(),
            footer_stats: FooterStats::default(),
            task_cache: TaskCache::new(),
            layout_cache: LayoutCache::default(),
            report_cache: ReportCache::new(),
            burndown_state: BurndownState::default(),
            pending_editor_command: None,
            task_detail: TaskDetailState::default(),
            command_palette: CommandPaletteState::default(),
            task_list_state: std::cell::RefCell::new(ratatui::widgets::ListState::default()),
            sidebar_list_state: std::cell::RefCell::new(ratatui::widgets::ListState::default()),
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
    /// Uses [`SIDEBAR_FIRST_PROJECT_INDEX`] as the base count, plus projects,
    /// plus contexts section, plus saved filters section.
    #[must_use]
    pub fn sidebar_item_count(&self) -> usize {
        // Base items (views + separator + Projects header) + project count
        let projects_section = SIDEBAR_FIRST_PROJECT_INDEX + self.projects.len().max(1);
        // +1 for separator, +1 for "Contexts" header, + contexts count (min 1 for hint message)
        let contexts = self.all_contexts();
        let contexts_section = projects_section + 2 + contexts.len().max(1);
        // +1 for separator, +1 for "Saved Filters" header, + filters count (min 1 for "Press F" message)
        contexts_section + 2 + self.saved_filters.len().max(1)
    }

    /// Returns the index where the contexts section starts in the sidebar.
    ///
    /// This is after the projects section, accounting for the separator and header.
    #[must_use]
    pub fn sidebar_contexts_start(&self) -> usize {
        // After projects section + separator + header
        SIDEBAR_FIRST_PROJECT_INDEX + self.projects.len().max(1) + 2
    }

    /// Returns the index where saved filters start in the sidebar.
    #[must_use]
    pub fn sidebar_saved_filters_start(&self) -> usize {
        // After contexts section + separator + header
        let contexts = self.all_contexts();
        self.sidebar_contexts_start() + contexts.len().max(1) + 2
    }

    /// Returns all unique context tags (@-prefixed) from tasks, sorted alphabetically.
    ///
    /// Context tags follow GTD (Getting Things Done) convention and represent
    /// where or when a task can be done (e.g., @home, @work, @errands).
    #[must_use]
    pub fn all_contexts(&self) -> Vec<String> {
        crate::domain::extract_contexts(self.tasks.values())
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
                t.due_date == Some(date)
                    && (self.filtering.show_completed || !t.status.is_complete())
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
    ///
    /// Uses cached task→log index for O(k) lookup where k = logs for task.
    #[must_use]
    pub fn work_logs_for_task(&self, task_id: &TaskId) -> Vec<&WorkLogEntry> {
        let mut logs: Vec<_> = self
            .task_cache
            .work_logs_by_task
            .get(task_id)
            .map(|ids| ids.iter().filter_map(|id| self.work_logs.get(id)).collect())
            .unwrap_or_default();
        logs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        logs
    }

    /// Returns the count of work log entries for a specific task.
    ///
    /// Uses cached task→log index for O(1) lookup.
    #[must_use]
    pub fn work_log_count_for_task(&self, task_id: &TaskId) -> usize {
        self.task_cache
            .work_logs_by_task
            .get(task_id)
            .map_or(0, Vec::len)
    }

    /// Refresh the visible habits list based on archive status filter.
    pub fn refresh_visible_habits(&mut self) {
        self.visible_habits = self
            .habits
            .values()
            .filter(|h| self.habit_view.show_archived || !h.archived)
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
        if self.visible_habits.is_empty() {
            self.habit_view.selected = 0;
        } else {
            self.habit_view.selected = self.habit_view.selected.min(self.visible_habits.len() - 1);
        }
    }

    /// Returns the currently selected habit (if any).
    #[must_use]
    pub fn selected_habit(&self) -> Option<&Habit> {
        self.visible_habits
            .get(self.habit_view.selected)
            .and_then(|id| self.habits.get(id))
    }

    /// Returns all habits as a vector for export.
    #[must_use]
    pub fn habits_for_export(&self) -> Vec<Habit> {
        self.habits.values().cloned().collect()
    }

    /// Returns calendar events occurring on a specific day.
    ///
    /// Events are sorted by start time.
    #[must_use]
    pub fn events_for_day(&self, date: NaiveDate) -> Vec<&CalendarEvent> {
        let mut events: Vec<_> = self
            .calendar_events
            .values()
            .filter(|e| e.occurs_on(date))
            .collect();
        events.sort_by_key(|e| e.start);
        events
    }

    /// Refresh the visible goals list based on filters.
    ///
    /// Filters by archived status and optionally by quarter.
    pub fn refresh_visible_goals(&mut self) {
        self.visible_goals = self
            .goals
            .values()
            .filter(|g| {
                // Filter by archived status
                if !self.goal_view.show_archived && !g.is_active() {
                    return false;
                }
                // Filter by quarter if set
                if let Some((year, quarter)) = self.goal_view.filter_quarter {
                    if g.quarter != Some((year, quarter)) {
                        return false;
                    }
                }
                true
            })
            .map(|g| g.id)
            .collect();

        // Sort by quarter (if present), then by name
        self.visible_goals.sort_by(|a, b| {
            let goal_a = self.goals.get(a);
            let goal_b = self.goals.get(b);
            match (goal_a, goal_b) {
                (Some(a), Some(b)) => {
                    // Sort by quarter first (goals with quarters come first)
                    match (&a.quarter, &b.quarter) {
                        (Some(qa), Some(qb)) => qa.cmp(qb).then(a.name.cmp(&b.name)),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.name.cmp(&b.name),
                    }
                }
                _ => std::cmp::Ordering::Equal,
            }
        });

        // Clamp selection
        if self.visible_goals.is_empty() {
            self.goal_view.selected_goal = 0;
        } else {
            self.goal_view.selected_goal = self
                .goal_view
                .selected_goal
                .min(self.visible_goals.len() - 1);
        }
    }

    /// Returns the currently selected goal (if any).
    #[must_use]
    pub fn selected_goal(&self) -> Option<&Goal> {
        self.visible_goals
            .get(self.goal_view.selected_goal)
            .and_then(|id| self.goals.get(id))
    }

    /// Returns all key results for a specific goal.
    ///
    /// Results are sorted by name.
    #[must_use]
    pub fn key_results_for_goal(&self, goal_id: GoalId) -> Vec<&KeyResult> {
        let mut krs: Vec<_> = self
            .key_results
            .values()
            .filter(|kr| kr.goal_id == goal_id)
            .collect();
        krs.sort_by(|a, b| a.name.cmp(&b.name));
        krs
    }

    /// Calculate goal progress (0-100).
    ///
    /// Returns manual progress if set, otherwise averages key result progress.
    #[must_use]
    pub fn goal_progress(&self, goal_id: GoalId) -> u8 {
        // Check manual override first
        if let Some(goal) = self.goals.get(&goal_id) {
            if let Some(manual) = goal.manual_progress {
                return manual;
            }
        }

        // Auto-calculate from key results
        let krs = self.key_results_for_goal(goal_id);
        if krs.is_empty() {
            return 0;
        }

        let total: u32 = krs
            .iter()
            .map(|kr| u32::from(self.key_result_progress(kr.id)))
            .sum();
        (total / krs.len() as u32) as u8
    }

    /// Calculate key result progress (0-100).
    ///
    /// Priority:
    /// 1. Manual progress override
    /// 2. Target/current value ratio
    /// 3. Linked task completion percentage
    #[must_use]
    pub fn key_result_progress(&self, kr_id: KeyResultId) -> u8 {
        if let Some(kr) = self.key_results.get(&kr_id) {
            // Manual override
            if let Some(manual) = kr.manual_progress {
                return manual;
            }

            // From target/current values
            if kr.target_value > 0.0 {
                let pct = (kr.current_value / kr.target_value * 100.0).min(100.0);
                return pct as u8;
            }

            // From linked tasks
            let completed = kr
                .linked_task_ids
                .iter()
                .filter(|id| self.tasks.get(id).is_some_and(|t| t.status.is_complete()))
                .count();
            let total = kr.linked_task_ids.len();
            if total > 0 {
                return (completed * 100 / total) as u8;
            }
        }
        0
    }

    /// Returns all goals as a vector for export.
    #[must_use]
    pub fn goals_for_export(&self) -> Vec<Goal> {
        self.goals.values().cloned().collect()
    }

    /// Returns all key results as a vector for export.
    #[must_use]
    pub fn key_results_for_export(&self) -> Vec<KeyResult> {
        self.key_results.values().cloned().collect()
    }
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Report Cache Methods
// ============================================================================

impl Model {
    /// Ensure the report cache is populated with analytics reports.
    ///
    /// This should be called before rendering the Reports view to avoid
    /// regenerating reports on every frame. Reports are only generated
    /// if the cache is empty.
    pub fn ensure_report_cache_populated(&mut self) {
        use crate::app::analytics::AnalyticsEngine;
        use crate::domain::analytics::ReportConfig;

        // Generate 30-day report if missing (used by overview, tags, time panels)
        if self.report_cache.report_30d.is_none() {
            let engine = AnalyticsEngine::new(self);
            let config = ReportConfig::last_n_days(30);
            self.report_cache.report_30d = Some(engine.generate_report(&config));
        }

        // Generate 60-day report if missing (used by velocity panel)
        if self.report_cache.report_60d.is_none() {
            let engine = AnalyticsEngine::new(self);
            let config = ReportConfig::last_n_days(60);
            self.report_cache.report_60d = Some(engine.generate_report(&config));
        }

        // Generate 90-day report if missing (used by insights panel)
        if self.report_cache.report_90d.is_none() {
            let engine = AnalyticsEngine::new(self);
            let config = ReportConfig::last_n_days(90);
            self.report_cache.report_90d = Some(engine.generate_report(&config));
        }
    }

    /// Invalidate the report cache.
    ///
    /// Call this when tasks or time entries are modified to ensure
    /// fresh reports are generated on the next Reports view render.
    pub fn invalidate_report_cache(&mut self) {
        self.report_cache.clear();
    }
}

// Re-export UndoAction for use in submodules
pub(crate) use super::UndoAction;

#[cfg(test)]
mod tests;
