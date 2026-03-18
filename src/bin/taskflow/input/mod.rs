//! Input event handling.
//!
//! This module provides centralized input handling for the TaskFlow TUI,
//! including keyboard and mouse events.

mod handlers;
mod mouse;
mod util;

use crossterm::event::{self, KeyCode};

use taskflow::app::{Message, Model, SystemMessage, UiMessage};
use taskflow::config::Keybindings;
use taskflow::ui::InputMode;

pub use handlers::{
    handle_calendar_view, handle_daily_review, handle_description_editor, handle_eisenhower_view,
    handle_evening_review, handle_goals_view, handle_habits_view, handle_kanban_view,
    handle_keybindings_editor, handle_macro_slot, handle_network_view, handle_reports_view,
    handle_task_detail, handle_template_picker, handle_time_log, handle_timeline_view,
    handle_weekly_planner_view, handle_weekly_review, handle_work_log,
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

    // If help is showing, any key closes it
    if model.show_help {
        return Message::Ui(UiMessage::HideHelp);
    }

    // If focus mode is active, handle special keys
    if model.focus_mode {
        match key.code {
            KeyCode::Esc => return Message::Ui(UiMessage::ToggleFocusMode),
            KeyCode::Char(']') => return Message::Ui(UiMessage::ChainNext),
            KeyCode::Char('[') => return Message::Ui(UiMessage::ChainPrev),
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

    // In Network view, handle task navigation
    if model.current_view == taskflow::app::ViewId::Network {
        if let Some(msg) = handle_network_view(key) {
            return msg;
        }
    }

    // In Goals view, handle goal-specific actions
    if model.current_view == taskflow::app::ViewId::Goals {
        if let Some(msg) = handle_goals_view(key, model) {
            return msg;
        }
    }

    // If habit analytics is showing, handle it
    if model.habit_view.show_analytics {
        return match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('a') => {
                Message::Ui(UiMessage::HideHabitAnalytics)
            }
            _ => Message::None,
        };
    }

    // If daily review is showing, handle navigation
    if model.daily_review.visible {
        return handle_daily_review(key);
    }

    // If weekly review is showing, handle navigation
    if model.weekly_review.visible {
        return handle_weekly_review(key);
    }

    // If evening review is showing, handle navigation
    if model.evening_review.visible {
        return handle_evening_review(key);
    }

    // If task detail is showing, handle navigation
    if model.task_detail.visible {
        return handle_task_detail(key);
    }

    // Handle macro slot selection if pending
    if model.macro_state.pending_slot.is_some() {
        return handle_macro_slot(key, model);
    }

    // Enter opens task detail when task list is focused
    if key.code == KeyCode::Enter && model.focus_pane == taskflow::app::FocusPane::TaskList {
        return Message::Ui(UiMessage::ShowTaskDetail);
    }

    // Convert key event to string for lookup
    let key_str = key_event_to_string(&key);

    // Look up action in keybindings
    if let Some(action) = keybindings.get_action(&key_str) {
        return action_to_message(action);
    }

    Message::None
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
