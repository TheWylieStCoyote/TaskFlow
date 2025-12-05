use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::Settings;

/// Action that can be triggered by a keybinding
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    // Navigation
    MoveUp,
    MoveDown,
    MoveFirst,
    MoveLast,
    PageUp,
    PageDown,

    // Task actions
    ToggleComplete,
    CreateTask,
    CreateProject,
    EditTask,
    EditDueDate,
    EditTags,
    DeleteTask,
    CyclePriority,

    // Time tracking
    ToggleTimeTracking,

    // UI actions
    ToggleSidebar,
    ToggleShowCompleted,
    ShowHelp,
    FocusSidebar,
    FocusTaskList,
    Select,
    Search,
    ClearSearch,
    CycleSortField,
    ToggleSortOrder,

    // System
    Save,
    Undo,
    Redo,
    Quit,
}

/// Key modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modifier {
    #[default]
    None,
    Ctrl,
    Alt,
    Shift,
}

/// A key combination
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyBinding {
    /// The key character or name
    pub key: String,

    /// Modifier key
    #[serde(default)]
    pub modifier: Modifier,
}

impl KeyBinding {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            modifier: Modifier::None,
        }
    }

    pub fn with_ctrl(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            modifier: Modifier::Ctrl,
        }
    }

    pub fn with_shift(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            modifier: Modifier::Shift,
        }
    }
}

/// Keybindings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Keybindings {
    /// Map of keybindings to actions
    pub bindings: HashMap<String, Action>,
}

impl Default for Keybindings {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // Navigation
        bindings.insert("j".to_string(), Action::MoveDown);
        bindings.insert("k".to_string(), Action::MoveUp);
        bindings.insert("down".to_string(), Action::MoveDown);
        bindings.insert("up".to_string(), Action::MoveUp);
        bindings.insert("g".to_string(), Action::MoveFirst);
        bindings.insert("G".to_string(), Action::MoveLast);
        bindings.insert("ctrl+u".to_string(), Action::PageUp);
        bindings.insert("ctrl+d".to_string(), Action::PageDown);
        bindings.insert("pageup".to_string(), Action::PageUp);
        bindings.insert("pagedown".to_string(), Action::PageDown);

        // Task actions
        bindings.insert("x".to_string(), Action::ToggleComplete);
        bindings.insert("space".to_string(), Action::ToggleComplete);
        bindings.insert("a".to_string(), Action::CreateTask);
        bindings.insert("P".to_string(), Action::CreateProject);
        bindings.insert("e".to_string(), Action::EditTask);
        bindings.insert("D".to_string(), Action::EditDueDate);
        bindings.insert("T".to_string(), Action::EditTags);
        bindings.insert("d".to_string(), Action::DeleteTask);
        bindings.insert("p".to_string(), Action::CyclePriority);

        // Time tracking
        bindings.insert("t".to_string(), Action::ToggleTimeTracking);

        // UI actions
        bindings.insert("b".to_string(), Action::ToggleSidebar);
        bindings.insert("c".to_string(), Action::ToggleShowCompleted);
        bindings.insert("?".to_string(), Action::ShowHelp);
        bindings.insert("h".to_string(), Action::FocusSidebar);
        bindings.insert("l".to_string(), Action::FocusTaskList);
        bindings.insert("left".to_string(), Action::FocusSidebar);
        bindings.insert("right".to_string(), Action::FocusTaskList);
        bindings.insert("enter".to_string(), Action::Select);
        bindings.insert("/".to_string(), Action::Search);
        bindings.insert("ctrl+l".to_string(), Action::ClearSearch);
        bindings.insert("s".to_string(), Action::CycleSortField);
        bindings.insert("S".to_string(), Action::ToggleSortOrder);

        // System
        bindings.insert("ctrl+s".to_string(), Action::Save);
        bindings.insert("u".to_string(), Action::Undo);
        bindings.insert("ctrl+z".to_string(), Action::Undo);
        bindings.insert("ctrl+r".to_string(), Action::Redo);
        bindings.insert("U".to_string(), Action::Redo);
        bindings.insert("q".to_string(), Action::Quit);
        bindings.insert("esc".to_string(), Action::Quit);

        Self { bindings }
    }
}

impl Keybindings {
    /// Load keybindings from the default config path
    pub fn load() -> Self {
        Self::load_from_path(Self::config_path())
    }

    /// Load keybindings from a specific path
    pub fn load_from_path(path: PathBuf) -> Self {
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(keybindings) => return keybindings,
                    Err(e) => eprintln!("Warning: Failed to parse keybindings: {}", e),
                },
                Err(e) => eprintln!("Warning: Failed to read keybindings: {}", e),
            }
        }
        Self::default()
    }

    /// Get the default keybindings file path
    pub fn config_path() -> PathBuf {
        Settings::config_dir().join("keybindings.toml")
    }

    /// Look up action for a key
    pub fn get_action(&self, key: &str) -> Option<&Action> {
        self.bindings.get(key)
    }
}

#[cfg(test)]
mod tests {
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
        assert_eq!(kb.get_action("esc"), Some(&Action::Quit));
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
}
