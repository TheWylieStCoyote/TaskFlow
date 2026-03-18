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
    QuickCapture,
    CreateSubtask,
    CreateProject,
    EditProject,
    DeleteProject,
    EditTask,
    EditDueDate,
    EditScheduledDate,
    EditScheduledTime,
    EditTags,
    EditDescription,
    EditDescriptionMultiline,
    DeleteTask,
    DuplicateTask,
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
    ToggleFullScreenFocus,
    AddToFocusQueue,
    ClearFocusQueue,
    AdvanceFocusQueue,
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

    // Quick reschedule
    RescheduleTomorrow,
    RescheduleNextWeek,
    RescheduleNextMonday,

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

    // Burndown chart controls
    BurndownCycleWindow,
    BurndownToggleMode,
    BurndownToggleScopeCreep,

    // Duplicate detection controls
    DismissDuplicate,
    MergeDuplicates,
    RefreshDuplicates,
    // Git integration
    ViewGitTodos,
    ScanGitTodos,
    OpenInEditor,

    // Review modes
    ShowDailyReview,
    ShowWeeklyReview,
    ShowEveningReview,

    // Task detail
    ShowTaskDetail,

    // Command palette
    ShowCommandPalette,
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
    Burndown,
    Duplicates,
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
            Self::Burndown => "Burndown",
            Self::Duplicates => "Duplicates",
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
            Self::Burndown => 12,
            Self::Duplicates => 13,
            Self::Export => 14,
            Self::Import => 15,
            Self::Macros => 16,
            Self::Templates => 17,
            Self::Pomodoro => 18,
            Self::System => 19,
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
            Self::QuickCapture => "Quick capture with hints",
            Self::CreateSubtask => "Create subtask",
            Self::EditTask => "Edit task title",
            Self::EditDueDate => "Edit due date",
            Self::EditScheduledDate => "Edit scheduled date",
            Self::EditScheduledTime => "Edit scheduled time block",
            Self::EditTags => "Edit tags",
            Self::EditDescription => "Edit description",
            Self::EditDescriptionMultiline => "Edit description (multi-line)",
            Self::DeleteTask => "Delete task",
            Self::DuplicateTask => "Duplicate task",
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
            Self::ToggleFullScreenFocus => "Toggle full-screen focus",
            Self::AddToFocusQueue => "Add to focus queue",
            Self::ClearFocusQueue => "Clear focus queue",
            Self::AdvanceFocusQueue => "Next in focus queue",
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
            // Quick reschedule
            Self::RescheduleTomorrow => "Reschedule to tomorrow",
            Self::RescheduleNextWeek => "Reschedule to next week",
            Self::RescheduleNextMonday => "Reschedule to next Monday",
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
            // Burndown
            Self::BurndownCycleWindow => "Cycle time window",
            Self::BurndownToggleMode => "Toggle task/time mode",
            Self::BurndownToggleScopeCreep => "Toggle scope creep display",
            // Duplicates
            Self::DismissDuplicate => "Dismiss duplicate pair",
            Self::MergeDuplicates => "Merge (delete second task)",
            Self::RefreshDuplicates => "Refresh duplicate list",
            // Git integration
            Self::ViewGitTodos => "View Git TODOs",
            Self::ScanGitTodos => "Scan git repository for TODOs",
            Self::OpenInEditor => "Open in editor",
            // Reviews
            Self::ShowDailyReview => "Daily review",
            Self::ShowWeeklyReview => "Weekly review",
            Self::ShowEveningReview => "Evening review",
            // Task detail
            Self::ShowTaskDetail => "Show task details",
            // Command palette
            Self::ShowCommandPalette => "Open command palette",
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
            | Self::QuickCapture
            | Self::CreateSubtask
            | Self::EditTask
            | Self::EditDueDate
            | Self::EditScheduledDate
            | Self::EditScheduledTime
            | Self::EditTags
            | Self::EditDescription
            | Self::EditDescriptionMultiline
            | Self::DeleteTask
            | Self::DuplicateTask
            | Self::CyclePriority
            | Self::MoveToProject
            | Self::MoveTaskUp
            | Self::MoveTaskDown
            | Self::RescheduleTomorrow
            | Self::RescheduleNextWeek
            | Self::RescheduleNextMonday
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
            | Self::ToggleFullScreenFocus
            | Self::AddToFocusQueue
            | Self::ClearFocusQueue
            | Self::AdvanceFocusQueue
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

            Self::BurndownCycleWindow
            | Self::BurndownToggleMode
            | Self::BurndownToggleScopeCreep => ActionCategory::Burndown,

            Self::DismissDuplicate | Self::MergeDuplicates | Self::RefreshDuplicates => {
                ActionCategory::Duplicates
            }
            Self::ViewGitTodos | Self::ScanGitTodos | Self::OpenInEditor => {
                ActionCategory::ViewFilter
            }

            Self::ShowDailyReview | Self::ShowWeeklyReview | Self::ShowEveningReview => {
                ActionCategory::ViewFilter
            }

            Self::ShowTaskDetail => ActionCategory::Tasks,
            Self::ShowCommandPalette => ActionCategory::ViewFilter,
        }
    }
}

/// All available actions for iteration.
///
/// Used by the command palette to list all available commands.
pub const ALL_ACTIONS: &[Action] = &[
    // Navigation
    Action::MoveUp,
    Action::MoveDown,
    Action::MoveFirst,
    Action::MoveLast,
    Action::PageUp,
    Action::PageDown,
    // Task actions
    Action::ToggleComplete,
    Action::CreateTask,
    Action::QuickCapture,
    Action::CreateSubtask,
    Action::CreateProject,
    Action::EditProject,
    Action::DeleteProject,
    Action::EditTask,
    Action::EditDueDate,
    Action::EditScheduledDate,
    Action::EditScheduledTime,
    Action::EditTags,
    Action::EditDescription,
    Action::EditDescriptionMultiline,
    Action::DeleteTask,
    Action::DuplicateTask,
    Action::CyclePriority,
    Action::MoveToProject,
    // Time tracking
    Action::ToggleTimeTracking,
    Action::ShowTimeLog,
    Action::ShowWorkLog,
    Action::EditEstimate,
    // UI actions
    Action::ToggleSidebar,
    Action::ToggleShowCompleted,
    Action::ShowHelp,
    Action::ToggleFocusMode,
    Action::ToggleFullScreenFocus,
    Action::AddToFocusQueue,
    Action::ClearFocusQueue,
    Action::AdvanceFocusQueue,
    Action::FocusSidebar,
    Action::FocusTaskList,
    Action::Select,
    Action::Search,
    Action::ClearSearch,
    Action::FilterByTag,
    Action::ClearTagFilter,
    Action::CycleSortField,
    Action::ToggleSortOrder,
    // Multi-select
    Action::ToggleMultiSelect,
    Action::ToggleTaskSelection,
    Action::SelectAll,
    Action::ClearSelection,
    Action::BulkDelete,
    Action::BulkMoveToProject,
    Action::BulkSetStatus,
    // Dependencies
    Action::EditDependencies,
    // Recurrence
    Action::EditRecurrence,
    // Manual ordering
    Action::MoveTaskUp,
    Action::MoveTaskDown,
    // Task chains
    Action::LinkTask,
    Action::UnlinkTask,
    // Calendar navigation
    Action::CalendarPrevMonth,
    Action::CalendarNextMonth,
    Action::CalendarPrevDay,
    Action::CalendarNextDay,
    // Reports navigation
    Action::ReportsNextPanel,
    Action::ReportsPrevPanel,
    // System
    Action::Save,
    Action::Undo,
    Action::Redo,
    Action::Quit,
    Action::RefreshStorage,
    // Export
    Action::ExportCsv,
    Action::ExportIcs,
    Action::ExportChainsDot,
    Action::ExportChainsMermaid,
    Action::ExportReportMarkdown,
    Action::ExportReportHtml,
    // Import
    Action::ImportCsv,
    Action::ImportIcs,
    // Macros
    Action::RecordMacro,
    Action::StopRecordMacro,
    Action::PlayMacro0,
    Action::PlayMacro1,
    Action::PlayMacro2,
    Action::PlayMacro3,
    Action::PlayMacro4,
    Action::PlayMacro5,
    Action::PlayMacro6,
    Action::PlayMacro7,
    Action::PlayMacro8,
    Action::PlayMacro9,
    // Templates
    Action::ShowTemplates,
    // Keybindings editor
    Action::ShowKeybindingsEditor,
    // Quick reschedule
    Action::RescheduleTomorrow,
    Action::RescheduleNextWeek,
    Action::RescheduleNextMonday,
    // Task snooze
    Action::SnoozeTask,
    Action::ClearSnooze,
    // Pomodoro timer
    Action::PomodoroStart,
    Action::PomodoroPause,
    Action::PomodoroResume,
    Action::PomodoroTogglePause,
    Action::PomodoroSkip,
    Action::PomodoroStop,
    // Habit tracking
    Action::CreateHabit,
    Action::EditHabit,
    Action::DeleteHabit,
    Action::ToggleHabitToday,
    Action::ShowHabitAnalytics,
    Action::HabitToggleShowArchived,
    Action::HabitArchive,
    // Burndown chart controls
    Action::BurndownCycleWindow,
    Action::BurndownToggleMode,
    Action::BurndownToggleScopeCreep,
    // Duplicate detection controls
    Action::DismissDuplicate,
    Action::MergeDuplicates,
    Action::RefreshDuplicates,
    // Git integration
    Action::ViewGitTodos,
    Action::ScanGitTodos,
    Action::OpenInEditor,
    // Review modes
    Action::ShowDailyReview,
    Action::ShowWeeklyReview,
    Action::ShowEveningReview,
    // Task detail
    Action::ShowTaskDetail,
    // Command palette
    Action::ShowCommandPalette,
];
