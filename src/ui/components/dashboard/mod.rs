//! Statistics dashboard widget
//!
//! This module provides the dashboard view displaying various statistics
//! about tasks, projects, time tracking, and productivity.
//!
//! # Module Structure
//!
//! - `stats` - Statistics calculation methods
//! - `panels` - Individual panel rendering
//! - `tests` - Unit tests

mod panels;
pub mod stats;

#[cfg(test)]
mod tests;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Widget,
};

use crate::app::Model;
use crate::config::Theme;

/// Statistics dashboard widget
pub struct Dashboard<'a> {
    model: &'a Model,
    theme: &'a Theme,
}

impl<'a> Dashboard<'a> {
    /// Create a new dashboard widget
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }
}

impl Widget for Dashboard<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        // Split into 2 columns
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left column: 3 panels
        let left_panels = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Completion
                Constraint::Length(6), // Time Tracking
                Constraint::Min(5),    // Projects
            ])
            .split(columns[0]);

        // Right column: 4 panels
        let right_panels = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // Status Distribution
                Constraint::Length(6), // Estimation
                Constraint::Length(6), // Focus Sessions
                Constraint::Min(5),    // Weekly Activity
            ])
            .split(columns[1]);

        // Render each panel
        self.render_completion_panel(left_panels[0], buf, theme);
        self.render_time_panel(left_panels[1], buf, theme);
        self.render_projects_panel(left_panels[2], buf, theme);
        self.render_status_panel(right_panels[0], buf, theme);
        self.render_estimation_panel(right_panels[1], buf, theme);
        self.render_focus_panel(right_panels[2], buf, theme);
        self.render_activity_panel(right_panels[3], buf, theme);
    }
}
