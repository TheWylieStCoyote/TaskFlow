//! Timeline/Gantt view component.
//!
//! Displays tasks as horizontal bars on a time axis, allowing users to
//! visualize project schedules, identify overlaps, and see task dependencies.

mod render;
#[cfg(test)]
mod tests;

use chrono::{Duration, NaiveDate, Utc};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::app::{Model, TimelineZoom};
use crate::config::Theme;
use crate::domain::Task;

/// Width of the task label column
pub(crate) const LABEL_WIDTH: u16 = 22;

/// Timeline/Gantt view widget showing tasks as bars on a time axis.
pub struct Timeline<'a> {
    pub(crate) model: &'a Model,
    pub(crate) theme: &'a Theme,
}

impl<'a> Timeline<'a> {
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }

    /// Get tasks with at least one date, sorted by start date.
    pub(crate) fn timeline_tasks(&self) -> Vec<&Task> {
        use crate::domain::Priority;

        let mut tasks: Vec<_> = self
            .model
            .visible_tasks
            .iter()
            .filter_map(|id| self.model.tasks.get(id))
            .filter(|t| t.scheduled_date.is_some() || t.due_date.is_some())
            .collect();

        // Priority ordering (higher number = higher priority for sorting)
        let priority_order = |p: &Priority| match p {
            Priority::Urgent => 4,
            Priority::High => 3,
            Priority::Medium => 2,
            Priority::Low => 1,
            Priority::None => 0,
        };

        // Sort by start date (scheduled or due), then by priority (descending), then by title
        tasks.sort_by(|a, b| {
            let a_start = a.scheduled_date.or(a.due_date);
            let b_start = b.scheduled_date.or(b.due_date);
            a_start
                .cmp(&b_start)
                .then_with(|| priority_order(&b.priority).cmp(&priority_order(&a.priority)))
                .then_with(|| a.title.cmp(&b.title))
        });

        tasks
    }

    /// Compute the date span for a task.
    pub(crate) fn task_span(task: &Task) -> (NaiveDate, NaiveDate) {
        let today = Utc::now().date_naive();

        match (task.scheduled_date, task.due_date, task.estimated_minutes) {
            // Both dates present
            (Some(start), Some(end), _) => (start, end),
            // Only scheduled date with estimate
            (Some(start), None, Some(mins)) => {
                let days = (mins / 480).max(1) as i64; // 8 hours per day
                (start, start + Duration::days(days))
            }
            // Only due date with estimate
            (None, Some(end), Some(mins)) => {
                let days = (mins / 480).max(1) as i64;
                (end - Duration::days(days), end)
            }
            // Single date (milestone)
            (Some(d), None, None) | (None, Some(d), None) => (d, d),
            // No dates (shouldn't happen due to filter)
            (None, None, _) => (today, today),
        }
    }

    /// Convert a date to a column position relative to viewport start.
    /// Returns (column_index, is_within_viewport)
    pub(crate) fn date_to_column(&self, date: NaiveDate) -> Option<i64> {
        let state = &self.model.timeline_state;
        let viewport_start = state.viewport_start;
        let days = (date - viewport_start).num_days();

        let (num_columns, days_per_column) = self.zoom_params();

        let column = days / days_per_column;

        if column >= 0 && column < num_columns as i64 {
            Some(column)
        } else {
            None
        }
    }

    /// Get zoom parameters: (number of columns, days per column)
    pub(crate) fn zoom_params(&self) -> (usize, i64) {
        let state = &self.model.timeline_state;
        match state.zoom_level {
            TimelineZoom::Day => (state.viewport_days as usize, 1),
            TimelineZoom::Week => (state.viewport_days as usize, 7),
        }
    }

    /// Get the color for a task based on status and priority.
    pub(crate) fn task_color(&self, task: &Task) -> Color {
        let theme = self.theme;

        // Status takes precedence
        if task.status.is_complete() {
            return theme.status.done.to_color();
        }
        if task.status == crate::domain::TaskStatus::InProgress {
            return theme.status.in_progress.to_color();
        }
        if task.status == crate::domain::TaskStatus::Blocked {
            return theme.colors.warning.to_color();
        }

        // Then priority
        match task.priority {
            crate::domain::Priority::Urgent => theme.priority.urgent.to_color(),
            crate::domain::Priority::High => theme.priority.high.to_color(),
            _ => theme.colors.accent.to_color(),
        }
    }
}

impl Widget for Timeline<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 5 || area.width < 40 {
            return; // Too small to render
        }

        let theme = self.theme;
        let state = &self.model.timeline_state;
        let viewport_days = state.viewport_days as usize;
        let today = Utc::now().date_naive();

        // Layout: header (2 lines) + tasks + footer (1 line)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title bar
                Constraint::Length(2), // Date headers
                Constraint::Min(3),    // Task rows
                Constraint::Length(1), // Footer hints
            ])
            .split(area);

        // === Title Bar ===
        self.render_title_bar(chunks[0], buf, today);

        // === Date Headers ===
        self.render_date_headers(chunks[1], buf, today);

        // === Task Rows ===
        self.render_task_rows(chunks[2], buf, today, viewport_days);

        // === Footer Hints ===
        let hints = if self.model.timeline_state.show_dependencies {
            " h/l scroll  j/k select  Enter view  </> zoom  t today  d deps:ON  ? help "
        } else {
            " h/l scroll  j/k select  Enter view  </> zoom  t today  d deps  ? help "
        };
        let footer = Line::from(Span::styled(
            hints,
            Style::default().fg(theme.colors.muted.to_color()),
        ));
        buf.set_line(chunks[3].x, chunks[3].y, &footer, chunks[3].width);
    }
}
