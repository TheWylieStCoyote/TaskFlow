//! Habit tracking view component.
//!
//! Displays habits with streaks, check-in calendar, and analytics.

mod analytics;
mod list;

#[cfg(test)]
mod tests;

pub use analytics::HabitAnalyticsPopup;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    widgets::Widget,
};

use crate::app::Model;
use crate::config::Theme;

/// Habit tracking view widget.
pub struct HabitsView<'a> {
    pub(crate) model: &'a Model,
    pub(crate) theme: &'a Theme,
}

impl<'a> HabitsView<'a> {
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }

    pub(crate) fn completion_rate_color(&self, rate: f64) -> Color {
        let theme = self.theme;
        if rate >= 0.8 {
            theme.colors.success.to_color()
        } else if rate >= 0.5 {
            theme.colors.accent.to_color()
        } else if rate >= 0.3 {
            theme.colors.warning.to_color()
        } else {
            theme.colors.danger.to_color()
        }
    }
}

impl Widget for HabitsView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Split into habit list (left) and analytics (right)
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        self.render_habit_list(chunks[0], buf);
        self.render_analytics(chunks[1], buf);
    }
}
