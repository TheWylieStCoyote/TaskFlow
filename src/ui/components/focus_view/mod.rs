//! Focus mode view component.
//!
//! A minimalist, distraction-free view for working on a single task.
//! Displays the current task prominently with an optional Pomodoro timer
//! and task chain navigation.
//!
//! # Features
//!
//! - Large, centered task display
//! - Pomodoro timer with visual progress
//! - Task chain navigation (previous/next in sequence)
//! - Subtask progress indicator

mod render;
mod utils;

#[cfg(test)]
mod tests;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::Model;
use crate::config::Theme;

/// Focus mode view - minimalist single-task view with timer
pub struct FocusView<'a> {
    pub(crate) model: &'a Model,
    pub(crate) theme: &'a Theme,
}

impl<'a> FocusView<'a> {
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }
}

impl Widget for FocusView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        // Get the selected task
        let Some(task) = self.model.selected_task() else {
            // No task selected - shouldn't happen but handle gracefully
            let msg = Paragraph::new("No task selected")
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.colors.muted.to_color()));
            msg.render(area, buf);
            return;
        };

        // Create centered layout
        let outer_block = Block::default()
            .borders(Borders::ALL)
            .title(" FOCUS MODE ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(theme.colors.accent.to_color()));
        let inner = outer_block.inner(area);
        outer_block.render(area, buf);

        // Check if task is part of a chain
        let has_chain = task.next_task_id.is_some()
            || self
                .model
                .tasks
                .values()
                .any(|t| t.next_task_id == Some(task.id));

        // Layout: padding, content, chain info, timer, help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),                             // Top padding
                Constraint::Min(6),                                // Main content
                Constraint::Length(if has_chain { 2 } else { 0 }), // Chain info (conditional)
                Constraint::Length(3),                             // Timer
                Constraint::Length(2),                             // Help text
            ])
            .split(inner);

        // Render task title with status
        self.render_task_title(task, chunks[1], buf, theme);

        // Render chain info if applicable
        if has_chain {
            self.render_chain_info(task, chunks[2], buf, theme);
        }

        // Render timer
        self.render_timer(task, chunks[3], buf, theme);

        // Render help
        self.render_help(chunks[4], buf, theme, task);
    }
}
