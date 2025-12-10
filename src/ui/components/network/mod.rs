//! Network graph view component - dependency visualization.
//!
//! Displays task dependencies as an interactive ASCII graph,
//! showing relationships between blocked and blocking tasks.

mod queries;
mod render;

#[cfg(test)]
mod tests;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Widget},
};

use crate::app::Model;
use crate::config::Theme;

/// Network graph view widget showing task dependencies
pub struct Network<'a> {
    pub(crate) model: &'a Model,
    pub(crate) theme: &'a Theme,
    pub(crate) selected_task_index: usize,
}

impl<'a> Network<'a> {
    /// Create a new network widget
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme, selected_task_index: usize) -> Self {
        Self {
            model,
            theme,
            selected_task_index,
        }
    }
}

impl Widget for Network<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Network - Dependency Visualization ")
            .title_style(
                Style::default()
                    .fg(self.theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.colors.border.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 30 || inner.height < 10 {
            return;
        }

        // Layout: graph on left, stats and legend on right
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(inner);

        let right_panel = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(6), Constraint::Min(5)])
            .split(chunks[1]);

        self.render_task_tree(chunks[0], buf);
        self.render_stats(right_panel[0], buf);
        self.render_legend(right_panel[1], buf);
    }
}
