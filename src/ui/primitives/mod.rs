//! Shared UI primitives for consistent widget styling.
//!
//! This module provides reusable building blocks for UI components:
//! - [`blocks`] - Styled block builders with consistent theming
//! - [`lists`] - List highlight configurations
//! - [`modals`] - Modal/dialog wrapper widgets
//!
//! # Example
//!
//! ```ignore
//! use crate::ui::primitives::{panel_block, with_highlight_style};
//!
//! let block = panel_block("Tasks", theme);
//! let list = with_highlight_style(List::new(items), theme);
//! ```

mod blocks;
mod lists;
mod modals;

pub use blocks::*;
pub use lists::*;
pub use modals::*;
