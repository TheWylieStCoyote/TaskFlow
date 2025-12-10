//! Keybindings editor tests.

use crate::app::{update::update, Message, Model, UiMessage};
use crate::config::Action;

#[test]
fn test_show_keybindings_editor() {
    let mut model = Model::new();
    assert!(!model.keybindings_editor.visible);

    update(&mut model, Message::Ui(UiMessage::ShowKeybindingsEditor));
    assert!(model.keybindings_editor.visible);
    assert_eq!(model.keybindings_editor.selected, 0);
    assert!(!model.keybindings_editor.capturing);
}

#[test]
fn test_hide_keybindings_editor() {
    let mut model = Model::new();
    model.keybindings_editor.visible = true;
    model.keybindings_editor.capturing = true;

    update(&mut model, Message::Ui(UiMessage::HideKeybindingsEditor));
    assert!(!model.keybindings_editor.visible);
    assert!(!model.keybindings_editor.capturing);
}

#[test]
fn test_keybindings_navigation() {
    let mut model = Model::new();
    model.keybindings_editor.visible = true;
    model.keybindings_editor.selected = 5;

    update(&mut model, Message::Ui(UiMessage::KeybindingsUp));
    assert_eq!(model.keybindings_editor.selected, 4);

    update(&mut model, Message::Ui(UiMessage::KeybindingsDown));
    assert_eq!(model.keybindings_editor.selected, 5);

    // Navigate up at 0 should stay at 0
    model.keybindings_editor.selected = 0;
    update(&mut model, Message::Ui(UiMessage::KeybindingsUp));
    assert_eq!(model.keybindings_editor.selected, 0);
}

#[test]
fn test_start_edit_keybinding() {
    let mut model = Model::new();
    model.keybindings_editor.visible = true;

    update(&mut model, Message::Ui(UiMessage::StartEditKeybinding));
    assert!(model.keybindings_editor.capturing);
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_cancel_edit_keybinding() {
    let mut model = Model::new();
    model.keybindings_editor.visible = true;
    model.keybindings_editor.capturing = true;
    model.alerts.status_message = Some("Press a key...".to_string());

    update(&mut model, Message::Ui(UiMessage::CancelEditKeybinding));
    assert!(!model.keybindings_editor.capturing);
    assert!(model.alerts.status_message.is_none());
}

#[test]
fn test_apply_keybinding() {
    let mut model = Model::new();
    model.keybindings_editor.visible = true;
    model.keybindings_editor.capturing = true;

    // Get the first binding's action
    let bindings = model.keybindings.sorted_bindings();
    let (_, first_action) = &bindings[0];
    let original_action = first_action.clone();

    // Apply a new key to that action
    update(
        &mut model,
        Message::Ui(UiMessage::ApplyKeybinding("1".to_string())),
    );

    assert!(!model.keybindings_editor.capturing);
    // The action should now be bound to '1'
    assert_eq!(model.keybindings.get_action("1"), Some(&original_action));
}

#[test]
fn test_reset_all_keybindings() {
    let mut model = Model::new();
    model.keybindings_editor.visible = true;

    // Modify a keybinding (use "1" which is not a default binding)
    model.keybindings.set_binding("1".to_string(), Action::Quit);

    // Verify it was changed
    assert_eq!(model.keybindings.get_action("1"), Some(&Action::Quit));

    // Reset all
    update(&mut model, Message::Ui(UiMessage::ResetAllKeybindings));

    // Should be back to default ("1" is not a default binding)
    assert_eq!(model.keybindings.get_action("1"), None);
    assert!(model.alerts.status_message.is_some());
}
