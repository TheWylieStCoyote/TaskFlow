//! Utility functions for input handling.

use crossterm::event::{self, KeyCode, KeyModifiers};

use taskflow::app::{
    Message, NavigationMessage, PomodoroMessage, SystemMessage, UiMessage, ViewId,
};
use taskflow::config::Action;

/// Convert a key event to the string format used in keybindings
pub fn key_event_to_string(key: &event::KeyEvent) -> String {
    let mut parts = Vec::new();

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("ctrl");
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        parts.push("alt");
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) && !matches!(key.code, KeyCode::Char(_)) {
        parts.push("shift");
    }

    let key_name = match key.code {
        KeyCode::Char(' ') => "space".to_string(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "enter".to_string(),
        KeyCode::Esc => "esc".to_string(),
        KeyCode::Backspace => "backspace".to_string(),
        KeyCode::Delete => "delete".to_string(),
        KeyCode::Up => "up".to_string(),
        KeyCode::Down => "down".to_string(),
        KeyCode::Left => "left".to_string(),
        KeyCode::Right => "right".to_string(),
        KeyCode::Home => "home".to_string(),
        KeyCode::End => "end".to_string(),
        KeyCode::PageUp => "pageup".to_string(),
        KeyCode::PageDown => "pagedown".to_string(),
        KeyCode::Tab => "tab".to_string(),
        KeyCode::F(n) => format!("f{n}"),
        _ => return String::new(),
    };

    if parts.is_empty() {
        key_name
    } else {
        parts.push(&key_name);
        parts.join("+")
    }
}

/// Convert an Action to a Message
pub const fn action_to_message(action: &Action) -> Message {
    match action {
        Action::MoveUp => Message::Navigation(NavigationMessage::Up),
        Action::MoveDown => Message::Navigation(NavigationMessage::Down),
        Action::MoveFirst => Message::Navigation(NavigationMessage::First),
        Action::MoveLast => Message::Navigation(NavigationMessage::Last),
        Action::PageUp => Message::Navigation(NavigationMessage::PageUp),
        Action::PageDown => Message::Navigation(NavigationMessage::PageDown),
        Action::ToggleComplete => Message::Task(taskflow::app::TaskMessage::ToggleComplete),
        Action::CreateTask => Message::Ui(UiMessage::StartCreateTask),
        Action::QuickCapture => Message::Ui(UiMessage::StartQuickCapture),
        Action::CreateSubtask => Message::Ui(UiMessage::StartCreateSubtask),
        Action::CreateProject => Message::Ui(UiMessage::StartCreateProject),
        Action::EditProject => Message::Ui(UiMessage::StartEditProject),
        Action::DeleteProject => Message::Ui(UiMessage::DeleteProject),
        Action::EditTask => Message::Ui(UiMessage::StartEditTask),
        Action::EditDueDate => Message::Ui(UiMessage::StartEditDueDate),
        Action::EditScheduledDate => Message::Ui(UiMessage::StartEditScheduledDate),
        Action::EditScheduledTime => Message::Ui(UiMessage::StartEditScheduledTime),
        Action::EditTags => Message::Ui(UiMessage::StartEditTags),
        Action::EditDescription => Message::Ui(UiMessage::StartEditDescription),
        Action::EditDescriptionMultiline => Message::Ui(UiMessage::StartEditDescriptionMultiline),
        Action::DeleteTask => Message::Ui(UiMessage::ShowDeleteConfirm),
        Action::DuplicateTask => Message::Task(taskflow::app::TaskMessage::Duplicate),
        Action::CyclePriority => Message::Task(taskflow::app::TaskMessage::CyclePriority),
        Action::MoveToProject => Message::Ui(UiMessage::StartMoveToProject),
        Action::ToggleTimeTracking => Message::Time(taskflow::app::TimeMessage::ToggleTracking),
        Action::ShowTimeLog => Message::Ui(UiMessage::ShowTimeLog),
        Action::ShowWorkLog => Message::Ui(UiMessage::ShowWorkLog),
        Action::EditEstimate => Message::Ui(UiMessage::StartEditEstimate),
        Action::ToggleSidebar => Message::Ui(UiMessage::ToggleSidebar),
        Action::ToggleShowCompleted => Message::Ui(UiMessage::ToggleShowCompleted),
        Action::ShowHelp => Message::Ui(UiMessage::ShowHelp),
        Action::FocusSidebar => Message::Navigation(NavigationMessage::FocusSidebar),
        Action::FocusTaskList => Message::Navigation(NavigationMessage::FocusTaskList),
        Action::Select => Message::Navigation(NavigationMessage::SelectSidebarItem),
        Action::Search => Message::Ui(UiMessage::StartSearch),
        Action::ClearSearch => Message::Ui(UiMessage::ClearSearch),
        Action::FilterByTag => Message::Ui(UiMessage::StartFilterByTag),
        Action::ClearTagFilter => Message::Ui(UiMessage::ClearTagFilter),
        Action::CycleSortField => Message::Ui(UiMessage::CycleSortField),
        Action::ToggleSortOrder => Message::Ui(UiMessage::ToggleSortOrder),
        Action::ToggleMultiSelect => Message::Ui(UiMessage::ToggleMultiSelect),
        Action::ToggleTaskSelection => Message::Ui(UiMessage::ToggleTaskSelection),
        Action::SelectAll => Message::Ui(UiMessage::SelectAll),
        Action::ClearSelection => Message::Ui(UiMessage::ClearSelection),
        Action::BulkDelete => Message::Ui(UiMessage::BulkDelete),
        Action::BulkMoveToProject => Message::Ui(UiMessage::StartBulkMoveToProject),
        Action::BulkSetStatus => Message::Ui(UiMessage::StartBulkSetStatus),
        Action::EditDependencies => Message::Ui(UiMessage::StartEditDependencies),
        Action::EditRecurrence => Message::Ui(UiMessage::StartEditRecurrence),
        Action::MoveTaskUp => Message::Ui(UiMessage::MoveTaskUp),
        Action::MoveTaskDown => Message::Ui(UiMessage::MoveTaskDown),
        Action::LinkTask => Message::Ui(UiMessage::StartLinkTask),
        Action::UnlinkTask => Message::Ui(UiMessage::UnlinkTask),
        Action::CalendarPrevMonth => Message::Navigation(NavigationMessage::CalendarPrevMonth),
        Action::CalendarNextMonth => Message::Navigation(NavigationMessage::CalendarNextMonth),
        Action::CalendarPrevDay => Message::Ui(UiMessage::CalendarPrevDay),
        Action::CalendarNextDay => Message::Ui(UiMessage::CalendarNextDay),
        Action::Save => Message::System(SystemMessage::Save),
        Action::Undo => Message::System(SystemMessage::Undo),
        Action::Redo => Message::System(SystemMessage::Redo),
        Action::Quit => Message::System(SystemMessage::Quit),
        Action::ExportCsv => Message::System(SystemMessage::ExportCsv),
        Action::ExportIcs => Message::System(SystemMessage::ExportIcs),
        Action::ExportChainsDot => Message::System(SystemMessage::ExportChainsDot),
        Action::ExportChainsMermaid => Message::System(SystemMessage::ExportChainsMermaid),
        Action::ExportReportMarkdown => Message::System(SystemMessage::ExportReportMarkdown),
        Action::ExportReportHtml => Message::System(SystemMessage::ExportReportHtml),
        Action::ImportCsv => Message::System(SystemMessage::StartImportCsv),
        Action::ImportIcs => Message::System(SystemMessage::StartImportIcs),
        Action::RecordMacro => Message::Ui(UiMessage::StartRecordMacro),
        Action::StopRecordMacro => Message::Ui(UiMessage::StopRecordMacro),
        Action::PlayMacro0 => Message::Ui(UiMessage::PlayMacro(0)),
        Action::PlayMacro1 => Message::Ui(UiMessage::PlayMacro(1)),
        Action::PlayMacro2 => Message::Ui(UiMessage::PlayMacro(2)),
        Action::PlayMacro3 => Message::Ui(UiMessage::PlayMacro(3)),
        Action::PlayMacro4 => Message::Ui(UiMessage::PlayMacro(4)),
        Action::PlayMacro5 => Message::Ui(UiMessage::PlayMacro(5)),
        Action::PlayMacro6 => Message::Ui(UiMessage::PlayMacro(6)),
        Action::PlayMacro7 => Message::Ui(UiMessage::PlayMacro(7)),
        Action::PlayMacro8 => Message::Ui(UiMessage::PlayMacro(8)),
        Action::PlayMacro9 => Message::Ui(UiMessage::PlayMacro(9)),
        Action::ShowTemplates => Message::Ui(UiMessage::ShowTemplates),
        Action::ToggleFocusMode => Message::Ui(UiMessage::ToggleFocusMode),
        Action::ToggleFullScreenFocus => Message::Ui(UiMessage::ToggleFullScreenFocus),
        Action::AddToFocusQueue => Message::Ui(UiMessage::AddToFocusQueue),
        Action::ClearFocusQueue => Message::Ui(UiMessage::ClearFocusQueue),
        Action::AdvanceFocusQueue => Message::Ui(UiMessage::AdvanceFocusQueue),
        Action::ShowKeybindingsEditor => Message::Ui(UiMessage::ShowKeybindingsEditor),
        Action::RescheduleTomorrow => Message::Ui(UiMessage::RescheduleTomorrow),
        Action::RescheduleNextWeek => Message::Ui(UiMessage::RescheduleNextWeek),
        Action::RescheduleNextMonday => Message::Ui(UiMessage::RescheduleNextMonday),
        Action::SnoozeTask => Message::Ui(UiMessage::StartSnoozeTask),
        Action::ClearSnooze => Message::Ui(UiMessage::ClearSnooze),
        Action::ReportsNextPanel => Message::Navigation(NavigationMessage::ReportsNextPanel),
        Action::ReportsPrevPanel => Message::Navigation(NavigationMessage::ReportsPrevPanel),
        Action::PomodoroStart => Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
        Action::PomodoroPause => Message::Pomodoro(PomodoroMessage::Pause),
        Action::PomodoroResume => Message::Pomodoro(PomodoroMessage::Resume),
        Action::PomodoroTogglePause => Message::Pomodoro(PomodoroMessage::TogglePause),
        Action::PomodoroSkip => Message::Pomodoro(PomodoroMessage::Skip),
        Action::PomodoroStop => Message::Pomodoro(PomodoroMessage::Stop),
        Action::RefreshStorage => Message::System(SystemMessage::RefreshStorage),
        // Habits
        Action::CreateHabit => Message::Ui(UiMessage::StartCreateHabit),
        Action::EditHabit => {
            // Edit the selected habit (need to get the ID from model)
            // This requires special handling in handle_key_event
            Message::None
        }
        Action::DeleteHabit => Message::Ui(UiMessage::HabitDelete),
        Action::ToggleHabitToday => Message::Ui(UiMessage::HabitToggleToday),
        Action::ShowHabitAnalytics => Message::Ui(UiMessage::ShowHabitAnalytics),
        Action::HabitToggleShowArchived => Message::Ui(UiMessage::HabitToggleShowArchived),
        Action::HabitArchive => Message::Ui(UiMessage::HabitArchive),
        // Burndown
        Action::BurndownCycleWindow => Message::Ui(UiMessage::BurndownCycleWindow),
        Action::BurndownToggleMode => Message::Ui(UiMessage::BurndownToggleMode),
        Action::BurndownToggleScopeCreep => Message::Ui(UiMessage::BurndownToggleScopeCreep),
        // Duplicates
        Action::DismissDuplicate => Message::Ui(UiMessage::DismissDuplicate),
        Action::MergeDuplicates => Message::Ui(UiMessage::MergeDuplicates),
        Action::RefreshDuplicates => Message::Ui(UiMessage::RefreshDuplicates),
        // Git integration
        Action::ViewGitTodos => Message::Navigation(NavigationMessage::GoToView(ViewId::GitTodos)),
        Action::ScanGitTodos => Message::System(SystemMessage::ScanGitTodos),
        Action::OpenInEditor => Message::Ui(UiMessage::OpenInEditor),
        // Reviews
        Action::ShowDailyReview => Message::Ui(UiMessage::ShowDailyReview),
        Action::ShowWeeklyReview => Message::Ui(UiMessage::ShowWeeklyReview),
        Action::ShowEveningReview => Message::Ui(UiMessage::ShowEveningReview),
        // Task detail
        Action::ShowTaskDetail => Message::Ui(UiMessage::ShowTaskDetail),
        // Command palette
        Action::ShowCommandPalette => Message::Ui(UiMessage::ShowCommandPalette),
        // Quick-open fuzzy task picker
        Action::ShowQuickOpen => Message::Ui(UiMessage::ShowQuickOpen),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;
    use taskflow::app::{TaskMessage, TimeMessage};
    use taskflow::config::ALL_ACTIONS;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_with(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    // ========================================================================
    // key_event_to_string
    // ========================================================================

    #[test]
    fn plain_characters_map_to_themselves() {
        assert_eq!(key_event_to_string(&key(KeyCode::Char('j'))), "j");
        assert_eq!(key_event_to_string(&key(KeyCode::Char('G'))), "G");
        assert_eq!(key_event_to_string(&key(KeyCode::Char('?'))), "?");
        assert_eq!(key_event_to_string(&key(KeyCode::Char('@'))), "@");
    }

    #[test]
    fn space_is_named() {
        assert_eq!(key_event_to_string(&key(KeyCode::Char(' '))), "space");
    }

    #[test]
    fn named_keys_use_lowercase_names() {
        let cases = [
            (KeyCode::Enter, "enter"),
            (KeyCode::Esc, "esc"),
            (KeyCode::Backspace, "backspace"),
            (KeyCode::Delete, "delete"),
            (KeyCode::Up, "up"),
            (KeyCode::Down, "down"),
            (KeyCode::Left, "left"),
            (KeyCode::Right, "right"),
            (KeyCode::Home, "home"),
            (KeyCode::End, "end"),
            (KeyCode::PageUp, "pageup"),
            (KeyCode::PageDown, "pagedown"),
            (KeyCode::Tab, "tab"),
            (KeyCode::F(5), "f5"),
            (KeyCode::F(12), "f12"),
        ];
        for (code, expected) in cases {
            assert_eq!(key_event_to_string(&key(code)), expected, "{code:?}");
        }
    }

    #[test]
    fn unsupported_keys_yield_empty_string() {
        assert_eq!(key_event_to_string(&key(KeyCode::BackTab)), "");
        assert_eq!(key_event_to_string(&key(KeyCode::Insert)), "");
        assert_eq!(key_event_to_string(&key(KeyCode::Null)), "");
        // Modifiers do not rescue an unsupported key
        assert_eq!(
            key_event_to_string(&key_with(KeyCode::Insert, KeyModifiers::CONTROL)),
            ""
        );
    }

    #[test]
    fn control_and_alt_prefixes() {
        assert_eq!(
            key_event_to_string(&key_with(KeyCode::Char('s'), KeyModifiers::CONTROL)),
            "ctrl+s"
        );
        assert_eq!(
            key_event_to_string(&key_with(KeyCode::Char('t'), KeyModifiers::ALT)),
            "alt+t"
        );
        assert_eq!(
            key_event_to_string(&key_with(
                KeyCode::Char('x'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )),
            "ctrl+alt+x"
        );
        assert_eq!(
            key_event_to_string(&key_with(KeyCode::Up, KeyModifiers::CONTROL)),
            "ctrl+up"
        );
        assert_eq!(
            key_event_to_string(&key_with(KeyCode::Char(' '), KeyModifiers::CONTROL)),
            "ctrl+space"
        );
    }

    #[test]
    fn shift_is_only_emitted_for_non_character_keys() {
        assert_eq!(
            key_event_to_string(&key_with(KeyCode::Tab, KeyModifiers::SHIFT)),
            "shift+tab"
        );
        assert_eq!(
            key_event_to_string(&key_with(
                KeyCode::Up,
                KeyModifiers::CONTROL | KeyModifiers::SHIFT
            )),
            "ctrl+shift+up"
        );
        // Terminals report shifted letters as the uppercase character with SHIFT set
        assert_eq!(
            key_event_to_string(&key_with(KeyCode::Char('J'), KeyModifiers::SHIFT)),
            "J"
        );
        assert_eq!(
            key_event_to_string(&key_with(KeyCode::Char('?'), KeyModifiers::SHIFT)),
            "?"
        );
    }

    // ========================================================================
    // action_to_message
    // ========================================================================

    macro_rules! assert_maps {
        ($action:expr, $pattern:pat) => {
            assert!(
                matches!(action_to_message(&$action), $pattern),
                "{:?} did not map to {}",
                $action,
                stringify!($pattern)
            );
        };
    }

    #[test]
    fn every_action_has_a_message_except_edit_habit() {
        for action in ALL_ACTIONS {
            let is_none = matches!(action_to_message(action), Message::None);
            assert_eq!(
                is_none,
                matches!(action, Action::EditHabit),
                "unexpected mapping for {action:?}"
            );
        }
    }

    #[test]
    fn navigation_actions() {
        assert_maps!(Action::MoveUp, Message::Navigation(NavigationMessage::Up));
        assert_maps!(
            Action::MoveDown,
            Message::Navigation(NavigationMessage::Down)
        );
        assert_maps!(
            Action::MoveFirst,
            Message::Navigation(NavigationMessage::First)
        );
        assert_maps!(
            Action::MoveLast,
            Message::Navigation(NavigationMessage::Last)
        );
        assert_maps!(
            Action::PageUp,
            Message::Navigation(NavigationMessage::PageUp)
        );
        assert_maps!(
            Action::PageDown,
            Message::Navigation(NavigationMessage::PageDown)
        );
        assert_maps!(
            Action::FocusSidebar,
            Message::Navigation(NavigationMessage::FocusSidebar)
        );
        assert_maps!(
            Action::FocusTaskList,
            Message::Navigation(NavigationMessage::FocusTaskList)
        );
        assert_maps!(
            Action::Select,
            Message::Navigation(NavigationMessage::SelectSidebarItem)
        );
        assert_maps!(
            Action::CalendarPrevMonth,
            Message::Navigation(NavigationMessage::CalendarPrevMonth)
        );
        assert_maps!(
            Action::CalendarNextMonth,
            Message::Navigation(NavigationMessage::CalendarNextMonth)
        );
        assert_maps!(
            Action::CalendarPrevDay,
            Message::Ui(UiMessage::CalendarPrevDay)
        );
        assert_maps!(
            Action::CalendarNextDay,
            Message::Ui(UiMessage::CalendarNextDay)
        );
        assert_maps!(
            Action::ReportsNextPanel,
            Message::Navigation(NavigationMessage::ReportsNextPanel)
        );
        assert_maps!(
            Action::ReportsPrevPanel,
            Message::Navigation(NavigationMessage::ReportsPrevPanel)
        );
        assert_maps!(
            Action::ViewGitTodos,
            Message::Navigation(NavigationMessage::GoToView(ViewId::GitTodos))
        );
    }

    #[test]
    fn task_actions() {
        assert_maps!(
            Action::ToggleComplete,
            Message::Task(TaskMessage::ToggleComplete)
        );
        assert_maps!(Action::CreateTask, Message::Ui(UiMessage::StartCreateTask));
        assert_maps!(
            Action::QuickCapture,
            Message::Ui(UiMessage::StartQuickCapture)
        );
        assert_maps!(
            Action::CreateSubtask,
            Message::Ui(UiMessage::StartCreateSubtask)
        );
        assert_maps!(
            Action::CreateProject,
            Message::Ui(UiMessage::StartCreateProject)
        );
        assert_maps!(
            Action::EditProject,
            Message::Ui(UiMessage::StartEditProject)
        );
        assert_maps!(Action::DeleteProject, Message::Ui(UiMessage::DeleteProject));
        assert_maps!(Action::EditTask, Message::Ui(UiMessage::StartEditTask));
        assert_maps!(
            Action::EditDueDate,
            Message::Ui(UiMessage::StartEditDueDate)
        );
        assert_maps!(
            Action::EditScheduledDate,
            Message::Ui(UiMessage::StartEditScheduledDate)
        );
        assert_maps!(
            Action::EditScheduledTime,
            Message::Ui(UiMessage::StartEditScheduledTime)
        );
        assert_maps!(Action::EditTags, Message::Ui(UiMessage::StartEditTags));
        assert_maps!(
            Action::EditDescription,
            Message::Ui(UiMessage::StartEditDescription)
        );
        assert_maps!(
            Action::EditDescriptionMultiline,
            Message::Ui(UiMessage::StartEditDescriptionMultiline)
        );
        assert_maps!(
            Action::DeleteTask,
            Message::Ui(UiMessage::ShowDeleteConfirm)
        );
        assert_maps!(Action::DuplicateTask, Message::Task(TaskMessage::Duplicate));
        assert_maps!(
            Action::CyclePriority,
            Message::Task(TaskMessage::CyclePriority)
        );
        assert_maps!(
            Action::MoveToProject,
            Message::Ui(UiMessage::StartMoveToProject)
        );
        assert_maps!(
            Action::EditDependencies,
            Message::Ui(UiMessage::StartEditDependencies)
        );
        assert_maps!(
            Action::EditRecurrence,
            Message::Ui(UiMessage::StartEditRecurrence)
        );
        assert_maps!(Action::MoveTaskUp, Message::Ui(UiMessage::MoveTaskUp));
        assert_maps!(Action::MoveTaskDown, Message::Ui(UiMessage::MoveTaskDown));
        assert_maps!(Action::LinkTask, Message::Ui(UiMessage::StartLinkTask));
        assert_maps!(Action::UnlinkTask, Message::Ui(UiMessage::UnlinkTask));
        assert_maps!(
            Action::RescheduleTomorrow,
            Message::Ui(UiMessage::RescheduleTomorrow)
        );
        assert_maps!(
            Action::RescheduleNextWeek,
            Message::Ui(UiMessage::RescheduleNextWeek)
        );
        assert_maps!(
            Action::RescheduleNextMonday,
            Message::Ui(UiMessage::RescheduleNextMonday)
        );
        assert_maps!(Action::SnoozeTask, Message::Ui(UiMessage::StartSnoozeTask));
        assert_maps!(Action::ClearSnooze, Message::Ui(UiMessage::ClearSnooze));
        assert_maps!(Action::OpenInEditor, Message::Ui(UiMessage::OpenInEditor));
        assert_maps!(
            Action::ShowTaskDetail,
            Message::Ui(UiMessage::ShowTaskDetail)
        );
    }

    #[test]
    fn time_tracking_actions() {
        assert_maps!(
            Action::ToggleTimeTracking,
            Message::Time(TimeMessage::ToggleTracking)
        );
        assert_maps!(Action::ShowTimeLog, Message::Ui(UiMessage::ShowTimeLog));
        assert_maps!(Action::ShowWorkLog, Message::Ui(UiMessage::ShowWorkLog));
        assert_maps!(
            Action::EditEstimate,
            Message::Ui(UiMessage::StartEditEstimate)
        );
    }

    #[test]
    fn ui_actions() {
        assert_maps!(Action::ToggleSidebar, Message::Ui(UiMessage::ToggleSidebar));
        assert_maps!(
            Action::ToggleShowCompleted,
            Message::Ui(UiMessage::ToggleShowCompleted)
        );
        assert_maps!(Action::ShowHelp, Message::Ui(UiMessage::ShowHelp));
        assert_maps!(Action::Search, Message::Ui(UiMessage::StartSearch));
        assert_maps!(Action::ClearSearch, Message::Ui(UiMessage::ClearSearch));
        assert_maps!(
            Action::FilterByTag,
            Message::Ui(UiMessage::StartFilterByTag)
        );
        assert_maps!(
            Action::ClearTagFilter,
            Message::Ui(UiMessage::ClearTagFilter)
        );
        assert_maps!(
            Action::CycleSortField,
            Message::Ui(UiMessage::CycleSortField)
        );
        assert_maps!(
            Action::ToggleSortOrder,
            Message::Ui(UiMessage::ToggleSortOrder)
        );
        assert_maps!(Action::ShowTemplates, Message::Ui(UiMessage::ShowTemplates));
        assert_maps!(
            Action::ToggleFocusMode,
            Message::Ui(UiMessage::ToggleFocusMode)
        );
        assert_maps!(
            Action::ToggleFullScreenFocus,
            Message::Ui(UiMessage::ToggleFullScreenFocus)
        );
        assert_maps!(
            Action::AddToFocusQueue,
            Message::Ui(UiMessage::AddToFocusQueue)
        );
        assert_maps!(
            Action::ClearFocusQueue,
            Message::Ui(UiMessage::ClearFocusQueue)
        );
        assert_maps!(
            Action::AdvanceFocusQueue,
            Message::Ui(UiMessage::AdvanceFocusQueue)
        );
        assert_maps!(
            Action::ShowKeybindingsEditor,
            Message::Ui(UiMessage::ShowKeybindingsEditor)
        );
        assert_maps!(
            Action::ShowDailyReview,
            Message::Ui(UiMessage::ShowDailyReview)
        );
        assert_maps!(
            Action::ShowWeeklyReview,
            Message::Ui(UiMessage::ShowWeeklyReview)
        );
        assert_maps!(
            Action::ShowEveningReview,
            Message::Ui(UiMessage::ShowEveningReview)
        );
        assert_maps!(
            Action::ShowCommandPalette,
            Message::Ui(UiMessage::ShowCommandPalette)
        );
    }

    #[test]
    fn multi_select_actions() {
        assert_maps!(
            Action::ToggleMultiSelect,
            Message::Ui(UiMessage::ToggleMultiSelect)
        );
        assert_maps!(
            Action::ToggleTaskSelection,
            Message::Ui(UiMessage::ToggleTaskSelection)
        );
        assert_maps!(Action::SelectAll, Message::Ui(UiMessage::SelectAll));
        assert_maps!(
            Action::ClearSelection,
            Message::Ui(UiMessage::ClearSelection)
        );
        assert_maps!(Action::BulkDelete, Message::Ui(UiMessage::BulkDelete));
        assert_maps!(
            Action::BulkMoveToProject,
            Message::Ui(UiMessage::StartBulkMoveToProject)
        );
        assert_maps!(
            Action::BulkSetStatus,
            Message::Ui(UiMessage::StartBulkSetStatus)
        );
    }

    #[test]
    fn system_import_export_actions() {
        assert_maps!(Action::Save, Message::System(SystemMessage::Save));
        assert_maps!(Action::Undo, Message::System(SystemMessage::Undo));
        assert_maps!(Action::Redo, Message::System(SystemMessage::Redo));
        assert_maps!(Action::Quit, Message::System(SystemMessage::Quit));
        assert_maps!(
            Action::RefreshStorage,
            Message::System(SystemMessage::RefreshStorage)
        );
        assert_maps!(Action::ExportCsv, Message::System(SystemMessage::ExportCsv));
        assert_maps!(Action::ExportIcs, Message::System(SystemMessage::ExportIcs));
        assert_maps!(
            Action::ExportChainsDot,
            Message::System(SystemMessage::ExportChainsDot)
        );
        assert_maps!(
            Action::ExportChainsMermaid,
            Message::System(SystemMessage::ExportChainsMermaid)
        );
        assert_maps!(
            Action::ExportReportMarkdown,
            Message::System(SystemMessage::ExportReportMarkdown)
        );
        assert_maps!(
            Action::ExportReportHtml,
            Message::System(SystemMessage::ExportReportHtml)
        );
        assert_maps!(
            Action::ImportCsv,
            Message::System(SystemMessage::StartImportCsv)
        );
        assert_maps!(
            Action::ImportIcs,
            Message::System(SystemMessage::StartImportIcs)
        );
        assert_maps!(
            Action::ScanGitTodos,
            Message::System(SystemMessage::ScanGitTodos)
        );
    }

    #[test]
    fn macro_actions() {
        assert_maps!(
            Action::RecordMacro,
            Message::Ui(UiMessage::StartRecordMacro)
        );
        assert_maps!(
            Action::StopRecordMacro,
            Message::Ui(UiMessage::StopRecordMacro)
        );
        assert_maps!(Action::PlayMacro0, Message::Ui(UiMessage::PlayMacro(0)));
        assert_maps!(Action::PlayMacro1, Message::Ui(UiMessage::PlayMacro(1)));
        assert_maps!(Action::PlayMacro2, Message::Ui(UiMessage::PlayMacro(2)));
        assert_maps!(Action::PlayMacro3, Message::Ui(UiMessage::PlayMacro(3)));
        assert_maps!(Action::PlayMacro4, Message::Ui(UiMessage::PlayMacro(4)));
        assert_maps!(Action::PlayMacro5, Message::Ui(UiMessage::PlayMacro(5)));
        assert_maps!(Action::PlayMacro6, Message::Ui(UiMessage::PlayMacro(6)));
        assert_maps!(Action::PlayMacro7, Message::Ui(UiMessage::PlayMacro(7)));
        assert_maps!(Action::PlayMacro8, Message::Ui(UiMessage::PlayMacro(8)));
        assert_maps!(Action::PlayMacro9, Message::Ui(UiMessage::PlayMacro(9)));
    }

    #[test]
    fn pomodoro_actions() {
        assert_maps!(
            Action::PomodoroStart,
            Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 })
        );
        assert_maps!(
            Action::PomodoroPause,
            Message::Pomodoro(PomodoroMessage::Pause)
        );
        assert_maps!(
            Action::PomodoroResume,
            Message::Pomodoro(PomodoroMessage::Resume)
        );
        assert_maps!(
            Action::PomodoroTogglePause,
            Message::Pomodoro(PomodoroMessage::TogglePause)
        );
        assert_maps!(
            Action::PomodoroSkip,
            Message::Pomodoro(PomodoroMessage::Skip)
        );
        assert_maps!(
            Action::PomodoroStop,
            Message::Pomodoro(PomodoroMessage::Stop)
        );
    }

    #[test]
    fn habit_burndown_duplicate_actions() {
        assert_maps!(
            Action::CreateHabit,
            Message::Ui(UiMessage::StartCreateHabit)
        );
        assert_maps!(Action::EditHabit, Message::None);
        assert_maps!(Action::DeleteHabit, Message::Ui(UiMessage::HabitDelete));
        assert_maps!(
            Action::ToggleHabitToday,
            Message::Ui(UiMessage::HabitToggleToday)
        );
        assert_maps!(
            Action::ShowHabitAnalytics,
            Message::Ui(UiMessage::ShowHabitAnalytics)
        );
        assert_maps!(
            Action::HabitToggleShowArchived,
            Message::Ui(UiMessage::HabitToggleShowArchived)
        );
        assert_maps!(Action::HabitArchive, Message::Ui(UiMessage::HabitArchive));
        assert_maps!(
            Action::BurndownCycleWindow,
            Message::Ui(UiMessage::BurndownCycleWindow)
        );
        assert_maps!(
            Action::BurndownToggleMode,
            Message::Ui(UiMessage::BurndownToggleMode)
        );
        assert_maps!(
            Action::BurndownToggleScopeCreep,
            Message::Ui(UiMessage::BurndownToggleScopeCreep)
        );
        assert_maps!(
            Action::DismissDuplicate,
            Message::Ui(UiMessage::DismissDuplicate)
        );
        assert_maps!(
            Action::MergeDuplicates,
            Message::Ui(UiMessage::MergeDuplicates)
        );
        assert_maps!(
            Action::RefreshDuplicates,
            Message::Ui(UiMessage::RefreshDuplicates)
        );
    }
}
