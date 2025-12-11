//! ASCII chart widgets for terminal UI.
//!
//! This module provides chart widgets for displaying analytics data
//! in the terminal using ASCII/Unicode characters.
//!
//! # Available Charts
//!
//! - [`BarChart`]: Horizontal bar chart for categorical data
//! - [`Sparkline`]: Compact trend line using Unicode block characters
//! - [`BurndownChart`]: Project burndown visualization
//! - [`ProgressGauge`]: Progress bar with percentage
//! - [`StatBox`]: Stat display with optional trend indicator

mod bar;
mod burndown;
mod simple;

#[cfg(test)]
mod tests;

pub use bar::BarChart;
pub use burndown::BurndownChart;
pub use simple::{ProgressGauge, Sparkline, StatBox};

/// Characters for sparkline chart.
pub(crate) const SPARKLINE_CHARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
