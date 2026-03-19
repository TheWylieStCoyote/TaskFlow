//! Application settings and configuration.
//!
//! This module defines user-configurable settings that control application
//! behavior. Settings are loaded from a TOML file in the config directory
//! and can be overridden via CLI arguments.
//!
//! # Configuration File
//!
//! Settings are stored in `config.toml` inside the config directory, which
//! is `~/.config/taskflow/` on Linux (or the platform equivalent). Missing
//! settings use sensible defaults.
//!
//! # `TASKFLOW_CONFIG_DIR`
//!
//! Setting the `TASKFLOW_CONFIG_DIR` environment variable overrides the
//! config directory. Both `config.toml` and `keybindings.toml` are then read
//! from and written to that directory instead of the platform default, which
//! is useful for portable setups and for testing against a scratch directory.
//!
//! # Available Settings
//!
//! - Storage backend (JSON, YAML, SQLite, Markdown)
//! - Default priority for new tasks
//! - Sidebar visibility
//! - Completed task visibility
//! - Auto-save interval
//! - Pomodoro timer durations

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::warn;

use crate::domain::Priority;
use crate::storage::BackendType;

/// Settings for the editor/Neovim integration via Unix Domain Socket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorSettings {
    /// Enable the editor socket server (default: true)
    pub enabled: bool,
    /// Path to the Unix domain socket file; empty string means auto-resolve
    pub socket_path: String,
    /// Unix permission bits for the socket file (default: 0o600 = 384)
    pub socket_permissions: u32,
    /// Timeout in milliseconds for write-op responses (default: 5000)
    pub query_timeout_ms: u64,
    /// Automatically open the TUI when an editor command arrives (default: false)
    pub auto_open_tui: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            socket_path: String::new(),
            socket_permissions: 0o600,
            query_timeout_ms: 5000,
            auto_open_tui: false,
        }
    }
}

/// A user-defined task template loaded from `config.toml`.
///
/// Example TOML:
/// ```toml
/// [[custom_templates]]
/// name = "PR Review"
/// title = "Review PR: "
/// priority = "medium"
/// tags = ["review", "pr"]
/// due_days = 1
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    /// Display name shown in the template picker
    pub name: String,
    /// Default task title (user can edit after selection)
    pub title: String,
    /// Default priority: "none", "low", "medium", "high", "urgent"
    #[serde(default)]
    pub priority: String,
    /// Default tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional description pre-fill
    #[serde(default)]
    pub description: Option<String>,
    /// Days from now for the due date (0 = today, 1 = tomorrow, None = no due date)
    #[serde(default)]
    pub due_days: Option<i64>,
}

/// What to show when the database is empty on first run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FirstRunMode {
    /// Load sample/demo data for new users to explore
    Demo,
    /// Start with an empty database
    #[default]
    Empty,
}

impl FirstRunMode {
    /// Parse a string into a FirstRunMode
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "demo" | "sample" => Some(Self::Demo),
            "empty" | "blank" => Some(Self::Empty),
            _ => None,
        }
    }
}

/// Main application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Storage backend type
    pub backend: String,

    /// Path to data file/directory
    pub data_path: Option<PathBuf>,

    /// Active theme name
    pub theme: String,

    /// Show sidebar on startup
    pub show_sidebar: bool,

    /// Show completed tasks on startup
    pub show_completed: bool,

    /// Auto-save interval in seconds (0 to disable)
    pub auto_save_interval: u64,

    /// Default priority for new tasks
    pub default_priority: String,

    /// What to show when database is empty on first run
    pub first_run_mode: FirstRunMode,

    /// User-defined task templates loaded from config
    #[serde(default)]
    pub custom_templates: Vec<TemplateConfig>,

    /// Editor / Neovim integration settings
    #[serde(default)]
    pub editor: EditorSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            backend: "json".to_string(),
            data_path: None,
            theme: "default".to_string(),
            show_sidebar: true,
            show_completed: false,
            auto_save_interval: 300, // 5 minutes
            default_priority: "none".to_string(),
            first_run_mode: FirstRunMode::default(),
            custom_templates: Vec::new(),
            editor: EditorSettings::default(),
        }
    }
}

/// Error type for settings loading
#[derive(Debug, Error)]
pub enum SettingsError {
    /// File could not be read
    #[error("failed to read settings file: {0}")]
    ReadError(#[from] std::io::Error),
    /// File content could not be parsed as TOML
    #[error("failed to parse settings file (check TOML syntax): {0}")]
    ParseError(#[from] toml::de::Error),
}

impl Settings {
    /// Check if the config file exists.
    #[must_use]
    pub fn config_exists() -> bool {
        Self::config_path().exists()
    }

    /// Load settings from the default config path.
    ///
    /// Returns default settings if the config file doesn't exist or can't be parsed.
    /// Use [`Self::try_load`] for explicit error handling.
    #[must_use]
    pub fn load() -> Self {
        Self::load_from_path(Self::config_path())
    }

    /// Try to load settings from the default config path.
    ///
    /// Returns `Ok(None)` if the config file doesn't exist.
    /// Returns `Err` if the file exists but can't be read or parsed.
    ///
    /// # Errors
    ///
    /// Returns an error if the config file exists but cannot be read or parsed.
    pub fn try_load() -> Result<Option<Self>, SettingsError> {
        Self::try_load_from_path(Self::config_path())
    }

    /// Load settings from a specific path.
    ///
    /// Returns default settings if the file doesn't exist or can't be parsed.
    /// Prints warnings to stderr on errors (for backward compatibility).
    /// Use [`Self::try_load_from_path`] for explicit error handling.
    #[must_use]
    pub fn load_from_path(path: PathBuf) -> Self {
        match Self::try_load_from_path(path) {
            Ok(Some(settings)) => settings,
            Ok(None) => Self::default(),
            Err(e) => {
                warn!("Settings file error: {e}");
                Self::default()
            }
        }
    }

    /// Try to load settings from a specific path.
    ///
    /// Returns `Ok(None)` if the file doesn't exist.
    /// Returns `Err` if the file exists but can't be read or parsed.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read or parsed.
    pub fn try_load_from_path(path: PathBuf) -> Result<Option<Self>, SettingsError> {
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path)?;
        let settings: Self = toml::from_str(&content)?;
        Ok(Some(settings))
    }

    /// Saves settings to the default config path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save(&self) -> anyhow::Result<()> {
        self.save_to_path(Self::config_path())
    }

    /// Saves settings to a specific path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save_to_path(&self, path: PathBuf) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the default config directory.
    ///
    /// Honours the `TASKFLOW_CONFIG_DIR` environment variable; otherwise
    /// uses the platform config directory (see [`Self::config_dir_from`]).
    #[must_use]
    pub fn config_dir() -> PathBuf {
        Self::config_dir_from(std::env::var_os("TASKFLOW_CONFIG_DIR").as_deref())
    }

    /// Resolve the config directory from an optional override.
    ///
    /// A non-empty `override_dir` (the value of `TASKFLOW_CONFIG_DIR`) is used
    /// verbatim. Otherwise the platform project directory is used, falling
    /// back to the current directory when no home directory can be found.
    ///
    /// This is the pure core of [`Self::config_dir`], kept free of
    /// environment access so it can be unit-tested.
    #[must_use]
    pub fn config_dir_from(override_dir: Option<&OsStr>) -> PathBuf {
        if let Some(dir) = override_dir.filter(|d| !d.is_empty()) {
            return PathBuf::from(dir);
        }
        directories::ProjectDirs::from("com", "taskflow", "taskflow").map_or_else(
            || PathBuf::from("."),
            |dirs| dirs.config_dir().to_path_buf(),
        )
    }

    /// Get the default config file path
    #[must_use]
    pub fn config_path() -> PathBuf {
        Self::path_in(&Self::config_dir())
    }

    /// The settings file path inside a given config directory.
    ///
    /// This is the single place that knows the file is called `config.toml`.
    #[must_use]
    pub fn path_in(dir: &Path) -> PathBuf {
        dir.join("config.toml")
    }

    /// Get the backend type
    #[must_use]
    pub fn backend_type(&self) -> BackendType {
        BackendType::parse(&self.backend).unwrap_or_default()
    }

    /// Get the data path, using defaults if not set
    #[must_use]
    pub fn get_data_path(&self) -> PathBuf {
        self.data_path.clone().unwrap_or_else(|| {
            let data_dir = directories::ProjectDirs::from("com", "taskflow", "taskflow")
                .map_or_else(|| PathBuf::from("."), |dirs| dirs.data_dir().to_path_buf());

            let ext = self.backend_type().file_extension();
            data_dir.join(format!("tasks.{ext}"))
        })
    }

    /// Get the default priority for new tasks
    #[must_use]
    pub fn default_priority(&self) -> Priority {
        Priority::parse(&self.default_priority).unwrap_or_default()
    }

    /// Resolve the Unix domain socket path for the editor integration.
    ///
    /// Uses `editor.socket_path` if set, otherwise falls back to
    /// `$XDG_DATA_HOME/taskflow/taskflow.sock`.
    #[must_use]
    pub fn socket_path(&self) -> PathBuf {
        if !self.editor.socket_path.is_empty() {
            return PathBuf::from(&self.editor.socket_path);
        }
        directories::ProjectDirs::from("com", "taskflow", "taskflow").map_or_else(
            || PathBuf::from("taskflow.sock"),
            |dirs| dirs.data_dir().join("taskflow.sock"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn config_dir_from_uses_override_verbatim() {
        let dir = Settings::config_dir_from(Some(OsStr::new("/x/y")));
        assert_eq!(dir, PathBuf::from("/x/y"));
    }

    #[test]
    fn config_dir_from_ignores_empty_override() {
        let empty = Settings::config_dir_from(Some(OsStr::new("")));
        let none = Settings::config_dir_from(None);
        assert_eq!(empty, none);
    }

    #[test]
    fn config_dir_from_without_override_uses_platform_fallback() {
        let dir = Settings::config_dir_from(None);
        // Either the platform project dir (whose final component is the
        // "taskflow" application name) or "." when no home dir is available.
        let is_project_dir = dir.file_name().is_some_and(|name| name == "taskflow");
        assert!(
            is_project_dir || dir == Path::new("."),
            "unexpected fallback config dir: {}",
            dir.display()
        );
    }

    #[test]
    fn path_in_joins_config_file_name() {
        assert_eq!(
            Settings::path_in(Path::new("/cfg")),
            PathBuf::from("/cfg/config.toml")
        );
    }

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();

        assert_eq!(settings.backend, "json");
        assert!(settings.data_path.is_none());
        assert_eq!(settings.theme, "default");
        assert!(settings.show_sidebar);
        assert!(!settings.show_completed);
        assert_eq!(settings.auto_save_interval, 300);
        assert_eq!(settings.default_priority, "none");
        assert_eq!(settings.first_run_mode, FirstRunMode::Empty);
    }

    #[test]
    fn test_first_run_mode_parse() {
        assert_eq!(FirstRunMode::parse("demo"), Some(FirstRunMode::Demo));
        assert_eq!(FirstRunMode::parse("sample"), Some(FirstRunMode::Demo));
        assert_eq!(FirstRunMode::parse("empty"), Some(FirstRunMode::Empty));
        assert_eq!(FirstRunMode::parse("blank"), Some(FirstRunMode::Empty));
        assert_eq!(FirstRunMode::parse("DEMO"), Some(FirstRunMode::Demo));
        assert_eq!(FirstRunMode::parse("EMPTY"), Some(FirstRunMode::Empty));
        assert_eq!(FirstRunMode::parse("invalid"), None);
    }

    #[test]
    fn test_first_run_mode_default() {
        assert_eq!(FirstRunMode::default(), FirstRunMode::Empty);
    }

    #[test]
    fn test_settings_load_missing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.toml");

        let settings = Settings::load_from_path(path);

        // Should return defaults
        assert_eq!(settings.backend, "json");
    }

    #[test]
    fn test_settings_load_invalid_toml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("invalid.toml");

        std::fs::write(&path, "this is not { valid toml").unwrap();

        let settings = Settings::load_from_path(path);

        // Should return defaults on parse error
        assert_eq!(settings.backend, "json");
    }

    #[test]
    fn test_settings_save_creates_dirs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("subdir").join("nested").join("config.toml");

        let settings = Settings::default();
        settings.save_to_path(path.clone()).unwrap();

        assert!(path.exists());
    }

    #[test]
    fn test_settings_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let settings = Settings {
            backend: "yaml".to_string(),
            show_completed: true,
            auto_save_interval: 600,
            ..Settings::default()
        };

        settings.save_to_path(path.clone()).unwrap();

        let loaded = Settings::load_from_path(path);

        assert_eq!(loaded.backend, "yaml");
        assert!(loaded.show_completed);
        assert_eq!(loaded.auto_save_interval, 600);
    }

    #[test]
    fn test_settings_backend_type() {
        let json_settings = Settings {
            backend: "json".to_string(),
            ..Settings::default()
        };
        assert_eq!(json_settings.backend_type(), BackendType::Json);

        let yaml_settings = Settings {
            backend: "yaml".to_string(),
            ..Settings::default()
        };
        assert_eq!(yaml_settings.backend_type(), BackendType::Yaml);

        let sqlite_settings = Settings {
            backend: "sqlite".to_string(),
            ..Settings::default()
        };
        assert_eq!(sqlite_settings.backend_type(), BackendType::Sqlite);

        let md_settings = Settings {
            backend: "markdown".to_string(),
            ..Settings::default()
        };
        assert_eq!(md_settings.backend_type(), BackendType::Markdown);
    }

    #[test]
    fn test_settings_get_data_path_explicit() {
        let settings = Settings {
            data_path: Some(PathBuf::from("/custom/path/data.json")),
            ..Settings::default()
        };

        let path = settings.get_data_path();
        assert_eq!(path, PathBuf::from("/custom/path/data.json"));
    }

    #[test]
    fn test_settings_get_data_path_default_uses_backend_extension() {
        let yaml_settings = Settings {
            backend: "yaml".to_string(),
            data_path: None,
            ..Settings::default()
        };
        let path = yaml_settings.get_data_path();
        assert!(path.to_string_lossy().ends_with("tasks.yaml"));

        let sqlite_settings = Settings {
            backend: "sqlite".to_string(),
            data_path: None,
            ..Settings::default()
        };
        let path = sqlite_settings.get_data_path();
        assert!(path.to_string_lossy().ends_with("tasks.db"));
    }

    #[test]
    fn test_settings_default_priority() {
        let mut settings = Settings::default();

        // Default is "none"
        assert_eq!(settings.default_priority(), Priority::None);

        // Valid priorities
        settings.default_priority = "low".to_string();
        assert_eq!(settings.default_priority(), Priority::Low);

        settings.default_priority = "high".to_string();
        assert_eq!(settings.default_priority(), Priority::High);

        settings.default_priority = "urgent".to_string();
        assert_eq!(settings.default_priority(), Priority::Urgent);

        // Invalid falls back to None
        settings.default_priority = "invalid".to_string();
        assert_eq!(settings.default_priority(), Priority::None);
    }

    #[test]
    fn test_try_load_missing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.toml");

        let result = Settings::try_load_from_path(path);

        // Should return Ok(None) for missing file
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_try_load_invalid_toml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("invalid.toml");

        std::fs::write(&path, "this is not { valid toml").unwrap();

        let result = Settings::try_load_from_path(path);

        // Should return Err for invalid TOML
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SettingsError::ParseError(_)));
    }

    #[test]
    fn test_try_load_valid_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let settings = Settings {
            backend: "sqlite".to_string(),
            ..Settings::default()
        };
        settings.save_to_path(path.clone()).unwrap();

        let result = Settings::try_load_from_path(path);

        assert!(result.is_ok());
        let loaded = result.unwrap().unwrap();
        assert_eq!(loaded.backend, "sqlite");
    }

    #[test]
    fn test_settings_error_display() {
        let io_err = SettingsError::ReadError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        let display = io_err.to_string();
        assert!(display.contains("failed to read"));
        assert!(display.contains("settings file"));

        // Parse error display with helpful hint
        let parse_result: Result<Settings, _> = toml::from_str("invalid { toml");
        if let Err(e) = parse_result {
            let settings_err = SettingsError::ParseError(e);
            let display = settings_err.to_string();
            assert!(display.contains("failed to parse"));
            assert!(display.contains("check TOML syntax"));
        }
    }

    #[test]
    fn test_settings_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let settings_err: SettingsError = io_err.into();
        assert!(matches!(settings_err, SettingsError::ReadError(_)));
    }

    #[test]
    fn test_settings_error_from_toml() {
        let parse_result: Result<Settings, toml::de::Error> = toml::from_str("not valid toml {");
        if let Err(toml_err) = parse_result {
            let settings_err: SettingsError = toml_err.into();
            assert!(matches!(settings_err, SettingsError::ParseError(_)));
        }
    }
}
