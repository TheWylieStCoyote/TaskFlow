//! Modal and dialog input handlers.

use crossterm::event::{self, KeyCode, KeyModifiers};

use taskflow::app::{Message, Model, UiMessage};
use taskflow::ui::{TimeLogMode, WorkLogMode};

use crate::input::util::key_event_to_string;

/// Handle template picker input
pub fn handle_template_picker(key: event::KeyEvent, model: &mut Model) -> Message {
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

/// Handle keybindings editor input
pub fn handle_keybindings_editor(key: event::KeyEvent, model: &Model) -> Message {
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

/// Handle time log editor input
pub fn handle_time_log(key: event::KeyEvent, model: &Model) -> Message {
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

/// Handle work log editor input
pub fn handle_work_log(key: event::KeyEvent, model: &Model) -> Message {
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
                KeyCode::Esc | KeyCode::Char('q') => {
                    // If search is active, clear it; otherwise close
                    if model.work_log_search_query.is_empty() {
                        Message::Ui(UiMessage::HideWorkLog)
                    } else {
                        Message::Ui(UiMessage::WorkLogSearchClear)
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => Message::Ui(UiMessage::WorkLogUp),
                KeyCode::Down | KeyCode::Char('j') => Message::Ui(UiMessage::WorkLogDown),
                KeyCode::Enter => Message::Ui(UiMessage::WorkLogView),
                KeyCode::Char('a') => Message::Ui(UiMessage::WorkLogAdd),
                KeyCode::Char('e') => Message::Ui(UiMessage::WorkLogEdit),
                KeyCode::Char('d') => Message::Ui(UiMessage::WorkLogConfirmDelete),
                KeyCode::Char('/') => Message::Ui(UiMessage::WorkLogSearchStart),
                _ => Message::None,
            }
        }
        WorkLogMode::Search => {
            // Search input mode
            match key.code {
                KeyCode::Esc => Message::Ui(UiMessage::WorkLogSearchCancel),
                KeyCode::Enter => Message::Ui(UiMessage::WorkLogSearchApply),
                KeyCode::Backspace => Message::Ui(UiMessage::WorkLogSearchBackspace),
                KeyCode::Char(c) => Message::Ui(UiMessage::WorkLogSearchChar(c)),
                _ => Message::None,
            }
        }
    }
}

/// Handle description editor input
pub fn handle_description_editor(key: event::KeyEvent) -> Message {
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

/// Handle macro slot selection
pub fn handle_macro_slot(key: event::KeyEvent, model: &mut Model) -> Message {
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
