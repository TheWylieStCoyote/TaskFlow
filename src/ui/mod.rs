//! Terminal UI rendering with Ratatui.
//!
//! This module handles rendering the application state to the terminal.
//! It uses the Ratatui library for widget-based TUI rendering.
//!
//! ## Components
//!
//! The UI is built from reusable components:
//!
//! - Task list with priority and status indicators
//! - Sidebar with views and project navigation
//! - Input fields for text entry
//! - Calendar grid view
//! - Dashboard with statistics
//! - Help overlay with keyboard shortcuts
//!
//! ## View Function
//!
//! The main [`view()`] function renders the complete UI based on the
//! current [`crate::app::Model`] state. It follows the TEA pattern
//! where the view is a pure function of state.
//!
//! ## Theming
//!
//! Colors and styles are controlled by [`crate::config::Theme`],
//! allowing customization of the visual appearance.

mod components;
pub mod primitives;
mod view;

#[cfg(test)]
pub mod test_utils;

pub use components::*;
pub use view::*;
