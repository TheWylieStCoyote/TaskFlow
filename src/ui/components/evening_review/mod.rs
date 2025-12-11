//! Evening review component for end-of-day reflection.
//!
//! The evening review is a structured 6-phase workflow that helps users:
//!
//! 1. **Celebrate accomplishments** - Review tasks completed today
//! 2. **Address incomplete work** - Handle tasks that weren't finished
//! 3. **Prepare for tomorrow** - Preview upcoming tasks
//! 4. **Review time spent** - Analyze time tracking data
//!
//! # Phases
//!
//! | Phase | Title | Purpose |
//! |-------|-------|---------|
//! | 1 | Evening Review | Welcome with day summary stats |
//! | 2 | Today's Wins | Celebrate completed tasks |
//! | 3 | Unfinished Business | Handle incomplete tasks (reschedule/snooze) |
//! | 4 | Tomorrow's Plan | Preview tomorrow's tasks |
//! | 5 | Time Spent | Time tracking summary (auto-skips if empty) |
//! | 6 | Day Complete | Final stats and encouraging close |
//!
//! # Keybinding
//!
//! The evening review is triggered with `Shift+E` by default.

mod phase;
mod queries;
mod render;

#[cfg(test)]
mod tests;

pub use phase::EveningReviewPhase;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, Widget},
};

use crate::app::Model;
use crate::config::Theme;

/// Evening review widget for end-of-day reflection.
///
/// This widget renders the evening review overlay, guiding users through
/// a structured workflow to close out their day.
pub struct EveningReview<'a> {
    pub(crate) model: &'a Model,
    pub(crate) theme: &'a Theme,
    pub(crate) phase: EveningReviewPhase,
    pub(crate) selected: usize,
}

impl<'a> EveningReview<'a> {
    /// Create a new evening review widget.
    #[must_use]
    pub fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self {
            model,
            theme,
            phase: model.evening_review.phase,
            selected: model.evening_review.selected,
        }
    }

    /// Create a new evening review widget with explicit phase and selection.
    #[must_use]
    pub const fn with_state(
        model: &'a Model,
        theme: &'a Theme,
        phase: EveningReviewPhase,
        selected: usize,
    ) -> Self {
        Self {
            model,
            theme,
            phase,
            selected,
        }
    }

    /// Render the evening review as a popup overlay.
    pub(crate) fn render_popup(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let theme = self.theme;

        // Main container
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " Evening Review ({}/{}) - {} ",
                self.phase.number(),
                EveningReviewPhase::TOTAL_PHASES,
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
            EveningReviewPhase::Welcome => self.render_welcome(chunks[0], buf),
            EveningReviewPhase::CompletedToday => self.render_completed_today(chunks[0], buf),
            EveningReviewPhase::IncompleteTasks => self.render_incomplete_tasks(chunks[0], buf),
            EveningReviewPhase::TomorrowPreview => self.render_tomorrow_preview(chunks[0], buf),
            EveningReviewPhase::TimeReview => self.render_time_review(chunks[0], buf),
            EveningReviewPhase::Summary => self.render_summary(chunks[0], buf),
        }

        // Render progress and help
        self.render_footer(chunks[1], buf);
    }
}

impl Widget for EveningReview<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_popup(area, buf);
    }
}
