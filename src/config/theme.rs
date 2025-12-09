//! Theme configuration and color management.
//!
//! This module handles visual theming for the application. Themes define
//! colors for various UI elements and can be loaded from TOML configuration
//! files or use built-in defaults.
//!
//! # Color Specification
//!
//! Colors can be specified in multiple formats:
//! - Named colors: `"red"`, `"blue"`, `"cyan"`
//! - Hex colors: `"#ff0000"`, `"#3498db"`
//! - RGB tuples: `{ r = 255, g = 0, b = 0 }`
//!
//! # Built-in Themes
//!
//! - `default`: Dark theme with cyan accents
//! - `light`: Light background with dark text
//! - `gruvbox`: Warm retro colors

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
    #[must_use]
    pub fn to_color(&self) -> Color {
        match self {
            Self::Named(name) => match name.to_lowercase().as_str() {
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
            Self::Hex(hex) => {
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
            Self::Rgb { r, g, b } => Color::Rgb(*r, *g, *b),
        }
    }
}

impl Default for ColorSpec {
    fn default() -> Self {
        Self::Named("reset".to_string())
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
    #[must_use]
    pub fn load(name: &str) -> Self {
        if name == "default" {
            return Self::default();
        }

        let path = Self::theme_path(name);
        Self::load_from_path(path)
    }

    /// Load theme from a specific path
    #[must_use]
    pub fn load_from_path(path: PathBuf) -> Self {
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(theme) => return theme,
                    Err(e) => eprintln!("Warning: Failed to parse theme: {e}"),
                },
                Err(e) => eprintln!("Warning: Failed to read theme: {e}"),
            }
        }
        Self::default()
    }

    /// Get the path for a theme by name
    #[must_use]
    pub fn theme_path(name: &str) -> PathBuf {
        Settings::config_dir()
            .join("themes")
            .join(format!("{name}.toml"))
    }

    /// Get the themes directory
    #[must_use]
    pub fn themes_dir() -> PathBuf {
        Settings::config_dir().join("themes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_color_spec_named() {
        let color = ColorSpec::Named("red".to_string());
        assert_eq!(color.to_color(), Color::Red);

        let color = ColorSpec::Named("cyan".to_string());
        assert_eq!(color.to_color(), Color::Cyan);

        let color = ColorSpec::Named("white".to_string());
        assert_eq!(color.to_color(), Color::White);
    }

    #[test]
    fn test_color_spec_hex() {
        let color = ColorSpec::Hex("#ff0000".to_string());
        assert_eq!(color.to_color(), Color::Rgb(255, 0, 0));

        let color = ColorSpec::Hex("#00ff00".to_string());
        assert_eq!(color.to_color(), Color::Rgb(0, 255, 0));

        let color = ColorSpec::Hex("#0000ff".to_string());
        assert_eq!(color.to_color(), Color::Rgb(0, 0, 255));
    }

    #[test]
    fn test_color_spec_hex_invalid() {
        let color = ColorSpec::Hex("#fff".to_string()); // Too short
        assert_eq!(color.to_color(), Color::Reset);
    }

    #[test]
    fn test_color_spec_rgb() {
        let color = ColorSpec::Rgb {
            r: 128,
            g: 64,
            b: 32,
        };
        assert_eq!(color.to_color(), Color::Rgb(128, 64, 32));
    }

    #[test]
    fn test_theme_default() {
        let theme = Theme::default();

        assert_eq!(theme.name, "default");
        assert_eq!(theme.colors.accent.to_color(), Color::Cyan);
        assert_eq!(theme.priority.urgent.to_color(), Color::Red);
        assert_eq!(theme.status.done.to_color(), Color::Green);
    }

    #[test]
    fn test_theme_load_default_name() {
        let theme = Theme::load("default");
        assert_eq!(theme.name, "default");
    }

    #[test]
    fn test_theme_load_missing_file() {
        let theme = Theme::load("nonexistent_theme");
        // Should return default theme
        assert_eq!(theme.name, "default");
    }

    #[test]
    fn test_theme_load_from_path() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("custom.toml");

        let content = r##"
name = "custom"

[colors]
accent = "magenta"

[priority]
urgent = "#ff0000"

[status]
done = { r = 0, g = 255, b = 0 }
"##;
        std::fs::write(&path, content).unwrap();

        let theme = Theme::load_from_path(path);

        assert_eq!(theme.name, "custom");
        assert_eq!(theme.colors.accent.to_color(), Color::Magenta);
    }

    #[test]
    fn test_priority_colors_default() {
        let priority = PriorityColors::default();

        assert_eq!(priority.urgent.to_color(), Color::Red);
        assert_eq!(priority.high.to_color(), Color::Yellow);
        assert_eq!(priority.medium.to_color(), Color::Cyan);
        assert_eq!(priority.low.to_color(), Color::Green);
        assert_eq!(priority.none.to_color(), Color::DarkGray);
    }

    #[test]
    fn test_status_colors_default() {
        let status = StatusColors::default();

        assert_eq!(status.pending.to_color(), Color::White);
        assert_eq!(status.in_progress.to_color(), Color::Yellow);
        assert_eq!(status.done.to_color(), Color::Green);
        assert_eq!(status.cancelled.to_color(), Color::DarkGray);
    }

    #[test]
    fn test_color_spec_default() {
        let color = ColorSpec::default();
        assert_eq!(color.to_color(), Color::Reset);
    }

    #[test]
    fn test_color_spec_named_unknown() {
        let color = ColorSpec::Named("unknowncolor".to_string());
        assert_eq!(color.to_color(), Color::Reset);
    }

    #[test]
    fn test_theme_colors_default() {
        let colors = ThemeColors::default();

        assert_eq!(colors.foreground.to_color(), Color::White);
        assert_eq!(colors.border.to_color(), Color::Blue);
        assert_eq!(colors.danger.to_color(), Color::Red);
        assert_eq!(colors.success.to_color(), Color::Green);
    }
}
