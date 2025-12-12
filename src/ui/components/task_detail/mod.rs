//! Task detail modal component.
//!
//! Displays comprehensive task information in a scrollable popup modal.
//! Shows all task data: description, subtasks, dependencies, time entries,
//! work logs, and git integration info.

mod render;

#[cfg(test)]
mod tests;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::Line,
    widgets::{
        Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget, Widget, Wrap,
    },
};

use crate::app::Model;
use crate::config::Theme;

/// Task detail modal widget.
///
/// Renders a scrollable popup showing all task information.
pub struct TaskDetail<'a> {
    model: &'a Model,
    theme: &'a Theme,
    scroll: usize,
}

impl<'a> TaskDetail<'a> {
    /// Create a new task detail widget.
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme, scroll: usize) -> Self {
        Self {
            model,
            theme,
            scroll,
        }
    }
}

impl Widget for TaskDetail<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let theme = self.theme;

        // Get the selected task
        let Some(task) = self.model.selected_task() else {
            // No task selected - shouldn't happen but handle gracefully
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Task Details ")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(theme.colors.muted.to_color()));
            let inner = block.inner(area);
            block.render(area, buf);

            let msg = Paragraph::new("No task selected")
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.colors.muted.to_color()));
            msg.render(inner, buf);
            return;
        };

        // Main container with title
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Task Details (Esc to close, j/k to scroll) ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(theme.colors.accent.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        // Build content lines
        let content_lines = self.build_content_lines(task);

        // Render scrollable content
        let content_height = inner.height as usize;
        let total_lines = content_lines.len();
        let max_scroll = total_lines.saturating_sub(content_height);
        let scroll = self.scroll.min(max_scroll);

        let visible_lines: Vec<Line<'_>> = content_lines
            .into_iter()
            .skip(scroll)
            .take(content_height)
            .collect();

        let paragraph = Paragraph::new(visible_lines).wrap(Wrap { trim: false });

        paragraph.render(inner, buf);

        // Render scrollbar if content exceeds viewport
        if total_lines > content_height {
            let mut scrollbar_state =
                ScrollbarState::new(total_lines.saturating_sub(1)).position(scroll);

            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .track_symbol(Some("│"))
                .thumb_symbol("█")
                .track_style(Style::default().fg(theme.colors.muted.to_color()))
                .thumb_style(Style::default().fg(theme.colors.accent.to_color()));

            StatefulWidget::render(scrollbar, inner, buf, &mut scrollbar_state);
        }
    }
}

impl TaskDetail<'_> {
    /// Build all content lines for the task detail view.
    fn build_content_lines(&self, task: &crate::domain::Task) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Header: Title with status and priority
        lines.push(self.render_header(task));
        lines.push(Line::from(""));

        // Metadata section
        lines.extend(self.render_metadata(task));
        lines.push(Line::from(""));

        // Dates section
        lines.extend(self.render_dates(task));
        lines.push(Line::from(""));

        // Description section
        lines.extend(self.render_description(task));

        // Time tracking section
        lines.extend(self.render_time_tracking(task));

        // Subtasks section
        lines.extend(self.render_subtasks(task));

        // Dependencies section
        lines.extend(self.render_dependencies(task));

        // Task chain section
        lines.extend(self.render_task_chain(task));

        // Git integration section
        lines.extend(self.render_git_info(task));

        // Time entries section
        lines.extend(self.render_time_entries(task));

        // Work logs section
        lines.extend(self.render_work_logs(task));

        lines
    }
}
