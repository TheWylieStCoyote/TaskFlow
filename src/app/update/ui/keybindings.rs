//! Keybindings editor handlers

use crate::app::{Model, UiMessage};

/// Handle keybindings editor messages
pub fn handle_ui_keybindings(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ShowKeybindingsEditor => {
            model.show_keybindings_editor = true;
            model.keybinding_selected = 0;
            model.keybinding_capturing = false;
        }
        UiMessage::HideKeybindingsEditor => {
            model.show_keybindings_editor = false;
            model.keybinding_capturing = false;
        }
        UiMessage::KeybindingsUp => {
            if model.keybinding_selected > 0 {
                model.keybinding_selected -= 1;
            }
        }
        UiMessage::KeybindingsDown => {
            let bindings = model.keybindings.sorted_bindings();
            if model.keybinding_selected < bindings.len().saturating_sub(1) {
                model.keybinding_selected += 1;
            }
        }
        UiMessage::StartEditKeybinding => {
            model.keybinding_capturing = true;
            model.status_message = Some("Press a key combination...".to_string());
        }
        UiMessage::CancelEditKeybinding => {
            model.keybinding_capturing = false;
            model.status_message = None;
        }
        UiMessage::ApplyKeybinding(new_key) => {
            let bindings = model.keybindings.sorted_bindings();
            if let Some((_, action)) = bindings.get(model.keybinding_selected) {
                // Check for conflicts and provide detailed feedback
                let conflicts = model.keybindings.find_all_conflicts(&new_key, action);
                model
                    .keybindings
                    .set_binding(new_key.clone(), action.clone());

                if conflicts.is_empty() {
                    model.status_message = Some(format!("Bound '{new_key}' to {:?}", action));
                } else {
                    model.status_message = Some(format!(
                        "Bound '{new_key}' to {:?}. {}",
                        action,
                        conflicts.join("; ")
                    ));
                }
            }
            model.keybinding_capturing = false;
        }
        UiMessage::ResetKeybinding => {
            let bindings = model.keybindings.sorted_bindings();
            if let Some((_, action)) = bindings.get(model.keybinding_selected) {
                // Find the default key for this action
                let defaults = crate::config::Keybindings::default();
                if let Some(default_key) = defaults.key_for_action(action) {
                    // Check for conflicts
                    let conflicts = model.keybindings.find_all_conflicts(default_key, action);
                    model
                        .keybindings
                        .set_binding(default_key.clone(), action.clone());

                    if conflicts.is_empty() {
                        model.status_message = Some(format!(
                            "Reset {:?} to default key '{}'",
                            action, default_key
                        ));
                    } else {
                        model.status_message = Some(format!(
                            "Reset {:?} to '{}'. {}",
                            action,
                            default_key,
                            conflicts.join("; ")
                        ));
                    }
                } else {
                    model.status_message = Some("No default binding for this action".to_string());
                }
            }
        }
        UiMessage::ResetAllKeybindings => {
            model.keybindings = crate::config::Keybindings::default();
            model.status_message = Some("All keybindings reset to defaults".to_string());
        }
        UiMessage::SaveKeybindings => match model.keybindings.save() {
            Ok(()) => {
                model.status_message = Some("Keybindings saved".to_string());
            }
            Err(e) => {
                model.status_message = Some(format!("Failed to save keybindings: {e}"));
            }
        },
        UiMessage::DismissOverdueAlert => {
            model.show_overdue_alert = false;
        }
        UiMessage::DismissStorageErrorAlert => {
            model.show_storage_error_alert = false;
            model.storage_load_error = None;
        }
        _ => {}
    }
}
