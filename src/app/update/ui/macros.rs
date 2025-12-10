//! Macro recording and playback handlers

use crate::app::{Model, UiMessage};

/// Handle macro recording and playback messages
pub fn handle_ui_macros(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::StartRecordMacro => {
            if model.macro_state.is_recording() {
                // Already recording - treat as entering slot number mode
                model.pending_macro_slot = Some(0); // Will be set by digit input
                model.alerts.status_message = Some("Press 0-9 to select macro slot".to_string());
            } else if let Some(slot) = model.pending_macro_slot.take() {
                // We have a pending slot, start recording
                if model.macro_state.start_recording(slot) {
                    model.alerts.status_message = Some(format!("Recording macro {slot}..."));
                }
            } else {
                // First press - prompt for slot
                model.pending_macro_slot = Some(0);
                model.alerts.status_message =
                    Some("Press 0-9 to start recording macro".to_string());
            }
        }
        UiMessage::StopRecordMacro => {
            if let Some(slot) = model.pending_macro_slot.take() {
                if model.macro_state.is_recording() {
                    if model.macro_state.stop_recording(slot) {
                        model.alerts.status_message = Some(format!("Macro {slot} saved"));
                    } else {
                        model.alerts.status_message =
                            Some("Macro was empty, not saved".to_string());
                    }
                }
            } else if model.macro_state.is_recording() {
                // No slot specified, cancel recording
                model.macro_state.cancel_recording();
                model.alerts.status_message = Some("Recording cancelled".to_string());
            }
        }
        UiMessage::PlayMacro(slot) => {
            // Playback is handled in main.rs by dispatching stored messages
            if model.macro_state.has_macro(slot) {
                model.alerts.status_message = Some(format!("Playing macro {slot}..."));
            } else {
                model.alerts.status_message = Some(format!("No macro in slot {slot}"));
            }
        }
        _ => {}
    }
}
