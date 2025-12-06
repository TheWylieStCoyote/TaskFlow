//! Application configuration management.
//!
//! This module handles loading and managing user configuration:
//!
//! - [`Settings`] - General application settings
//! - [`Keybindings`] - Custom keyboard shortcuts
//! - [`Theme`] - Visual styling and colors
//!
//! ## Configuration Files
//!
//! Configuration is stored in `~/.config/taskflow/`:
//!
//! ```text
//! ~/.config/taskflow/
//! ├── config.toml        # General settings
//! ├── keybindings.toml   # Custom key mappings
//! └── themes/
//!     └── default.toml   # Color themes
//! ```
//!
//! ## Defaults
//!
//! All configuration values have sensible defaults, so the application
//! works without any configuration files.
//!
//! ## Theme Customization
//!
//! Themes control the visual appearance of the application:
//!
//! ```
//! use taskflow::config::{Theme, ColorSpec};
//!
//! // Use the default theme
//! let theme = Theme::default();
//! assert_eq!(theme.name, "default");
//!
//! // Access theme colors
//! let accent = theme.colors.accent.to_color();
//! let urgent_color = theme.priority.urgent.to_color();
//! ```
//!
//! ### Color Specification
//!
//! Colors can be specified in multiple formats:
//!
//! ```
//! use taskflow::config::ColorSpec;
//! use ratatui::style::Color;
//!
//! // Named color
//! let red = ColorSpec::Named("red".to_string());
//! assert_eq!(red.to_color(), Color::Red);
//!
//! // Hex color
//! let custom = ColorSpec::Hex("#ff5500".to_string());
//!
//! // RGB tuple
//! let rgb = ColorSpec::Rgb { r: 100, g: 150, b: 200 };
//! assert_eq!(rgb.to_color(), Color::Rgb(100, 150, 200));
//! ```
//!
//! ### Available Named Colors
//!
//! | Color | Name |
//! |-------|------|
//! | Black | `black` |
//! | Red | `red`, `lightred` |
//! | Green | `green`, `lightgreen` |
//! | Yellow | `yellow`, `lightyellow` |
//! | Blue | `blue`, `lightblue` |
//! | Magenta | `magenta`, `lightmagenta` |
//! | Cyan | `cyan`, `lightcyan` |
//! | Gray | `gray`, `darkgray` |
//! | White | `white` |
//!
//! ## Settings
//!
//! General application settings:
//!
//! ```toml
//! # config.toml example
//! backend = "json"
//! data_dir = "~/.local/share/taskflow"
//! theme = "default"
//! show_completed = false
//! ```

mod keybindings;
mod settings;
mod theme;

pub use keybindings::*;
pub use settings::*;
pub use theme::*;
