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
    CreateSubtask,
    CreateProject,
    EditTask,
    EditDueDate,
    EditScheduledDate,
    EditTags,
    EditDescription,
    DeleteTask,
    CyclePriority,
    MoveToProject,

    // Time tracking
    ToggleTimeTracking,

    // UI actions
    ToggleSidebar,
    ToggleShowCompleted,
    ShowHelp,
    ToggleFocusMode,
    FocusSidebar,
    FocusTaskList,
    Select,
    Search,
    ClearSearch,
    FilterByTag,
    ClearTagFilter,
    CycleSortField,
    ToggleSortOrder,

    // Multi-select / Bulk operations
    ToggleMultiSelect,
    ToggleTaskSelection,
    SelectAll,
    ClearSelection,
    BulkDelete,
    BulkMoveToProject,
    BulkSetStatus,

    // Dependencies
    EditDependencies,

    // Recurrence
    EditRecurrence,

    // Manual ordering
    MoveTaskUp,
    MoveTaskDown,

    // Task chains
    LinkTask,
    UnlinkTask,

    // Calendar navigation
    CalendarPrevMonth,
    CalendarNextMonth,
    CalendarPrevDay,
    CalendarNextDay,

    // System
    Save,
    Undo,
    Redo,
    Quit,

    // Export
    ExportCsv,
    ExportIcs,
    ExportChainsDot,
    ExportChainsMermaid,

    // Macros
    RecordMacro,
    StopRecordMacro,
    PlayMacro0,
    PlayMacro1,
    PlayMacro2,
    PlayMacro3,
    PlayMacro4,
    PlayMacro5,
    PlayMacro6,
    PlayMacro7,
    PlayMacro8,
    PlayMacro9,

    // Templates
    ShowTemplates,

    // Keybindings editor
    ShowKeybindingsEditor,
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
        bindings.insert("A".to_string(), Action::CreateSubtask);
        bindings.insert("P".to_string(), Action::CreateProject);
        bindings.insert("e".to_string(), Action::EditTask);
        bindings.insert("D".to_string(), Action::EditDueDate);
        bindings.insert("S".to_string(), Action::EditScheduledDate);
        bindings.insert("T".to_string(), Action::EditTags);
        bindings.insert("n".to_string(), Action::EditDescription);
        bindings.insert("d".to_string(), Action::DeleteTask);
        bindings.insert("p".to_string(), Action::CyclePriority);
        bindings.insert("m".to_string(), Action::MoveToProject);

        // Time tracking
        bindings.insert("t".to_string(), Action::ToggleTimeTracking);

        // UI actions
        bindings.insert("b".to_string(), Action::ToggleSidebar);
        bindings.insert("c".to_string(), Action::ToggleShowCompleted);
        bindings.insert("?".to_string(), Action::ShowHelp);
        bindings.insert("f".to_string(), Action::ToggleFocusMode);
        bindings.insert("h".to_string(), Action::FocusSidebar);
        bindings.insert("l".to_string(), Action::FocusTaskList);
        bindings.insert("left".to_string(), Action::FocusSidebar);
        bindings.insert("right".to_string(), Action::FocusTaskList);
        bindings.insert("enter".to_string(), Action::Select);
        bindings.insert("/".to_string(), Action::Search);
        bindings.insert("ctrl+l".to_string(), Action::ClearSearch);
        bindings.insert("#".to_string(), Action::FilterByTag);
        bindings.insert("ctrl+t".to_string(), Action::ClearTagFilter);
        bindings.insert("s".to_string(), Action::CycleSortField);
        bindings.insert("ctrl+s".to_string(), Action::ToggleSortOrder);

        // Multi-select / Bulk operations
        bindings.insert("v".to_string(), Action::ToggleMultiSelect);
        bindings.insert("V".to_string(), Action::SelectAll);
        bindings.insert("ctrl+v".to_string(), Action::ClearSelection);

        // Dependencies
        bindings.insert("B".to_string(), Action::EditDependencies);

        // Recurrence
        bindings.insert("R".to_string(), Action::EditRecurrence);

        // Manual ordering
        bindings.insert("ctrl+up".to_string(), Action::MoveTaskUp);
        bindings.insert("ctrl+down".to_string(), Action::MoveTaskDown);

        // Task chains
        bindings.insert("ctrl+l".to_string(), Action::LinkTask);
        bindings.insert("ctrl+shift+l".to_string(), Action::UnlinkTask);

        // Calendar navigation
        bindings.insert("<".to_string(), Action::CalendarPrevMonth);
        bindings.insert(">".to_string(), Action::CalendarNextMonth);

        // System
        bindings.insert("ctrl+s".to_string(), Action::Save);
        bindings.insert("u".to_string(), Action::Undo);
        bindings.insert("ctrl+z".to_string(), Action::Undo);
        bindings.insert("ctrl+r".to_string(), Action::Redo);
        bindings.insert("U".to_string(), Action::Redo);
        bindings.insert("q".to_string(), Action::Quit);
        bindings.insert("esc".to_string(), Action::Quit);

        // Export
        bindings.insert("ctrl+e".to_string(), Action::ExportCsv);
        bindings.insert("ctrl+i".to_string(), Action::ExportIcs);
        bindings.insert("ctrl+g".to_string(), Action::ExportChainsDot);
        bindings.insert("ctrl+m".to_string(), Action::ExportChainsMermaid);

        // Macros - q to record, Q to stop, @0-9 to play
        bindings.insert("ctrl+q".to_string(), Action::RecordMacro);
        bindings.insert("ctrl+Q".to_string(), Action::StopRecordMacro);
        bindings.insert("@0".to_string(), Action::PlayMacro0);
        bindings.insert("@1".to_string(), Action::PlayMacro1);
        bindings.insert("@2".to_string(), Action::PlayMacro2);
        bindings.insert("@3".to_string(), Action::PlayMacro3);
        bindings.insert("@4".to_string(), Action::PlayMacro4);
        bindings.insert("@5".to_string(), Action::PlayMacro5);
        bindings.insert("@6".to_string(), Action::PlayMacro6);
        bindings.insert("@7".to_string(), Action::PlayMacro7);
        bindings.insert("@8".to_string(), Action::PlayMacro8);
        bindings.insert("@9".to_string(), Action::PlayMacro9);

        // Templates
        bindings.insert("ctrl+n".to_string(), Action::ShowTemplates);

        // Keybindings editor
        bindings.insert("ctrl+k".to_string(), Action::ShowKeybindingsEditor);

        Self { bindings }
    }
}

impl Keybindings {
    /// Load keybindings from the default config path
    #[must_use]
    pub fn load() -> Self {
        Self::load_from_path(Self::config_path())
    }

    /// Load keybindings from a specific path
    #[must_use]
    pub fn load_from_path(path: PathBuf) -> Self {
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(keybindings) => return keybindings,
                    Err(e) => eprintln!("Warning: Failed to parse keybindings: {e}"),
                },
                Err(e) => eprintln!("Warning: Failed to read keybindings: {e}"),
            }
        }
        Self::default()
    }

    /// Get the default keybindings file path
    #[must_use]
    pub fn config_path() -> PathBuf {
        Settings::config_dir().join("keybindings.toml")
    }

    /// Look up action for a key
    #[must_use]
    pub fn get_action(&self, key: &str) -> Option<&Action> {
        self.bindings.get(key)
    }

    /// Returns a sorted list of (key, action) pairs for display
    #[must_use]
    pub fn sorted_bindings(&self) -> Vec<(String, Action)> {
        let mut pairs: Vec<_> = self
            .bindings
            .iter()
            .map(|(k, a)| (k.clone(), a.clone()))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs
    }

    /// Set a keybinding for an action
    pub fn set_binding(&mut self, key: String, action: Action) {
        // Remove any existing binding for this action
        self.bindings.retain(|_, a| a != &action);
        // Add the new binding
        self.bindings.insert(key, action);
    }

    /// Find the key bound to an action
    #[must_use]
    pub fn key_for_action(&self, action: &Action) -> Option<&String> {
        self.bindings
            .iter()
            .find(|(_, a)| *a == action)
            .map(|(k, _)| k)
    }

    /// Save keybindings to the config file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, content)
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
