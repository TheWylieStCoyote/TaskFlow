use crate::domain::{Priority, ProjectId, TaskId, TaskStatus};

/// Top-level message enum for the application
#[derive(Debug, Clone)]
pub enum Message {
    Navigation(NavigationMessage),
    Task(TaskMessage),
    Ui(UiMessage),
    System(SystemMessage),
    None,
}

/// Navigation messages
#[derive(Debug, Clone)]
pub enum NavigationMessage {
    Up,
    Down,
    First,
    Last,
    PageUp,
    PageDown,
    Select(usize),
    GoToView(ViewId),
}

/// View identifiers
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ViewId {
    #[default]
    TaskList,
    Today,
    Upcoming,
    Projects,
}

/// Task operations
#[derive(Debug, Clone)]
pub enum TaskMessage {
    ToggleComplete,
    SetStatus(TaskId, TaskStatus),
    SetPriority(TaskId, Priority),
    Create(String),
    Delete(TaskId),
    MoveToProject(TaskId, Option<ProjectId>),
}

/// UI messages
#[derive(Debug, Clone)]
pub enum UiMessage {
    ToggleShowCompleted,
    ToggleSidebar,
    ShowHelp,
    HideHelp,
}

/// System messages
#[derive(Debug, Clone)]
pub enum SystemMessage {
    Quit,
    Resize { width: u16, height: u16 },
    Tick,
}

impl From<NavigationMessage> for Message {
    fn from(msg: NavigationMessage) -> Self {
        Message::Navigation(msg)
    }
}

impl From<TaskMessage> for Message {
    fn from(msg: TaskMessage) -> Self {
        Message::Task(msg)
    }
}

impl From<UiMessage> for Message {
    fn from(msg: UiMessage) -> Self {
        Message::Ui(msg)
    }
}

impl From<SystemMessage> for Message {
    fn from(msg: SystemMessage) -> Self {
        Message::System(msg)
    }
}
