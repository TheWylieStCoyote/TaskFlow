//! Tests for keybindings module.

use super::*;
use tempfile::tempdir;

#[test]
fn test_keybinding_new() {
    let kb = KeyBinding::new("j");
    assert_eq!(kb.key, "j");
    assert_eq!(kb.modifier, Modifier::None);
}

#[test]
fn test_keybinding_with_ctrl() {
    let kb = KeyBinding::with_ctrl("s");
    assert_eq!(kb.key, "s");
    assert_eq!(kb.modifier, Modifier::Ctrl);
}

#[test]
fn test_keybinding_with_shift() {
    let kb = KeyBinding::with_shift("G");
    assert_eq!(kb.key, "G");
    assert_eq!(kb.modifier, Modifier::Shift);
}

#[test]
fn test_keybindings_default_navigation() {
    let kb = Keybindings::default();

    assert_eq!(kb.get_action("j"), Some(&Action::MoveDown));
    assert_eq!(kb.get_action("k"), Some(&Action::MoveUp));
    assert_eq!(kb.get_action("up"), Some(&Action::MoveUp));
    assert_eq!(kb.get_action("down"), Some(&Action::MoveDown));
    assert_eq!(kb.get_action("g"), Some(&Action::MoveFirst));
    assert_eq!(kb.get_action("G"), Some(&Action::MoveLast));
}

#[test]
fn test_keybindings_default_tasks() {
    let kb = Keybindings::default();

    assert_eq!(kb.get_action("x"), Some(&Action::ToggleComplete));
    assert_eq!(kb.get_action("space"), Some(&Action::ToggleComplete));
    assert_eq!(kb.get_action("a"), Some(&Action::CreateTask));
    assert_eq!(kb.get_action("d"), Some(&Action::DeleteTask));
    assert_eq!(kb.get_action("t"), Some(&Action::ToggleTimeTracking));
}

#[test]
fn test_keybindings_default_system() {
    let kb = Keybindings::default();

    assert_eq!(kb.get_action("q"), Some(&Action::Quit));
    // esc is not bound by default (handled specially in input.rs for modals)
    assert_eq!(kb.get_action("ctrl+s"), Some(&Action::Save));
}

#[test]
fn test_keybindings_get_action() {
    let kb = Keybindings::default();

    assert_eq!(kb.get_action("j"), Some(&Action::MoveDown));
    assert_eq!(kb.get_action("?"), Some(&Action::ShowHelp));
}

#[test]
fn test_keybindings_get_action_unknown() {
    let kb = Keybindings::default();

    assert_eq!(kb.get_action("unknown_key"), None);
}

#[test]
fn test_keybindings_load_missing_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nonexistent.toml");

    let kb = Keybindings::load_from_path(path);

    // Should return defaults
    assert_eq!(kb.get_action("j"), Some(&Action::MoveDown));
}

#[test]
fn test_keybindings_load_custom() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("keybindings.toml");

    let content = r#"
[bindings]
j = "move_down"
k = "move_up"
z = "quit"
"#;
    std::fs::write(&path, content).unwrap();

    let kb = Keybindings::load_from_path(path);

    assert_eq!(kb.get_action("j"), Some(&Action::MoveDown));
    assert_eq!(kb.get_action("k"), Some(&Action::MoveUp));
    assert_eq!(kb.get_action("z"), Some(&Action::Quit));
}

#[test]
fn test_modifier_default() {
    let modifier = Modifier::default();
    assert_eq!(modifier, Modifier::None);
}

#[test]
fn test_find_conflict_exists() {
    let kb = Keybindings::default();

    // "j" is bound to MoveDown by default
    assert_eq!(kb.find_conflict("j"), Some(&Action::MoveDown));
}

#[test]
fn test_find_conflict_none() {
    let kb = Keybindings::default();

    // "1" is not bound by default
    assert_eq!(kb.find_conflict("1"), None);
}

#[test]
fn test_set_binding_checked_no_conflict() {
    let mut kb = Keybindings::default();

    // "1" is not bound, so no conflict
    let previous = kb.set_binding_checked("1".to_string(), Action::Quit);
    assert!(previous.is_none());
    assert_eq!(kb.get_action("1"), Some(&Action::Quit));
}

#[test]
fn test_set_binding_checked_with_conflict() {
    let mut kb = Keybindings::default();

    // "j" is already bound to MoveDown
    let previous = kb.set_binding_checked("j".to_string(), Action::Search);
    assert_eq!(previous, Some(Action::MoveDown));
    assert_eq!(kb.get_action("j"), Some(&Action::Search));
}

#[test]
fn test_set_binding_checked_removes_old_action_binding() {
    let mut kb = Keybindings::default();

    // "j" is bound to MoveDown, now bind "1" to MoveDown
    let previous = kb.set_binding_checked("1".to_string(), Action::MoveDown);
    assert!(previous.is_none()); // "1" wasn't bound before

    // "j" should no longer be bound to MoveDown (action can only have one key)
    assert_eq!(kb.get_action("j"), None);
    assert_eq!(kb.get_action("1"), Some(&Action::MoveDown));
}

#[test]
fn test_swap_bindings() {
    let mut kb = Keybindings::default();

    // "j" = MoveDown, "k" = MoveUp
    assert_eq!(kb.get_action("j"), Some(&Action::MoveDown));
    assert_eq!(kb.get_action("k"), Some(&Action::MoveUp));

    kb.swap_bindings("j", "k");

    // After swap: "j" = MoveUp, "k" = MoveDown
    assert_eq!(kb.get_action("j"), Some(&Action::MoveUp));
    assert_eq!(kb.get_action("k"), Some(&Action::MoveDown));
}

#[test]
fn test_remove_binding() {
    let mut kb = Keybindings::default();

    assert_eq!(kb.get_action("j"), Some(&Action::MoveDown));

    let removed = kb.remove_binding("j");
    assert_eq!(removed, Some(Action::MoveDown));
    assert_eq!(kb.get_action("j"), None);
}

#[test]
fn test_remove_binding_nonexistent() {
    let mut kb = Keybindings::default();

    let removed = kb.remove_binding("nonexistent");
    assert!(removed.is_none());
}
