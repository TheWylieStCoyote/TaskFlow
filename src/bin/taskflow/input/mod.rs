//! Input event handling.
//!
//! This module provides centralized input handling for the TaskFlow TUI,
//! including keyboard and mouse events.

mod handlers;
mod mouse;
mod util;

#[cfg(test)]
mod tests;

use crossterm::event::{self, KeyCode};

use taskflow::app::{
    FocusPane, Message, Model, NavigationMessage, SystemMessage, UiMessage, ViewId,
};
use taskflow::config::Keybindings;
use taskflow::ui::InputMode;

pub use handlers::{
    handle_calendar_view, handle_daily_review, handle_description_editor, handle_duplicates_view,
    handle_eisenhower_view, handle_evening_review, handle_goals_view, handle_habits_view,
    handle_kanban_view, handle_keybindings_editor, handle_macro_slot, handle_network_view,
    handle_reports_view, handle_task_detail, handle_template_picker, handle_time_log,
    handle_timeline_view, handle_weekly_planner_view, handle_weekly_review, handle_work_log,
};
pub use mouse::handle_mouse_event;
pub use util::{action_to_message, key_event_to_string};

/// Handle a key event and return the appropriate message.
pub fn handle_key_event(
    key: event::KeyEvent,
    model: &mut Model,
    keybindings: &Keybindings,
) -> Message {
    // Handle command palette first (highest priority when visible)
    if model.command_palette.visible {
        return handle_command_palette_input(key, model as &mut Model);
    }

    // Handle delete confirmation dialog first
    if model.show_confirm_delete {
        return match key.code {
            KeyCode::Char('y' | 'Y') => Message::Ui(UiMessage::ConfirmDelete),
            KeyCode::Char('n' | 'N') | KeyCode::Esc => Message::Ui(UiMessage::CancelDelete),
            _ => Message::None,
        };
    }

    // Handle config file generation prompt
    if model.show_generate_config_prompt {
        return match key.code {
            KeyCode::Char('y' | 'Y') => Message::Ui(UiMessage::ConfirmGenerateConfig),
            KeyCode::Char('n' | 'N') | KeyCode::Esc => Message::Ui(UiMessage::CancelGenerateConfig),
            _ => Message::None,
        };
    }

    // Handle import preview dialog
    if model.import.show_preview {
        return match key.code {
            KeyCode::Enter | KeyCode::Char('y' | 'Y') => {
                Message::System(SystemMessage::ConfirmImport)
            }
            KeyCode::Esc | KeyCode::Char('n' | 'N') => Message::System(SystemMessage::CancelImport),
            _ => Message::None,
        };
    }

    // Handle input mode
    if model.input.mode == InputMode::Editing {
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
    if model.alerts.show_storage_error {
        return Message::Ui(UiMessage::DismissStorageErrorAlert);
    }

    // If overdue alert is showing, any key dismisses it
    if model.alerts.show_overdue {
        return Message::Ui(UiMessage::DismissOverdueAlert);
    }

    // If help is showing, scroll with j/k/arrows/page keys; any other key closes it
    if model.show_help {
        return match key.code {
            KeyCode::Char('j') | KeyCode::Down => Message::Ui(UiMessage::HelpScrollDown),
            KeyCode::Char('k') | KeyCode::Up => Message::Ui(UiMessage::HelpScrollUp),
            // A match guard applies to the whole or-pattern, so the ctrl check
            // must not be shared with the plain PageDown/PageUp keys.
            KeyCode::PageDown => Message::Ui(UiMessage::HelpPageDown),
            KeyCode::PageUp => Message::Ui(UiMessage::HelpPageUp),
            KeyCode::Char('d') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                Message::Ui(UiMessage::HelpPageDown)
            }
            KeyCode::Char('u') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                Message::Ui(UiMessage::HelpPageUp)
            }
            _ => Message::Ui(UiMessage::HideHelp),
        };
    }

    // If focus mode is active, handle special keys
    if model.focus_mode {
        match key.code {
            KeyCode::Esc => return Message::Ui(UiMessage::ToggleFocusMode),
            KeyCode::Char(']') => return Message::Ui(UiMessage::ChainNext),
            KeyCode::Char('[') => return Message::Ui(UiMessage::ChainPrev),
            // Adjust the Pomodoro cycle goal while a session is running.
            KeyCode::Char('+') | KeyCode::Char('=') if model.pomodoro.session.is_some() => {
                return Message::Pomodoro(taskflow::app::PomodoroMessage::IncrementGoal);
            }
            KeyCode::Char('-') if model.pomodoro.session.is_some() => {
                return Message::Pomodoro(taskflow::app::PomodoroMessage::DecrementGoal);
            }
            _ => {}
        }
    }

    // If template picker is showing, handle navigation and selection
    if model.template_picker.visible {
        return handle_template_picker(key, model);
    }

    // If keybindings editor is showing, handle navigation and editing
    if model.keybindings_editor.visible {
        return handle_keybindings_editor(key, model);
    }

    // If time log editor is showing, handle navigation and editing
    if model.time_log.visible {
        return handle_time_log(key, model);
    }

    // If work log editor is showing, handle navigation and editing
    if model.work_log_editor.visible {
        return handle_work_log(key, model);
    }

    // If description editor is showing, handle multi-line input
    if model.description_editor.visible {
        return handle_description_editor(key);
    }

    // In multi-select mode, Space toggles task selection and certain keys
    // trigger bulk operations instead of their single-task equivalents
    if model.multi_select.mode {
        match key.code {
            KeyCode::Char(' ') => return Message::Ui(UiMessage::ToggleTaskSelection),
            KeyCode::Char('p') if !model.multi_select.selected.is_empty() => {
                return Message::Ui(UiMessage::StartBulkSetPriority)
            }
            KeyCode::Char('T') if !model.multi_select.selected.is_empty() => {
                return Message::Ui(UiMessage::StartBulkAddTags)
            }
            KeyCode::Char('D') if !model.multi_select.selected.is_empty() => {
                return Message::Ui(UiMessage::StartBulkSetDueDate)
            }
            KeyCode::Char('z') if !model.multi_select.selected.is_empty() => {
                return Message::Ui(UiMessage::StartBulkSnooze)
            }
            _ => {}
        }
    }

    // Popups drawn over the current view take every key before the view does.
    if model.habit_view.show_analytics {
        return match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('a') => {
                Message::Ui(UiMessage::HideHabitAnalytics)
            }
            _ => Message::None,
        };
    }
    if model.daily_review.visible {
        return handle_daily_review(key);
    }
    if model.weekly_review.visible {
        return handle_weekly_review(key);
    }
    if model.evening_review.visible {
        return handle_evening_review(key);
    }
    if model.task_detail.visible {
        return handle_task_detail(key);
    }

    // Esc is the one way back to the sidebar from every view. With the
    // sidebar already focused it falls through to the keymap, where it is
    // unbound, so it never quits.
    if key.code == KeyCode::Esc && model.focus_pane == FocusPane::TaskList {
        return Message::Navigation(NavigationMessage::FocusSidebar);
    }

    // View-specific keys apply only while the main pane has focus. With the
    // sidebar focused, j/k/Enter/l fall through to the sidebar bindings so
    // the user can pick another view from any view.
    if model.focus_pane == FocusPane::TaskList {
        if let Some(msg) = handle_view_keys(key, model) {
            return msg;
        }
    }

    // Handle macro slot selection if pending
    if model.macro_state.pending_slot.is_some() {
        return handle_macro_slot(key, model);
    }

    // Complete a pending multi-key chord (e.g. `@` then `3` plays macro 3)
    if let Some(prefix) = model.pending_chord.take() {
        let chord = format!("{prefix}{}", key_event_to_string(&key));
        return keybindings
            .get_action(&chord)
            .map_or(Message::None, action_to_message);
    }

    // Enter opens task detail when task list is focused
    if key.code == KeyCode::Enter && model.focus_pane == taskflow::app::FocusPane::TaskList {
        return Message::Ui(UiMessage::ShowTaskDetail);
    }

    // Convert key event to string for lookup
    let key_str = key_event_to_string(&key);

    // Start a chord: remember the prefix and wait for the next key
    if keybindings.is_chord_prefix(&key_str) {
        model.pending_chord = Some(key_str);
        return Message::None;
    }

    // Look up action in keybindings
    if let Some(action) = keybindings.get_action(&key_str) {
        return action_to_message(action);
    }

    Message::None
}

/// Keys that belong to the view in the main pane.
///
/// Returns `None` when the view does not claim the key, so it falls through
/// to the global keymap.
fn handle_view_keys(key: event::KeyEvent, model: &mut Model) -> Option<Message> {
    match model.current_view {
        ViewId::Calendar => handle_calendar_view(key, model),
        ViewId::Habits => handle_habits_view(key, model),
        ViewId::Goals => handle_goals_view(key, model),
        ViewId::Timeline => handle_timeline_view(key),
        ViewId::Kanban => handle_kanban_view(key),
        ViewId::Eisenhower => handle_eisenhower_view(key),
        ViewId::WeeklyPlanner => handle_weekly_planner_view(key),
        ViewId::Reports => handle_reports_view(key),
        ViewId::Network => handle_network_view(key),
        ViewId::Duplicates => handle_duplicates_view(key),
        // r triggers an in-app rescan
        ViewId::GitTodos if key.code == KeyCode::Char('r') => {
            Some(Message::System(SystemMessage::ScanGitTodos))
        }
        _ => None,
    }
}

/// Handle input for the command palette.
fn handle_command_palette_input(key: event::KeyEvent, model: &mut Model) -> Message {
    match key.code {
        KeyCode::Esc => Message::Ui(UiMessage::HideCommandPalette),
        KeyCode::Enter => {
            // Execute the selected command and close the palette
            if let Some(action) = taskflow::app::get_palette_action(model) {
                // Close the palette immediately so it doesn't remain visible
                // while the dispatched action runs
                model.command_palette.visible = false;
                return action_to_message(&action);
            }
            Message::Ui(UiMessage::HideCommandPalette)
        }
        KeyCode::Up => Message::Ui(UiMessage::CommandPaletteUp),
        KeyCode::Down => Message::Ui(UiMessage::CommandPaletteDown),
        KeyCode::Char('k') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            Message::Ui(UiMessage::CommandPaletteUp)
        }
        KeyCode::Char('j') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            Message::Ui(UiMessage::CommandPaletteDown)
        }
        KeyCode::Backspace => Message::Ui(UiMessage::CommandPaletteBackspace),
        KeyCode::Char(c) => Message::Ui(UiMessage::CommandPaletteInput(c)),
        _ => Message::None,
    }
}
