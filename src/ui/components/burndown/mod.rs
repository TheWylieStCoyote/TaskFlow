//! Burndown chart view component - progress toward completion.
//!
//! Displays project progress as a burndown chart, showing remaining work
//! over time compared to an ideal completion line.

mod chart;
mod data;
mod stats;

#[cfg(test)]
mod tests;

pub use data::BurndownData;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Widget},
};

use crate::app::Model;
use crate::config::Theme;

/// Burndown chart view widget
pub struct Burndown<'a> {
    pub(crate) model: &'a Model,
    pub(crate) theme: &'a Theme,
}

impl<'a> Burndown<'a> {
    /// Create a new burndown widget
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }
}

impl Widget for Burndown<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Build dynamic title with mode and window info
        let mode_label = self.model.burndown_state.mode.label();
        let window_label = self.model.burndown_state.time_window.label();
        let title = format!(" Burndown - {mode_label} ({window_label}) ");

        let block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(self.theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.colors.border.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 40 || inner.height < 15 {
            return;
        }

        let data = self.get_burndown_data(self.model.selected_project);

        // Layout: chart on left, stats on right
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(inner);

        let right_panel = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(14), Constraint::Length(8)])
            .split(chunks[1]);

        // Chart area with border - show scope creep indicator if enabled
        let scope_indicator = if self.model.burndown_state.show_scope_creep {
            " [+scope] "
        } else {
            ""
        };
        let chart_title = format!(" Last {window_label}{scope_indicator}");
        let chart_block = Block::default()
            .title(chart_title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.colors.border.to_color()));
        let chart_inner = chart_block.inner(chunks[0]);
        chart_block.render(chunks[0], buf);

        self.render_chart(chart_inner, buf, &data);
        self.render_stats(right_panel[0], buf, &data);
        self.render_projects(right_panel[1], buf);
    }
}
