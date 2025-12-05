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

mod keybindings;
mod settings;
mod theme;

pub use keybindings::*;
pub use settings::*;
pub use theme::*;
