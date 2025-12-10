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

mod habit;
mod navigation;
mod pomodoro;
mod system;
mod task;
mod time;
mod ui;

pub use habit::HabitMessage;
pub use navigation::{NavigationMessage, ViewId};
pub use pomodoro::PomodoroMessage;
pub use system::SystemMessage;
pub use task::TaskMessage;
pub use time::TimeMessage;
pub use ui::UiMessage;

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
