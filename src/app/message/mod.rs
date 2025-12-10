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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_pane_default() {
        let pane = FocusPane::default();
        assert_eq!(pane, FocusPane::TaskList);
    }

    #[test]
    fn test_focus_pane_equality() {
        assert_eq!(FocusPane::TaskList, FocusPane::TaskList);
        assert_eq!(FocusPane::Sidebar, FocusPane::Sidebar);
        assert_ne!(FocusPane::TaskList, FocusPane::Sidebar);
    }

    #[test]
    fn test_message_from_navigation() {
        let nav_msg = NavigationMessage::Down;
        let msg: Message = nav_msg.into();
        assert!(matches!(msg, Message::Navigation(NavigationMessage::Down)));
    }

    #[test]
    fn test_message_from_task() {
        let task_msg = TaskMessage::Create("Test".to_string());
        let msg: Message = task_msg.into();
        assert!(matches!(msg, Message::Task(TaskMessage::Create(_))));
    }

    #[test]
    fn test_message_from_ui() {
        let ui_msg = UiMessage::ShowHelp;
        let msg: Message = ui_msg.into();
        assert!(matches!(msg, Message::Ui(UiMessage::ShowHelp)));
    }

    #[test]
    fn test_message_from_system() {
        let sys_msg = SystemMessage::Quit;
        let msg: Message = sys_msg.into();
        assert!(matches!(msg, Message::System(SystemMessage::Quit)));
    }

    #[test]
    fn test_message_from_time() {
        let time_msg = TimeMessage::StartTracking;
        let msg: Message = time_msg.into();
        assert!(matches!(msg, Message::Time(TimeMessage::StartTracking)));
    }

    #[test]
    fn test_message_from_pomodoro() {
        let pom_msg = PomodoroMessage::Start { goal_cycles: 4 };
        let msg: Message = pom_msg.into();
        assert!(matches!(
            msg,
            Message::Pomodoro(PomodoroMessage::Start { .. })
        ));
    }

    #[test]
    fn test_message_from_habit() {
        let habit_msg = HabitMessage::Create("Exercise".to_string());
        let msg: Message = habit_msg.into();
        assert!(matches!(msg, Message::Habit(HabitMessage::Create(_))));
    }

    #[test]
    fn test_message_none_variant() {
        let msg = Message::None;
        assert!(matches!(msg, Message::None));
    }
}
