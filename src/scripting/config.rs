//! Configuration types for the scripting system.

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::error::{ScriptError, ScriptResult};

/// Root configuration for the scripting system.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ScriptConfig {
    /// Global settings.
    #[serde(default)]
    pub settings: ScriptSettings,

    /// Hook configurations keyed by hook name.
    #[serde(default)]
    pub hooks: HashMap<String, HookConfig>,

    /// Custom command configurations.
    #[serde(default)]
    pub commands: HashMap<String, CommandConfig>,
}

impl ScriptConfig {
    /// Loads configuration from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: &Path) -> ScriptResult<Self> {
        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Returns an empty configuration (no hooks or commands).
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Gets a hook by name if it exists and is enabled.
    #[must_use]
    pub fn get_hook(&self, name: &str) -> Option<&HookConfig> {
        self.hooks
            .get(name)
            .filter(|h| h.enabled && self.settings.enabled)
    }

    /// Gets a command by name.
    #[must_use]
    pub fn get_command(&self, name: &str) -> Option<&CommandConfig> {
        self.commands.get(name).filter(|_| self.settings.enabled)
    }

    /// Returns all enabled hook names.
    #[must_use]
    pub fn enabled_hooks(&self) -> Vec<&str> {
        if !self.settings.enabled {
            return Vec::new();
        }
        self.hooks
            .iter()
            .filter(|(_, h)| h.enabled)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Returns all command names with descriptions.
    #[must_use]
    pub fn command_list(&self) -> Vec<(&str, &str)> {
        if !self.settings.enabled {
            return Vec::new();
        }
        self.commands
            .iter()
            .map(|(name, cmd)| (name.as_str(), cmd.description.as_str()))
            .collect()
    }
}

/// Global settings for the scripting system.
#[derive(Debug, Clone, Deserialize)]
pub struct ScriptSettings {
    /// Whether the scripting system is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Maximum script execution time in seconds.
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Enable debug logging for script execution.
    #[serde(default)]
    pub debug: bool,
}

impl Default for ScriptSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout: 5,
            debug: false,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    5
}

/// Configuration for a single hook.
#[derive(Debug, Clone, Deserialize)]
pub struct HookConfig {
    /// Whether this hook is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// The Rhai script to execute.
    #[serde(default)]
    pub script: String,

    /// Path to an external script file (alternative to inline script).
    #[serde(default)]
    pub script_file: Option<String>,
}

impl Default for HookConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            script: String::new(),
            script_file: None,
        }
    }
}

impl HookConfig {
    /// Gets the script content, loading from file if specified.
    ///
    /// # Errors
    ///
    /// Returns an error if the script file cannot be read.
    pub fn get_script(&self, config_dir: &Path) -> ScriptResult<String> {
        if let Some(ref file) = self.script_file {
            let path = config_dir.join(file);
            let content = fs::read_to_string(&path).map_err(|e| {
                ScriptError::Config(format!(
                    "Failed to read script file {}: {}",
                    path.display(),
                    e
                ))
            })?;
            Ok(content)
        } else {
            Ok(self.script.clone())
        }
    }
}

/// Configuration for a custom command.
#[derive(Debug, Clone, Deserialize)]
pub struct CommandConfig {
    /// Description shown in command list.
    #[serde(default)]
    pub description: String,

    /// The Rhai script to execute.
    #[serde(default)]
    pub script: String,

    /// Path to an external script file.
    #[serde(default)]
    pub script_file: Option<String>,

    /// Keyboard shortcut (optional).
    #[serde(default)]
    pub keybinding: Option<String>,
}

impl CommandConfig {
    /// Gets the script content, loading from file if specified.
    ///
    /// # Errors
    ///
    /// Returns an error if the script file cannot be read.
    pub fn get_script(&self, config_dir: &Path) -> ScriptResult<String> {
        if let Some(ref file) = self.script_file {
            let path = config_dir.join(file);
            let content = fs::read_to_string(&path).map_err(|e| {
                ScriptError::Config(format!(
                    "Failed to read script file {}: {}",
                    path.display(),
                    e
                ))
            })?;
            Ok(content)
        } else {
            Ok(self.script.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_config() {
        let config: ScriptConfig = toml::from_str("").unwrap();
        assert!(config.settings.enabled);
        assert_eq!(config.settings.timeout, 5);
        assert!(config.hooks.is_empty());
        assert!(config.commands.is_empty());
    }

    #[test]
    fn test_parse_settings() {
        let toml = r#"
            [settings]
            enabled = false
            timeout = 10
            debug = true
        "#;
        let config: ScriptConfig = toml::from_str(toml).unwrap();
        assert!(!config.settings.enabled);
        assert_eq!(config.settings.timeout, 10);
        assert!(config.settings.debug);
    }

    #[test]
    fn test_parse_hook() {
        let toml = r#"
            [hooks.on_task_completed]
            enabled = true
            script = """
                log("Task completed!");
            """
        "#;
        let config: ScriptConfig = toml::from_str(toml).unwrap();
        let hook = config.get_hook("on_task_completed").unwrap();
        assert!(hook.enabled);
        assert!(hook.script.contains("log"));
    }

    #[test]
    fn test_parse_command() {
        let toml = r#"
            [commands.archive_done]
            description = "Archive completed tasks"
            script = """
                let done = get_tasks_with_status("done");
                for task in done {
                    add_tag(task.id, "archived");
                }
            """
        "#;
        let config: ScriptConfig = toml::from_str(toml).unwrap();
        let cmd = config.get_command("archive_done").unwrap();
        assert_eq!(cmd.description, "Archive completed tasks");
        assert!(cmd.script.contains("archived"));
    }

    #[test]
    fn test_disabled_hook_not_returned() {
        let toml = r#"
            [hooks.on_task_completed]
            enabled = false
            script = "log('test');"
        "#;
        let config: ScriptConfig = toml::from_str(toml).unwrap();
        assert!(config.get_hook("on_task_completed").is_none());
    }

    #[test]
    fn test_globally_disabled() {
        let toml = r#"
            [settings]
            enabled = false

            [hooks.on_task_completed]
            enabled = true
            script = "log('test');"
        "#;
        let config: ScriptConfig = toml::from_str(toml).unwrap();
        assert!(config.get_hook("on_task_completed").is_none());
        assert!(config.enabled_hooks().is_empty());
    }

    #[test]
    fn test_enabled_hooks_list() {
        let toml = r#"
            [hooks.on_task_created]
            enabled = true
            script = ""

            [hooks.on_task_completed]
            enabled = false
            script = ""

            [hooks.on_task_deleted]
            enabled = true
            script = ""
        "#;
        let config: ScriptConfig = toml::from_str(toml).unwrap();
        let hooks = config.enabled_hooks();
        assert_eq!(hooks.len(), 2);
        assert!(hooks.contains(&"on_task_created"));
        assert!(hooks.contains(&"on_task_deleted"));
    }
}
