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

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{Datelike, NaiveDate, Utc};

use crate::domain::{
    Filter, Priority, Project, ProjectId, SortSpec, Task, TaskId, TimeEntry, TimeEntryId,
};
#[allow(unused_imports)]
use crate::storage::{self, BackendType, ProjectRepository, StorageBackend, TaskRepository};
use crate::ui::{InputMode, InputTarget};

use super::{FocusPane, MacroState, TemplateManager, UndoStack, ViewId};

// ============================================================================
// Sidebar Layout Constants
// ============================================================================
// These constants define the sidebar structure. When adding/removing views,
// update SIDEBAR_VIEW_COUNT and the indices will adjust automatically.
//
// Layout:
//   [0..SIDEBAR_VIEW_COUNT-1]     = View items (All Tasks, Today, etc.)
//   SIDEBAR_SEPARATOR_INDEX       = Separator line
//   SIDEBAR_PROJECTS_HEADER_INDEX = "Projects" header
//   SIDEBAR_FIRST_PROJECT_INDEX+  = Individual projects

/// Number of view items in the sidebar (before the separator).
/// Views: All Tasks, Today, Upcoming, Overdue, Scheduled, Calendar,
///        Dashboard, Reports, Blocked, Untagged, No Project, Recent
pub const SIDEBAR_VIEW_COUNT: usize = 12;

/// Index of the separator line in the sidebar.
pub const SIDEBAR_SEPARATOR_INDEX: usize = SIDEBAR_VIEW_COUNT; // 12

/// Index of the "Projects" header in the sidebar.
pub const SIDEBAR_PROJECTS_HEADER_INDEX: usize = SIDEBAR_SEPARATOR_INDEX + 1; // 13

/// Index where individual projects start in the sidebar.
pub const SIDEBAR_FIRST_PROJECT_INDEX: usize = SIDEBAR_PROJECTS_HEADER_INDEX + 1; // 14

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
    storage: Option<Box<dyn StorageBackend>>,
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

    /// Configures storage and loads existing data.
    ///
    /// Initializes a storage backend and loads any existing tasks and projects.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend fails to initialize or load data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use taskflow::app::Model;
    /// use taskflow::storage::BackendType;
    /// use std::path::PathBuf;
    ///
    /// let model = Model::new()
    ///     .with_storage(BackendType::Json, PathBuf::from("tasks.json"))
    ///     .expect("Failed to load storage");
    /// ```
    pub fn with_storage(
        mut self,
        backend_type: BackendType,
        path: PathBuf,
    ) -> anyhow::Result<Self> {
        let mut backend = storage::create_backend(backend_type, &path)?;

        // Load tasks from storage
        let tasks = backend.list_tasks()?;
        for task in tasks {
            self.tasks.insert(task.id.clone(), task);
        }

        // Load projects from storage
        let projects = storage::ProjectRepository::list_projects(backend.as_mut())?;
        for project in projects {
            self.projects.insert(project.id.clone(), project);
        }

        // Load Pomodoro state
        let export_data = backend.export_all()?;
        if let Some(mut session) = export_data.pomodoro_session {
            // Recalculate remaining time based on elapsed time since last save
            let config = export_data
                .pomodoro_config
                .as_ref()
                .unwrap_or(&self.pomodoro_config);
            session.recalculate_remaining_time(config);

            // Validate that the task still exists
            if self.tasks.contains_key(&session.task_id) {
                self.pomodoro_session = Some(session);
            }
            // If task doesn't exist, discard the session
        }
        if let Some(config) = export_data.pomodoro_config {
            self.pomodoro_config = config;
        }
        if let Some(stats) = export_data.pomodoro_stats {
            self.pomodoro_stats = stats;
        }

        self.storage = Some(backend);
        self.data_path = Some(path);
        self.refresh_visible_tasks();

        Ok(self)
    }

    /// Saves current state to storage.
    ///
    /// Flushes any pending changes to the configured storage backend.
    /// Clears the dirty flag on success.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage backend fails to flush data.
    pub fn save(&mut self) -> anyhow::Result<()> {
        if let Some(ref mut backend) = self.storage {
            // Sync Pomodoro state before flushing
            backend.set_pomodoro_session(self.pomodoro_session.as_ref())?;
            backend.set_pomodoro_config(&self.pomodoro_config)?;
            backend.set_pomodoro_stats(&self.pomodoro_stats)?;

            backend.flush()?;
            self.dirty = false;
        }
        Ok(())
    }

    /// Syncs a task change to storage.
    ///
    /// Creates or updates the task in the storage backend.
    /// Sets the dirty flag to indicate unsaved changes.
    pub fn sync_task(&mut self, task: &Task) {
        if let Some(ref mut backend) = self.storage {
            // Try update first, if not found, create
            if backend.update_task(task).is_err() {
                let _ = backend.create_task(task);
            }
            self.dirty = true;
        }
    }

    /// Deletes a task from storage.
    ///
    /// Removes the task from the storage backend.
    pub fn delete_task_from_storage(&mut self, id: &TaskId) {
        if let Some(ref mut backend) = self.storage {
            let _ = backend.delete_task(id);
            self.dirty = true;
        }
    }

    /// Syncs a project to storage.
    ///
    /// Creates or updates the project in the storage backend.
    pub fn sync_project(&mut self, project: &Project) {
        if let Some(ref mut backend) = self.storage {
            // Try update first, if not found, create
            if backend.update_project(project).is_err() {
                let _ = backend.create_project(project);
            }
            self.dirty = true;
        }
    }

    /// Adds sample tasks and projects for testing.
    ///
    /// Creates a set of example tasks across multiple projects with
    /// various priorities, statuses, and due dates. Useful for
    /// development and demonstration.
    ///
    /// # Panics
    ///
    /// Panics if the current date cannot be computed for sample due dates.
    ///
    /// # Examples
    ///
    /// ```
    /// use taskflow::app::Model;
    ///
    /// let model = Model::new().with_sample_data();
    /// assert!(!model.tasks.is_empty());
    /// assert!(!model.projects.is_empty());
    /// ```
    #[must_use]
    pub fn with_sample_data(mut self) -> Self {
        use crate::domain::{Priority, Project, TaskStatus};
        use chrono::{NaiveDate, Utc};

        // Create sample projects
        let backend_project = Project::new("Backend API");
        let frontend_project = Project::new("Frontend UI");
        let docs_project = Project::new("Documentation");

        let backend_id = backend_project.id.clone();
        let frontend_id = frontend_project.id.clone();
        let docs_id = docs_project.id.clone();

        self.projects.insert(backend_id.clone(), backend_project);
        self.projects.insert(frontend_id.clone(), frontend_project);
        self.projects.insert(docs_id.clone(), docs_project);

        let today = Utc::now().date_naive();
        let yesterday = today - chrono::Duration::days(1);
        let tomorrow = today + chrono::Duration::days(1);
        let next_week = today + chrono::Duration::days(7);

        let tasks = vec![
            // Backend tasks
            Task::new("Set up database schema")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::High)
                .with_project(backend_id.clone())
                .with_tags(vec!["database".into(), "setup".into()]),
            Task::new("Implement REST endpoints")
                .with_status(TaskStatus::InProgress)
                .with_priority(Priority::High)
                .with_project(backend_id.clone())
                .with_tags(vec!["api".into(), "rust".into()]),
            Task::new("Add authentication middleware")
                .with_priority(Priority::Urgent)
                .with_due_date(tomorrow)
                .with_project(backend_id.clone())
                .with_tags(vec!["security".into(), "api".into()]),
            Task::new("Write integration tests")
                .with_priority(Priority::Medium)
                .with_due_date(next_week)
                .with_project(backend_id)
                .with_tags(vec!["testing".into()]),
            // Frontend tasks
            Task::new("Design component library")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::High)
                .with_project(frontend_id.clone())
                .with_tags(vec!["design".into(), "ui".into()]),
            Task::new("Build task list widget")
                .with_status(TaskStatus::InProgress)
                .with_priority(Priority::High)
                .with_project(frontend_id.clone())
                .with_tags(vec!["ui".into(), "rust".into()]),
            Task::new("Add keyboard navigation")
                .with_priority(Priority::Medium)
                .with_due_date(today)
                .with_project(frontend_id.clone())
                .with_tags(vec!["ux".into(), "accessibility".into()]),
            Task::new("Implement dark mode")
                .with_priority(Priority::Low)
                .with_project(frontend_id)
                .with_tags(vec!["ui".into(), "design".into()]),
            // Documentation tasks
            Task::new("Write API documentation")
                .with_priority(Priority::Medium)
                .with_due_date(next_week)
                .with_project(docs_id.clone())
                .with_tags(vec!["docs".into(), "api".into()]),
            Task::new("Create user guide")
                .with_priority(Priority::Low)
                .with_project(docs_id)
                .with_tags(vec!["docs".into()]),
            // Standalone tasks (no project)
            Task::new("Fix critical bug in parser")
                .with_priority(Priority::Urgent)
                .with_due_date(yesterday)
                .with_tags(vec!["bug".into(), "urgent".into()]),
            Task::new("Review pull requests")
                .with_status(TaskStatus::InProgress)
                .with_priority(Priority::Medium)
                .with_due_date(today)
                .with_tags(vec!["review".into()]),
            Task::new("Update dependencies")
                .with_priority(Priority::Low)
                .with_tags(vec!["maintenance".into()]),
            Task::new("Plan next sprint")
                .with_priority(Priority::Medium)
                .with_due_date(NaiveDate::from_ymd_opt(2025, 12, 15).unwrap())
                .with_tags(vec!["planning".into()]),
            Task::new("Team sync meeting")
                .with_status(TaskStatus::Done)
                .with_priority(Priority::None)
                .with_tags(vec!["meeting".into()]),
        ];

        for task in tasks {
            self.tasks.insert(task.id.clone(), task);
        }

        self.refresh_visible_tasks();
        self
    }

    /// Recalculates visible tasks based on current filters and sort.
    ///
    /// This should be called after any change that affects which tasks
    /// are visible (adding/removing tasks, changing filters, switching views).
    /// Updates `visible_tasks` with the filtered and sorted task IDs.
    ///
    /// Subtasks are displayed directly after their parent task.
    pub fn refresh_visible_tasks(&mut self) {
        use crate::domain::{SortField, SortOrder};
        use std::collections::HashMap;

        // Collect all tasks that pass the filter
        let filtered_tasks: Vec<_> = self
            .tasks
            .values()
            .filter(|task| self.task_matches_filter(task))
            .collect();

        // Separate into parent tasks and subtasks
        let mut parent_tasks: Vec<_> = filtered_tasks
            .iter()
            .filter(|t| t.parent_task_id.is_none())
            .copied()
            .collect();

        // Build a map of parent_id -> subtasks for quick lookup
        let mut subtasks_by_parent: HashMap<&TaskId, Vec<&Task>> = HashMap::new();
        for task in &filtered_tasks {
            if let Some(ref parent_id) = task.parent_task_id {
                subtasks_by_parent.entry(parent_id).or_default().push(task);
            }
        }

        // Sort parent tasks based on SortSpec
        let sort_field = self.sort.field;
        let sort_order = self.sort.order;

        let sort_fn = |a: &&Task, b: &&Task| {
            let primary_cmp = match sort_field {
                SortField::CreatedAt => a.created_at.cmp(&b.created_at),
                SortField::UpdatedAt => a.updated_at.cmp(&b.updated_at),
                SortField::DueDate => {
                    // Tasks with no due date go last
                    match (a.due_date, b.due_date) {
                        (Some(da), Some(db)) => da.cmp(&db),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                }
                SortField::Priority => {
                    let priority_order = |p: &crate::domain::Priority| match p {
                        crate::domain::Priority::Urgent => 0,
                        crate::domain::Priority::High => 1,
                        crate::domain::Priority::Medium => 2,
                        crate::domain::Priority::Low => 3,
                        crate::domain::Priority::None => 4,
                    };
                    priority_order(&a.priority).cmp(&priority_order(&b.priority))
                }
                SortField::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                SortField::Status => {
                    let status_order = |s: &crate::domain::TaskStatus| match s {
                        crate::domain::TaskStatus::InProgress => 0,
                        crate::domain::TaskStatus::Todo => 1,
                        crate::domain::TaskStatus::Blocked => 2,
                        crate::domain::TaskStatus::Done => 3,
                        crate::domain::TaskStatus::Cancelled => 4,
                    };
                    status_order(&a.status).cmp(&status_order(&b.status))
                }
            };

            // Use sort_order as secondary sort key when primary values are equal
            // Tasks with sort_order come before tasks without
            let cmp = if primary_cmp == std::cmp::Ordering::Equal {
                match (a.sort_order, b.sort_order) {
                    (Some(oa), Some(ob)) => oa.cmp(&ob),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            } else {
                primary_cmp
            };

            match sort_order {
                SortOrder::Ascending => cmp,
                SortOrder::Descending => cmp.reverse(),
            }
        };

        parent_tasks.sort_by(sort_fn);

        // Also sort subtasks within each parent group
        for subtasks in subtasks_by_parent.values_mut() {
            subtasks.sort_by(sort_fn);
        }

        // Build final list: parent followed by its subtasks (recursively)
        let mut result = Vec::new();

        // Recursive helper to add a task and all its descendants
        fn add_with_descendants(
            task_id: &TaskId,
            subtasks_by_parent: &HashMap<&TaskId, Vec<&Task>>,
            result: &mut Vec<TaskId>,
        ) {
            result.push(task_id.clone());
            if let Some(children) = subtasks_by_parent.get(task_id) {
                for child in children {
                    add_with_descendants(&child.id, subtasks_by_parent, result);
                }
            }
        }

        for parent in parent_tasks {
            add_with_descendants(&parent.id, &subtasks_by_parent, &mut result);
        }

        // Handle orphaned subtasks (subtasks whose parent is not visible)
        // These are shown at the end
        for task in &filtered_tasks {
            if task.parent_task_id.is_some() && !result.contains(&task.id) {
                result.push(task.id.clone());
            }
        }

        self.visible_tasks = result;

        // Adjust selection if needed
        if self.selected_index >= self.visible_tasks.len() && !self.visible_tasks.is_empty() {
            self.selected_index = self.visible_tasks.len() - 1;
        }
    }

    fn task_matches_filter(&self, task: &Task) -> bool {
        // Filter out completed tasks unless show_completed is true
        if !self.show_completed && task.status.is_complete() {
            return false;
        }

        // Filter by search text (case-insensitive, matches title or tags)
        if let Some(ref search) = self.filter.search_text {
            let search_lower = search.to_lowercase();
            let title_matches = task.title.to_lowercase().contains(&search_lower);
            let tags_match = task
                .tags
                .iter()
                .any(|t| t.to_lowercase().contains(&search_lower));
            if !title_matches && !tags_match {
                return false;
            }
        }

        // Filter by tags (if set)
        if let Some(ref filter_tags) = self.filter.tags {
            use crate::domain::TagFilterMode;
            let has_tags = match self.filter.tags_mode {
                TagFilterMode::Any => {
                    // Task must have at least one of the filter tags
                    filter_tags.iter().any(|ft| {
                        task.tags
                            .iter()
                            .any(|t| t.to_lowercase() == ft.to_lowercase())
                    })
                }
                TagFilterMode::All => {
                    // Task must have all of the filter tags
                    filter_tags.iter().all(|ft| {
                        task.tags
                            .iter()
                            .any(|t| t.to_lowercase() == ft.to_lowercase())
                    })
                }
            };
            if !has_tags {
                return false;
            }
        }

        // Filter by selected project if any
        if let Some(ref project_id) = self.selected_project {
            if task.project_id.as_ref() != Some(project_id) {
                return false;
            }
        }

        // Filter by current view
        match self.current_view {
            // TaskList and Dashboard show all tasks
            ViewId::TaskList | ViewId::Dashboard => true,
            ViewId::Today => {
                // Show tasks due today
                task.due_date
                    .is_some_and(|d| d == chrono::Utc::now().date_naive())
            }
            ViewId::Upcoming => {
                // Show tasks with future due dates
                task.due_date
                    .is_some_and(|d| d > chrono::Utc::now().date_naive())
            }
            ViewId::Overdue => {
                // Show tasks with past due dates (before today)
                task.due_date
                    .is_some_and(|d| d < chrono::Utc::now().date_naive())
            }
            ViewId::Scheduled => {
                // Show tasks with scheduled dates
                task.scheduled_date.is_some()
            }
            ViewId::Calendar => {
                // Show tasks for the selected day in calendar (if any)
                self.calendar_state.selected_day.map_or_else(
                    || {
                        // No day selected, show tasks for the entire month
                        task.due_date.is_some_and(|d| {
                            d.year() == self.calendar_state.year
                                && d.month() == self.calendar_state.month
                        })
                    },
                    |selected_day| {
                        NaiveDate::from_ymd_opt(
                            self.calendar_state.year,
                            self.calendar_state.month,
                            selected_day,
                        )
                        .is_some_and(|date| task.due_date == Some(date))
                    },
                )
            }
            ViewId::Projects => {
                // Show tasks that belong to a project
                task.project_id.is_some()
            }
            ViewId::Blocked => {
                // Show tasks with incomplete dependencies
                !task.dependencies.is_empty()
                    && task.dependencies.iter().any(|dep_id| {
                        self.tasks
                            .get(dep_id)
                            .is_none_or(|d| !d.status.is_complete())
                    })
            }
            ViewId::Untagged => {
                // Show tasks without any tags
                task.tags.is_empty()
            }
            ViewId::NoProject => {
                // Show tasks not assigned to any project
                task.project_id.is_none()
            }
            ViewId::RecentlyModified => {
                // Show tasks modified in the last 7 days
                let week_ago = chrono::Utc::now() - chrono::Duration::days(7);
                task.updated_at >= week_ago
            }
            ViewId::Reports => {
                // Reports view shows all tasks (used for analytics)
                true
            }
        }
    }

    /// Returns the currently selected task, if any.
    ///
    /// Returns `None` if no tasks are visible or the selection is invalid.
    #[must_use]
    pub fn selected_task(&self) -> Option<&Task> {
        self.visible_tasks
            .get(self.selected_index)
            .and_then(|id| self.tasks.get(id))
    }

    /// Returns the currently selected task mutably, if any.
    #[must_use]
    pub fn selected_task_mut(&mut self) -> Option<&mut Task> {
        let id = self.visible_tasks.get(self.selected_index)?.clone();
        self.tasks.get_mut(&id)
    }

    /// Returns true if a storage backend is configured.
    #[must_use]
    pub const fn has_storage(&self) -> bool {
        self.storage.is_some()
    }

    /// Returns visible tasks grouped by project.
    ///
    /// Returns a `Vec` of (`Option<ProjectId>`, `project_name`, `Vec<TaskId>`).
    /// Projects are sorted alphabetically, with "No Project" last.
    /// Tasks within each project follow the current sort order.
    #[must_use]
    pub fn get_tasks_grouped_by_project(&self) -> Vec<(Option<ProjectId>, String, Vec<TaskId>)> {
        // Group visible tasks by project_id using a Vec to preserve order
        let mut grouped: Vec<(Option<ProjectId>, Vec<TaskId>)> = Vec::new();

        for task_id in &self.visible_tasks {
            if let Some(task) = self.tasks.get(task_id) {
                let project_id = task.project_id.clone();
                // Find existing group or create new one
                if let Some(group) = grouped.iter_mut().find(|(pid, _)| *pid == project_id) {
                    group.1.push(task_id.clone());
                } else {
                    grouped.push((project_id, vec![task_id.clone()]));
                }
            }
        }

        // Convert to vec with project names
        let mut result: Vec<(Option<ProjectId>, String, Vec<TaskId>)> = grouped
            .into_iter()
            .map(|(project_id, task_ids)| {
                let name = project_id
                    .as_ref()
                    .and_then(|pid| self.projects.get(pid))
                    .map_or_else(|| "No Project".to_string(), |p| p.name.clone());
                (project_id, name, task_ids)
            })
            .collect();

        // Sort by project name (No Project goes last)
        result.sort_by(|a, b| match (&a.0, &b.0) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Greater, // No Project last
            (Some(_), None) => std::cmp::Ordering::Less,
            (Some(_), Some(_)) => a.1.to_lowercase().cmp(&b.1.to_lowercase()),
        });

        result
    }

    /// Starts time tracking for a task.
    ///
    /// Automatically stops any currently running timer before starting
    /// a new one. Creates a new time entry and sets it as active.
    pub fn start_time_tracking(&mut self, task_id: TaskId) {
        // Stop any currently running timer
        self.stop_time_tracking();

        // Start new timer
        let entry = TimeEntry::start(task_id);
        let entry_id = entry.id.clone();
        self.time_entries.insert(entry_id.clone(), entry);
        self.active_time_entry = Some(entry_id);
        self.dirty = true;
    }

    /// Stops the currently active time tracking session.
    ///
    /// Records the end time and calculates duration for the active entry.
    pub fn stop_time_tracking(&mut self) {
        if let Some(ref entry_id) = self.active_time_entry.clone() {
            if let Some(entry) = self.time_entries.get_mut(entry_id) {
                entry.stop();
                self.dirty = true;
            }
            self.active_time_entry = None;
        }
    }

    /// Returns the currently active time entry, if any.
    #[must_use]
    pub fn active_time_entry(&self) -> Option<&TimeEntry> {
        self.active_time_entry
            .as_ref()
            .and_then(|id| self.time_entries.get(id))
    }

    /// Returns true if time is being tracked for the given task.
    #[must_use]
    pub fn is_tracking_task(&self, task_id: &TaskId) -> bool {
        self.active_time_entry()
            .is_some_and(|e| &e.task_id == task_id)
    }

    /// Returns total time tracked for a task in minutes.
    ///
    /// Sums the duration of all time entries for the given task.
    #[must_use]
    pub fn total_time_for_task(&self, task_id: &TaskId) -> u32 {
        self.time_entries
            .values()
            .filter(|e| &e.task_id == task_id)
            .map(TimeEntry::calculated_duration_minutes)
            .sum()
    }

    /// Returns subtask completion progress for a task.
    ///
    /// Returns a tuple of (completed_count, total_count) for subtasks
    /// that have this task as their parent.
    ///
    /// # Examples
    ///
    /// ```
    /// use taskflow::app::Model;
    /// use taskflow::domain::Task;
    ///
    /// let mut model = Model::new();
    /// let parent = Task::new("Parent task");
    /// let parent_id = parent.id.clone();
    ///
    /// let subtask1 = Task::new("Subtask 1").with_parent(parent_id.clone());
    /// let subtask2 = Task::new("Subtask 2").with_parent(parent_id.clone());
    ///
    /// model.tasks.insert(parent.id.clone(), parent);
    /// model.tasks.insert(subtask1.id.clone(), subtask1);
    /// model.tasks.insert(subtask2.id.clone(), subtask2);
    ///
    /// let (completed, total) = model.subtask_progress(&parent_id);
    /// assert_eq!(total, 2);
    /// assert_eq!(completed, 0);
    /// ```
    #[must_use]
    pub fn subtask_progress(&self, task_id: &TaskId) -> (usize, usize) {
        let descendants = self.get_all_descendants(task_id);
        let total = descendants.len();
        let completed = descendants
            .iter()
            .filter(|id| self.tasks.get(*id).is_some_and(|t| t.status.is_complete()))
            .count();
        (completed, total)
    }

    /// Returns the nesting depth of a task (0 for root tasks, 1 for direct children, etc.)
    ///
    /// Includes cycle detection to prevent infinite loops from corrupted data.
    #[must_use]
    pub fn task_depth(&self, task_id: &TaskId) -> usize {
        let mut depth = 0;
        let mut current_id = task_id.clone();
        let mut visited = std::collections::HashSet::new();

        while let Some(task) = self.tasks.get(&current_id) {
            if let Some(ref parent_id) = task.parent_task_id {
                if visited.contains(parent_id) {
                    // Circular reference detected - break to prevent infinite loop
                    break;
                }
                visited.insert(current_id.clone());
                depth += 1;
                current_id = parent_id.clone();
            } else {
                break;
            }
        }
        depth
    }

    /// Returns all descendant task IDs (children, grandchildren, etc.)
    ///
    /// Uses iterative approach with cycle detection.
    #[must_use]
    pub fn get_all_descendants(&self, task_id: &TaskId) -> Vec<TaskId> {
        let mut descendants = Vec::new();
        let mut stack = vec![task_id.clone()];
        let mut visited = std::collections::HashSet::new();

        while let Some(current_id) = stack.pop() {
            if visited.contains(&current_id) {
                continue; // Prevent cycles
            }
            visited.insert(current_id.clone());

            for (id, task) in &self.tasks {
                if task.parent_task_id.as_ref() == Some(&current_id) {
                    descendants.push(id.clone());
                    stack.push(id.clone());
                }
            }
        }
        descendants
    }

    /// Returns all ancestor task IDs (parent, grandparent, etc.)
    ///
    /// Uses iterative approach with cycle detection.
    #[must_use]
    pub fn get_all_ancestors(&self, task_id: &TaskId) -> Vec<TaskId> {
        let mut ancestors = Vec::new();
        let mut current_id = task_id.clone();
        let mut visited = std::collections::HashSet::new();

        while let Some(task) = self.tasks.get(&current_id) {
            if let Some(ref parent_id) = task.parent_task_id {
                if visited.contains(parent_id) {
                    break; // Circular reference
                }
                visited.insert(current_id.clone());
                ancestors.push(parent_id.clone());
                current_id = parent_id.clone();
            } else {
                break;
            }
        }
        ancestors
    }

    /// Checks if setting `new_parent_id` as parent of `task_id` would create a circular reference.
    #[must_use]
    pub fn would_create_cycle(&self, task_id: &TaskId, new_parent_id: &TaskId) -> bool {
        if task_id == new_parent_id {
            return true;
        }
        // Check if new_parent is a descendant of task_id
        self.get_all_descendants(task_id).contains(new_parent_id)
    }

    /// Returns true if the task has any subtasks (direct children).
    #[must_use]
    pub fn has_subtasks(&self, task_id: &TaskId) -> bool {
        self.tasks
            .values()
            .any(|t| t.parent_task_id.as_ref() == Some(task_id))
    }

    /// Returns recursive subtask completion as a percentage (0-100).
    ///
    /// Returns `None` if the task has no subtasks.
    #[must_use]
    pub fn subtask_percentage(&self, task_id: &TaskId) -> Option<u8> {
        let (completed, total) = self.subtask_progress(task_id);
        if total == 0 {
            None
        } else {
            Some(((completed * 100) / total) as u8)
        }
    }
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Priority, TaskStatus};

    #[test]
    fn test_model_new_defaults() {
        let model = Model::new();

        assert_eq!(model.running, RunningState::Running);
        assert!(model.tasks.is_empty());
        assert!(model.projects.is_empty());
        assert!(model.time_entries.is_empty());
        assert!(model.active_time_entry.is_none());
        assert_eq!(model.selected_index, 0);
        assert!(model.visible_tasks.is_empty());
        assert!(!model.show_completed);
        assert!(model.show_sidebar);
        assert!(!model.show_help);
        assert_eq!(model.input_mode, InputMode::Normal);
        assert!(model.input_buffer.is_empty());
        assert!(!model.dirty);
    }

    #[test]
    fn test_model_with_sample_data() {
        let model = Model::new().with_sample_data();

        // Sample data creates 15 tasks across 3 projects
        assert_eq!(model.tasks.len(), 15);
        assert_eq!(model.projects.len(), 3);
        // Some are completed, so visible should be less
        assert!(model.visible_tasks.len() < 15);
    }

    #[test]
    fn test_model_refresh_visible_tasks_sorts_by_priority() {
        use crate::domain::{SortField, SortOrder};

        let mut model = Model::new();

        // Add tasks with different priorities
        let urgent = Task::new("Urgent").with_priority(Priority::Urgent);
        let low = Task::new("Low").with_priority(Priority::Low);
        let high = Task::new("High").with_priority(Priority::High);

        model.tasks.insert(low.id.clone(), low.clone());
        model.tasks.insert(urgent.id.clone(), urgent.clone());
        model.tasks.insert(high.id.clone(), high.clone());

        // Set sort to priority (default is CreatedAt)
        model.sort = SortSpec {
            field: SortField::Priority,
            order: SortOrder::Ascending,
        };
        model.refresh_visible_tasks();

        // Order should be: Urgent, High, Low
        assert_eq!(model.visible_tasks.len(), 3);
        assert_eq!(model.visible_tasks[0], urgent.id);
        assert_eq!(model.visible_tasks[1], high.id);
        assert_eq!(model.visible_tasks[2], low.id);
    }

    #[test]
    fn test_model_refresh_visible_tasks_hides_completed() {
        let mut model = Model::new();
        model.show_completed = false;

        let todo = Task::new("Todo");
        let done = Task::new("Done").with_status(TaskStatus::Done);
        let cancelled = Task::new("Cancelled").with_status(TaskStatus::Cancelled);

        model.tasks.insert(todo.id.clone(), todo);
        model.tasks.insert(done.id.clone(), done);
        model.tasks.insert(cancelled.id.clone(), cancelled);

        model.refresh_visible_tasks();

        // Only non-completed tasks should be visible
        assert_eq!(model.visible_tasks.len(), 1);
    }

    #[test]
    fn test_model_refresh_visible_tasks_shows_completed() {
        let mut model = Model::new();
        model.show_completed = true;

        let todo = Task::new("Todo");
        let done = Task::new("Done").with_status(TaskStatus::Done);
        let cancelled = Task::new("Cancelled").with_status(TaskStatus::Cancelled);

        model.tasks.insert(todo.id.clone(), todo);
        model.tasks.insert(done.id.clone(), done);
        model.tasks.insert(cancelled.id.clone(), cancelled);

        model.refresh_visible_tasks();

        // All tasks should be visible
        assert_eq!(model.visible_tasks.len(), 3);
    }

    #[test]
    fn test_model_refresh_visible_tasks_subtasks_follow_parent() {
        use crate::domain::{SortField, SortOrder};

        let mut model = Model::new();

        // Create a parent task and two subtasks
        let parent1 = Task::new("Parent 1").with_priority(Priority::High);
        let subtask1a = Task::new("Subtask 1a").with_parent(parent1.id.clone());
        let subtask1b = Task::new("Subtask 1b").with_parent(parent1.id.clone());

        let parent2 = Task::new("Parent 2").with_priority(Priority::Low);
        let subtask2a = Task::new("Subtask 2a").with_parent(parent2.id.clone());

        // Insert in random order
        model.tasks.insert(subtask1b.id.clone(), subtask1b.clone());
        model.tasks.insert(parent2.id.clone(), parent2.clone());
        model.tasks.insert(subtask2a.id.clone(), subtask2a.clone());
        model.tasks.insert(parent1.id.clone(), parent1.clone());
        model.tasks.insert(subtask1a.id.clone(), subtask1a.clone());

        // Sort by priority so parent order is deterministic
        model.sort = SortSpec {
            field: SortField::Priority,
            order: SortOrder::Ascending,
        };
        model.refresh_visible_tasks();

        // Should be: Parent 1 (High), Subtask 1a, Subtask 1b, Parent 2 (Low), Subtask 2a
        assert_eq!(model.visible_tasks.len(), 5);
        assert_eq!(model.visible_tasks[0], parent1.id);
        // Subtasks of parent1 should immediately follow
        assert!(
            model.visible_tasks[1] == subtask1a.id || model.visible_tasks[1] == subtask1b.id,
            "Subtask 1 should follow parent 1"
        );
        assert!(
            model.visible_tasks[2] == subtask1a.id || model.visible_tasks[2] == subtask1b.id,
            "Subtask 2 should follow parent 1"
        );
        assert_eq!(model.visible_tasks[3], parent2.id);
        assert_eq!(model.visible_tasks[4], subtask2a.id);
    }

    #[test]
    fn test_model_selected_task_returns_correct() {
        let mut model = Model::new();

        let task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");

        model.tasks.insert(task1.id.clone(), task1.clone());
        model.tasks.insert(task2.id.clone(), task2.clone());
        model.refresh_visible_tasks();

        // Select first task
        model.selected_index = 0;
        let selected = model.selected_task().unwrap();
        assert_eq!(selected.id, model.visible_tasks[0]);

        // Select second task
        model.selected_index = 1;
        let selected = model.selected_task().unwrap();
        assert_eq!(selected.id, model.visible_tasks[1]);
    }

    #[test]
    fn test_model_selected_task_empty_list() {
        let model = Model::new();

        assert!(model.selected_task().is_none());
    }

    #[test]
    fn test_model_selected_index_adjustment() {
        let mut model = Model::new();

        // Add 3 tasks
        for i in 0..3 {
            let task = Task::new(format!("Task {}", i));
            model.tasks.insert(task.id.clone(), task);
        }
        model.refresh_visible_tasks();

        // Select last item
        model.selected_index = 2;

        // Remove all tasks except one
        let ids: Vec<_> = model.tasks.keys().skip(1).cloned().collect();
        for id in ids {
            model.tasks.remove(&id);
        }

        model.refresh_visible_tasks();

        // Selection should be adjusted to valid range
        assert!(model.selected_index < model.visible_tasks.len());
    }

    #[test]
    fn test_model_start_time_tracking() {
        let mut model = Model::new();

        let task = Task::new("Task");
        model.tasks.insert(task.id.clone(), task.clone());

        model.start_time_tracking(task.id.clone());

        assert!(model.active_time_entry.is_some());
        assert!(model.time_entries.len() == 1);
        assert!(model.dirty);

        let entry = model.active_time_entry().unwrap();
        assert_eq!(entry.task_id, task.id);
        assert!(entry.is_running());
    }

    #[test]
    fn test_model_stop_time_tracking() {
        let mut model = Model::new();

        let task = Task::new("Task");
        model.tasks.insert(task.id.clone(), task.clone());

        model.start_time_tracking(task.id.clone());
        model.stop_time_tracking();

        assert!(model.active_time_entry.is_none());

        // Entry should still exist but be stopped
        let entry = model.time_entries.values().next().unwrap();
        assert!(!entry.is_running());
    }

    #[test]
    fn test_model_start_stops_previous() {
        let mut model = Model::new();

        let task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");
        model.tasks.insert(task1.id.clone(), task1.clone());
        model.tasks.insert(task2.id.clone(), task2.clone());

        // Start tracking task1
        model.start_time_tracking(task1.id.clone());
        let first_entry_id = model.active_time_entry.clone().unwrap();

        // Start tracking task2 (should stop task1)
        model.start_time_tracking(task2.id.clone());

        // Two entries total
        assert_eq!(model.time_entries.len(), 2);

        // First entry should be stopped
        let first_entry = model.time_entries.get(&first_entry_id).unwrap();
        assert!(!first_entry.is_running());

        // Active entry should be for task2
        let active = model.active_time_entry().unwrap();
        assert_eq!(active.task_id, task2.id);
    }

    #[test]
    fn test_model_is_tracking_task() {
        let mut model = Model::new();

        let task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");
        model.tasks.insert(task1.id.clone(), task1.clone());
        model.tasks.insert(task2.id.clone(), task2.clone());

        // Not tracking anything initially
        assert!(!model.is_tracking_task(&task1.id));
        assert!(!model.is_tracking_task(&task2.id));

        // Start tracking task1
        model.start_time_tracking(task1.id.clone());

        assert!(model.is_tracking_task(&task1.id));
        assert!(!model.is_tracking_task(&task2.id));
    }

    #[test]
    fn test_model_total_time_for_task() {
        let mut model = Model::new();

        let task = Task::new("Task");
        model.tasks.insert(task.id.clone(), task.clone());

        // Add multiple completed time entries
        let mut entry1 = TimeEntry::start(task.id.clone());
        entry1.duration_minutes = Some(30);
        entry1.ended_at = Some(chrono::Utc::now());

        let mut entry2 = TimeEntry::start(task.id.clone());
        entry2.duration_minutes = Some(45);
        entry2.ended_at = Some(chrono::Utc::now());

        model.time_entries.insert(entry1.id.clone(), entry1);
        model.time_entries.insert(entry2.id.clone(), entry2);

        let total = model.total_time_for_task(&task.id);
        assert_eq!(total, 75); // 30 + 45
    }

    #[test]
    fn test_model_dirty_flag() {
        let mut model = Model::new();
        assert!(!model.dirty);

        let task = Task::new("Task");
        model.tasks.insert(task.id.clone(), task.clone());

        model.start_time_tracking(task.id.clone());
        assert!(model.dirty);
    }

    #[test]
    fn test_model_has_storage() {
        let model = Model::new();
        assert!(!model.has_storage());
    }

    #[test]
    fn test_running_state_default() {
        let state = RunningState::default();
        assert_eq!(state, RunningState::Running);
    }

    #[test]
    fn test_view_tasklist_shows_all() {
        let mut model = Model::new();
        model.current_view = ViewId::TaskList;

        // Create tasks with various due dates and project associations
        let task_no_date = Task::new("No due date");
        let task_with_date = Task::new("Has date")
            .with_due_date(chrono::NaiveDate::from_ymd_opt(2025, 12, 15).unwrap());
        let task_with_project =
            Task::new("Has project").with_project(crate::domain::ProjectId::new());

        model.tasks.insert(task_no_date.id.clone(), task_no_date);
        model
            .tasks
            .insert(task_with_date.id.clone(), task_with_date);
        model
            .tasks
            .insert(task_with_project.id.clone(), task_with_project);

        model.refresh_visible_tasks();

        // TaskList view should show all tasks
        assert_eq!(model.visible_tasks.len(), 3);
    }

    #[test]
    fn test_view_today_filters_due_today() {
        let mut model = Model::new();
        model.current_view = ViewId::Today;

        let today = chrono::Utc::now().date_naive();
        let tomorrow = today + chrono::Duration::days(1);

        let task_today = Task::new("Due today").with_due_date(today);
        let task_tomorrow = Task::new("Due tomorrow").with_due_date(tomorrow);
        let task_no_date = Task::new("No due date");

        model
            .tasks
            .insert(task_today.id.clone(), task_today.clone());
        model.tasks.insert(task_tomorrow.id.clone(), task_tomorrow);
        model.tasks.insert(task_no_date.id.clone(), task_no_date);

        model.refresh_visible_tasks();

        // Only today's task should be visible
        assert_eq!(model.visible_tasks.len(), 1);
        assert_eq!(model.visible_tasks[0], task_today.id);
    }

    #[test]
    fn test_view_upcoming_filters_future() {
        let mut model = Model::new();
        model.current_view = ViewId::Upcoming;

        let today = chrono::Utc::now().date_naive();
        let tomorrow = today + chrono::Duration::days(1);
        let next_week = today + chrono::Duration::days(7);

        let task_today = Task::new("Due today").with_due_date(today);
        let task_tomorrow = Task::new("Due tomorrow").with_due_date(tomorrow);
        let task_next_week = Task::new("Due next week").with_due_date(next_week);
        let task_no_date = Task::new("No due date");

        model.tasks.insert(task_today.id.clone(), task_today);
        model
            .tasks
            .insert(task_tomorrow.id.clone(), task_tomorrow.clone());
        model
            .tasks
            .insert(task_next_week.id.clone(), task_next_week.clone());
        model.tasks.insert(task_no_date.id.clone(), task_no_date);

        model.refresh_visible_tasks();

        // Only future tasks should be visible (not today, not tasks without dates)
        assert_eq!(model.visible_tasks.len(), 2);
        assert!(model.visible_tasks.contains(&task_tomorrow.id));
        assert!(model.visible_tasks.contains(&task_next_week.id));
    }

    #[test]
    fn test_view_projects_filters_with_project() {
        let mut model = Model::new();
        model.current_view = ViewId::Projects;

        let project_id = crate::domain::ProjectId::new();

        let task_with_project = Task::new("Has project").with_project(project_id);
        let task_no_project = Task::new("No project");

        model
            .tasks
            .insert(task_with_project.id.clone(), task_with_project.clone());
        model
            .tasks
            .insert(task_no_project.id.clone(), task_no_project);

        model.refresh_visible_tasks();

        // Only tasks with projects should be visible
        assert_eq!(model.visible_tasks.len(), 1);
        assert_eq!(model.visible_tasks[0], task_with_project.id);
    }

    #[test]
    fn test_view_overdue_filters_past_due() {
        let mut model = Model::new();
        model.current_view = ViewId::Overdue;

        let today = chrono::Utc::now().date_naive();
        let yesterday = today - chrono::Duration::days(1);
        let last_week = today - chrono::Duration::days(7);
        let tomorrow = today + chrono::Duration::days(1);

        let task_yesterday = Task::new("Due yesterday").with_due_date(yesterday);
        let task_last_week = Task::new("Due last week").with_due_date(last_week);
        let task_today = Task::new("Due today").with_due_date(today);
        let task_tomorrow = Task::new("Due tomorrow").with_due_date(tomorrow);
        let task_no_date = Task::new("No due date");

        model
            .tasks
            .insert(task_yesterday.id.clone(), task_yesterday.clone());
        model
            .tasks
            .insert(task_last_week.id.clone(), task_last_week.clone());
        model.tasks.insert(task_today.id.clone(), task_today);
        model.tasks.insert(task_tomorrow.id.clone(), task_tomorrow);
        model.tasks.insert(task_no_date.id.clone(), task_no_date);

        model.refresh_visible_tasks();

        // Only overdue tasks (past due dates) should be visible
        assert_eq!(model.visible_tasks.len(), 2);
        assert!(model.visible_tasks.contains(&task_yesterday.id));
        assert!(model.visible_tasks.contains(&task_last_week.id));
    }

    #[test]
    fn test_view_overdue_excludes_today() {
        let mut model = Model::new();
        model.current_view = ViewId::Overdue;

        let today = chrono::Utc::now().date_naive();
        let task_today = Task::new("Due today").with_due_date(today);

        model.tasks.insert(task_today.id.clone(), task_today);

        model.refresh_visible_tasks();

        // Today's tasks are not overdue
        assert!(model.visible_tasks.is_empty());
    }

    #[test]
    fn test_view_overdue_excludes_no_due_date() {
        let mut model = Model::new();
        model.current_view = ViewId::Overdue;

        let task_no_date = Task::new("No due date");
        model.tasks.insert(task_no_date.id.clone(), task_no_date);

        model.refresh_visible_tasks();

        // Tasks without due dates are not overdue
        assert!(model.visible_tasks.is_empty());
    }

    #[test]
    fn test_search_filter_matches_title() {
        let mut model = Model::new();

        let task_match = Task::new("Build the feature");
        let task_no_match = Task::new("Fix the bug");

        model
            .tasks
            .insert(task_match.id.clone(), task_match.clone());
        model.tasks.insert(task_no_match.id.clone(), task_no_match);

        model.filter.search_text = Some("build".to_string());
        model.refresh_visible_tasks();

        assert_eq!(model.visible_tasks.len(), 1);
        assert_eq!(model.visible_tasks[0], task_match.id);
    }

    #[test]
    fn test_search_filter_case_insensitive() {
        let mut model = Model::new();

        let task = Task::new("Build Feature");
        model.tasks.insert(task.id.clone(), task.clone());

        // Search with different cases
        model.filter.search_text = Some("BUILD".to_string());
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks.len(), 1);

        model.filter.search_text = Some("feature".to_string());
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks.len(), 1);
    }

    #[test]
    fn test_search_filter_matches_tags() {
        let mut model = Model::new();

        let task_with_tag = Task::new("Some task").with_tags(vec!["urgent".to_string()]);
        let task_no_tag = Task::new("Other task");

        model
            .tasks
            .insert(task_with_tag.id.clone(), task_with_tag.clone());
        model.tasks.insert(task_no_tag.id.clone(), task_no_tag);

        model.filter.search_text = Some("urgent".to_string());
        model.refresh_visible_tasks();

        assert_eq!(model.visible_tasks.len(), 1);
        assert_eq!(model.visible_tasks[0], task_with_tag.id);
    }

    #[test]
    fn test_search_filter_partial_match() {
        let mut model = Model::new();

        let task = Task::new("Implement authentication");
        model.tasks.insert(task.id.clone(), task.clone());

        model.filter.search_text = Some("auth".to_string());
        model.refresh_visible_tasks();

        assert_eq!(model.visible_tasks.len(), 1);
    }

    #[test]
    fn test_search_filter_empty_clears() {
        let mut model = Model::new();

        let task1 = Task::new("Task one");
        let task2 = Task::new("Task two");

        model.tasks.insert(task1.id.clone(), task1);
        model.tasks.insert(task2.id.clone(), task2);

        // With filter
        model.filter.search_text = Some("one".to_string());
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks.len(), 1);

        // Without filter
        model.filter.search_text = None;
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks.len(), 2);
    }

    #[test]
    fn test_tag_filter_any_mode() {
        use crate::domain::TagFilterMode;

        let mut model = Model::new();

        let task_rust = Task::new("Task Rust").with_tags(vec!["rust".to_string()]);
        let task_python = Task::new("Task Python").with_tags(vec!["python".to_string()]);
        let task_both =
            Task::new("Task Both").with_tags(vec!["rust".to_string(), "python".to_string()]);
        let task_none = Task::new("Task None");

        model.tasks.insert(task_rust.id.clone(), task_rust.clone());
        model
            .tasks
            .insert(task_python.id.clone(), task_python.clone());
        model.tasks.insert(task_both.id.clone(), task_both.clone());
        model.tasks.insert(task_none.id.clone(), task_none);

        // Filter by "rust" tag (Any mode - default)
        model.filter.tags = Some(vec!["rust".to_string()]);
        model.filter.tags_mode = TagFilterMode::Any;
        model.refresh_visible_tasks();

        assert_eq!(model.visible_tasks.len(), 2);
        assert!(model.visible_tasks.contains(&task_rust.id));
        assert!(model.visible_tasks.contains(&task_both.id));
    }

    #[test]
    fn test_tag_filter_all_mode() {
        use crate::domain::TagFilterMode;

        let mut model = Model::new();

        let task_rust = Task::new("Task Rust").with_tags(vec!["rust".to_string()]);
        let task_both =
            Task::new("Task Both").with_tags(vec!["rust".to_string(), "python".to_string()]);
        let task_none = Task::new("Task None");

        model.tasks.insert(task_rust.id.clone(), task_rust.clone());
        model.tasks.insert(task_both.id.clone(), task_both.clone());
        model.tasks.insert(task_none.id.clone(), task_none);

        // Filter by "rust" AND "python" tags (All mode)
        model.filter.tags = Some(vec!["rust".to_string(), "python".to_string()]);
        model.filter.tags_mode = TagFilterMode::All;
        model.refresh_visible_tasks();

        // Only task_both has both tags
        assert_eq!(model.visible_tasks.len(), 1);
        assert!(model.visible_tasks.contains(&task_both.id));
    }

    #[test]
    fn test_tag_filter_case_insensitive() {
        let mut model = Model::new();

        let task = Task::new("Task").with_tags(vec!["Rust".to_string()]);
        model.tasks.insert(task.id.clone(), task.clone());

        // Filter with different case
        model.filter.tags = Some(vec!["rust".to_string()]);
        model.refresh_visible_tasks();

        assert_eq!(model.visible_tasks.len(), 1);
        assert!(model.visible_tasks.contains(&task.id));
    }

    #[test]
    fn test_tag_filter_clear() {
        let mut model = Model::new();

        let task_tagged = Task::new("Tagged").with_tags(vec!["work".to_string()]);
        let task_untagged = Task::new("Untagged");

        model
            .tasks
            .insert(task_tagged.id.clone(), task_tagged.clone());
        model.tasks.insert(task_untagged.id.clone(), task_untagged);

        // With filter
        model.filter.tags = Some(vec!["work".to_string()]);
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks.len(), 1);

        // Clear filter
        model.filter.tags = None;
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks.len(), 2);
    }

    #[test]
    fn test_tag_filter_with_search() {
        let mut model = Model::new();

        let task_match =
            Task::new("Important Task").with_tags(vec!["work".to_string(), "urgent".to_string()]);
        let task_wrong_tag = Task::new("Important Other").with_tags(vec!["home".to_string()]);
        let task_wrong_title = Task::new("Regular Task").with_tags(vec!["work".to_string()]);

        model
            .tasks
            .insert(task_match.id.clone(), task_match.clone());
        model
            .tasks
            .insert(task_wrong_tag.id.clone(), task_wrong_tag);
        model
            .tasks
            .insert(task_wrong_title.id.clone(), task_wrong_title);

        // Both search and tag filter
        model.filter.search_text = Some("Important".to_string());
        model.filter.tags = Some(vec!["work".to_string()]);
        model.refresh_visible_tasks();

        // Only task_match matches both criteria
        assert_eq!(model.visible_tasks.len(), 1);
        assert!(model.visible_tasks.contains(&task_match.id));
    }

    #[test]
    fn test_sort_by_title() {
        use crate::domain::{SortField, SortOrder};

        let mut model = Model::new();

        let task_b = Task::new("Banana");
        let task_a = Task::new("Apple");
        let task_c = Task::new("Cherry");

        model.tasks.insert(task_b.id.clone(), task_b.clone());
        model.tasks.insert(task_a.id.clone(), task_a.clone());
        model.tasks.insert(task_c.id.clone(), task_c.clone());

        model.sort = SortSpec {
            field: SortField::Title,
            order: SortOrder::Ascending,
        };
        model.refresh_visible_tasks();

        assert_eq!(model.visible_tasks[0], task_a.id);
        assert_eq!(model.visible_tasks[1], task_b.id);
        assert_eq!(model.visible_tasks[2], task_c.id);
    }

    #[test]
    fn test_sort_by_title_descending() {
        use crate::domain::{SortField, SortOrder};

        let mut model = Model::new();

        let task_b = Task::new("Banana");
        let task_a = Task::new("Apple");
        let task_c = Task::new("Cherry");

        model.tasks.insert(task_b.id.clone(), task_b.clone());
        model.tasks.insert(task_a.id.clone(), task_a.clone());
        model.tasks.insert(task_c.id.clone(), task_c.clone());

        model.sort = SortSpec {
            field: SortField::Title,
            order: SortOrder::Descending,
        };
        model.refresh_visible_tasks();

        assert_eq!(model.visible_tasks[0], task_c.id);
        assert_eq!(model.visible_tasks[1], task_b.id);
        assert_eq!(model.visible_tasks[2], task_a.id);
    }

    #[test]
    fn test_sort_by_due_date() {
        use crate::domain::{SortField, SortOrder};

        let mut model = Model::new();

        let today = chrono::Utc::now().date_naive();
        let tomorrow = today + chrono::Duration::days(1);
        let next_week = today + chrono::Duration::days(7);

        let task_soon = Task::new("Soon").with_due_date(tomorrow);
        let task_later = Task::new("Later").with_due_date(next_week);
        let task_no_date = Task::new("No date");

        model
            .tasks
            .insert(task_later.id.clone(), task_later.clone());
        model.tasks.insert(task_soon.id.clone(), task_soon.clone());
        model
            .tasks
            .insert(task_no_date.id.clone(), task_no_date.clone());

        model.sort = SortSpec {
            field: SortField::DueDate,
            order: SortOrder::Ascending,
        };
        model.refresh_visible_tasks();

        // Tasks with dates come first, then tasks without dates
        assert_eq!(model.visible_tasks[0], task_soon.id);
        assert_eq!(model.visible_tasks[1], task_later.id);
        assert_eq!(model.visible_tasks[2], task_no_date.id);
    }

    #[test]
    fn test_sort_by_status() {
        use crate::domain::{SortField, SortOrder};

        let mut model = Model::new();
        model.show_completed = true; // Show completed for this test

        let task_todo = Task::new("Todo").with_status(TaskStatus::Todo);
        let task_in_progress = Task::new("In Progress").with_status(TaskStatus::InProgress);
        let task_done = Task::new("Done").with_status(TaskStatus::Done);

        model.tasks.insert(task_done.id.clone(), task_done.clone());
        model.tasks.insert(task_todo.id.clone(), task_todo.clone());
        model
            .tasks
            .insert(task_in_progress.id.clone(), task_in_progress.clone());

        model.sort = SortSpec {
            field: SortField::Status,
            order: SortOrder::Ascending,
        };
        model.refresh_visible_tasks();

        // Order: InProgress, Todo, Blocked, Done, Cancelled
        assert_eq!(model.visible_tasks[0], task_in_progress.id);
        assert_eq!(model.visible_tasks[1], task_todo.id);
        assert_eq!(model.visible_tasks[2], task_done.id);
    }

    #[test]
    fn test_sort_order_toggle() {
        use crate::domain::{SortField, SortOrder};

        let mut model = Model::new();

        let task_high = Task::new("High").with_priority(Priority::High);
        let task_low = Task::new("Low").with_priority(Priority::Low);

        model.tasks.insert(task_high.id.clone(), task_high.clone());
        model.tasks.insert(task_low.id.clone(), task_low.clone());

        // Ascending: High first (lower priority number)
        model.sort = SortSpec {
            field: SortField::Priority,
            order: SortOrder::Ascending,
        };
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks[0], task_high.id);
        assert_eq!(model.visible_tasks[1], task_low.id);

        // Descending: Low first
        model.sort.order = SortOrder::Descending;
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks[0], task_low.id);
        assert_eq!(model.visible_tasks[1], task_high.id);
    }

    #[test]
    fn test_get_tasks_grouped_by_project_basic() {
        use crate::domain::Project;

        let mut model = Model::new();
        model.current_view = ViewId::Projects;

        // Create two projects
        let project_a = Project::new("Alpha Project");
        let project_b = Project::new("Beta Project");
        let project_a_id = project_a.id.clone();
        let project_b_id = project_b.id.clone();

        model.projects.insert(project_a_id.clone(), project_a);
        model.projects.insert(project_b_id.clone(), project_b);

        // Create tasks for each project
        let task_a1 = Task::new("Alpha Task 1").with_project(project_a_id.clone());
        let task_a2 = Task::new("Alpha Task 2").with_project(project_a_id.clone());
        let task_b1 = Task::new("Beta Task 1").with_project(project_b_id.clone());

        model.tasks.insert(task_a1.id.clone(), task_a1);
        model.tasks.insert(task_a2.id.clone(), task_a2);
        model.tasks.insert(task_b1.id.clone(), task_b1);

        model.refresh_visible_tasks();

        let grouped = model.get_tasks_grouped_by_project();

        // Should have 2 groups (Alpha and Beta, sorted alphabetically)
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].1, "Alpha Project");
        assert_eq!(grouped[0].2.len(), 2); // 2 tasks in Alpha
        assert_eq!(grouped[1].1, "Beta Project");
        assert_eq!(grouped[1].2.len(), 1); // 1 task in Beta
    }

    #[test]
    fn test_get_tasks_grouped_by_project_alphabetical_order() {
        use crate::domain::Project;

        let mut model = Model::new();
        model.current_view = ViewId::Projects;

        // Create projects out of alphabetical order
        let project_z = Project::new("Zebra");
        let project_a = Project::new("Apple");
        let project_m = Project::new("Mango");

        let z_id = project_z.id.clone();
        let a_id = project_a.id.clone();
        let m_id = project_m.id.clone();

        model.projects.insert(z_id.clone(), project_z);
        model.projects.insert(a_id.clone(), project_a);
        model.projects.insert(m_id.clone(), project_m);

        // Create one task per project
        let task_z = Task::new("Z task").with_project(z_id);
        let task_a = Task::new("A task").with_project(a_id);
        let task_m = Task::new("M task").with_project(m_id);

        model.tasks.insert(task_z.id.clone(), task_z);
        model.tasks.insert(task_a.id.clone(), task_a);
        model.tasks.insert(task_m.id.clone(), task_m);

        model.refresh_visible_tasks();

        let grouped = model.get_tasks_grouped_by_project();

        // Should be sorted alphabetically: Apple, Mango, Zebra
        assert_eq!(grouped.len(), 3);
        assert_eq!(grouped[0].1, "Apple");
        assert_eq!(grouped[1].1, "Mango");
        assert_eq!(grouped[2].1, "Zebra");
    }

    #[test]
    fn test_get_tasks_grouped_no_project_goes_last() {
        use crate::domain::Project;

        let mut model = Model::new();
        model.current_view = ViewId::Projects;

        // Create one project
        let project = Project::new("My Project");
        let project_id = project.id.clone();
        model.projects.insert(project_id.clone(), project);

        // Task with project
        let task_with = Task::new("With project").with_project(project_id);
        // Task without project (shouldn't appear in Projects view normally,
        // but test the grouping logic)
        let task_without = Task::new("Without project");

        model.tasks.insert(task_with.id.clone(), task_with);
        model.tasks.insert(task_without.id.clone(), task_without);

        // For this test, we need to make both visible
        // Override the view filtering by using TaskList view
        model.current_view = ViewId::TaskList;
        model.refresh_visible_tasks();

        // Now get grouped (the function doesn't filter, just groups visible tasks)
        let grouped = model.get_tasks_grouped_by_project();

        // Should have 2 groups: My Project first, No Project last
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].1, "My Project");
        assert_eq!(grouped[1].1, "No Project");
    }

    #[test]
    fn test_get_tasks_grouped_empty() {
        let mut model = Model::new();
        model.current_view = ViewId::Projects;
        model.refresh_visible_tasks();

        let grouped = model.get_tasks_grouped_by_project();

        // No tasks, no groups
        assert!(grouped.is_empty());
    }

    #[test]
    fn test_get_tasks_grouped_preserves_task_order_within_group() {
        use crate::domain::{Project, SortField, SortOrder};

        let mut model = Model::new();
        model.current_view = ViewId::Projects;

        // Sort by title ascending
        model.sort.field = SortField::Title;
        model.sort.order = SortOrder::Ascending;

        let project = Project::new("Test Project");
        let project_id = project.id.clone();
        model.projects.insert(project_id.clone(), project);

        // Create tasks with different titles (will be sorted alphabetically)
        let task_c = Task::new("Charlie").with_project(project_id.clone());
        let task_a = Task::new("Alpha").with_project(project_id.clone());
        let task_b = Task::new("Bravo").with_project(project_id.clone());

        let task_a_id = task_a.id.clone();
        let task_b_id = task_b.id.clone();
        let task_c_id = task_c.id.clone();

        model.tasks.insert(task_c.id.clone(), task_c);
        model.tasks.insert(task_a.id.clone(), task_a);
        model.tasks.insert(task_b.id.clone(), task_b);

        model.refresh_visible_tasks();

        let grouped = model.get_tasks_grouped_by_project();

        assert_eq!(grouped.len(), 1);
        let task_ids = &grouped[0].2;
        assert_eq!(task_ids.len(), 3);

        // Tasks should be in order based on visible_tasks order (sorted by title)
        // Alpha, Bravo, Charlie
        assert_eq!(task_ids[0], task_a_id);
        assert_eq!(task_ids[1], task_b_id);
        assert_eq!(task_ids[2], task_c_id);
    }

    #[test]
    fn test_view_blocked_shows_tasks_with_unmet_dependencies() {
        let mut model = Model::new();
        model.current_view = ViewId::Blocked;

        // Task A is a prerequisite (incomplete)
        let task_a = Task::new("Prerequisite task");
        let task_a_id = task_a.id.clone();

        // Task B depends on task A (blocked because A is not done)
        let mut task_b = Task::new("Blocked task");
        task_b.dependencies.push(task_a_id.clone());

        // Task C has no dependencies
        let task_c = Task::new("Independent task");

        let task_b_id = task_b.id.clone();
        model.tasks.insert(task_a.id.clone(), task_a);
        model.tasks.insert(task_b.id.clone(), task_b);
        model.tasks.insert(task_c.id.clone(), task_c);

        model.refresh_visible_tasks();

        // Only task B should be visible (blocked because task A is not done)
        assert_eq!(model.visible_tasks.len(), 1);
        assert_eq!(model.visible_tasks[0], task_b_id);
    }

    #[test]
    fn test_view_blocked_excludes_tasks_with_completed_dependencies() {
        let mut model = Model::new();
        model.current_view = ViewId::Blocked;

        // Task A is a completed prerequisite
        let task_a = Task::new("Done prerequisite").with_status(TaskStatus::Done);
        let task_a_id = task_a.id.clone();

        // Task B depends on task A (NOT blocked because A is done)
        let mut task_b = Task::new("Unblocked task");
        task_b.dependencies.push(task_a_id.clone());

        model.tasks.insert(task_a.id.clone(), task_a);
        model.tasks.insert(task_b.id.clone(), task_b);

        model.show_completed = true; // Include completed tasks
        model.refresh_visible_tasks();

        // Task B should NOT be visible in Blocked view since its dependency is complete
        assert!(!model.visible_tasks.iter().any(|id| {
            model
                .tasks
                .get(id)
                .map_or(false, |t| t.title == "Unblocked task")
        }));
    }

    #[test]
    fn test_view_untagged_shows_tasks_without_tags() {
        let mut model = Model::new();
        model.current_view = ViewId::Untagged;

        let task_with_tags = Task::new("Has tags").with_tags(vec!["work".to_string()]);
        let task_no_tags = Task::new("No tags");

        model
            .tasks
            .insert(task_with_tags.id.clone(), task_with_tags);
        model
            .tasks
            .insert(task_no_tags.id.clone(), task_no_tags.clone());

        model.refresh_visible_tasks();

        // Only the task without tags should be visible
        assert_eq!(model.visible_tasks.len(), 1);
        assert_eq!(model.visible_tasks[0], task_no_tags.id);
    }

    #[test]
    fn test_view_no_project_shows_tasks_without_project() {
        let mut model = Model::new();
        model.current_view = ViewId::NoProject;

        let project_id = crate::domain::ProjectId::new();
        let task_with_project = Task::new("Has project").with_project(project_id);
        let task_no_project = Task::new("No project");

        model
            .tasks
            .insert(task_with_project.id.clone(), task_with_project);
        model
            .tasks
            .insert(task_no_project.id.clone(), task_no_project.clone());

        model.refresh_visible_tasks();

        // Only the task without a project should be visible
        assert_eq!(model.visible_tasks.len(), 1);
        assert_eq!(model.visible_tasks[0], task_no_project.id);
    }

    #[test]
    fn test_view_recently_modified_filters_by_date() {
        let mut model = Model::new();
        model.current_view = ViewId::RecentlyModified;

        // Create a task modified now (recent)
        let recent_task = Task::new("Recent task");

        // Create a task and modify its updated_at to be old
        let mut old_task = Task::new("Old task");
        old_task.updated_at = chrono::Utc::now() - chrono::Duration::days(14);

        model
            .tasks
            .insert(recent_task.id.clone(), recent_task.clone());
        model.tasks.insert(old_task.id.clone(), old_task);

        model.refresh_visible_tasks();

        // Only the recent task should be visible (modified within 7 days)
        assert_eq!(model.visible_tasks.len(), 1);
        assert_eq!(model.visible_tasks[0], recent_task.id);
    }

    // ==================== Hierarchy Helper Method Tests ====================

    #[test]
    fn test_task_depth_root_task() {
        let mut model = Model::new();
        let task = Task::new("Root");
        model.tasks.insert(task.id.clone(), task.clone());
        assert_eq!(model.task_depth(&task.id), 0);
    }

    #[test]
    fn test_task_depth_nested() {
        let mut model = Model::new();
        let root = Task::new("Root");
        let child = Task::new("Child").with_parent(root.id.clone());
        let grandchild = Task::new("Grandchild").with_parent(child.id.clone());

        model.tasks.insert(root.id.clone(), root.clone());
        model.tasks.insert(child.id.clone(), child.clone());
        model
            .tasks
            .insert(grandchild.id.clone(), grandchild.clone());

        assert_eq!(model.task_depth(&root.id), 0);
        assert_eq!(model.task_depth(&child.id), 1);
        assert_eq!(model.task_depth(&grandchild.id), 2);
    }

    #[test]
    fn test_task_depth_missing_parent() {
        let mut model = Model::new();
        // Create a task with a parent_task_id that doesn't exist
        let orphan_parent_id = TaskId::new();
        let orphan = Task::new("Orphan").with_parent(orphan_parent_id);
        model.tasks.insert(orphan.id.clone(), orphan.clone());

        // Returns 1 because the function counts parent hops: orphan → missing parent (1 hop).
        // Note that orphaned tasks will display indented even though their parent doesn't exist.
        assert_eq!(model.task_depth(&orphan.id), 1);
    }

    #[test]
    fn test_get_all_descendants_empty() {
        let mut model = Model::new();
        let task = Task::new("Standalone");
        model.tasks.insert(task.id.clone(), task.clone());

        let descendants = model.get_all_descendants(&task.id);
        assert!(descendants.is_empty());
    }

    #[test]
    fn test_get_all_descendants_nested() {
        let mut model = Model::new();
        let root = Task::new("Root");
        let child1 = Task::new("Child1").with_parent(root.id.clone());
        let child2 = Task::new("Child2").with_parent(root.id.clone());
        let grandchild = Task::new("Grandchild").with_parent(child1.id.clone());

        model.tasks.insert(root.id.clone(), root.clone());
        model.tasks.insert(child1.id.clone(), child1.clone());
        model.tasks.insert(child2.id.clone(), child2.clone());
        model
            .tasks
            .insert(grandchild.id.clone(), grandchild.clone());

        let descendants = model.get_all_descendants(&root.id);
        assert_eq!(descendants.len(), 3);
        assert!(descendants.contains(&child1.id));
        assert!(descendants.contains(&child2.id));
        assert!(descendants.contains(&grandchild.id));
    }

    #[test]
    fn test_get_all_ancestors_empty() {
        let mut model = Model::new();
        let task = Task::new("Root");
        model.tasks.insert(task.id.clone(), task.clone());

        let ancestors = model.get_all_ancestors(&task.id);
        assert!(ancestors.is_empty());
    }

    #[test]
    fn test_get_all_ancestors_nested() {
        let mut model = Model::new();
        let root = Task::new("Root");
        let child = Task::new("Child").with_parent(root.id.clone());
        let grandchild = Task::new("Grandchild").with_parent(child.id.clone());

        model.tasks.insert(root.id.clone(), root.clone());
        model.tasks.insert(child.id.clone(), child.clone());
        model
            .tasks
            .insert(grandchild.id.clone(), grandchild.clone());

        let ancestors = model.get_all_ancestors(&grandchild.id);
        assert_eq!(ancestors.len(), 2);
        assert_eq!(ancestors[0], child.id); // Direct parent first
        assert_eq!(ancestors[1], root.id); // Then grandparent
    }

    #[test]
    fn test_would_create_cycle_self_reference() {
        let mut model = Model::new();
        let task = Task::new("Task");
        model.tasks.insert(task.id.clone(), task.clone());

        assert!(model.would_create_cycle(&task.id, &task.id));
    }

    #[test]
    fn test_would_create_cycle_descendant() {
        let mut model = Model::new();
        let root = Task::new("Root");
        let child = Task::new("Child").with_parent(root.id.clone());
        let grandchild = Task::new("Grandchild").with_parent(child.id.clone());

        model.tasks.insert(root.id.clone(), root.clone());
        model.tasks.insert(child.id.clone(), child.clone());
        model
            .tasks
            .insert(grandchild.id.clone(), grandchild.clone());

        // Setting root's parent to grandchild would create a cycle
        assert!(model.would_create_cycle(&root.id, &grandchild.id));
        assert!(model.would_create_cycle(&root.id, &child.id));

        // Setting grandchild's parent to a new task is fine
        let new_task = Task::new("New");
        model.tasks.insert(new_task.id.clone(), new_task.clone());
        assert!(!model.would_create_cycle(&grandchild.id, &new_task.id));
    }

    #[test]
    fn test_has_subtasks() {
        let mut model = Model::new();
        let parent = Task::new("Parent");
        let child = Task::new("Child").with_parent(parent.id.clone());
        let standalone = Task::new("Standalone");

        model.tasks.insert(parent.id.clone(), parent.clone());
        model.tasks.insert(child.id.clone(), child);
        model
            .tasks
            .insert(standalone.id.clone(), standalone.clone());

        assert!(model.has_subtasks(&parent.id));
        assert!(!model.has_subtasks(&standalone.id));
    }

    #[test]
    fn test_subtask_progress_recursive() {
        let mut model = Model::new();
        let root = Task::new("Root");
        let child1 = Task::new("Child1")
            .with_parent(root.id.clone())
            .with_status(TaskStatus::Done);
        let child2 = Task::new("Child2").with_parent(root.id.clone());
        let grandchild = Task::new("Grandchild")
            .with_parent(child2.id.clone())
            .with_status(TaskStatus::Done);

        model.tasks.insert(root.id.clone(), root.clone());
        model.tasks.insert(child1.id.clone(), child1);
        model.tasks.insert(child2.id.clone(), child2);
        model.tasks.insert(grandchild.id.clone(), grandchild);

        let (completed, total) = model.subtask_progress(&root.id);
        assert_eq!(total, 3); // child1, child2, grandchild
        assert_eq!(completed, 2); // child1, grandchild
    }

    #[test]
    fn test_subtask_percentage() {
        let mut model = Model::new();
        let root = Task::new("Root");
        let child1 = Task::new("Child1")
            .with_parent(root.id.clone())
            .with_status(TaskStatus::Done);
        let child2 = Task::new("Child2").with_parent(root.id.clone());

        model.tasks.insert(root.id.clone(), root.clone());
        model.tasks.insert(child1.id.clone(), child1);
        model.tasks.insert(child2.id.clone(), child2);

        // 1 of 2 completed = 50%
        assert_eq!(model.subtask_percentage(&root.id), Some(50));
    }

    #[test]
    fn test_subtask_percentage_no_subtasks() {
        let mut model = Model::new();
        let task = Task::new("Standalone");
        model.tasks.insert(task.id.clone(), task.clone());

        assert_eq!(model.subtask_percentage(&task.id), None);
    }

    #[test]
    fn test_refresh_visible_tasks_deep_nesting_order() {
        // Test that visible_tasks orders: Root -> Child -> Grandchild -> Root2
        let mut model = Model::new();

        let root1 = Task::new("Root1");
        let child1 = Task::new("Child1").with_parent(root1.id.clone());
        let grandchild = Task::new("Grandchild").with_parent(child1.id.clone());
        let root2 = Task::new("Root2");

        let root1_id = root1.id.clone();
        let child1_id = child1.id.clone();
        let grandchild_id = grandchild.id.clone();
        let root2_id = root2.id.clone();

        // Insert in random order
        model.tasks.insert(grandchild.id.clone(), grandchild);
        model.tasks.insert(root2.id.clone(), root2);
        model.tasks.insert(child1.id.clone(), child1);
        model.tasks.insert(root1.id.clone(), root1);

        model.refresh_visible_tasks();

        // Check ordering: should be Root1 -> Child1 -> Grandchild -> Root2
        // (roots sorted by created_at, subtasks inserted after their parents)
        let root1_pos = model
            .visible_tasks
            .iter()
            .position(|id| id == &root1_id)
            .unwrap();
        let child1_pos = model
            .visible_tasks
            .iter()
            .position(|id| id == &child1_id)
            .unwrap();
        let grandchild_pos = model
            .visible_tasks
            .iter()
            .position(|id| id == &grandchild_id)
            .unwrap();
        let root2_pos = model
            .visible_tasks
            .iter()
            .position(|id| id == &root2_id)
            .unwrap();

        // Child1 should come after Root1
        assert!(child1_pos > root1_pos, "Child1 should appear after Root1");

        // Grandchild should come after Child1
        assert!(
            grandchild_pos > child1_pos,
            "Grandchild should appear after Child1"
        );

        // Grandchild should come before Root2 (if Root2 comes after Root1)
        // This ensures the hierarchy is kept together
        if root2_pos > root1_pos {
            assert!(
                grandchild_pos < root2_pos,
                "Grandchild should appear before Root2"
            );
        }
    }
}
