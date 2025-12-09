//! Application settings and configuration.
//!
//! This module defines user-configurable settings that control application
//! behavior. Settings are loaded from a TOML file in the config directory
//! and can be overridden via CLI arguments.
//!
//! # Configuration File
//!
//! Settings are stored in `~/.config/taskflow/settings.toml` (or equivalent
//! on other platforms). Missing settings use sensible defaults.
//!
//! # Available Settings
//!
//! - Storage backend (JSON, YAML, SQLite, Markdown)
//! - Default priority for new tasks
//! - Sidebar visibility
//! - Completed task visibility
//! - Auto-save interval
//! - Pomodoro timer durations

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::Priority;
use crate::storage::BackendType;

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
        }
    }
}

/// Error type for settings loading
#[derive(Debug)]
pub enum SettingsError {
    /// File could not be read
    ReadError(std::io::Error),
    /// File content could not be parsed as TOML
    ParseError(toml::de::Error),
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadError(e) => write!(f, "failed to read config file: {e}"),
            Self::ParseError(e) => write!(f, "failed to parse config file: {e}"),
        }
    }
}

impl std::error::Error for SettingsError {}

impl Settings {
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
                eprintln!("Warning: {e}");
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

        let content = std::fs::read_to_string(&path).map_err(SettingsError::ReadError)?;
        let settings: Self = toml::from_str(&content).map_err(SettingsError::ParseError)?;
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

    /// Get the default config directory
    #[must_use]
    pub fn config_dir() -> PathBuf {
        directories::ProjectDirs::from("com", "taskflow", "taskflow").map_or_else(
            || PathBuf::from("."),
            |dirs| dirs.config_dir().to_path_buf(),
        )
    }

    /// Get the default config file path
    #[must_use]
    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
        assert!(io_err.to_string().contains("failed to read"));

        // Parse error display
        let parse_result: Result<Settings, _> = toml::from_str("invalid { toml");
        if let Err(e) = parse_result {
            let settings_err = SettingsError::ParseError(e);
            assert!(settings_err.to_string().contains("failed to parse"));
        }
    }
}
