//! Daily review mode component.
//!
//! Provides a guided workflow for morning planning:
//! 1. Review overdue tasks
//! 2. Review tasks due today
//! 3. Review scheduled tasks for today
//! 4. Quick summary and planning

mod phase;
mod queries;
mod render;

#[cfg(test)]
mod tests;

pub use phase::DailyReviewPhase;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, Widget},
};

use crate::app::Model;
use crate::config::Theme;

/// Daily review view widget
pub struct DailyReview<'a> {
    pub(crate) model: &'a Model,
    pub(crate) theme: &'a Theme,
    pub(crate) phase: DailyReviewPhase,
    pub(crate) selected: usize,
}

impl<'a> DailyReview<'a> {
    #[must_use]
    pub const fn new(
        model: &'a Model,
        theme: &'a Theme,
        phase: DailyReviewPhase,
        selected: usize,
    ) -> Self {
        Self {
            model,
            theme,
            phase,
            selected,
        }
    }
}

impl Widget for DailyReview<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let theme = self.theme;

        // Main container
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " Daily Review ({}/5) - {} ",
                self.phase.number(),
                self.phase.title()
            ))
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(theme.colors.accent.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        // Layout: content area + progress/help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // Content
                Constraint::Length(3), // Progress bar + help
            ])
            .split(inner);

        // Render phase-specific content
        match self.phase {
            DailyReviewPhase::Welcome => self.render_welcome(chunks[0], buf),
            DailyReviewPhase::OverdueTasks => self.render_task_list(
                chunks[0],
                buf,
                &self.overdue_tasks(),
                "No overdue tasks! 🎉",
            ),
            DailyReviewPhase::TodayTasks => {
                self.render_task_list(chunks[0], buf, &self.today_tasks(), "No tasks due today");
            }
            DailyReviewPhase::ScheduledTasks => self.render_task_list(
                chunks[0],
                buf,
                &self.scheduled_today_tasks(),
                "No scheduled tasks for today",
            ),
            DailyReviewPhase::Summary => self.render_summary(chunks[0], buf),
        }

        // Render progress and help
        self.render_footer(chunks[1], buf);
    }
}
