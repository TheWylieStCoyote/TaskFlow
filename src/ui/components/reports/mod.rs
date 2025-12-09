//! Reports view component for analytics display.
//!
//! This module provides the reports view widget that displays analytics
//! and statistics about tasks using chart widgets.
//!
//! # Module Structure
//!
//! - `panels` - Individual panel rendering (overview, velocity, tags, etc.)
//! - `tests` - Unit tests

mod panels;

#[cfg(test)]
mod tests;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Tabs, Widget},
};

use crate::app::Model;
use crate::config::Theme;

/// The currently selected report panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReportPanel {
    #[default]
    Overview,
    Velocity,
    Tags,
    Time,
    Focus,
    Insights,
    Estimation,
}

impl ReportPanel {
    /// Get the next panel (wrapping).
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Overview => Self::Velocity,
            Self::Velocity => Self::Tags,
            Self::Tags => Self::Time,
            Self::Time => Self::Focus,
            Self::Focus => Self::Insights,
            Self::Insights => Self::Estimation,
            Self::Estimation => Self::Overview,
        }
    }

    /// Get the previous panel (wrapping).
    #[must_use]
    pub const fn prev(self) -> Self {
        match self {
            Self::Overview => Self::Estimation,
            Self::Velocity => Self::Overview,
            Self::Tags => Self::Velocity,
            Self::Time => Self::Tags,
            Self::Focus => Self::Time,
            Self::Insights => Self::Focus,
            Self::Estimation => Self::Insights,
        }
    }

    /// Get the panel index.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Overview => 0,
            Self::Velocity => 1,
            Self::Tags => 2,
            Self::Time => 3,
            Self::Focus => 4,
            Self::Insights => 5,
            Self::Estimation => 6,
        }
    }

    /// Get panel names for tabs.
    #[must_use]
    pub const fn names() -> [&'static str; 7] {
        [
            "Overview",
            "Velocity",
            "Tags",
            "Time",
            "Focus",
            "Insights",
            "Estimation",
        ]
    }
}

/// Reports view widget.
pub struct ReportsView<'a> {
    model: &'a Model,
    selected_panel: ReportPanel,
    theme: &'a Theme,
}

impl<'a> ReportsView<'a> {
    /// Create a new reports view.
    #[must_use]
    pub const fn new(model: &'a Model, selected_panel: ReportPanel, theme: &'a Theme) -> Self {
        Self {
            model,
            selected_panel,
            theme,
        }
    }
}

impl Widget for ReportsView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        // Render outer border
        let block = Block::default()
            .title(" Reports ")
            .title_style(
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.border.to_color()));
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 20 || inner.height < 10 {
            return;
        }

        // Split into tabs area and content area
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(inner);

        // Render tabs
        let tab_titles: Vec<Line<'_>> = ReportPanel::names()
            .iter()
            .map(|t| Line::from(*t))
            .collect();
        let tabs = Tabs::new(tab_titles)
            .select(self.selected_panel.index())
            .highlight_style(
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .divider(" | ");
        tabs.render(chunks[0], buf);

        // Render selected panel
        match self.selected_panel {
            ReportPanel::Overview => self.render_overview(chunks[1], buf),
            ReportPanel::Velocity => self.render_velocity(chunks[1], buf),
            ReportPanel::Tags => self.render_tags(chunks[1], buf),
            ReportPanel::Time => self.render_time(chunks[1], buf),
            ReportPanel::Focus => self.render_focus(chunks[1], buf),
            ReportPanel::Insights => self.render_insights(chunks[1], buf),
            ReportPanel::Estimation => self.render_estimation(chunks[1], buf),
        }
    }
}

/// Format minutes as hours and minutes
pub(crate) fn format_duration(minutes: u32) -> String {
    let hours = minutes / 60;
    let mins = minutes % 60;
    if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}
