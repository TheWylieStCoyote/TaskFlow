//! Keybinding actions and categories.

use serde::{Deserialize, Serialize};

/// Action that can be triggered by a keybinding
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    // Navigation
    MoveUp,
    MoveDown,
    MoveFirst,
    MoveLast,
    PageUp,
    PageDown,

    // Task actions
    ToggleComplete,
    CreateTask,
    CreateSubtask,
    CreateProject,
    EditProject,
    DeleteProject,
    EditTask,
    EditDueDate,
    EditScheduledDate,
    EditTags,
    EditDescription,
    EditDescriptionMultiline,
    DeleteTask,
    CyclePriority,
    MoveToProject,

    // Time tracking
    ToggleTimeTracking,
    ShowTimeLog,
    ShowWorkLog,
    EditEstimate,

    // UI actions
    ToggleSidebar,
    ToggleShowCompleted,
    ShowHelp,
    ToggleFocusMode,
    FocusSidebar,
    FocusTaskList,
    Select,
    Search,
    ClearSearch,
    FilterByTag,
    ClearTagFilter,
    CycleSortField,
    ToggleSortOrder,

    // Multi-select / Bulk operations
    ToggleMultiSelect,
    ToggleTaskSelection,
    SelectAll,
    ClearSelection,
    BulkDelete,
    BulkMoveToProject,
    BulkSetStatus,

    // Dependencies
    EditDependencies,

    // Recurrence
    EditRecurrence,

    // Manual ordering
    MoveTaskUp,
    MoveTaskDown,

    // Task chains
    LinkTask,
    UnlinkTask,

    // Calendar navigation
    CalendarPrevMonth,
    CalendarNextMonth,
    CalendarPrevDay,
    CalendarNextDay,

    // Reports navigation
    ReportsNextPanel,
    ReportsPrevPanel,

    // System
    Save,
    Undo,
    Redo,
    Quit,
    RefreshStorage,

    // Export
    ExportCsv,
    ExportIcs,
    ExportChainsDot,
    ExportChainsMermaid,
    ExportReportMarkdown,
    ExportReportHtml,

    // Import
    ImportCsv,
    ImportIcs,

    // Macros
    RecordMacro,
    StopRecordMacro,
    PlayMacro0,
    PlayMacro1,
    PlayMacro2,
    PlayMacro3,
    PlayMacro4,
    PlayMacro5,
    PlayMacro6,
    PlayMacro7,
    PlayMacro8,
    PlayMacro9,

    // Templates
    ShowTemplates,

    // Keybindings editor
    ShowKeybindingsEditor,

    // Task snooze
    SnoozeTask,
    ClearSnooze,

    // Pomodoro timer
    PomodoroStart,
    PomodoroPause,
    PomodoroResume,
    PomodoroTogglePause,
    PomodoroSkip,
    PomodoroStop,

    // Habit tracking
    CreateHabit,
    EditHabit,
    DeleteHabit,
    ToggleHabitToday,
    ShowHabitAnalytics,
    HabitToggleShowArchived,
    HabitArchive,
}

/// Category for grouping actions in help display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionCategory {
    Navigation,
    Tasks,
    Projects,
    TimeTracking,
    Habits,
    ViewFilter,
    MultiSelect,
    Dependencies,
    Recurrence,
    TaskChains,
    Calendar,
    Reports,
    Export,
    Import,
    Macros,
    Templates,
    Pomodoro,
    System,
}

impl ActionCategory {
    /// Get display name for the category
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Navigation => "Navigation",
            Self::Tasks => "Tasks",
            Self::Projects => "Projects",
            Self::TimeTracking => "Time Tracking",
            Self::Habits => "Habits",
            Self::ViewFilter => "View & Filter",
            Self::MultiSelect => "Multi-Select",
            Self::Dependencies => "Dependencies",
            Self::Recurrence => "Recurrence",
            Self::TaskChains => "Task Chains",
            Self::Calendar => "Calendar",
            Self::Reports => "Reports",
            Self::Export => "Export",
            Self::Import => "Import",
            Self::Macros => "Macros",
            Self::Templates => "Templates",
            Self::Pomodoro => "Pomodoro Timer",
            Self::System => "System",
        }
    }

    /// Get display order for sorting categories
    #[must_use]
    pub const fn display_order(&self) -> u8 {
        match self {
            Self::Navigation => 0,
            Self::Tasks => 1,
            Self::Projects => 2,
            Self::TimeTracking => 3,
            Self::Habits => 4,
            Self::ViewFilter => 5,
            Self::MultiSelect => 6,
            Self::Dependencies => 7,
            Self::Recurrence => 8,
            Self::TaskChains => 9,
            Self::Calendar => 10,
            Self::Reports => 11,
            Self::Export => 12,
            Self::Import => 13,
            Self::Macros => 14,
            Self::Templates => 15,
            Self::Pomodoro => 16,
            Self::System => 17,
        }
    }
}

impl Action {
    /// Get a human-readable description of the action
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            // Navigation
            Self::MoveUp => "Move up",
            Self::MoveDown => "Move down",
            Self::MoveFirst => "Go to first",
            Self::MoveLast => "Go to last",
            Self::PageUp => "Page up",
            Self::PageDown => "Page down",
            // Task actions
            Self::ToggleComplete => "Toggle complete",
            Self::CreateTask => "Create task",
            Self::CreateSubtask => "Create subtask",
            Self::EditTask => "Edit task title",
            Self::EditDueDate => "Edit due date",
            Self::EditScheduledDate => "Edit scheduled date",
            Self::EditTags => "Edit tags",
            Self::EditDescription => "Edit description",
            Self::EditDescriptionMultiline => "Edit description (multi-line)",
            Self::DeleteTask => "Delete task",
            Self::CyclePriority => "Cycle priority",
            Self::MoveToProject => "Move to project",
            // Project actions
            Self::CreateProject => "Create project",
            Self::EditProject => "Edit project",
            Self::DeleteProject => "Delete project",
            // Time tracking
            Self::ToggleTimeTracking => "Toggle time tracking",
            Self::ShowTimeLog => "Show time log",
            Self::ShowWorkLog => "Show work log",
            Self::EditEstimate => "Edit time estimate",
            // UI actions
            Self::ToggleSidebar => "Toggle sidebar",
            Self::ToggleShowCompleted => "Toggle show completed",
            Self::ShowHelp => "Show help",
            Self::ToggleFocusMode => "Toggle focus mode",
            Self::FocusSidebar => "Focus sidebar",
            Self::FocusTaskList => "Focus task list",
            Self::Select => "Select item",
            Self::Search => "Search tasks",
            Self::ClearSearch => "Clear search",
            Self::FilterByTag => "Filter by tag",
            Self::ClearTagFilter => "Clear tag filter",
            Self::CycleSortField => "Cycle sort field",
            Self::ToggleSortOrder => "Toggle sort order",
            // Multi-select
            Self::ToggleMultiSelect => "Toggle multi-select",
            Self::ToggleTaskSelection => "Toggle task selection",
            Self::SelectAll => "Select all",
            Self::ClearSelection => "Clear selection",
            Self::BulkDelete => "Bulk delete",
            Self::BulkMoveToProject => "Bulk move to project",
            Self::BulkSetStatus => "Bulk set status",
            // Dependencies
            Self::EditDependencies => "Edit dependencies",
            // Recurrence
            Self::EditRecurrence => "Edit recurrence",
            // Manual ordering
            Self::MoveTaskUp => "Move task up",
            Self::MoveTaskDown => "Move task down",
            // Task chains
            Self::LinkTask => "Link to next task",
            Self::UnlinkTask => "Unlink from chain",
            // Calendar
            Self::CalendarPrevMonth => "Previous month",
            Self::CalendarNextMonth => "Next month",
            Self::CalendarPrevDay => "Previous day",
            Self::CalendarNextDay => "Next day",
            // Reports
            Self::ReportsNextPanel => "Next panel",
            Self::ReportsPrevPanel => "Previous panel",
            // System
            Self::Save => "Save",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::Quit => "Quit",
            // Export
            Self::ExportCsv => "Export to CSV",
            Self::ExportIcs => "Export to ICS",
            Self::ExportChainsDot => "Export chains (DOT)",
            Self::ExportChainsMermaid => "Export chains (Mermaid)",
            Self::ExportReportMarkdown => "Export report (Markdown)",
            Self::ExportReportHtml => "Export report (HTML)",
            // Import
            Self::ImportCsv => "Import from CSV",
            Self::ImportIcs => "Import from ICS",
            // Macros
            Self::RecordMacro => "Record macro",
            Self::StopRecordMacro => "Stop recording",
            Self::PlayMacro0 => "Play macro 0",
            Self::PlayMacro1 => "Play macro 1",
            Self::PlayMacro2 => "Play macro 2",
            Self::PlayMacro3 => "Play macro 3",
            Self::PlayMacro4 => "Play macro 4",
            Self::PlayMacro5 => "Play macro 5",
            Self::PlayMacro6 => "Play macro 6",
            Self::PlayMacro7 => "Play macro 7",
            Self::PlayMacro8 => "Play macro 8",
            Self::PlayMacro9 => "Play macro 9",
            // Templates
            Self::ShowTemplates => "Show templates",
            // Keybindings
            Self::ShowKeybindingsEditor => "Edit keybindings",
            // Snooze
            Self::SnoozeTask => "Snooze task",
            Self::ClearSnooze => "Clear snooze",
            // Pomodoro
            Self::PomodoroStart => "Start Pomodoro",
            Self::PomodoroPause => "Pause timer",
            Self::PomodoroResume => "Resume timer",
            Self::PomodoroTogglePause => "Toggle pause",
            Self::PomodoroSkip => "Skip phase",
            Self::PomodoroStop => "Stop Pomodoro",
            // Storage
            Self::RefreshStorage => "Refresh storage",
            // Habits
            Self::CreateHabit => "Create habit",
            Self::EditHabit => "Edit habit",
            Self::DeleteHabit => "Delete habit",
            Self::ToggleHabitToday => "Toggle today's check-in",
            Self::ShowHabitAnalytics => "Show habit analytics",
            Self::HabitToggleShowArchived => "Toggle show archived",
            Self::HabitArchive => "Archive habit",
        }
    }

    /// Get the category this action belongs to
    #[must_use]
    pub const fn category(&self) -> ActionCategory {
        match self {
            Self::MoveUp
            | Self::MoveDown
            | Self::MoveFirst
            | Self::MoveLast
            | Self::PageUp
            | Self::PageDown => ActionCategory::Navigation,

            Self::ToggleComplete
            | Self::CreateTask
            | Self::CreateSubtask
            | Self::EditTask
            | Self::EditDueDate
            | Self::EditScheduledDate
            | Self::EditTags
            | Self::EditDescription
            | Self::EditDescriptionMultiline
            | Self::DeleteTask
            | Self::CyclePriority
            | Self::MoveToProject
            | Self::MoveTaskUp
            | Self::MoveTaskDown
            | Self::SnoozeTask
            | Self::ClearSnooze => ActionCategory::Tasks,

            Self::CreateProject | Self::EditProject | Self::DeleteProject => {
                ActionCategory::Projects
            }

            Self::ToggleTimeTracking
            | Self::ShowTimeLog
            | Self::ShowWorkLog
            | Self::EditEstimate => ActionCategory::TimeTracking,

            Self::ToggleSidebar
            | Self::ToggleShowCompleted
            | Self::ShowHelp
            | Self::ToggleFocusMode
            | Self::FocusSidebar
            | Self::FocusTaskList
            | Self::Select
            | Self::Search
            | Self::ClearSearch
            | Self::FilterByTag
            | Self::ClearTagFilter
            | Self::CycleSortField
            | Self::ToggleSortOrder => ActionCategory::ViewFilter,

            Self::ToggleMultiSelect
            | Self::ToggleTaskSelection
            | Self::SelectAll
            | Self::ClearSelection
            | Self::BulkDelete
            | Self::BulkMoveToProject
            | Self::BulkSetStatus => ActionCategory::MultiSelect,

            Self::EditDependencies => ActionCategory::Dependencies,
            Self::EditRecurrence => ActionCategory::Recurrence,
            Self::LinkTask | Self::UnlinkTask => ActionCategory::TaskChains,

            Self::CalendarPrevMonth
            | Self::CalendarNextMonth
            | Self::CalendarPrevDay
            | Self::CalendarNextDay => ActionCategory::Calendar,

            Self::ReportsNextPanel | Self::ReportsPrevPanel => ActionCategory::Reports,

            Self::ExportCsv
            | Self::ExportIcs
            | Self::ExportChainsDot
            | Self::ExportChainsMermaid
            | Self::ExportReportMarkdown
            | Self::ExportReportHtml => ActionCategory::Export,

            Self::ImportCsv | Self::ImportIcs => ActionCategory::Import,

            Self::RecordMacro
            | Self::StopRecordMacro
            | Self::PlayMacro0
            | Self::PlayMacro1
            | Self::PlayMacro2
            | Self::PlayMacro3
            | Self::PlayMacro4
            | Self::PlayMacro5
            | Self::PlayMacro6
            | Self::PlayMacro7
            | Self::PlayMacro8
            | Self::PlayMacro9 => ActionCategory::Macros,

            Self::ShowTemplates => ActionCategory::Templates,
            Self::ShowKeybindingsEditor => ActionCategory::System,

            Self::PomodoroStart
            | Self::PomodoroPause
            | Self::PomodoroResume
            | Self::PomodoroTogglePause
            | Self::PomodoroSkip
            | Self::PomodoroStop => ActionCategory::Pomodoro,

            Self::Save | Self::Undo | Self::Redo | Self::Quit | Self::RefreshStorage => {
                ActionCategory::System
            }

            Self::CreateHabit
            | Self::EditHabit
            | Self::DeleteHabit
            | Self::ToggleHabitToday
            | Self::ShowHabitAnalytics
            | Self::HabitToggleShowArchived
            | Self::HabitArchive => ActionCategory::Habits,
        }
    }
}
