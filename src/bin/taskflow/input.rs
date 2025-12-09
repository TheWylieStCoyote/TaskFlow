//! Key event handling.

use crossterm::event::{self, KeyCode, KeyModifiers};

use taskflow::app::{Message, Model, NavigationMessage, PomodoroMessage, SystemMessage, UiMessage};
use taskflow::config::{Action, Keybindings};
use taskflow::ui::InputMode;

/// Handle a key event and return the appropriate message.
pub fn handle_key_event(
    key: event::KeyEvent,
    model: &mut Model,
    keybindings: &Keybindings,
) -> Message {
    // Handle delete confirmation dialog first
    if model.show_confirm_delete {
        return match key.code {
            KeyCode::Char('y' | 'Y') => Message::Ui(UiMessage::ConfirmDelete),
            KeyCode::Char('n' | 'N') | KeyCode::Esc => Message::Ui(UiMessage::CancelDelete),
            _ => Message::None,
        };
    }

    // Handle import preview dialog
    if model.show_import_preview {
        return match key.code {
            KeyCode::Enter | KeyCode::Char('y' | 'Y') => {
                Message::System(SystemMessage::ConfirmImport)
            }
            KeyCode::Esc | KeyCode::Char('n' | 'N') => Message::System(SystemMessage::CancelImport),
            _ => Message::None,
        };
    }

    // Handle input mode
    if model.input_mode == InputMode::Editing {
        return match key.code {
            KeyCode::Enter => Message::Ui(UiMessage::SubmitInput),
            KeyCode::Esc => Message::Ui(UiMessage::CancelInput),
            KeyCode::Backspace => Message::Ui(UiMessage::InputBackspace),
            KeyCode::Delete => Message::Ui(UiMessage::InputDelete),
            KeyCode::Left => Message::Ui(UiMessage::InputCursorLeft),
            KeyCode::Right => Message::Ui(UiMessage::InputCursorRight),
            KeyCode::Home => Message::Ui(UiMessage::InputCursorStart),
            KeyCode::End => Message::Ui(UiMessage::InputCursorEnd),
            KeyCode::Char(c) => Message::Ui(UiMessage::InputChar(c)),
            _ => Message::None,
        };
    }

    // If storage error alert is showing, any key dismisses it
    if model.show_storage_error_alert {
        return Message::Ui(UiMessage::DismissStorageErrorAlert);
    }

    // If overdue alert is showing, any key dismisses it
    if model.show_overdue_alert {
        return Message::Ui(UiMessage::DismissOverdueAlert);
    }

    // If help is showing, any key closes it
    if model.show_help {
        return Message::Ui(UiMessage::HideHelp);
    }

    // If focus mode is active, Esc exits it
    if model.focus_mode && key.code == KeyCode::Esc {
        return Message::Ui(UiMessage::ToggleFocusMode);
    }

    // If template picker is showing, handle navigation and selection
    if model.show_templates {
        return handle_template_picker(key, model);
    }

    // If keybindings editor is showing, handle navigation and editing
    if model.show_keybindings_editor {
        return handle_keybindings_editor(key, model);
    }

    // If time log editor is showing, handle navigation and editing
    if model.show_time_log {
        return handle_time_log(key, model);
    }

    // If work log editor is showing, handle navigation and editing
    if model.show_work_log {
        return handle_work_log(key, model);
    }

    // If description editor is showing, handle multi-line input
    if model.show_description_editor {
        return handle_description_editor(key);
    }

    // In multi-select mode, Space toggles task selection
    if model.multi_select_mode && key.code == KeyCode::Char(' ') {
        return Message::Ui(UiMessage::ToggleTaskSelection);
    }

    // In calendar view, handle focus switching and navigation
    if model.current_view == taskflow::app::ViewId::Calendar
        && model.focus_pane == taskflow::app::FocusPane::TaskList
    {
        if let Some(msg) = handle_calendar_view(key, model) {
            return msg;
        }
    }

    // In Habits view, handle habit-specific actions
    if model.current_view == taskflow::app::ViewId::Habits {
        if let Some(msg) = handle_habits_view(key, model) {
            return msg;
        }
    }

    // In Timeline view, handle timeline-specific actions
    if model.current_view == taskflow::app::ViewId::Timeline {
        if let Some(msg) = handle_timeline_view(key) {
            return msg;
        }
    }

    // In Kanban view, handle column navigation
    if model.current_view == taskflow::app::ViewId::Kanban {
        if let Some(msg) = handle_kanban_view(key) {
            return msg;
        }
    }

    // In Eisenhower view, handle quadrant navigation
    if model.current_view == taskflow::app::ViewId::Eisenhower {
        if let Some(msg) = handle_eisenhower_view(key) {
            return msg;
        }
    }

    // In WeeklyPlanner view, handle day navigation
    if model.current_view == taskflow::app::ViewId::WeeklyPlanner {
        if let Some(msg) = handle_weekly_planner_view(key) {
            return msg;
        }
    }

    // In Reports view, handle exit
    if model.current_view == taskflow::app::ViewId::Reports {
        if let Some(msg) = handle_reports_view(key) {
            return msg;
        }
    }

    // If habit analytics is showing, handle it
    if model.show_habit_analytics {
        return match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('a') => {
                Message::Ui(UiMessage::HideHabitAnalytics)
            }
            _ => Message::None,
        };
    }

    // Handle macro slot selection if pending
    if model.pending_macro_slot.is_some() {
        return handle_macro_slot(key, model);
    }

    // Convert key event to string for lookup
    let key_str = key_event_to_string(&key);

    // Look up action in keybindings
    if let Some(action) = keybindings.get_action(&key_str) {
        return action_to_message(action);
    }

    Message::None
}

fn handle_template_picker(key: event::KeyEvent, model: &mut Model) -> Message {
    match key.code {
        KeyCode::Esc => Message::Ui(UiMessage::HideTemplates),
        KeyCode::Enter => Message::Ui(UiMessage::SelectTemplate(model.template_selected)),
        KeyCode::Up | KeyCode::Char('k') => {
            if model.template_selected > 0 {
                model.template_selected -= 1;
            }
            Message::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let max = model.template_manager.len().saturating_sub(1);
            if model.template_selected < max {
                model.template_selected += 1;
            }
            Message::None
        }
        KeyCode::Char(c) if c.is_ascii_digit() => {
            if let Some(digit) = c.to_digit(10) {
                let index = digit as usize;
                if index < model.template_manager.len() {
                    Message::Ui(UiMessage::SelectTemplate(index))
                } else {
                    Message::None
                }
            } else {
                Message::None
            }
        }
        _ => Message::None,
    }
}

fn handle_keybindings_editor(key: event::KeyEvent, model: &mut Model) -> Message {
    // If capturing a key, any key except Esc sets the keybinding
    if model.keybinding_capturing {
        return match key.code {
            KeyCode::Esc => Message::Ui(UiMessage::CancelEditKeybinding),
            _ => {
                let key_str = key_event_to_string(&key);
                Message::Ui(UiMessage::ApplyKeybinding(key_str))
            }
        };
    }

    // Normal keybindings editor navigation
    match key.code {
        KeyCode::Esc => Message::Ui(UiMessage::HideKeybindingsEditor),
        KeyCode::Enter => Message::Ui(UiMessage::StartEditKeybinding),
        KeyCode::Up | KeyCode::Char('k') => Message::Ui(UiMessage::KeybindingsUp),
        KeyCode::Down | KeyCode::Char('j') => Message::Ui(UiMessage::KeybindingsDown),
        KeyCode::Char('r') => Message::Ui(UiMessage::ResetKeybinding),
        KeyCode::Char('R') => Message::Ui(UiMessage::ResetAllKeybindings),
        KeyCode::Char('s') => Message::Ui(UiMessage::SaveKeybindings),
        _ => Message::None,
    }
}

fn handle_time_log(key: event::KeyEvent, model: &Model) -> Message {
    use taskflow::ui::TimeLogMode;

    match model.time_log_mode {
        TimeLogMode::EditStart | TimeLogMode::EditEnd => {
            // Editing time - handle character input
            match key.code {
                KeyCode::Esc => Message::Ui(UiMessage::TimeLogCancel),
                KeyCode::Enter => Message::Ui(UiMessage::TimeLogSubmit),
                KeyCode::Backspace => Message::Ui(UiMessage::InputBackspace),
                KeyCode::Char(c) => Message::Ui(UiMessage::InputChar(c)),
                _ => Message::None,
            }
        }
        TimeLogMode::ConfirmDelete => {
            // Confirm delete mode
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => Message::Ui(UiMessage::TimeLogDelete),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    Message::Ui(UiMessage::TimeLogCancel)
                }
                _ => Message::None,
            }
        }
        TimeLogMode::Browse => {
            // Normal time log navigation
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => Message::Ui(UiMessage::HideTimeLog),
                KeyCode::Up | KeyCode::Char('k') => Message::Ui(UiMessage::TimeLogUp),
                KeyCode::Down | KeyCode::Char('j') => Message::Ui(UiMessage::TimeLogDown),
                KeyCode::Char('s') => Message::Ui(UiMessage::TimeLogEditStart),
                KeyCode::Char('e') => Message::Ui(UiMessage::TimeLogEditEnd),
                KeyCode::Char('d') => Message::Ui(UiMessage::TimeLogConfirmDelete),
                KeyCode::Char('a') => Message::Ui(UiMessage::TimeLogAddEntry),
                _ => Message::None,
            }
        }
    }
}

fn handle_work_log(key: event::KeyEvent, model: &Model) -> Message {
    use taskflow::ui::WorkLogMode;

    match model.work_log_mode {
        WorkLogMode::Add | WorkLogMode::Edit => {
            // Multi-line editing mode - handle character input
            // Check for Ctrl+S to save first
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
                return Message::Ui(UiMessage::WorkLogSubmit);
            }
            match key.code {
                KeyCode::Esc => Message::Ui(UiMessage::WorkLogCancel),
                KeyCode::Enter => Message::Ui(UiMessage::WorkLogNewline),
                KeyCode::Backspace => Message::Ui(UiMessage::WorkLogInputBackspace),
                KeyCode::Delete => Message::Ui(UiMessage::WorkLogInputDelete),
                KeyCode::Left => Message::Ui(UiMessage::WorkLogCursorLeft),
                KeyCode::Right => Message::Ui(UiMessage::WorkLogCursorRight),
                KeyCode::Up => Message::Ui(UiMessage::WorkLogCursorUp),
                KeyCode::Down => Message::Ui(UiMessage::WorkLogCursorDown),
                KeyCode::Home => Message::Ui(UiMessage::WorkLogCursorHome),
                KeyCode::End => Message::Ui(UiMessage::WorkLogCursorEnd),
                KeyCode::Char(c) => Message::Ui(UiMessage::WorkLogInputChar(c)),
                _ => Message::None,
            }
        }
        WorkLogMode::ConfirmDelete => {
            // Confirm delete mode
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => Message::Ui(UiMessage::WorkLogDelete),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    Message::Ui(UiMessage::WorkLogCancel)
                }
                _ => Message::None,
            }
        }
        WorkLogMode::View => {
            // Viewing a single entry
            match key.code {
                KeyCode::Esc | KeyCode::Enter => Message::Ui(UiMessage::WorkLogCancel),
                KeyCode::Char('e') => Message::Ui(UiMessage::WorkLogEdit),
                KeyCode::Char('d') => Message::Ui(UiMessage::WorkLogConfirmDelete),
                _ => Message::None,
            }
        }
        WorkLogMode::Browse => {
            // Normal work log navigation
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => Message::Ui(UiMessage::HideWorkLog),
                KeyCode::Up | KeyCode::Char('k') => Message::Ui(UiMessage::WorkLogUp),
                KeyCode::Down | KeyCode::Char('j') => Message::Ui(UiMessage::WorkLogDown),
                KeyCode::Enter => Message::Ui(UiMessage::WorkLogView),
                KeyCode::Char('a') => Message::Ui(UiMessage::WorkLogAdd),
                KeyCode::Char('e') => Message::Ui(UiMessage::WorkLogEdit),
                KeyCode::Char('d') => Message::Ui(UiMessage::WorkLogConfirmDelete),
                _ => Message::None,
            }
        }
    }
}

fn handle_description_editor(key: event::KeyEvent) -> Message {
    // Check for Ctrl+S to save first
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        return Message::Ui(UiMessage::DescriptionSubmit);
    }
    match key.code {
        KeyCode::Esc => Message::Ui(UiMessage::HideDescriptionEditor),
        KeyCode::Enter => Message::Ui(UiMessage::DescriptionNewline),
        KeyCode::Backspace => Message::Ui(UiMessage::DescriptionInputBackspace),
        KeyCode::Delete => Message::Ui(UiMessage::DescriptionInputDelete),
        KeyCode::Left => Message::Ui(UiMessage::DescriptionCursorLeft),
        KeyCode::Right => Message::Ui(UiMessage::DescriptionCursorRight),
        KeyCode::Up => Message::Ui(UiMessage::DescriptionCursorUp),
        KeyCode::Down => Message::Ui(UiMessage::DescriptionCursorDown),
        KeyCode::Home => Message::Ui(UiMessage::DescriptionCursorHome),
        KeyCode::End => Message::Ui(UiMessage::DescriptionCursorEnd),
        KeyCode::Char(c) => Message::Ui(UiMessage::DescriptionInputChar(c)),
        _ => Message::None,
    }
}

fn handle_calendar_view(key: event::KeyEvent, model: &mut Model) -> Option<Message> {
    // Esc exits to task list
    if key.code == KeyCode::Esc {
        return Some(Message::Navigation(NavigationMessage::GoToView(
            taskflow::app::ViewId::TaskList,
        )));
    }

    // Tab toggles focus between calendar grid and task list
    if key.code == KeyCode::Tab {
        return Some(if model.calendar_state.focus_task_list {
            Message::Navigation(NavigationMessage::CalendarFocusGrid)
        } else {
            Message::Navigation(NavigationMessage::CalendarFocusTaskList)
        });
    }

    if model.calendar_state.focus_task_list {
        // When focused on task list, h goes back to calendar grid
        match key.code {
            KeyCode::Char('h') | KeyCode::Left => {
                return Some(Message::Navigation(NavigationMessage::CalendarFocusGrid));
            }
            _ => {}
        }
    } else {
        // When focused on calendar grid, navigate days
        match key.code {
            KeyCode::Left => return Some(Message::Ui(UiMessage::CalendarPrevDay)),
            KeyCode::Right => return Some(Message::Ui(UiMessage::CalendarNextDay)),
            KeyCode::Char('h') => return Some(Message::Ui(UiMessage::CalendarPrevDay)),
            KeyCode::Char('l') => {
                // l moves to task list if there are tasks, otherwise next day
                if !model.tasks_for_selected_day().is_empty() {
                    return Some(Message::Navigation(
                        NavigationMessage::CalendarFocusTaskList,
                    ));
                }
                return Some(Message::Ui(UiMessage::CalendarNextDay));
            }
            _ => {}
        }
    }

    None
}

fn handle_habits_view(key: event::KeyEvent, model: &Model) -> Option<Message> {
    match key.code {
        // Exit to task list
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::GoToView(
            taskflow::app::ViewId::TaskList,
        ))),
        // Navigation
        KeyCode::Up | KeyCode::Char('k') => Some(Message::Ui(UiMessage::HabitUp)),
        KeyCode::Down | KeyCode::Char('j') => Some(Message::Ui(UiMessage::HabitDown)),
        // Create new habit
        KeyCode::Char('n') => Some(Message::Ui(UiMessage::StartCreateHabit)),
        // Edit selected habit
        KeyCode::Char('e') => {
            if let Some(&habit_id) = model.visible_habits.get(model.habit_selected) {
                Some(Message::Ui(UiMessage::StartEditHabit(habit_id)))
            } else {
                None
            }
        }
        // Delete selected habit
        KeyCode::Char('d') => Some(Message::Ui(UiMessage::HabitDelete)),
        // Toggle today's check-in
        KeyCode::Char(' ') | KeyCode::Char('x') => Some(Message::Ui(UiMessage::HabitToggleToday)),
        // Show analytics
        KeyCode::Char('a') => Some(Message::Ui(UiMessage::ShowHabitAnalytics)),
        // Archive habit
        KeyCode::Char('A') => Some(Message::Ui(UiMessage::HabitArchive)),
        // Toggle showing archived habits
        KeyCode::Char('H') => Some(Message::Ui(UiMessage::HabitToggleShowArchived)),
        _ => None,
    }
}

fn handle_timeline_view(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        // Exit timeline view
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::GoToView(
            taskflow::app::ViewId::TaskList,
        ))),
        // Scroll time axis
        KeyCode::Char('h') | KeyCode::Left => {
            Some(Message::Navigation(NavigationMessage::TimelineScrollLeft))
        }
        KeyCode::Char('l') | KeyCode::Right => {
            Some(Message::Navigation(NavigationMessage::TimelineScrollRight))
        }
        // Navigate tasks
        KeyCode::Up | KeyCode::Char('k') => {
            Some(Message::Navigation(NavigationMessage::TimelineUp))
        }
        KeyCode::Down | KeyCode::Char('j') => {
            Some(Message::Navigation(NavigationMessage::TimelineDown))
        }
        // Zoom controls
        KeyCode::Char('<') | KeyCode::Char(',') => {
            Some(Message::Navigation(NavigationMessage::TimelineZoomOut))
        }
        KeyCode::Char('>') | KeyCode::Char('.') => {
            Some(Message::Navigation(NavigationMessage::TimelineZoomIn))
        }
        // Jump to today
        KeyCode::Char('t') => Some(Message::Navigation(NavigationMessage::TimelineGoToday)),
        // Toggle dependency lines
        KeyCode::Char('d') => Some(Message::Ui(UiMessage::TimelineToggleDependencies)),
        // View task details (focus mode)
        KeyCode::Enter => Some(Message::Ui(UiMessage::TimelineViewSelected)),
        _ => None,
    }
}

fn handle_macro_slot(key: event::KeyEvent, model: &mut Model) -> Message {
    if let KeyCode::Char(c) = key.code {
        if let Some(digit) = c.to_digit(10) {
            let slot = digit as usize;
            model.pending_macro_slot = Some(slot);
            if model.macro_state.is_recording() {
                // Stop recording and save to this slot
                return Message::Ui(UiMessage::StopRecordMacro);
            }
            // Start recording to this slot
            return Message::Ui(UiMessage::StartRecordMacro);
        }
    }
    // Escape cancels macro slot selection
    if key.code == KeyCode::Esc {
        model.pending_macro_slot = None;
        model.status_message = Some("Macro cancelled".to_string());
        return Message::None;
    }
    Message::None
}

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

fn handle_kanban_view(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::GoToView(
            taskflow::app::ViewId::TaskList,
        ))),
        KeyCode::Char('h') | KeyCode::Left => {
            Some(Message::Navigation(NavigationMessage::KanbanLeft))
        }
        KeyCode::Char('l') | KeyCode::Right => {
            Some(Message::Navigation(NavigationMessage::KanbanRight))
        }
        _ => None,
    }
}

fn handle_eisenhower_view(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::GoToView(
            taskflow::app::ViewId::TaskList,
        ))),
        KeyCode::Char('h') | KeyCode::Left => {
            Some(Message::Navigation(NavigationMessage::EisenhowerLeft))
        }
        KeyCode::Char('l') | KeyCode::Right => {
            Some(Message::Navigation(NavigationMessage::EisenhowerRight))
        }
        KeyCode::Char('k') | KeyCode::Up => {
            Some(Message::Navigation(NavigationMessage::EisenhowerUp))
        }
        KeyCode::Char('j') | KeyCode::Down => {
            Some(Message::Navigation(NavigationMessage::EisenhowerDown))
        }
        _ => None,
    }
}

fn handle_weekly_planner_view(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::GoToView(
            taskflow::app::ViewId::TaskList,
        ))),
        KeyCode::Char('h') | KeyCode::Left => {
            Some(Message::Navigation(NavigationMessage::WeeklyPlannerLeft))
        }
        KeyCode::Char('l') | KeyCode::Right => {
            Some(Message::Navigation(NavigationMessage::WeeklyPlannerRight))
        }
        _ => None,
    }
}

fn handle_reports_view(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::GoToView(
            taskflow::app::ViewId::TaskList,
        ))),
        KeyCode::Tab => Some(Message::Navigation(NavigationMessage::ReportsNextPanel)),
        KeyCode::BackTab => Some(Message::Navigation(NavigationMessage::ReportsPrevPanel)),
        _ => None,
    }
}
