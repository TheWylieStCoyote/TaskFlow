//! Modal and dialog input handlers.

use crossterm::event::{self, KeyCode, KeyModifiers};

use taskflow::app::{Message, Model, UiMessage};
use taskflow::ui::{TimeLogMode, WorkLogMode};

use crate::input::util::key_event_to_string;

/// Handle template picker input
pub fn handle_template_picker(key: event::KeyEvent, model: &mut Model) -> Message {
    match key.code {
        KeyCode::Esc => Message::Ui(UiMessage::HideTemplates),
        KeyCode::Enter => Message::Ui(UiMessage::SelectTemplate(model.template_picker.selected)),
        KeyCode::Up | KeyCode::Char('k') => {
            if model.template_picker.selected > 0 {
                model.template_picker.selected -= 1;
            }
            Message::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let max = model.template_manager.len().saturating_sub(1);
            if model.template_picker.selected < max {
                model.template_picker.selected += 1;
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
    if model.keybindings_editor.capturing {
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
    match model.time_log.mode {
        mode if mode.is_text_input() => {
            // Prompt open (start/end/note/date/add) - handle character input
            match key.code {
                KeyCode::Esc => Message::Ui(UiMessage::TimeLogCancel),
                KeyCode::Enter => Message::Ui(UiMessage::TimeLogSubmit),
                KeyCode::Backspace => Message::Ui(UiMessage::TimeLogInputBackspace),
                KeyCode::Char(c) => Message::Ui(UiMessage::TimeLogInputChar(c)),
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
        _ => {
            // Browse: navigation and entry actions
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => Message::Ui(UiMessage::HideTimeLog),
                KeyCode::Up | KeyCode::Char('k') => Message::Ui(UiMessage::TimeLogUp),
                KeyCode::Down | KeyCode::Char('j') => Message::Ui(UiMessage::TimeLogDown),
                KeyCode::Char('s') => Message::Ui(UiMessage::TimeLogEditStart),
                KeyCode::Char('e') => Message::Ui(UiMessage::TimeLogEditEnd),
                KeyCode::Char('n') => Message::Ui(UiMessage::TimeLogEditNote),
                KeyCode::Char('t') => Message::Ui(UiMessage::TimeLogEditDate),
                KeyCode::Char('+') | KeyCode::Char('=') => Message::Ui(UiMessage::TimeLogNudge(5)),
                KeyCode::Char('-') | KeyCode::Char('_') => Message::Ui(UiMessage::TimeLogNudge(-5)),
                KeyCode::Char('d') => Message::Ui(UiMessage::TimeLogConfirmDelete),
                KeyCode::Char('a') => Message::Ui(UiMessage::TimeLogAddEntry),
                _ => Message::None,
            }
        }
    }
}

/// Handle work log editor input
pub fn handle_work_log(key: event::KeyEvent, model: &Model) -> Message {
    match model.work_log_editor.mode {
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
                    if model.work_log_editor.search_query.is_empty() {
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

/// Handle daily review input
pub fn handle_daily_review(key: event::KeyEvent) -> Message {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Message::Ui(UiMessage::HideDailyReview),
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            Message::Ui(UiMessage::DailyReviewNext)
        }
        KeyCode::Left | KeyCode::Char('h') => Message::Ui(UiMessage::DailyReviewPrev),
        KeyCode::Up | KeyCode::Char('k') => Message::Ui(UiMessage::DailyReviewUp),
        KeyCode::Down | KeyCode::Char('j') => Message::Ui(UiMessage::DailyReviewDown),
        KeyCode::Char('x') | KeyCode::Char(' ') => Message::Ui(UiMessage::DailyReviewComplete),
        _ => Message::None,
    }
}

/// Handle weekly review input
pub fn handle_weekly_review(key: event::KeyEvent) -> Message {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Message::Ui(UiMessage::HideWeeklyReview),
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            Message::Ui(UiMessage::WeeklyReviewNext)
        }
        KeyCode::Left | KeyCode::Char('h') => Message::Ui(UiMessage::WeeklyReviewPrev),
        KeyCode::Up | KeyCode::Char('k') => Message::Ui(UiMessage::WeeklyReviewUp),
        KeyCode::Down | KeyCode::Char('j') => Message::Ui(UiMessage::WeeklyReviewDown),
        _ => Message::None,
    }
}

/// Handle evening review input
pub fn handle_evening_review(key: event::KeyEvent) -> Message {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Message::Ui(UiMessage::HideEveningReview),
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            Message::Ui(UiMessage::EveningReviewNext)
        }
        KeyCode::Left | KeyCode::Char('h') => Message::Ui(UiMessage::EveningReviewPrev),
        KeyCode::Up | KeyCode::Char('k') => Message::Ui(UiMessage::EveningReviewUp),
        KeyCode::Down | KeyCode::Char('j') => Message::Ui(UiMessage::EveningReviewDown),
        KeyCode::Char('r') => Message::Ui(UiMessage::EveningReviewReschedule),
        KeyCode::Char('s') => Message::Ui(UiMessage::EveningReviewSnooze),
        KeyCode::Char('x') | KeyCode::Char(' ') => Message::Ui(UiMessage::EveningReviewComplete),
        _ => Message::None,
    }
}

/// Handle task detail modal input
pub fn handle_task_detail(key: event::KeyEvent) -> Message {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Message::Ui(UiMessage::HideTaskDetail),
        KeyCode::Up | KeyCode::Char('k') => Message::Ui(UiMessage::TaskDetailScrollUp),
        KeyCode::Down | KeyCode::Char('j') => Message::Ui(UiMessage::TaskDetailScrollDown),
        KeyCode::PageUp => Message::Ui(UiMessage::TaskDetailPageUp),
        KeyCode::PageDown => Message::Ui(UiMessage::TaskDetailPageDown),
        KeyCode::Char('g') => Message::Ui(UiMessage::TaskDetailScrollTop),
        KeyCode::Char('G') => Message::Ui(UiMessage::TaskDetailScrollBottom),
        KeyCode::Home => Message::Ui(UiMessage::TaskDetailScrollTop),
        KeyCode::End => Message::Ui(UiMessage::TaskDetailScrollBottom),
        _ => Message::None,
    }
}

/// Handle macro slot selection
pub fn handle_macro_slot(key: event::KeyEvent, model: &mut Model) -> Message {
    if let KeyCode::Char(c) = key.code {
        if let Some(digit) = c.to_digit(10) {
            let slot = digit as usize;
            model.macro_state.pending_slot = Some(slot);
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
        model.macro_state.pending_slot = None;
        model.alerts.status_message = Some("Macro cancelled".to_string());
        return Message::None;
    }
    Message::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ch(c: char) -> KeyEvent {
        key(KeyCode::Char(c))
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    // ========================================================================
    // Template picker
    // ========================================================================

    #[test]
    fn template_picker_esc_and_enter() {
        let mut model = Model::new();
        model.template_picker.selected = 2;
        assert!(matches!(
            handle_template_picker(key(KeyCode::Esc), &mut model),
            Message::Ui(UiMessage::HideTemplates)
        ));
        assert!(matches!(
            handle_template_picker(key(KeyCode::Enter), &mut model),
            Message::Ui(UiMessage::SelectTemplate(2))
        ));
    }

    #[test]
    fn template_picker_moves_selection_within_bounds() {
        let mut model = Model::new();
        let count = model.template_manager.len();
        assert!(count >= 2, "default templates expected");

        // Up at the top stays at 0
        assert!(matches!(
            handle_template_picker(ch('k'), &mut model),
            Message::None
        ));
        assert_eq!(model.template_picker.selected, 0);

        // Down moves one at a time until the last template
        assert!(matches!(
            handle_template_picker(ch('j'), &mut model),
            Message::None
        ));
        assert_eq!(model.template_picker.selected, 1);
        assert!(matches!(
            handle_template_picker(key(KeyCode::Up), &mut model),
            Message::None
        ));
        assert_eq!(model.template_picker.selected, 0);

        for _ in 0..count + 3 {
            handle_template_picker(key(KeyCode::Down), &mut model);
        }
        assert_eq!(model.template_picker.selected, count - 1);
    }

    #[test]
    fn template_picker_digit_selects_or_ignores() {
        let mut model = Model::new();
        let count = model.template_manager.len();
        assert!(count < 9, "test assumes fewer than nine default templates");
        assert!(matches!(
            handle_template_picker(ch('0'), &mut model),
            Message::Ui(UiMessage::SelectTemplate(0))
        ));
        assert!(matches!(
            handle_template_picker(ch('1'), &mut model),
            Message::Ui(UiMessage::SelectTemplate(1))
        ));
        assert!(matches!(
            handle_template_picker(ch('9'), &mut model),
            Message::None
        ));
        assert!(matches!(
            handle_template_picker(ch('x'), &mut model),
            Message::None
        ));
        assert!(matches!(
            handle_template_picker(key(KeyCode::Tab), &mut model),
            Message::None
        ));
    }

    // ========================================================================
    // Keybindings editor
    // ========================================================================

    #[test]
    fn keybindings_editor_capturing_applies_key_string() {
        let mut model = Model::new();
        model.keybindings_editor.capturing = true;
        assert!(matches!(
            handle_keybindings_editor(key(KeyCode::Esc), &model),
            Message::Ui(UiMessage::CancelEditKeybinding)
        ));
        assert!(matches!(
            handle_keybindings_editor(ctrl('s'), &model),
            Message::Ui(UiMessage::ApplyKeybinding(k)) if k == "ctrl+s"
        ));
        assert!(matches!(
            handle_keybindings_editor(key(KeyCode::F(5)), &model),
            Message::Ui(UiMessage::ApplyKeybinding(k)) if k == "f5"
        ));
    }

    #[test]
    fn keybindings_editor_navigation() {
        let model = Model::new();
        assert!(matches!(
            handle_keybindings_editor(key(KeyCode::Esc), &model),
            Message::Ui(UiMessage::HideKeybindingsEditor)
        ));
        assert!(matches!(
            handle_keybindings_editor(key(KeyCode::Enter), &model),
            Message::Ui(UiMessage::StartEditKeybinding)
        ));
        for k in [key(KeyCode::Up), ch('k')] {
            assert!(matches!(
                handle_keybindings_editor(k, &model),
                Message::Ui(UiMessage::KeybindingsUp)
            ));
        }
        for k in [key(KeyCode::Down), ch('j')] {
            assert!(matches!(
                handle_keybindings_editor(k, &model),
                Message::Ui(UiMessage::KeybindingsDown)
            ));
        }
        assert!(matches!(
            handle_keybindings_editor(ch('r'), &model),
            Message::Ui(UiMessage::ResetKeybinding)
        ));
        assert!(matches!(
            handle_keybindings_editor(ch('R'), &model),
            Message::Ui(UiMessage::ResetAllKeybindings)
        ));
        assert!(matches!(
            handle_keybindings_editor(ch('s'), &model),
            Message::Ui(UiMessage::SaveKeybindings)
        ));
        assert!(matches!(
            handle_keybindings_editor(ch('x'), &model),
            Message::None
        ));
    }

    // ========================================================================
    // Time log
    // ========================================================================

    #[test]
    fn time_log_prompt_modes_take_text_input() {
        let mut model = Model::new();
        for mode in [
            TimeLogMode::EditStart,
            TimeLogMode::EditEnd,
            TimeLogMode::EditNote,
            TimeLogMode::EditDate,
            TimeLogMode::Add,
        ] {
            model.time_log.mode = mode;
            assert!(matches!(
                handle_time_log(key(KeyCode::Esc), &model),
                Message::Ui(UiMessage::TimeLogCancel)
            ));
            assert!(matches!(
                handle_time_log(key(KeyCode::Enter), &model),
                Message::Ui(UiMessage::TimeLogSubmit)
            ));
            assert!(matches!(
                handle_time_log(key(KeyCode::Backspace), &model),
                Message::Ui(UiMessage::TimeLogInputBackspace)
            ));
            assert!(matches!(
                handle_time_log(ch('7'), &model),
                Message::Ui(UiMessage::TimeLogInputChar('7'))
            ));
            // Action keys are plain text while a prompt is open
            for c in ['s', 'e', 'n', 't', 'd', 'a', '+', '-', 'j', 'k', 'q'] {
                assert!(matches!(
                    handle_time_log(ch(c), &model),
                    Message::Ui(UiMessage::TimeLogInputChar(x)) if x == c
                ));
            }
            assert!(matches!(
                handle_time_log(key(KeyCode::Tab), &model),
                Message::None
            ));
        }
    }

    #[test]
    fn time_log_confirm_delete_mode() {
        let mut model = Model::new();
        model.time_log.mode = TimeLogMode::ConfirmDelete;
        for k in [ch('y'), ch('Y')] {
            assert!(matches!(
                handle_time_log(k, &model),
                Message::Ui(UiMessage::TimeLogDelete)
            ));
        }
        for k in [ch('n'), ch('N'), key(KeyCode::Esc)] {
            assert!(matches!(
                handle_time_log(k, &model),
                Message::Ui(UiMessage::TimeLogCancel)
            ));
        }
        assert!(matches!(handle_time_log(ch('j'), &model), Message::None));
    }

    #[test]
    fn time_log_browse_mode() {
        let model = Model::new();
        assert!(matches!(model.time_log.mode, TimeLogMode::Browse));
        for k in [key(KeyCode::Esc), ch('q')] {
            assert!(matches!(
                handle_time_log(k, &model),
                Message::Ui(UiMessage::HideTimeLog)
            ));
        }
        for k in [key(KeyCode::Up), ch('k')] {
            assert!(matches!(
                handle_time_log(k, &model),
                Message::Ui(UiMessage::TimeLogUp)
            ));
        }
        for k in [key(KeyCode::Down), ch('j')] {
            assert!(matches!(
                handle_time_log(k, &model),
                Message::Ui(UiMessage::TimeLogDown)
            ));
        }
        assert!(matches!(
            handle_time_log(ch('s'), &model),
            Message::Ui(UiMessage::TimeLogEditStart)
        ));
        assert!(matches!(
            handle_time_log(ch('e'), &model),
            Message::Ui(UiMessage::TimeLogEditEnd)
        ));
        assert!(matches!(
            handle_time_log(ch('n'), &model),
            Message::Ui(UiMessage::TimeLogEditNote)
        ));
        assert!(matches!(
            handle_time_log(ch('t'), &model),
            Message::Ui(UiMessage::TimeLogEditDate)
        ));
        for k in [ch('+'), ch('=')] {
            assert!(matches!(
                handle_time_log(k, &model),
                Message::Ui(UiMessage::TimeLogNudge(5))
            ));
        }
        for k in [ch('-'), ch('_')] {
            assert!(matches!(
                handle_time_log(k, &model),
                Message::Ui(UiMessage::TimeLogNudge(-5))
            ));
        }
        assert!(matches!(
            handle_time_log(ch('d'), &model),
            Message::Ui(UiMessage::TimeLogConfirmDelete)
        ));
        assert!(matches!(
            handle_time_log(ch('a'), &model),
            Message::Ui(UiMessage::TimeLogAddEntry)
        ));
        assert!(matches!(handle_time_log(ch('x'), &model), Message::None));
    }

    // ========================================================================
    // Work log
    // ========================================================================

    #[test]
    fn work_log_add_and_edit_modes() {
        let mut model = Model::new();
        for mode in [WorkLogMode::Add, WorkLogMode::Edit] {
            model.work_log_editor.mode = mode;
            assert!(matches!(
                handle_work_log(ctrl('s'), &model),
                Message::Ui(UiMessage::WorkLogSubmit)
            ));
            // Plain 's' is ordinary text
            assert!(matches!(
                handle_work_log(ch('s'), &model),
                Message::Ui(UiMessage::WorkLogInputChar('s'))
            ));
            assert!(matches!(
                handle_work_log(key(KeyCode::Esc), &model),
                Message::Ui(UiMessage::WorkLogCancel)
            ));
            assert!(matches!(
                handle_work_log(key(KeyCode::Enter), &model),
                Message::Ui(UiMessage::WorkLogNewline)
            ));
            assert!(matches!(
                handle_work_log(key(KeyCode::Backspace), &model),
                Message::Ui(UiMessage::WorkLogInputBackspace)
            ));
            assert!(matches!(
                handle_work_log(key(KeyCode::Delete), &model),
                Message::Ui(UiMessage::WorkLogInputDelete)
            ));
            assert!(matches!(
                handle_work_log(key(KeyCode::Left), &model),
                Message::Ui(UiMessage::WorkLogCursorLeft)
            ));
            assert!(matches!(
                handle_work_log(key(KeyCode::Right), &model),
                Message::Ui(UiMessage::WorkLogCursorRight)
            ));
            assert!(matches!(
                handle_work_log(key(KeyCode::Up), &model),
                Message::Ui(UiMessage::WorkLogCursorUp)
            ));
            assert!(matches!(
                handle_work_log(key(KeyCode::Down), &model),
                Message::Ui(UiMessage::WorkLogCursorDown)
            ));
            assert!(matches!(
                handle_work_log(key(KeyCode::Home), &model),
                Message::Ui(UiMessage::WorkLogCursorHome)
            ));
            assert!(matches!(
                handle_work_log(key(KeyCode::End), &model),
                Message::Ui(UiMessage::WorkLogCursorEnd)
            ));
            assert!(matches!(
                handle_work_log(key(KeyCode::Tab), &model),
                Message::None
            ));
        }
    }

    #[test]
    fn work_log_confirm_delete_mode() {
        let mut model = Model::new();
        model.work_log_editor.mode = WorkLogMode::ConfirmDelete;
        for k in [ch('y'), ch('Y')] {
            assert!(matches!(
                handle_work_log(k, &model),
                Message::Ui(UiMessage::WorkLogDelete)
            ));
        }
        for k in [ch('n'), ch('N'), key(KeyCode::Esc)] {
            assert!(matches!(
                handle_work_log(k, &model),
                Message::Ui(UiMessage::WorkLogCancel)
            ));
        }
        assert!(matches!(handle_work_log(ch('j'), &model), Message::None));
    }

    #[test]
    fn work_log_view_mode() {
        let mut model = Model::new();
        model.work_log_editor.mode = WorkLogMode::View;
        for k in [key(KeyCode::Esc), key(KeyCode::Enter)] {
            assert!(matches!(
                handle_work_log(k, &model),
                Message::Ui(UiMessage::WorkLogCancel)
            ));
        }
        assert!(matches!(
            handle_work_log(ch('e'), &model),
            Message::Ui(UiMessage::WorkLogEdit)
        ));
        assert!(matches!(
            handle_work_log(ch('d'), &model),
            Message::Ui(UiMessage::WorkLogConfirmDelete)
        ));
        assert!(matches!(handle_work_log(ch('j'), &model), Message::None));
    }

    #[test]
    fn work_log_browse_mode() {
        let mut model = Model::new();
        assert!(matches!(model.work_log_editor.mode, WorkLogMode::Browse));
        for k in [key(KeyCode::Esc), ch('q')] {
            assert!(matches!(
                handle_work_log(k, &model),
                Message::Ui(UiMessage::HideWorkLog)
            ));
        }
        for k in [key(KeyCode::Up), ch('k')] {
            assert!(matches!(
                handle_work_log(k, &model),
                Message::Ui(UiMessage::WorkLogUp)
            ));
        }
        for k in [key(KeyCode::Down), ch('j')] {
            assert!(matches!(
                handle_work_log(k, &model),
                Message::Ui(UiMessage::WorkLogDown)
            ));
        }
        assert!(matches!(
            handle_work_log(key(KeyCode::Enter), &model),
            Message::Ui(UiMessage::WorkLogView)
        ));
        assert!(matches!(
            handle_work_log(ch('a'), &model),
            Message::Ui(UiMessage::WorkLogAdd)
        ));
        assert!(matches!(
            handle_work_log(ch('e'), &model),
            Message::Ui(UiMessage::WorkLogEdit)
        ));
        assert!(matches!(
            handle_work_log(ch('d'), &model),
            Message::Ui(UiMessage::WorkLogConfirmDelete)
        ));
        assert!(matches!(
            handle_work_log(ch('/'), &model),
            Message::Ui(UiMessage::WorkLogSearchStart)
        ));
        assert!(matches!(handle_work_log(ch('x'), &model), Message::None));

        // With an active search, Esc/q clear the search instead of closing
        model.work_log_editor.search_query = "meeting".to_string();
        for k in [key(KeyCode::Esc), ch('q')] {
            assert!(matches!(
                handle_work_log(k, &model),
                Message::Ui(UiMessage::WorkLogSearchClear)
            ));
        }
    }

    #[test]
    fn work_log_search_mode() {
        let mut model = Model::new();
        model.work_log_editor.mode = WorkLogMode::Search;
        assert!(matches!(
            handle_work_log(key(KeyCode::Esc), &model),
            Message::Ui(UiMessage::WorkLogSearchCancel)
        ));
        assert!(matches!(
            handle_work_log(key(KeyCode::Enter), &model),
            Message::Ui(UiMessage::WorkLogSearchApply)
        ));
        assert!(matches!(
            handle_work_log(key(KeyCode::Backspace), &model),
            Message::Ui(UiMessage::WorkLogSearchBackspace)
        ));
        assert!(matches!(
            handle_work_log(ch('m'), &model),
            Message::Ui(UiMessage::WorkLogSearchChar('m'))
        ));
        assert!(matches!(
            handle_work_log(key(KeyCode::Up), &model),
            Message::None
        ));
    }

    // ========================================================================
    // Description editor
    // ========================================================================

    #[test]
    fn description_editor_keys() {
        assert!(matches!(
            handle_description_editor(ctrl('s')),
            Message::Ui(UiMessage::DescriptionSubmit)
        ));
        assert!(matches!(
            handle_description_editor(ch('s')),
            Message::Ui(UiMessage::DescriptionInputChar('s'))
        ));
        assert!(matches!(
            handle_description_editor(key(KeyCode::Esc)),
            Message::Ui(UiMessage::HideDescriptionEditor)
        ));
        assert!(matches!(
            handle_description_editor(key(KeyCode::Enter)),
            Message::Ui(UiMessage::DescriptionNewline)
        ));
        assert!(matches!(
            handle_description_editor(key(KeyCode::Backspace)),
            Message::Ui(UiMessage::DescriptionInputBackspace)
        ));
        assert!(matches!(
            handle_description_editor(key(KeyCode::Delete)),
            Message::Ui(UiMessage::DescriptionInputDelete)
        ));
        assert!(matches!(
            handle_description_editor(key(KeyCode::Left)),
            Message::Ui(UiMessage::DescriptionCursorLeft)
        ));
        assert!(matches!(
            handle_description_editor(key(KeyCode::Right)),
            Message::Ui(UiMessage::DescriptionCursorRight)
        ));
        assert!(matches!(
            handle_description_editor(key(KeyCode::Up)),
            Message::Ui(UiMessage::DescriptionCursorUp)
        ));
        assert!(matches!(
            handle_description_editor(key(KeyCode::Down)),
            Message::Ui(UiMessage::DescriptionCursorDown)
        ));
        assert!(matches!(
            handle_description_editor(key(KeyCode::Home)),
            Message::Ui(UiMessage::DescriptionCursorHome)
        ));
        assert!(matches!(
            handle_description_editor(key(KeyCode::End)),
            Message::Ui(UiMessage::DescriptionCursorEnd)
        ));
        assert!(matches!(
            handle_description_editor(key(KeyCode::Tab)),
            Message::None
        ));
    }

    // ========================================================================
    // Reviews
    // ========================================================================

    #[test]
    fn daily_review_keys() {
        for k in [key(KeyCode::Esc), ch('q')] {
            assert!(matches!(
                handle_daily_review(k),
                Message::Ui(UiMessage::HideDailyReview)
            ));
        }
        for k in [key(KeyCode::Enter), key(KeyCode::Right), ch('l')] {
            assert!(matches!(
                handle_daily_review(k),
                Message::Ui(UiMessage::DailyReviewNext)
            ));
        }
        for k in [key(KeyCode::Left), ch('h')] {
            assert!(matches!(
                handle_daily_review(k),
                Message::Ui(UiMessage::DailyReviewPrev)
            ));
        }
        for k in [key(KeyCode::Up), ch('k')] {
            assert!(matches!(
                handle_daily_review(k),
                Message::Ui(UiMessage::DailyReviewUp)
            ));
        }
        for k in [key(KeyCode::Down), ch('j')] {
            assert!(matches!(
                handle_daily_review(k),
                Message::Ui(UiMessage::DailyReviewDown)
            ));
        }
        for k in [ch('x'), ch(' ')] {
            assert!(matches!(
                handle_daily_review(k),
                Message::Ui(UiMessage::DailyReviewComplete)
            ));
        }
        assert!(matches!(handle_daily_review(ch('r')), Message::None));
    }

    #[test]
    fn weekly_review_keys() {
        for k in [key(KeyCode::Esc), ch('q')] {
            assert!(matches!(
                handle_weekly_review(k),
                Message::Ui(UiMessage::HideWeeklyReview)
            ));
        }
        for k in [key(KeyCode::Enter), key(KeyCode::Right), ch('l')] {
            assert!(matches!(
                handle_weekly_review(k),
                Message::Ui(UiMessage::WeeklyReviewNext)
            ));
        }
        for k in [key(KeyCode::Left), ch('h')] {
            assert!(matches!(
                handle_weekly_review(k),
                Message::Ui(UiMessage::WeeklyReviewPrev)
            ));
        }
        for k in [key(KeyCode::Up), ch('k')] {
            assert!(matches!(
                handle_weekly_review(k),
                Message::Ui(UiMessage::WeeklyReviewUp)
            ));
        }
        for k in [key(KeyCode::Down), ch('j')] {
            assert!(matches!(
                handle_weekly_review(k),
                Message::Ui(UiMessage::WeeklyReviewDown)
            ));
        }
        // The weekly review has no complete/reschedule actions
        assert!(matches!(handle_weekly_review(ch('x')), Message::None));
    }

    #[test]
    fn evening_review_keys() {
        for k in [key(KeyCode::Esc), ch('q')] {
            assert!(matches!(
                handle_evening_review(k),
                Message::Ui(UiMessage::HideEveningReview)
            ));
        }
        for k in [key(KeyCode::Enter), key(KeyCode::Right), ch('l')] {
            assert!(matches!(
                handle_evening_review(k),
                Message::Ui(UiMessage::EveningReviewNext)
            ));
        }
        for k in [key(KeyCode::Left), ch('h')] {
            assert!(matches!(
                handle_evening_review(k),
                Message::Ui(UiMessage::EveningReviewPrev)
            ));
        }
        for k in [key(KeyCode::Up), ch('k')] {
            assert!(matches!(
                handle_evening_review(k),
                Message::Ui(UiMessage::EveningReviewUp)
            ));
        }
        for k in [key(KeyCode::Down), ch('j')] {
            assert!(matches!(
                handle_evening_review(k),
                Message::Ui(UiMessage::EveningReviewDown)
            ));
        }
        assert!(matches!(
            handle_evening_review(ch('r')),
            Message::Ui(UiMessage::EveningReviewReschedule)
        ));
        assert!(matches!(
            handle_evening_review(ch('s')),
            Message::Ui(UiMessage::EveningReviewSnooze)
        ));
        for k in [ch('x'), ch(' ')] {
            assert!(matches!(
                handle_evening_review(k),
                Message::Ui(UiMessage::EveningReviewComplete)
            ));
        }
        assert!(matches!(handle_evening_review(ch('z')), Message::None));
    }

    // ========================================================================
    // Task detail
    // ========================================================================

    #[test]
    fn task_detail_keys() {
        for k in [key(KeyCode::Esc), ch('q')] {
            assert!(matches!(
                handle_task_detail(k),
                Message::Ui(UiMessage::HideTaskDetail)
            ));
        }
        for k in [key(KeyCode::Up), ch('k')] {
            assert!(matches!(
                handle_task_detail(k),
                Message::Ui(UiMessage::TaskDetailScrollUp)
            ));
        }
        for k in [key(KeyCode::Down), ch('j')] {
            assert!(matches!(
                handle_task_detail(k),
                Message::Ui(UiMessage::TaskDetailScrollDown)
            ));
        }
        assert!(matches!(
            handle_task_detail(key(KeyCode::PageUp)),
            Message::Ui(UiMessage::TaskDetailPageUp)
        ));
        assert!(matches!(
            handle_task_detail(key(KeyCode::PageDown)),
            Message::Ui(UiMessage::TaskDetailPageDown)
        ));
        for k in [ch('g'), key(KeyCode::Home)] {
            assert!(matches!(
                handle_task_detail(k),
                Message::Ui(UiMessage::TaskDetailScrollTop)
            ));
        }
        for k in [ch('G'), key(KeyCode::End)] {
            assert!(matches!(
                handle_task_detail(k),
                Message::Ui(UiMessage::TaskDetailScrollBottom)
            ));
        }
        assert!(matches!(handle_task_detail(ch('x')), Message::None));
    }

    // ========================================================================
    // Macro slot
    // ========================================================================

    #[test]
    fn macro_slot_digit_starts_recording_when_idle() {
        let mut model = Model::new();
        model.macro_state.pending_slot = Some(0);
        assert!(!model.macro_state.is_recording());
        assert!(matches!(
            handle_macro_slot(ch('7'), &mut model),
            Message::Ui(UiMessage::StartRecordMacro)
        ));
        assert_eq!(model.macro_state.pending_slot, Some(7));
    }

    #[test]
    fn macro_slot_digit_stops_recording_when_recording() {
        let mut model = Model::new();
        model.macro_state.pending_slot = Some(0);
        assert!(model.macro_state.start_recording(2));
        assert!(model.macro_state.is_recording());
        assert!(matches!(
            handle_macro_slot(ch('2'), &mut model),
            Message::Ui(UiMessage::StopRecordMacro)
        ));
        assert_eq!(model.macro_state.pending_slot, Some(2));
    }

    #[test]
    fn macro_slot_esc_cancels_with_status_message() {
        let mut model = Model::new();
        model.macro_state.pending_slot = Some(4);
        assert!(matches!(
            handle_macro_slot(key(KeyCode::Esc), &mut model),
            Message::None
        ));
        assert_eq!(model.macro_state.pending_slot, None);
        assert_eq!(
            model.alerts.status_message.as_deref(),
            Some("Macro cancelled")
        );
    }

    #[test]
    fn macro_slot_other_keys_keep_waiting() {
        let mut model = Model::new();
        model.macro_state.pending_slot = Some(4);
        assert!(matches!(
            handle_macro_slot(ch('x'), &mut model),
            Message::None
        ));
        assert!(matches!(
            handle_macro_slot(key(KeyCode::Enter), &mut model),
            Message::None
        ));
        assert_eq!(model.macro_state.pending_slot, Some(4));
        assert!(model.alerts.status_message.is_none());
    }
}
