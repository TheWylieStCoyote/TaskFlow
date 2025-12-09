//! Weekly review mode component.
//!
//! Provides a GTD-style weekly review workflow:
//! 1. Review completed tasks from the past week
//! 2. Review and process overdue tasks
//! 3. Review upcoming week tasks
//! 4. Check projects for stalled work
//! 5. Weekly summary and stats

mod phase;
mod queries;
mod render;
#[cfg(test)]
mod tests;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, Widget},
};

use crate::app::Model;
use crate::config::Theme;

pub use phase::WeeklyReviewPhase;

/// Weekly review view widget
pub struct WeeklyReview<'a> {
    pub(crate) model: &'a Model,
    pub(crate) theme: &'a Theme,
    pub(crate) phase: WeeklyReviewPhase,
    pub(crate) selected: usize,
}

impl<'a> WeeklyReview<'a> {
    #[must_use]
    pub const fn new(
        model: &'a Model,
        theme: &'a Theme,
        phase: WeeklyReviewPhase,
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

impl Widget for WeeklyReview<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let theme = self.theme;

        // Main container
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " Weekly Review ({}/6) - {} ",
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
            WeeklyReviewPhase::Welcome => self.render_welcome(chunks[0], buf),
            WeeklyReviewPhase::CompletedTasks => {
                let tasks = self.completed_this_week();
                self.render_task_list(chunks[0], buf, &tasks, "No completed tasks this week", true)
            }
            WeeklyReviewPhase::OverdueTasks => {
                let tasks = self.overdue_tasks();
                self.render_task_list(chunks[0], buf, &tasks, "No overdue tasks! 🎉", false)
            }
            WeeklyReviewPhase::UpcomingWeek => {
                let tasks = self.upcoming_week_tasks();
                self.render_task_list(chunks[0], buf, &tasks, "No tasks due this week", false)
            }
            WeeklyReviewPhase::StaleProjects => self.render_stale_projects(chunks[0], buf),
            WeeklyReviewPhase::Summary => self.render_summary(chunks[0], buf),
        }

        // Render progress and help
        self.render_footer(chunks[1], buf);
    }
}
