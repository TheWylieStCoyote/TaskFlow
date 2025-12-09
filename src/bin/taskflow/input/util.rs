//! Utility functions for input handling.

use crossterm::event::{self, KeyCode, KeyModifiers};

use taskflow::app::{Message, NavigationMessage, PomodoroMessage, SystemMessage, UiMessage};
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
        Action::EditTags => Message::Ui(UiMessage::StartEditTags),
        Action::EditDescription => Message::Ui(UiMessage::StartEditDescription),
        Action::EditDescriptionMultiline => Message::Ui(UiMessage::StartEditDescriptionMultiline),
        Action::DeleteTask => Message::Ui(UiMessage::ShowDeleteConfirm),
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
        Action::ShowKeybindingsEditor => Message::Ui(UiMessage::ShowKeybindingsEditor),
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
    }
}
