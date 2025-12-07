//! Keybindings editor tests.

use crate::app::{update::update, Message, Model, UiMessage};
use crate::config::Action;

#[test]
fn test_show_keybindings_editor() {
    let mut model = Model::new();
    assert!(!model.show_keybindings_editor);

    update(&mut model, Message::Ui(UiMessage::ShowKeybindingsEditor));
    assert!(model.show_keybindings_editor);
    assert_eq!(model.keybinding_selected, 0);
    assert!(!model.keybinding_capturing);
}

#[test]
fn test_hide_keybindings_editor() {
    let mut model = Model::new();
    model.show_keybindings_editor = true;
    model.keybinding_capturing = true;

    update(&mut model, Message::Ui(UiMessage::HideKeybindingsEditor));
    assert!(!model.show_keybindings_editor);
    assert!(!model.keybinding_capturing);
}

#[test]
fn test_keybindings_navigation() {
    let mut model = Model::new();
    model.show_keybindings_editor = true;
    model.keybinding_selected = 5;

    update(&mut model, Message::Ui(UiMessage::KeybindingsUp));
    assert_eq!(model.keybinding_selected, 4);

    update(&mut model, Message::Ui(UiMessage::KeybindingsDown));
    assert_eq!(model.keybinding_selected, 5);

    // Navigate up at 0 should stay at 0
    model.keybinding_selected = 0;
    update(&mut model, Message::Ui(UiMessage::KeybindingsUp));
    assert_eq!(model.keybinding_selected, 0);
}

#[test]
fn test_start_edit_keybinding() {
    let mut model = Model::new();
    model.show_keybindings_editor = true;

    update(&mut model, Message::Ui(UiMessage::StartEditKeybinding));
    assert!(model.keybinding_capturing);
    assert!(model.status_message.is_some());
}

#[test]
fn test_cancel_edit_keybinding() {
    let mut model = Model::new();
    model.show_keybindings_editor = true;
    model.keybinding_capturing = true;
    model.status_message = Some("Press a key...".to_string());

    update(&mut model, Message::Ui(UiMessage::CancelEditKeybinding));
    assert!(!model.keybinding_capturing);
    assert!(model.status_message.is_none());
}

#[test]
fn test_apply_keybinding() {
    let mut model = Model::new();
    model.show_keybindings_editor = true;
    model.keybinding_capturing = true;

    // Get the first binding's action
    let bindings = model.keybindings.sorted_bindings();
    let (_, first_action) = &bindings[0];
    let original_action = first_action.clone();

    // Apply a new key to that action
    update(
        &mut model,
        Message::Ui(UiMessage::ApplyKeybinding("z".to_string())),
    );

    assert!(!model.keybinding_capturing);
    // The action should now be bound to 'z'
    assert_eq!(model.keybindings.get_action("z"), Some(&original_action));
}

#[test]
fn test_reset_all_keybindings() {
    let mut model = Model::new();
    model.show_keybindings_editor = true;

    // Modify a keybinding
    model.keybindings.set_binding("z".to_string(), Action::Quit);

    // Verify it was changed
    assert_eq!(model.keybindings.get_action("z"), Some(&Action::Quit));

    // Reset all
    update(&mut model, Message::Ui(UiMessage::ResetAllKeybindings));

    // Should be back to default (z is not a default binding)
    assert_eq!(model.keybindings.get_action("z"), None);
    assert!(model.status_message.is_some());
}
