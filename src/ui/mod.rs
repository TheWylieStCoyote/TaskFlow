//! Terminal UI rendering with Ratatui.
//!
//! This module handles rendering the application state to the terminal.
//! It uses the Ratatui library for widget-based TUI rendering.
//!
//! # Architecture Overview
//!
//! The UI follows a three-layer architecture:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                         View Layer                          │
//! │  view.rs - Main render function, layout orchestration       │
//! └────────────────────────────┬────────────────────────────────┘
//!                              │
//! ┌────────────────────────────▼────────────────────────────────┐
//! │                     Components Layer                         │
//! │  components/*.rs - Reusable view-specific widgets           │
//! │  (TaskList, Calendar, Kanban, Dashboard, Sidebar, etc.)     │
//! └────────────────────────────┬────────────────────────────────┘
//!                              │
//! ┌────────────────────────────▼────────────────────────────────┐
//! │                     Primitives Layer                         │
//! │  primitives/*.rs - Low-level rendering utilities            │
//! │  (Icons, colors, spinners, date formatting)                 │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Module Structure
//!
//! | Module | Purpose |
//! |--------|---------|
//! | `view` | Main `view()` function, layout rendering |
//! | `components` | View components (TaskList, Calendar, etc.) |
//! | [`primitives`] | Low-level helpers (icons, colors, formatting) |
//! | `test_utils` | Test helpers for widget rendering (test-only) |
//!
//! # Data Flow (TEA Pattern)
//!
//! The UI is a pure function of [`Model`] state:
//!
//! ```text
//! Model ──► view() ──► Frame ──► Terminal
//!   ▲                                │
//!   │         Event Loop             │
//!   └────────────────────────────────┘
//! ```
//!
//! The view function:
//! 1. Receives immutable `Model` reference
//! 2. Computes layout areas (header, sidebar, content, footer)
//! 3. Delegates to component widgets for each area
//! 4. Renders popups/overlays on top if active
//!
//! # Layout Caching for Mouse Events
//!
//! The [`LayoutCache`] stores rendered positions for mouse event handling:
//!
//! - **Cleared** at the start of each render cycle
//! - **Populated** by components during rendering
//! - **Read** by mouse event handlers to map clicks to actions
//!
//! This allows components to be purely functional while supporting
//! mouse interactions without recalculating layouts.
//!
//! # Component Composition
//!
//! Components implement Ratatui's `Widget` trait:
//!
//! ```text
//! struct TaskList<'a> { model: &'a Model, theme: &'a Theme }
//!
//! impl Widget for TaskList<'_> {
//!     fn render(self, area: Rect, buf: &mut Buffer) { ... }
//! }
//! ```
//!
//! This enables:
//! - **Composition**: Components contain other components
//! - **Testability**: Components can be rendered to test buffers
//! - **Reusability**: Same component in multiple contexts
//!
//! # Popup Layering
//!
//! Popups render on top of the main content in z-order:
//!
//! 1. Main layout (header, sidebar, content, footer)
//! 2. Modal dialogs (task details, confirmations)
//! 3. Input overlays (quick add, command palette)
//! 4. Help overlay (highest z-order)
//!
//! Each popup checks `model.popups.*` state to determine visibility.
//!
//! # Theming
//!
//! Colors and styles are controlled by [`crate::config::Theme`],
//! allowing customization of the visual appearance. Components
//! receive `&Theme` and use `theme.colors.*` for all styling.
//!
//! [`Model`]: crate::app::Model
//! [`LayoutCache`]: crate::app::LayoutCache

mod components;
pub mod primitives;
mod view;

#[cfg(test)]
pub mod test_utils;

pub use components::*;
pub use view::*;
