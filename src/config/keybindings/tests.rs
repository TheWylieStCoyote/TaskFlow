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

// ---------------------------------------------------------------------------
// Default-table invariants
// ---------------------------------------------------------------------------

/// Named keys the input layer can produce (see `key_event_to_string` in the
/// binary). Anything else must be a single character or a `ctrl+`/`alt+`
/// prefixed one.
const NAMED_KEYS: &[&str] = &[
    "space",
    "enter",
    "esc",
    "backspace",
    "delete",
    "up",
    "down",
    "left",
    "right",
    "home",
    "end",
    "pageup",
    "pagedown",
    "tab",
];

fn is_named_key(base: &str) -> bool {
    NAMED_KEYS.contains(&base)
        || (base.len() > 1 && base.starts_with('f') && base[1..].parse::<u8>().is_ok())
}

#[test]
fn default_bindings_have_unique_keys() {
    let mut seen = std::collections::HashMap::new();
    for (key, action) in super::defaults::DEFAULT_BINDINGS {
        if let Some(previous) = seen.insert(*key, action) {
            panic!("default key {key:?} is bound to both {previous:?} and {action:?}");
        }
    }
    // And the map really does contain every entry.
    assert_eq!(Keybindings::default().bindings.len(), seen.len());
}

#[test]
fn default_bindings_are_reachable_from_key_events() {
    for (key, action) in super::defaults::DEFAULT_BINDINGS {
        // "+" on its own is a key, not a separator.
        let (mods, base): (Vec<&str>, &str) = if *key == "+" {
            (Vec::new(), "+")
        } else {
            key.rsplit_once('+')
                .map_or((Vec::new(), *key), |(m, b)| (m.split('+').collect(), b))
        };

        for m in &mods {
            assert!(
                matches!(*m, "ctrl" | "alt" | "shift"),
                "{key:?} ({action:?}): unknown modifier {m:?}"
            );
        }

        let is_single_char = base.chars().count() == 1;
        let is_chord = mods.is_empty()
            && base.chars().count() == 2
            && !base.chars().next().unwrap().is_alphanumeric();
        assert!(
            is_single_char || is_named_key(base) || is_chord,
            "{key:?} ({action:?}) can never be produced by a key event"
        );
        if mods.contains(&"shift") {
            assert!(
                !is_single_char,
                "{key:?} ({action:?}): shift is never emitted for characters; use the uppercase char"
            );
        }
    }
}

#[test]
fn chord_prefix_detection() {
    let kb = Keybindings::default();
    assert!(kb.is_chord_prefix("@"), "@ starts the @0..@9 macro chords");
    // Bound on its own, so not a prefix even though "ctrl+..." etc. exist.
    assert!(!kb.is_chord_prefix("c"));
    assert!(!kb.is_chord_prefix("s")); // "space" starts with s but s is a letter
    assert!(!kb.is_chord_prefix("+")); // bound directly
    assert!(!kb.is_chord_prefix("?"));
    assert!(!kb.is_chord_prefix(""));
    assert!(!kb.is_chord_prefix("ctrl"));
    assert_eq!(kb.get_action("@3"), Some(&Action::PlayMacro3));
}

#[test]
fn documented_keys_resolve_to_documented_actions() {
    // Regression guard for the collisions fixed in the default table.
    let kb = Keybindings::default();
    assert_eq!(kb.get_action("E"), Some(&Action::EditProject));
    assert_eq!(kb.get_action("D"), Some(&Action::EditDueDate));
    assert_eq!(kb.get_action("M"), Some(&Action::RescheduleNextMonday));
    assert_eq!(kb.get_action("Q"), Some(&Action::AddToFocusQueue));
    assert_eq!(kb.get_action("ctrl+l"), Some(&Action::ClearSearch));
    assert_eq!(kb.get_action("ctrl+s"), Some(&Action::Save));
    assert_eq!(kb.get_action("f5"), Some(&Action::PomodoroStart));
    assert!(kb.key_for_action(&Action::EditEstimate).is_some());
    assert!(kb.key_for_action(&Action::LinkTask).is_some());
    assert!(kb.key_for_action(&Action::UnlinkTask).is_some());
    assert!(kb.key_for_action(&Action::RefreshStorage).is_some());
    assert!(kb.key_for_action(&Action::ToggleSortOrder).is_some());
    assert!(kb.key_for_action(&Action::QuickCapture).is_some());
    // Subtask collapse lives on alt+z; z/Z stay snooze / clear snooze
    assert_eq!(kb.get_action("alt+z"), Some(&Action::ToggleSubtaskCollapse));
    assert_eq!(kb.get_action("z"), Some(&Action::SnoozeTask));
    assert_eq!(kb.get_action("Z"), Some(&Action::ClearSnooze));
    // The calendar's `s` (schedule to day) is view-scoped, not a global binding
    assert_eq!(kb.get_action("s"), Some(&Action::CycleSortField));
    assert!(kb.key_for_action(&Action::ScheduleTaskToDay).is_none());
}
