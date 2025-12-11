//! Goal/OKR tracking view component.
//!
//! Displays goals with key results, progress bars, and quarterly organization.

mod list;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    widgets::Widget,
};

use crate::app::Model;
use crate::config::Theme;

/// Goal/OKR tracking view widget.
pub struct GoalsView<'a> {
    pub(crate) model: &'a Model,
    pub(crate) theme: &'a Theme,
}

impl<'a> GoalsView<'a> {
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }

    /// Returns a color based on progress percentage.
    pub(crate) fn progress_color(&self, progress: u8) -> Color {
        let theme = self.theme;
        if progress >= 80 {
            theme.colors.success.to_color()
        } else if progress >= 50 {
            theme.colors.accent.to_color()
        } else if progress >= 25 {
            theme.colors.warning.to_color()
        } else {
            theme.colors.danger.to_color()
        }
    }
}

impl Widget for GoalsView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Split into goal list (left) and detail panel (right)
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        self.render_goal_list(chunks[0], buf);
        self.render_detail_panel(chunks[1], buf);
    }
}
