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

use crate::domain::{Priority, ProjectId, TaskId, TaskStatus};

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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
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
    /// Monthly calendar view
    Calendar,
    /// Statistics and overview dashboard
    Dashboard,
    /// Tasks grouped by project
    Projects,
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

    // Input mode - starting various edit operations
    /// Enter input mode to create a new task
    StartCreateTask,
    /// Enter input mode to create a subtask
    StartCreateSubtask,
    /// Enter input mode to create a new project
    StartCreateProject,
    /// Enter input mode to edit task title
    StartEditTask,
    /// Enter input mode to edit due date
    StartEditDueDate,
    /// Enter input mode to edit tags
    StartEditTags,
    /// Enter input mode to edit description
    StartEditDescription,
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

    // Recurrence
    /// Enter input mode to edit task recurrence
    StartEditRecurrence,

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
