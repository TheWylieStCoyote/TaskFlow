//! UI state and interaction messages.

use crate::domain::HabitId;

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
