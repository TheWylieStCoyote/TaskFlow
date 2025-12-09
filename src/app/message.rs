//! Message types for the application.
//!
//! Messages represent events that can modify application state.
//! They are processed by the [`super::update()`] function.
//!
//! ## Message Hierarchy
//!
//! ```text
//! Message
//! ├── Navigation  - Movement and view switching
//! ├── Task        - Task CRUD operations
//! ├── Time        - Time tracking
//! ├── Ui          - UI state changes
//! ├── System      - App-level actions
//! └── None        - No-op
//! ```

use crate::domain::{HabitId, Priority, ProjectId, TaskId, TaskStatus};
use chrono::NaiveDate;

/// Which pane currently has focus.
///
/// The application has two main panes:
/// - [`FocusPane::TaskList`] - The main task list area
/// - [`FocusPane::Sidebar`] - The sidebar with views and projects
///
/// Keyboard navigation behavior changes based on which pane has focus.
///
/// # Examples
///
/// ```
/// use taskflow::app::FocusPane;
///
/// let focus = FocusPane::default();
/// assert_eq!(focus, FocusPane::TaskList);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusPane {
    /// Main task list (default focus)
    #[default]
    TaskList,
    /// Left sidebar with views and projects
    Sidebar,
}

/// Top-level message enum for the application.
///
/// All user actions and system events are represented as messages.
/// Messages are processed by [`super::update()`] which modifies the
/// application state accordingly.
///
/// # Examples
///
/// ```
/// use taskflow::app::{Message, NavigationMessage, TaskMessage};
///
/// // Navigation messages
/// let msg = Message::Navigation(NavigationMessage::Down);
///
/// // Task messages
/// let msg = Message::Task(TaskMessage::Create("New task".to_string()));
///
/// // Messages can be created using From trait
/// let msg: Message = NavigationMessage::Up.into();
/// ```
#[derive(Debug, Clone)]
pub enum Message {
    /// Navigation and movement messages
    Navigation(NavigationMessage),
    /// Task-related operations
    Task(TaskMessage),
    /// Time tracking operations
    Time(TimeMessage),
    /// Pomodoro timer operations
    Pomodoro(PomodoroMessage),
    /// Habit tracking operations
    Habit(HabitMessage),
    /// UI state changes
    Ui(UiMessage),
    /// System-level operations
    System(SystemMessage),
    /// No operation (useful for conditional message handling)
    None,
}

/// Navigation messages for movement and view switching.
///
/// These messages handle cursor movement within lists, switching
/// between views, and calendar navigation.
///
/// # Examples
///
/// ```
/// use taskflow::app::{Model, Message, NavigationMessage, ViewId, update};
///
/// let mut model = Model::new().with_sample_data();
///
/// // Move selection down
/// update(&mut model, NavigationMessage::Down.into());
///
/// // Switch to a different view
/// update(&mut model, NavigationMessage::GoToView(ViewId::Today).into());
/// ```
#[derive(Debug, Clone)]
pub enum NavigationMessage {
    /// Move selection up in the current list
    Up,
    /// Move selection down in the current list
    Down,
    /// Jump to the first item
    First,
    /// Jump to the last item
    Last,
    /// Move up by a page (10 items)
    PageUp,
    /// Move down by a page (10 items)
    PageDown,
    /// Select a specific item by index
    Select(usize),
    /// Switch to a different view
    GoToView(ViewId),
    /// Move focus to the sidebar
    FocusSidebar,
    /// Move focus to the task list
    FocusTaskList,
    /// Activate the selected sidebar item
    SelectSidebarItem,
    /// Navigate to previous month in calendar
    CalendarPrevMonth,
    /// Navigate to next month in calendar
    CalendarNextMonth,
    /// Select a specific day in calendar
    CalendarSelectDay(u32),
    /// Focus the task list panel in calendar view
    CalendarFocusTaskList,
    /// Focus the calendar grid in calendar view
    CalendarFocusGrid,
    /// Navigate to next panel in reports view
    ReportsNextPanel,
    /// Navigate to previous panel in reports view
    ReportsPrevPanel,
    /// Scroll timeline viewport left (earlier dates)
    TimelineScrollLeft,
    /// Scroll timeline viewport right (later dates)
    TimelineScrollRight,
    /// Zoom in on timeline (Week → Day)
    TimelineZoomIn,
    /// Zoom out on timeline (Day → Week)
    TimelineZoomOut,
    /// Jump to today in timeline
    TimelineGoToday,
    /// Navigate up in timeline task list
    TimelineUp,
    /// Navigate down in timeline task list
    TimelineDown,
    /// Navigate left in Kanban view (previous column)
    KanbanLeft,
    /// Navigate right in Kanban view (next column)
    KanbanRight,
    /// Navigate up in Kanban view (previous task in column)
    KanbanUp,
    /// Navigate down in Kanban view (next task in column)
    KanbanDown,
    /// Navigate up in Eisenhower view (to upper quadrant)
    EisenhowerUp,
    /// Navigate down in Eisenhower view (to lower quadrant)
    EisenhowerDown,
    /// Navigate left in Eisenhower view (to left quadrant)
    EisenhowerLeft,
    /// Navigate right in Eisenhower view (to right quadrant)
    EisenhowerRight,
    /// Navigate left in WeeklyPlanner view (previous day)
    WeeklyPlannerLeft,
    /// Navigate right in WeeklyPlanner view (next day)
    WeeklyPlannerRight,
    /// Navigate up in WeeklyPlanner view (previous task in day)
    WeeklyPlannerUp,
    /// Navigate down in WeeklyPlanner view (next task in day)
    WeeklyPlannerDown,
    /// Select a specific sidebar item by index (for mouse click)
    SidebarSelectIndex(usize),
    /// Select a specific Kanban column by index (for mouse click)
    KanbanSelectColumn(usize),
    /// Select a specific Eisenhower quadrant by index (for mouse click)
    EisenhowerSelectQuadrant(usize),
    /// Select a specific WeeklyPlanner day by index (for mouse click)
    WeeklyPlannerSelectDay(usize),
    /// Select a specific Reports panel by index (for mouse click)
    ReportsSelectPanel(usize),
    /// Navigate up in Network view (previous task)
    NetworkUp,
    /// Navigate down in Network view (next task)
    NetworkDown,
}

/// View identifiers for different application screens.
///
/// Each view shows tasks filtered and presented differently.
///
/// # Examples
///
/// ```
/// use taskflow::app::ViewId;
///
/// let view = ViewId::default();
/// assert_eq!(view, ViewId::TaskList);
///
/// // Compare views
/// let today = ViewId::Today;
/// let upcoming = ViewId::Upcoming;
/// assert_ne!(today, upcoming);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ViewId {
    /// All tasks view (default)
    #[default]
    TaskList,
    /// Tasks due today
    Today,
    /// Tasks with future due dates
    Upcoming,
    /// Tasks past their due date
    Overdue,
    /// Tasks with scheduled dates, sorted by scheduled date
    Scheduled,
    /// Monthly calendar view
    Calendar,
    /// Statistics and overview dashboard
    Dashboard,
    /// Tasks grouped by project
    Projects,
    /// Tasks with incomplete dependencies (blocked)
    Blocked,
    /// Tasks without any tags
    Untagged,
    /// Tasks not assigned to any project
    NoProject,
    /// Tasks modified in the last 7 days
    RecentlyModified,
    /// Analytics and reports view
    Reports,
    /// Kanban board view with columns for each status
    Kanban,
    /// Eisenhower matrix (urgent/important quadrants)
    Eisenhower,
    /// Weekly planner view with day columns
    WeeklyPlanner,
    /// Timeline/Gantt view showing tasks as bars on time axis
    Timeline,
    /// Snoozed tasks (hidden until snooze date)
    Snoozed,
    /// Habit tracking view
    Habits,
    /// Heatmap view (GitHub-style contribution graph)
    Heatmap,
    /// Forecast view (workload projection into future)
    Forecast,
    /// Network graph view (dependency visualization)
    Network,
    /// Burndown chart view (progress toward completion)
    Burndown,
}

/// Task operation messages.
///
/// These messages handle creating, modifying, and deleting tasks.
///
/// # Examples
///
/// ```
/// use taskflow::app::{Model, Message, TaskMessage, update};
/// use taskflow::domain::{Priority, TaskStatus, TaskId};
///
/// let mut model = Model::new();
///
/// // Create a new task
/// update(&mut model, TaskMessage::Create("Buy groceries".to_string()).into());
///
/// // Toggle completion of selected task
/// update(&mut model, TaskMessage::ToggleComplete.into());
///
/// // Cycle through priorities
/// update(&mut model, TaskMessage::CyclePriority.into());
/// ```
#[derive(Debug, Clone)]
pub enum TaskMessage {
    /// Toggle completion status of selected task
    ToggleComplete,
    /// Set specific status for a task
    SetStatus(TaskId, TaskStatus),
    /// Set specific priority for a task
    SetPriority(TaskId, Priority),
    /// Cycle through priority levels (None → Low → Medium → High → Urgent)
    CyclePriority,
    /// Create a new task with given title
    Create(String),
    /// Delete a task by ID
    Delete(TaskId),
    /// Move task to a project (or remove from project with None)
    MoveToProject(TaskId, Option<ProjectId>),
    /// Duplicate the selected task with "Copy of" prefix
    Duplicate,
}

/// Time tracking messages.
///
/// Control time tracking for the currently selected task.
///
/// # Examples
///
/// ```
/// use taskflow::app::{Model, Message, TimeMessage, TaskMessage, update};
///
/// let mut model = Model::new();
/// update(&mut model, TaskMessage::Create("Work on project".to_string()).into());
///
/// // Start tracking time
/// update(&mut model, TimeMessage::StartTracking.into());
///
/// // Stop tracking
/// update(&mut model, TimeMessage::StopTracking.into());
/// ```
#[derive(Debug, Clone)]
pub enum TimeMessage {
    /// Start time tracking for selected task
    StartTracking,
    /// Stop the current time tracking session
    StopTracking,
    /// Toggle time tracking (start if stopped, stop if running)
    ToggleTracking,
}

/// UI state and interaction messages.
///
/// These messages handle UI state changes like input mode, dialogs,
/// multi-select operations, and display toggles.
///
/// # Categories
///
/// - **Display toggles**: Show/hide completed tasks, sidebar, help
/// - **Input mode**: Start editing, handle keystrokes, submit/cancel
/// - **Delete confirmation**: Show confirmation dialog, confirm/cancel
/// - **Multi-select**: Bulk operations on multiple tasks
/// - **Macros**: Record and playback keyboard macros
/// - **Templates**: Task templates for quick creation
#[derive(Debug, Clone)]
pub enum UiMessage {
    // Display toggles
    /// Toggle visibility of completed tasks
    ToggleShowCompleted,
    /// Toggle sidebar visibility
    ToggleSidebar,
    /// Show help overlay
    ShowHelp,
    /// Hide help overlay
    HideHelp,
    /// Toggle focus mode (single-task view with timer)
    ToggleFocusMode,

    // Input mode - starting various edit operations
    /// Enter input mode to create a new task
    StartCreateTask,
    /// Enter quick capture mode with syntax hints
    StartQuickCapture,
    /// Enter input mode to create a subtask
    StartCreateSubtask,
    /// Enter input mode to create a new project
    StartCreateProject,
    /// Enter input mode to edit/rename the selected project
    StartEditProject,
    /// Delete the selected project (tasks are unassigned, not deleted)
    DeleteProject,
    /// Enter input mode to edit task title
    StartEditTask,
    /// Enter input mode to edit due date
    StartEditDueDate,
    /// Enter input mode to edit scheduled date
    StartEditScheduledDate,
    /// Enter input mode to edit tags
    StartEditTags,
    /// Enter input mode to edit description
    StartEditDescription,
    /// Enter input mode to edit time estimate
    StartEditEstimate,
    /// Enter input mode to move task to project
    StartMoveToProject,
    /// Enter input mode to search tasks
    StartSearch,
    /// Clear the current search filter
    ClearSearch,
    /// Enter input mode to filter by tag
    StartFilterByTag,
    /// Clear the current tag filter
    ClearTagFilter,
    /// Cycle through sort fields
    CycleSortField,
    /// Toggle between ascending/descending sort
    ToggleSortOrder,
    /// Cancel current input operation
    CancelInput,
    /// Submit current input
    SubmitInput,
    /// Insert a character at cursor
    InputChar(char),
    /// Delete character before cursor
    InputBackspace,
    /// Delete character at cursor
    InputDelete,
    /// Move cursor left
    InputCursorLeft,
    /// Move cursor right
    InputCursorRight,
    /// Move cursor to start of input
    InputCursorStart,
    /// Move cursor to end of input
    InputCursorEnd,

    // Delete confirmation
    /// Show delete confirmation dialog
    ShowDeleteConfirm,
    /// Confirm deletion
    ConfirmDelete,
    /// Cancel deletion
    CancelDelete,

    // Multi-select / Bulk operations
    /// Toggle multi-select mode
    ToggleMultiSelect,
    /// Toggle selection of current task
    ToggleTaskSelection,
    /// Select all visible tasks
    SelectAll,
    /// Clear all selections
    ClearSelection,
    /// Delete all selected tasks
    BulkDelete,
    /// Move all selected tasks to a project
    StartBulkMoveToProject,
    /// Set status for all selected tasks
    StartBulkSetStatus,

    // Dependencies
    /// Enter input mode to edit task dependencies
    StartEditDependencies,

    // Task chains
    /// Enter input mode to link current task to next task in chain
    StartLinkTask,
    /// Remove the link to next task in chain
    UnlinkTask,

    // Recurrence
    /// Enter input mode to edit task recurrence
    StartEditRecurrence,

    // Manual ordering
    /// Move selected task up in list order
    MoveTaskUp,
    /// Move selected task down in list order
    MoveTaskDown,

    // Calendar navigation
    /// Move to previous day in calendar
    CalendarPrevDay,
    /// Move to next day in calendar
    CalendarNextDay,

    // Keyboard macros
    /// Start recording a keyboard macro
    StartRecordMacro,
    /// Stop recording the current macro
    StopRecordMacro,
    /// Play back a recorded macro by slot number
    PlayMacro(usize),

    // Task templates
    /// Show template picker
    ShowTemplates,
    /// Hide template picker
    HideTemplates,
    /// Select and apply a template
    SelectTemplate(usize),

    // Keybindings editor
    /// Show the keybindings editor
    ShowKeybindingsEditor,
    /// Hide the keybindings editor
    HideKeybindingsEditor,
    /// Navigate up in keybindings list
    KeybindingsUp,
    /// Navigate down in keybindings list
    KeybindingsDown,
    /// Start editing the selected keybinding
    StartEditKeybinding,
    /// Cancel editing keybinding
    CancelEditKeybinding,
    /// Apply a new keybinding (key string)
    ApplyKeybinding(String),
    /// Reset the selected keybinding to default
    ResetKeybinding,
    /// Reset all keybindings to defaults
    ResetAllKeybindings,
    /// Save modified keybindings
    SaveKeybindings,

    // Time log editor
    /// Show the time log editor
    ShowTimeLog,
    /// Hide the time log editor
    HideTimeLog,
    /// Navigate up in time log
    TimeLogUp,
    /// Navigate down in time log
    TimeLogDown,
    /// Start editing start time
    TimeLogEditStart,
    /// Start editing end time
    TimeLogEditEnd,
    /// Confirm delete time entry
    TimeLogConfirmDelete,
    /// Cancel time log operation
    TimeLogCancel,
    /// Submit time log edit
    TimeLogSubmit,
    /// Add new time entry for selected task
    TimeLogAddEntry,
    /// Delete the selected time entry
    TimeLogDelete,

    // Work log editor
    /// Show the work log editor for selected task
    ShowWorkLog,
    /// Hide the work log editor
    HideWorkLog,
    /// Navigate up in work log list
    WorkLogUp,
    /// Navigate down in work log list
    WorkLogDown,
    /// View the selected work log entry
    WorkLogView,
    /// Start adding a new work log entry
    WorkLogAdd,
    /// Start editing the selected work log entry
    WorkLogEdit,
    /// Show delete confirmation for work log entry
    WorkLogConfirmDelete,
    /// Cancel work log operation (return to browse mode)
    WorkLogCancel,
    /// Submit work log entry (save add/edit)
    WorkLogSubmit,
    /// Delete the selected work log entry
    WorkLogDelete,
    /// Insert a character in work log buffer
    WorkLogInputChar(char),
    /// Delete character before cursor in work log buffer
    WorkLogInputBackspace,
    /// Delete character at cursor in work log buffer
    WorkLogInputDelete,
    /// Move cursor left in work log buffer
    WorkLogCursorLeft,
    /// Move cursor right in work log buffer
    WorkLogCursorRight,
    /// Move cursor up (to previous line)
    WorkLogCursorUp,
    /// Move cursor down (to next line)
    WorkLogCursorDown,
    /// Insert a newline in work log buffer
    WorkLogNewline,
    /// Move cursor to start of line
    WorkLogCursorHome,
    /// Move cursor to end of line
    WorkLogCursorEnd,
    /// Start work log search mode
    WorkLogSearchStart,
    /// Cancel work log search (return to browse without applying)
    WorkLogSearchCancel,
    /// Apply work log search filter (return to browse with filter active)
    WorkLogSearchApply,
    /// Clear work log search filter
    WorkLogSearchClear,
    /// Input character in work log search
    WorkLogSearchChar(char),
    /// Backspace in work log search
    WorkLogSearchBackspace,

    // Description editor (multi-line)
    /// Start editing description in multi-line mode
    StartEditDescriptionMultiline,
    /// Hide description editor (cancel)
    HideDescriptionEditor,
    /// Submit description edit (save)
    DescriptionSubmit,
    /// Insert a character in description buffer
    DescriptionInputChar(char),
    /// Delete character before cursor in description buffer
    DescriptionInputBackspace,
    /// Delete character at cursor in description buffer
    DescriptionInputDelete,
    /// Move cursor left in description buffer
    DescriptionCursorLeft,
    /// Move cursor right in description buffer
    DescriptionCursorRight,
    /// Move cursor up (to previous line)
    DescriptionCursorUp,
    /// Move cursor down (to next line)
    DescriptionCursorDown,
    /// Insert a newline in description buffer
    DescriptionNewline,
    /// Move cursor to start of line
    DescriptionCursorHome,
    /// Move cursor to end of line
    DescriptionCursorEnd,

    // Overdue alert
    /// Dismiss the overdue tasks alert
    DismissOverdueAlert,

    // Storage error alert
    /// Dismiss the storage error alert
    DismissStorageErrorAlert,

    // Quick reschedule
    /// Reschedule selected task to tomorrow
    RescheduleTomorrow,
    /// Reschedule selected task to next week (7 days from today)
    RescheduleNextWeek,
    /// Reschedule selected task to next Monday
    RescheduleNextMonday,

    // Saved filters
    /// Show saved filter picker
    ShowSavedFilters,
    /// Hide saved filter picker
    HideSavedFilters,
    /// Navigate up in saved filter list
    SavedFilterUp,
    /// Navigate down in saved filter list
    SavedFilterDown,
    /// Apply the selected saved filter
    ApplySavedFilter,
    /// Save current filter as a new saved filter
    SaveCurrentFilter,
    /// Delete the selected saved filter
    DeleteSavedFilter,
    /// Clear the active saved filter
    ClearSavedFilter,

    // Daily review mode
    /// Show daily review mode
    ShowDailyReview,
    /// Hide daily review mode
    HideDailyReview,
    /// Move to next phase in daily review
    DailyReviewNext,
    /// Move to previous phase in daily review
    DailyReviewPrev,
    /// Navigate up in daily review task list
    DailyReviewUp,
    /// Navigate down in daily review task list
    DailyReviewDown,
    /// Complete the selected task in daily review
    DailyReviewComplete,

    // Weekly review mode
    /// Show weekly review mode
    ShowWeeklyReview,
    /// Hide weekly review mode
    HideWeeklyReview,
    /// Move to next phase in weekly review
    WeeklyReviewNext,
    /// Move to previous phase in weekly review
    WeeklyReviewPrev,
    /// Navigate up in weekly review list
    WeeklyReviewUp,
    /// Navigate down in weekly review list
    WeeklyReviewDown,

    // Task snooze
    /// Start editing snooze date for selected task
    StartSnoozeTask,
    /// Clear snooze from selected task
    ClearSnooze,

    // Habit tracking
    /// Start creating a new habit
    StartCreateHabit,
    /// Start editing the selected habit
    StartEditHabit(HabitId),
    /// Navigate up in habit list
    HabitUp,
    /// Navigate down in habit list
    HabitDown,
    /// Toggle today's check-in for selected habit
    HabitToggleToday,
    /// Show habit analytics/details popup
    ShowHabitAnalytics,
    /// Hide habit analytics popup
    HideHabitAnalytics,
    /// Archive the selected habit
    HabitArchive,
    /// Delete the selected habit
    HabitDelete,
    /// Toggle showing archived habits
    HabitToggleShowArchived,

    // Timeline view
    /// Toggle showing dependency lines in timeline
    TimelineToggleDependencies,
    /// View selected task details from timeline (opens focus mode)
    TimelineViewSelected,
    /// View selected task details from Kanban (opens focus mode)
    KanbanViewSelected,
    /// View selected task details from Eisenhower matrix (opens focus mode)
    EisenhowerViewSelected,
    /// View selected task details from Weekly Planner (opens focus mode)
    WeeklyPlannerViewSelected,
    /// View selected task details from Network view (opens focus mode)
    NetworkViewSelected,
    /// Navigate to next task in chain (in focus mode)
    ChainNext,
    /// Navigate to previous task in chain (in focus mode)
    ChainPrev,
}

/// System-level messages for application control.
///
/// These messages handle application lifecycle, persistence,
/// undo/redo, and export operations.
///
/// # Examples
///
/// ```
/// use taskflow::app::{Model, Message, SystemMessage, TaskMessage, update};
///
/// let mut model = Model::new();
///
/// // Create some tasks
/// update(&mut model, TaskMessage::Create("Task 1".to_string()).into());
/// update(&mut model, TaskMessage::Create("Task 2".to_string()).into());
///
/// // Undo the last action
/// update(&mut model, SystemMessage::Undo.into());
///
/// // Redo if needed
/// update(&mut model, SystemMessage::Redo.into());
/// ```
#[derive(Debug, Clone)]
pub enum SystemMessage {
    /// Quit the application
    Quit,
    /// Save current state to storage
    Save,
    /// Undo the last action
    Undo,
    /// Redo the last undone action
    Redo,
    /// Handle terminal resize
    Resize {
        /// New terminal width
        width: u16,
        /// New terminal height
        height: u16,
    },
    /// Periodic tick for time-based updates
    Tick,
    /// Export tasks to CSV format
    ExportCsv,
    /// Export tasks to ICS (iCalendar) format
    ExportIcs,
    /// Export task chains to DOT (Graphviz) format
    ExportChainsDot,
    /// Export task chains to Mermaid format
    ExportChainsMermaid,
    /// Export analytics report to Markdown format
    ExportReportMarkdown,
    /// Export analytics report to HTML format
    ExportReportHtml,
    /// Start import from CSV (opens file path input)
    StartImportCsv,
    /// Start import from ICS (opens file path input)
    StartImportIcs,
    /// Execute import after file path is entered
    ExecuteImport,
    /// Confirm pending import
    ConfirmImport,
    /// Cancel pending import
    CancelImport,
    /// Refresh storage to detect external file changes
    RefreshStorage,
}

impl From<NavigationMessage> for Message {
    fn from(msg: NavigationMessage) -> Self {
        Self::Navigation(msg)
    }
}

impl From<TaskMessage> for Message {
    fn from(msg: TaskMessage) -> Self {
        Self::Task(msg)
    }
}

impl From<UiMessage> for Message {
    fn from(msg: UiMessage) -> Self {
        Self::Ui(msg)
    }
}

impl From<SystemMessage> for Message {
    fn from(msg: SystemMessage) -> Self {
        Self::System(msg)
    }
}

impl From<TimeMessage> for Message {
    fn from(msg: TimeMessage) -> Self {
        Self::Time(msg)
    }
}

impl From<PomodoroMessage> for Message {
    fn from(msg: PomodoroMessage) -> Self {
        Self::Pomodoro(msg)
    }
}

impl From<HabitMessage> for Message {
    fn from(msg: HabitMessage) -> Self {
        Self::Habit(msg)
    }
}

/// Pomodoro timer messages.
///
/// These messages control the Pomodoro timer in focus mode.
///
/// # Examples
///
/// ```
/// use taskflow::app::{Model, Message, PomodoroMessage, update};
///
/// let mut model = Model::new().with_sample_data();
///
/// // Start a Pomodoro session with a goal of 4 cycles
/// update(&mut model, PomodoroMessage::Start { goal_cycles: 4 }.into());
///
/// // Pause/resume the timer
/// update(&mut model, PomodoroMessage::TogglePause.into());
///
/// // Skip current phase
/// update(&mut model, PomodoroMessage::Skip.into());
/// ```
#[derive(Debug, Clone)]
pub enum PomodoroMessage {
    /// Start a new Pomodoro session
    Start {
        /// Target number of work cycles to complete
        goal_cycles: u32,
    },
    /// Pause the current timer
    Pause,
    /// Resume a paused timer
    Resume,
    /// Toggle between paused and running
    TogglePause,
    /// Skip the current phase (work/break)
    Skip,
    /// Stop the Pomodoro session entirely
    Stop,
    /// Timer tick (called every second when running)
    Tick,
    /// Configure work duration (in minutes)
    SetWorkDuration(u32),
    /// Configure short break duration (in minutes)
    SetShortBreak(u32),
    /// Configure long break duration (in minutes)
    SetLongBreak(u32),
    /// Configure cycles before long break
    SetCyclesBeforeLongBreak(u32),
    /// Increment session goal
    IncrementGoal,
    /// Decrement session goal
    DecrementGoal,
}

/// Habit tracking messages.
///
/// These messages handle creating, modifying, and checking in habits.
#[derive(Debug, Clone)]
pub enum HabitMessage {
    /// Create a new habit with the given name
    Create(String),
    /// Check in for today
    CheckInToday {
        /// The habit to check in
        habit_id: HabitId,
        /// Whether the habit was completed
        completed: bool,
    },
    /// Check in for a specific date
    CheckIn {
        /// The habit to check in
        habit_id: HabitId,
        /// The date to check in for
        date: NaiveDate,
        /// Whether the habit was completed
        completed: bool,
    },
    /// Toggle today's completion status
    ToggleToday(HabitId),
    /// Archive a habit
    Archive(HabitId),
    /// Unarchive a habit
    Unarchive(HabitId),
    /// Delete a habit
    Delete(HabitId),
    /// Update habit name
    UpdateName {
        /// The habit to update
        habit_id: HabitId,
        /// The new name
        name: String,
    },
}
