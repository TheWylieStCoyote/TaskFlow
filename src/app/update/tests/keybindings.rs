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

// ============================================================================
// Reset / conflict / bounds branches
// ============================================================================

mod remaining_branches {
    use crate::app::{update::update, Message, Model, UiMessage};
    use crate::config::{Action, Keybindings};

    fn ui(model: &mut Model, msg: UiMessage) {
        update(model, Message::Ui(msg));
    }

    fn index_of_action(model: &Model, action: &Action) -> usize {
        model
            .keybindings
            .sorted_bindings()
            .iter()
            .position(|(_, a)| a == action)
            .expect("action is bound")
    }

    #[test]
    fn test_keybindings_up_at_zero_and_down_at_end_stay_in_bounds() {
        let mut model = Model::new();
        model.keybindings_editor.visible = true;
        model.keybindings_editor.selected = 0;
        ui(&mut model, UiMessage::KeybindingsUp);
        assert_eq!(model.keybindings_editor.selected, 0);

        let last = model.keybindings.sorted_bindings().len() - 1;
        model.keybindings_editor.selected = last;
        ui(&mut model, UiMessage::KeybindingsDown);
        assert_eq!(model.keybindings_editor.selected, last);
    }

    #[test]
    fn test_reset_keybinding_restores_default_key() {
        let mut model = Model::new();
        model.keybindings_editor.visible = true;
        let default_key = Keybindings::default()
            .key_for_action(&Action::Quit)
            .cloned()
            .expect("quit has a default");

        // Rebind quit to an unused key, then reset the quit row
        model
            .keybindings
            .set_binding("ctrl+shift+f12".to_string(), Action::Quit);
        assert_eq!(model.keybindings.get_action(&default_key), None);
        model.keybindings_editor.selected = index_of_action(&model, &Action::Quit);

        ui(&mut model, UiMessage::ResetKeybinding);

        assert_eq!(
            model.keybindings.get_action(&default_key),
            Some(&Action::Quit)
        );
        assert_eq!(model.keybindings.get_action("ctrl+shift+f12"), None);
        // Moving the action off its temporary key is reported as a conflict note
        let msg = model.alerts.status_message.clone().unwrap_or_default();
        assert!(
            msg.starts_with(&format!("Reset Quit to '{default_key}'. ")),
            "got {msg:?}"
        );
        assert!(msg.contains("ctrl+shift+f12"));

        // Resetting an action that already sits on its default key is clean
        model.keybindings_editor.selected = index_of_action(&model, &Action::Quit);
        ui(&mut model, UiMessage::ResetKeybinding);
        assert_eq!(
            model.alerts.status_message.as_deref(),
            Some(format!("Reset Quit to default key '{default_key}'").as_str())
        );
    }

    #[test]
    fn test_reset_keybinding_reports_conflict_when_default_key_taken() {
        let mut model = Model::new();
        model.keybindings_editor.visible = true;
        let default_key = Keybindings::default()
            .key_for_action(&Action::Quit)
            .cloned()
            .expect("quit has a default");

        // Move quit away and give its default key to another action
        model
            .keybindings
            .set_binding("ctrl+shift+f11".to_string(), Action::Quit);
        model
            .keybindings
            .set_binding(default_key.clone(), Action::ShowHelp);
        model.keybindings_editor.selected = index_of_action(&model, &Action::Quit);

        ui(&mut model, UiMessage::ResetKeybinding);

        assert_eq!(
            model.keybindings.get_action(&default_key),
            Some(&Action::Quit)
        );
        let msg = model.alerts.status_message.clone().unwrap_or_default();
        assert!(
            msg.starts_with(&format!("Reset Quit to '{default_key}'. ")),
            "conflict message expected, got {msg:?}"
        );
    }

    #[test]
    fn test_reset_keybinding_without_default_reports_message() {
        let mut model = Model::new();
        model.keybindings_editor.visible = true;
        let no_default = Action::BulkDelete;
        assert!(Keybindings::default().key_for_action(&no_default).is_none());
        model
            .keybindings
            .set_binding("ctrl+shift+f10".to_string(), no_default.clone());
        model.keybindings_editor.selected = index_of_action(&model, &no_default);

        ui(&mut model, UiMessage::ResetKeybinding);

        assert_eq!(
            model.alerts.status_message.as_deref(),
            Some("No default binding for this action")
        );
        assert_eq!(
            model.keybindings.get_action("ctrl+shift+f10"),
            Some(&no_default)
        );
    }

    #[test]
    fn test_reset_keybinding_with_out_of_range_selection_is_noop() {
        let mut model = Model::new();
        model.keybindings_editor.visible = true;
        model.keybindings_editor.selected = 10_000;
        ui(&mut model, UiMessage::ResetKeybinding);
        assert!(model.alerts.status_message.is_none());
    }

    #[test]
    fn test_apply_keybinding_reports_conflict() {
        let mut model = Model::new();
        model.keybindings_editor.visible = true;
        model.keybindings_editor.capturing = true;
        let quit_key = model
            .keybindings
            .key_for_action(&Action::Quit)
            .cloned()
            .expect("quit bound");
        model.keybindings_editor.selected = index_of_action(&model, &Action::ShowHelp);

        ui(&mut model, UiMessage::ApplyKeybinding(quit_key.clone()));

        assert_eq!(
            model.keybindings.get_action(&quit_key),
            Some(&Action::ShowHelp)
        );
        assert!(!model.keybindings_editor.capturing);
        let msg = model.alerts.status_message.clone().unwrap_or_default();
        assert!(
            msg.starts_with(&format!("Bound '{quit_key}' to ShowHelp. ")),
            "got {msg:?}"
        );
    }

    #[test]
    fn test_apply_keybinding_with_out_of_range_selection_only_stops_capture() {
        let mut model = Model::new();
        model.keybindings_editor.visible = true;
        model.keybindings_editor.capturing = true;
        model.keybindings_editor.selected = 10_000;

        ui(
            &mut model,
            UiMessage::ApplyKeybinding("ctrl+shift+f9".to_string()),
        );

        assert!(!model.keybindings_editor.capturing);
        assert_eq!(model.keybindings.get_action("ctrl+shift+f9"), None);
    }

    #[test]
    fn test_show_keybindings_editor_resets_selection_and_capture() {
        let mut model = Model::new();
        model.keybindings_editor.selected = 5;
        model.keybindings_editor.capturing = true;
        ui(&mut model, UiMessage::ShowKeybindingsEditor);
        assert!(model.keybindings_editor.visible);
        assert_eq!(model.keybindings_editor.selected, 0);
        assert!(!model.keybindings_editor.capturing);
    }
}
