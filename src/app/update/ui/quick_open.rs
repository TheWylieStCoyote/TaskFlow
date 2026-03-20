//! Quick-open fuzzy task picker message handlers.

use crate::app::{Model, UiMessage};
use crate::ui::get_quick_open_result_count;

/// Handle quick-open UI messages.
pub fn handle_ui_quick_open(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ShowQuickOpen => {
            model.quick_open.visible = true;
            model.quick_open.query.clear();
            model.quick_open.cursor = 0;
            model.quick_open.selected = 0;
        }
        UiMessage::HideQuickOpen => {
            model.quick_open.visible = false;
        }
        UiMessage::QuickOpenInput(ch) => {
            model.quick_open.query.insert(model.quick_open.cursor, ch);
            model.quick_open.cursor += ch.len_utf8();
            model.quick_open.selected = 0;
        }
        UiMessage::QuickOpenBackspace => {
            if model.quick_open.cursor > 0 {
                let new_cursor = model.quick_open.query[..model.quick_open.cursor]
                    .char_indices()
                    .last()
                    .map_or(0, |(i, _)| i);
                model
                    .quick_open
                    .query
                    .drain(new_cursor..model.quick_open.cursor);
                model.quick_open.cursor = new_cursor;
                model.quick_open.selected = 0;
            }
        }
        UiMessage::QuickOpenUp => {
            if model.quick_open.selected > 0 {
                model.quick_open.selected -= 1;
            }
        }
        UiMessage::QuickOpenDown => {
            let count = get_quick_open_result_count(model);
            if model.quick_open.selected + 1 < count {
                model.quick_open.selected += 1;
            }
        }
        _ => {}
    }
}
