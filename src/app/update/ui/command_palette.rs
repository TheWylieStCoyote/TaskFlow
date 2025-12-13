//! Command palette message handlers.
//!
//! Handles showing, hiding, filtering, and executing commands from
//! the command palette popup.

use crate::app::Model;
use crate::app::UiMessage;
use crate::ui::{get_filtered_count, get_selected_action};

/// Handle command palette UI messages.
pub fn handle_ui_command_palette(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ShowCommandPalette => {
            model.command_palette.visible = true;
            model.command_palette.query.clear();
            model.command_palette.cursor = 0;
            model.command_palette.selected = 0;
        }
        UiMessage::HideCommandPalette => {
            model.command_palette.visible = false;
        }
        UiMessage::CommandPaletteInput(ch) => {
            model
                .command_palette
                .query
                .insert(model.command_palette.cursor, ch);
            model.command_palette.cursor += ch.len_utf8();
            // Reset selection when query changes
            model.command_palette.selected = 0;
        }
        UiMessage::CommandPaletteBackspace => {
            if model.command_palette.cursor > 0 {
                // Find previous character boundary
                let new_cursor = model.command_palette.query[..model.command_palette.cursor]
                    .char_indices()
                    .last()
                    .map_or(0, |(i, _)| i);
                model
                    .command_palette
                    .query
                    .drain(new_cursor..model.command_palette.cursor);
                model.command_palette.cursor = new_cursor;
                // Reset selection when query changes
                model.command_palette.selected = 0;
            }
        }
        UiMessage::CommandPaletteUp => {
            if model.command_palette.selected > 0 {
                model.command_palette.selected -= 1;
            }
        }
        UiMessage::CommandPaletteDown => {
            let count = get_filtered_count(&model.command_palette.query);
            if model.command_palette.selected + 1 < count {
                model.command_palette.selected += 1;
            }
        }
        UiMessage::CommandPaletteExecute => {
            // This case is handled specially in the main loop since we need
            // to return a different message to execute. See handle_command_palette_execute.
        }
        _ => {}
    }
}

/// Get the action to execute from the command palette.
///
/// Returns the selected action if the palette is visible and a valid
/// selection exists. This is used by the input handler to dispatch
/// the selected command.
#[must_use]
pub fn get_palette_action(model: &Model) -> Option<crate::config::Action> {
    if model.command_palette.visible {
        get_selected_action(&model.command_palette.query, model.command_palette.selected)
    } else {
        None
    }
}
