use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
        BackendType::from_str(&self.backend).unwrap_or_default()
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
}
