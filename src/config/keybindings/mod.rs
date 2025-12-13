//! Keybindings configuration.

mod action;
mod defaults;
mod key_binding;

#[cfg(test)]
mod tests;

pub use action::{Action, ActionCategory, ALL_ACTIONS};
pub use key_binding::{KeyBinding, Modifier};

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::warn;

use super::Settings;

/// Keybindings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Keybindings {
    /// Map of keybindings to actions
    pub bindings: HashMap<String, Action>,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            bindings: defaults::default_bindings(),
        }
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
                    Err(e) => warn!("Failed to parse keybindings: {e}"),
                },
                Err(e) => warn!("Failed to read keybindings: {e}"),
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

    /// Get all bindings grouped by category, sorted for display in help
    ///
    /// Returns a Vec of (category, Vec<(key, action, description)>) sorted by category order
    #[must_use]
    #[allow(clippy::type_complexity)]
    pub fn bindings_by_category(
        &self,
    ) -> Vec<(ActionCategory, Vec<(String, &Action, &'static str)>)> {
        use std::collections::BTreeMap;

        // Type alias for the grouped bindings
        type GroupedBindings<'a> = (ActionCategory, Vec<(String, &'a Action, &'static str)>);

        // Group bindings by category
        let mut groups: BTreeMap<u8, GroupedBindings<'_>> = BTreeMap::new();

        for (key, action) in &self.bindings {
            let category = action.category();
            let order = category.display_order();
            let description = action.description();

            groups
                .entry(order)
                .or_insert_with(|| (category, Vec::new()))
                .1
                .push((key.clone(), action, description));
        }

        // Sort each group's bindings alphabetically by key
        for (_, bindings) in groups.values_mut() {
            bindings.sort_by(|a, b| a.0.cmp(&b.0));
        }

        // Convert to Vec, already sorted by category order due to BTreeMap
        groups.into_values().collect()
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

    /// Check if a key is already bound to an action
    ///
    /// Returns the conflicting action if the key is already bound.
    #[must_use]
    pub fn find_conflict(&self, key: &str) -> Option<&Action> {
        self.bindings.get(key)
    }

    /// Set a binding with conflict detection
    ///
    /// Returns the previous action if the key was already bound (conflict).
    /// The binding is still set - caller should handle the conflict.
    pub fn set_binding_checked(&mut self, key: String, action: Action) -> Option<Action> {
        // First, check if this key is already bound to something else
        let previous = self.bindings.get(&key).cloned();

        // Remove any existing binding for this action (an action can only have one key)
        self.bindings.retain(|_, a| a != &action);

        // Add the new binding
        self.bindings.insert(key, action);

        // Return the previous action if there was one (and it was different)
        previous
    }

    /// Swap bindings between two keys
    ///
    /// If key1 is bound to action1 and key2 is bound to action2,
    /// after swap: key1 -> action2, key2 -> action1
    pub fn swap_bindings(&mut self, key1: &str, key2: &str) {
        let action1 = self.bindings.get(key1).cloned();
        let action2 = self.bindings.get(key2).cloned();

        if let Some(a1) = action1 {
            if let Some(a2) = action2 {
                self.bindings.insert(key1.to_string(), a2);
                self.bindings.insert(key2.to_string(), a1);
            }
        }
    }

    /// Remove binding for a specific key
    pub fn remove_binding(&mut self, key: &str) -> Option<Action> {
        self.bindings.remove(key)
    }

    /// Validate keybindings for issues
    ///
    /// Returns a list of warnings (not errors, since the app can still work).
    /// Currently checks for:
    /// - Actions without any key binding
    #[must_use]
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        // Check if any standard actions are missing bindings
        let defaults = defaults::default_bindings();
        for (default_key, action) in &defaults {
            if self.key_for_action(action).is_none() {
                warnings.push(format!(
                    "Action {action:?} has no keybinding (default was '{default_key}')"
                ));
            }
        }

        warnings
    }

    /// Find all conflicts (keys that would be displaced) if a new binding is added
    ///
    /// Returns (conflicting_action, the_action_that_would_lose_its_binding)
    #[must_use]
    pub fn find_all_conflicts(&self, new_key: &str, new_action: &Action) -> Vec<String> {
        let mut conflicts = Vec::new();

        // Check if the key is already bound to another action
        if let Some(existing) = self.bindings.get(new_key) {
            if existing != new_action {
                conflicts.push(format!(
                    "Key '{new_key}' is currently bound to {existing:?}"
                ));
            }
        }

        // Check if the action already has a different key
        if let Some(current_key) = self.key_for_action(new_action) {
            if current_key != new_key {
                conflicts.push(format!(
                    "Action {new_action:?} will be unbound from '{current_key}'"
                ));
            }
        }

        conflicts
    }
}
