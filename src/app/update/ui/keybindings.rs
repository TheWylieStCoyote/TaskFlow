//! Keybindings editor handlers

use crate::app::{Model, UiMessage};

/// Handle keybindings editor messages
pub fn handle_ui_keybindings(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ShowKeybindingsEditor => {
            model.keybindings_editor.visible = true;
            model.keybindings_editor.selected = 0;
            model.keybindings_editor.capturing = false;
        }
        UiMessage::HideKeybindingsEditor => {
            model.keybindings_editor.visible = false;
            model.keybindings_editor.capturing = false;
        }
        UiMessage::KeybindingsUp => {
            if model.keybindings_editor.selected > 0 {
                model.keybindings_editor.selected -= 1;
            }
        }
        UiMessage::KeybindingsDown => {
            let bindings = model.keybindings.sorted_bindings();
            if model.keybindings_editor.selected < bindings.len().saturating_sub(1) {
                model.keybindings_editor.selected += 1;
            }
        }
        UiMessage::StartEditKeybinding => {
            model.keybindings_editor.capturing = true;
            model.alerts.status_message = Some("Press a key combination...".to_string());
        }
        UiMessage::CancelEditKeybinding => {
            model.keybindings_editor.capturing = false;
            model.alerts.status_message = None;
        }
        UiMessage::ApplyKeybinding(new_key) => {
            let bindings = model.keybindings.sorted_bindings();
            if let Some((_, action)) = bindings.get(model.keybindings_editor.selected) {
                // Check for conflicts and provide detailed feedback
                let conflicts = model.keybindings.find_all_conflicts(&new_key, action);
                model
                    .keybindings
                    .set_binding(new_key.clone(), action.clone());

                if conflicts.is_empty() {
                    model.alerts.status_message = Some(format!("Bound '{new_key}' to {action:?}"));
                } else {
                    model.alerts.status_message = Some(format!(
                        "Bound '{new_key}' to {:?}. {}",
                        action,
                        conflicts.join("; ")
                    ));
                }
            }
            model.keybindings_editor.capturing = false;
        }
        UiMessage::ResetKeybinding => {
            let bindings = model.keybindings.sorted_bindings();
            if let Some((_, action)) = bindings.get(model.keybindings_editor.selected) {
                // Find the default key for this action
                let defaults = crate::config::Keybindings::default();
                if let Some(default_key) = defaults.key_for_action(action) {
                    // Check for conflicts
                    let conflicts = model.keybindings.find_all_conflicts(default_key, action);
                    model
                        .keybindings
                        .set_binding(default_key.clone(), action.clone());

                    if conflicts.is_empty() {
                        model.alerts.status_message =
                            Some(format!("Reset {action:?} to default key '{default_key}'"));
                    } else {
                        model.alerts.status_message = Some(format!(
                            "Reset {:?} to '{}'. {}",
                            action,
                            default_key,
                            conflicts.join("; ")
                        ));
                    }
                } else {
                    model.alerts.status_message = Some("No default binding for this action".to_string());
                }
            }
        }
        UiMessage::ResetAllKeybindings => {
            model.keybindings = crate::config::Keybindings::default();
            model.alerts.status_message = Some("All keybindings reset to defaults".to_string());
        }
        UiMessage::SaveKeybindings => match model.keybindings.save() {
            Ok(()) => {
                model.alerts.status_message = Some("Keybindings saved".to_string());
            }
            Err(e) => {
                model.alerts.status_message = Some(format!("Failed to save keybindings: {e}"));
            }
        },
        UiMessage::DismissOverdueAlert => {
            model.alerts.show_overdue = false;
        }
        UiMessage::DismissStorageErrorAlert => {
            model.alerts.show_storage_error = false;
            model.alerts.storage_error = None;
        }
        _ => {}
    }
}
