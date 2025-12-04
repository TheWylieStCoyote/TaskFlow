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
    DeleteTask,

    // UI actions
    ToggleSidebar,
    ToggleShowCompleted,
    ShowHelp,

    // System
    Save,
    Quit,
}

/// Key modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modifier {
    None,
    Ctrl,
    Alt,
    Shift,
}

impl Default for Modifier {
    fn default() -> Self {
        Self::None
    }
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
        bindings.insert("d".to_string(), Action::DeleteTask);

        // UI actions
        bindings.insert("b".to_string(), Action::ToggleSidebar);
        bindings.insert("c".to_string(), Action::ToggleShowCompleted);
        bindings.insert("?".to_string(), Action::ShowHelp);

        // System
        bindings.insert("ctrl+s".to_string(), Action::Save);
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
