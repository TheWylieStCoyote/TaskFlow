use std::path::PathBuf;

use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use super::Settings;

/// Color specification that can be parsed from config
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ColorSpec {
    /// Named color (e.g., "red", "blue", "cyan")
    Named(String),
    /// RGB color as hex (e.g., "#ff0000")
    Hex(String),
    /// RGB color as tuple
    Rgb { r: u8, g: u8, b: u8 },
}

impl ColorSpec {
    /// Convert to ratatui Color
    pub fn to_color(&self) -> Color {
        match self {
            ColorSpec::Named(name) => match name.to_lowercase().as_str() {
                "black" => Color::Black,
                "red" => Color::Red,
                "green" => Color::Green,
                "yellow" => Color::Yellow,
                "blue" => Color::Blue,
                "magenta" => Color::Magenta,
                "cyan" => Color::Cyan,
                "gray" | "grey" => Color::Gray,
                "darkgray" | "darkgrey" => Color::DarkGray,
                "lightred" => Color::LightRed,
                "lightgreen" => Color::LightGreen,
                "lightyellow" => Color::LightYellow,
                "lightblue" => Color::LightBlue,
                "lightmagenta" => Color::LightMagenta,
                "lightcyan" => Color::LightCyan,
                "white" => Color::White,
                "reset" => Color::Reset,
                _ => Color::Reset,
            },
            ColorSpec::Hex(hex) => {
                let hex = hex.trim_start_matches('#');
                if hex.len() == 6 {
                    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                    Color::Rgb(r, g, b)
                } else {
                    Color::Reset
                }
            }
            ColorSpec::Rgb { r, g, b } => Color::Rgb(*r, *g, *b),
        }
    }
}

impl Default for ColorSpec {
    fn default() -> Self {
        ColorSpec::Named("reset".to_string())
    }
}

/// Theme colors
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeColors {
    /// Background color
    pub background: ColorSpec,

    /// Primary text color
    pub foreground: ColorSpec,

    /// Accent color (selected items, highlights)
    pub accent: ColorSpec,

    /// Secondary accent color
    pub accent_secondary: ColorSpec,

    /// Border color
    pub border: ColorSpec,

    /// Error/danger color
    pub danger: ColorSpec,

    /// Warning color
    pub warning: ColorSpec,

    /// Success color
    pub success: ColorSpec,

    /// Muted/disabled color
    pub muted: ColorSpec,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            background: ColorSpec::Named("reset".to_string()),
            foreground: ColorSpec::Named("white".to_string()),
            accent: ColorSpec::Named("cyan".to_string()),
            accent_secondary: ColorSpec::Named("blue".to_string()),
            border: ColorSpec::Named("blue".to_string()),
            danger: ColorSpec::Named("red".to_string()),
            warning: ColorSpec::Named("yellow".to_string()),
            success: ColorSpec::Named("green".to_string()),
            muted: ColorSpec::Named("darkgray".to_string()),
        }
    }
}

/// Priority colors
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PriorityColors {
    pub urgent: ColorSpec,
    pub high: ColorSpec,
    pub medium: ColorSpec,
    pub low: ColorSpec,
    pub none: ColorSpec,
}

impl Default for PriorityColors {
    fn default() -> Self {
        Self {
            urgent: ColorSpec::Named("red".to_string()),
            high: ColorSpec::Named("yellow".to_string()),
            medium: ColorSpec::Named("cyan".to_string()),
            low: ColorSpec::Named("green".to_string()),
            none: ColorSpec::Named("darkgray".to_string()),
        }
    }
}

/// Status colors
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusColors {
    pub pending: ColorSpec,
    pub in_progress: ColorSpec,
    pub done: ColorSpec,
    pub cancelled: ColorSpec,
}

impl Default for StatusColors {
    fn default() -> Self {
        Self {
            pending: ColorSpec::Named("white".to_string()),
            in_progress: ColorSpec::Named("yellow".to_string()),
            done: ColorSpec::Named("green".to_string()),
            cancelled: ColorSpec::Named("darkgray".to_string()),
        }
    }
}

/// Complete theme definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    /// Theme name
    pub name: String,

    /// Base colors
    pub colors: ThemeColors,

    /// Priority-specific colors
    pub priority: PriorityColors,

    /// Status-specific colors
    pub status: StatusColors,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            colors: ThemeColors::default(),
            priority: PriorityColors::default(),
            status: StatusColors::default(),
        }
    }
}

impl Theme {
    /// Load theme from a name (looks in themes directory)
    pub fn load(name: &str) -> Self {
        if name == "default" {
            return Self::default();
        }

        let path = Self::theme_path(name);
        Self::load_from_path(path)
    }

    /// Load theme from a specific path
    pub fn load_from_path(path: PathBuf) -> Self {
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(theme) => return theme,
                    Err(e) => eprintln!("Warning: Failed to parse theme: {}", e),
                },
                Err(e) => eprintln!("Warning: Failed to read theme: {}", e),
            }
        }
        Self::default()
    }

    /// Get the path for a theme by name
    pub fn theme_path(name: &str) -> PathBuf {
        Settings::config_dir()
            .join("themes")
            .join(format!("{}.toml", name))
    }

    /// Get the themes directory
    pub fn themes_dir() -> PathBuf {
        Settings::config_dir().join("themes")
    }
}
