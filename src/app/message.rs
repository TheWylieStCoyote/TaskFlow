use crate::domain::{Priority, ProjectId, TaskId, TaskStatus};

/// Which pane currently has focus
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusPane {
    #[default]
    TaskList,
    Sidebar,
}

/// Top-level message enum for the application
#[derive(Debug, Clone)]
pub enum Message {
    Navigation(NavigationMessage),
    Task(TaskMessage),
    Time(TimeMessage),
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
    FocusSidebar,
    FocusTaskList,
    SelectSidebarItem,
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

/// Time tracking messages
#[derive(Debug, Clone)]
pub enum TimeMessage {
    StartTracking,
    StopTracking,
    ToggleTracking,
}

/// UI messages
#[derive(Debug, Clone)]
pub enum UiMessage {
    ToggleShowCompleted,
    ToggleSidebar,
    ShowHelp,
    HideHelp,
    // Input mode
    StartCreateTask,
    StartCreateProject,
    CancelInput,
    SubmitInput,
    InputChar(char),
    InputBackspace,
    InputDelete,
    InputCursorLeft,
    InputCursorRight,
    InputCursorStart,
    InputCursorEnd,
    // Delete confirmation
    ShowDeleteConfirm,
    ConfirmDelete,
    CancelDelete,
}

/// System messages
#[derive(Debug, Clone)]
pub enum SystemMessage {
    Quit,
    Save,
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

impl From<TimeMessage> for Message {
    fn from(msg: TimeMessage) -> Self {
        Message::Time(msg)
    }
}
