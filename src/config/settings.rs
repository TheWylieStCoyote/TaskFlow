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

impl Settings {
    /// Load settings from the default config path
    pub fn load() -> Self {
        Self::load_from_path(Self::config_path())
    }

    /// Load settings from a specific path
    pub fn load_from_path(path: PathBuf) -> Self {
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(settings) => return settings,
                    Err(e) => eprintln!("Warning: Failed to parse config: {}", e),
                },
                Err(e) => eprintln!("Warning: Failed to read config: {}", e),
            }
        }
        Self::default()
    }

    /// Save settings to the default config path
    pub fn save(&self) -> anyhow::Result<()> {
        self.save_to_path(Self::config_path())
    }

    /// Save settings to a specific path
    pub fn save_to_path(&self, path: PathBuf) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the default config directory
    pub fn config_dir() -> PathBuf {
        directories::ProjectDirs::from("com", "taskflow", "taskflow")
            .map(|dirs| dirs.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    }

    /// Get the default config file path
    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    /// Get the backend type
    pub fn backend_type(&self) -> BackendType {
        BackendType::parse(&self.backend).unwrap_or_default()
    }

    /// Get the data path, using defaults if not set
    pub fn get_data_path(&self) -> PathBuf {
        self.data_path.clone().unwrap_or_else(|| {
            let data_dir = directories::ProjectDirs::from("com", "taskflow", "taskflow")
                .map(|dirs| dirs.data_dir().to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."));

            let ext = self.backend_type().file_extension();
            data_dir.join(format!("tasks.{}", ext))
        })
    }

    /// Get the default priority for new tasks
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

        let mut settings = Settings::default();
        settings.backend = "yaml".to_string();
        settings.show_completed = true;
        settings.auto_save_interval = 600;

        settings.save_to_path(path.clone()).unwrap();

        let loaded = Settings::load_from_path(path);

        assert_eq!(loaded.backend, "yaml");
        assert!(loaded.show_completed);
        assert_eq!(loaded.auto_save_interval, 600);
    }

    #[test]
    fn test_settings_backend_type() {
        let mut settings = Settings::default();

        settings.backend = "json".to_string();
        assert_eq!(settings.backend_type(), BackendType::Json);

        settings.backend = "yaml".to_string();
        assert_eq!(settings.backend_type(), BackendType::Yaml);

        settings.backend = "sqlite".to_string();
        assert_eq!(settings.backend_type(), BackendType::Sqlite);

        settings.backend = "markdown".to_string();
        assert_eq!(settings.backend_type(), BackendType::Markdown);
    }

    #[test]
    fn test_settings_get_data_path_explicit() {
        let mut settings = Settings::default();
        settings.data_path = Some(PathBuf::from("/custom/path/data.json"));

        let path = settings.get_data_path();
        assert_eq!(path, PathBuf::from("/custom/path/data.json"));
    }

    #[test]
    fn test_settings_get_data_path_default_uses_backend_extension() {
        let mut settings = Settings::default();
        settings.data_path = None;

        settings.backend = "yaml".to_string();
        let path = settings.get_data_path();
        assert!(path.to_string_lossy().ends_with("tasks.yaml"));

        settings.backend = "sqlite".to_string();
        let path = settings.get_data_path();
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
}
