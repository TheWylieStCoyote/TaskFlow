//! Timeline/Gantt view component.
//!
//! Displays tasks as horizontal bars on a time axis, allowing users to
//! visualize project schedules, identify overlaps, and see task dependencies.

use chrono::{Datelike, Duration, NaiveDate, Utc};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::{Model, TimelineZoom};
use crate::config::Theme;
use crate::domain::Task;

/// Width of the task label column
const LABEL_WIDTH: u16 = 22;

/// Timeline/Gantt view widget showing tasks as bars on a time axis.
pub struct Timeline<'a> {
    model: &'a Model,
    theme: &'a Theme,
}

impl<'a> Timeline<'a> {
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }

    /// Get tasks with at least one date, sorted by start date.
    fn timeline_tasks(&self) -> Vec<&Task> {
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
    fn task_span(task: &Task) -> (NaiveDate, NaiveDate) {
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
    fn date_to_column(&self, date: NaiveDate) -> Option<i64> {
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
    fn zoom_params(&self) -> (usize, i64) {
        let state = &self.model.timeline_state;
        match state.zoom_level {
            TimelineZoom::Day => (state.viewport_days as usize, 1),
            TimelineZoom::Week => (state.viewport_days as usize, 7),
        }
    }

    /// Get the color for a task based on status and priority.
    fn task_color(&self, task: &Task) -> Color {
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

impl Timeline<'_> {
    fn render_title_bar(&self, area: Rect, buf: &mut Buffer, _today: NaiveDate) {
        let theme = self.theme;
        let state = &self.model.timeline_state;

        let viewport_end = state.viewport_start + Duration::days(state.viewport_days as i64 - 1);
        let zoom_str = match state.zoom_level {
            TimelineZoom::Day => "Day",
            TimelineZoom::Week => "Week",
        };

        let deps_str = if state.show_dependencies { "ON" } else { "off" };

        let title = format!(
            " Timeline - {} to {}   Zoom: {}   Deps: {} ",
            state.viewport_start.format("%b %d"),
            viewport_end.format("%b %d, %Y"),
            zoom_str,
            deps_str,
        );

        let title_line = Line::from(Span::styled(
            title,
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        ));
        buf.set_line(area.x, area.y, &title_line, area.width);
    }

    fn render_date_headers(&self, area: Rect, buf: &mut Buffer, today: NaiveDate) {
        let theme = self.theme;
        let state = &self.model.timeline_state;
        let (num_columns, days_per_column) = self.zoom_params();

        if area.height < 2 {
            return;
        }

        // Calculate available width for dates
        let date_area_width = area.width.saturating_sub(LABEL_WIDTH) as usize;
        let col_width = (date_area_width / num_columns).max(1);

        let label_padding = " ".repeat(LABEL_WIDTH as usize);

        match state.zoom_level {
            TimelineZoom::Day => {
                // Row 1: Day numbers
                let mut day_nums = String::new();
                let mut weekday_row = String::new();

                for i in 0..num_columns {
                    let date = state.viewport_start + Duration::days(i as i64);
                    let day_str = format!("{:>width$}", date.day(), width = col_width);
                    day_nums.push_str(&day_str);

                    let weekday = match date.weekday() {
                        chrono::Weekday::Mon => "Mo",
                        chrono::Weekday::Tue => "Tu",
                        chrono::Weekday::Wed => "We",
                        chrono::Weekday::Thu => "Th",
                        chrono::Weekday::Fri => "Fr",
                        chrono::Weekday::Sat => "Sa",
                        chrono::Weekday::Sun => "Su",
                    };
                    weekday_row.push_str(&format!("{:>width$}", weekday, width = col_width));
                }

                // Render day numbers row
                let day_line = Line::from(vec![
                    Span::styled(&label_padding, Style::default()),
                    Span::styled(
                        day_nums,
                        Style::default().fg(theme.colors.foreground.to_color()),
                    ),
                ]);
                buf.set_line(area.x, area.y, &day_line, area.width);

                // Render weekday row
                let weekday_line = Line::from(vec![
                    Span::styled(&label_padding, Style::default()),
                    Span::styled(
                        weekday_row,
                        Style::default().fg(theme.colors.muted.to_color()),
                    ),
                ]);
                buf.set_line(area.x, area.y + 1, &weekday_line, area.width);
            }
            TimelineZoom::Week => {
                // Row 1: Week start dates (e.g., "Dec 02", "Dec 09", ...)
                let mut week_dates = String::new();
                let mut week_nums = String::new();

                for i in 0..num_columns {
                    let date = state.viewport_start + Duration::days((i as i64) * days_per_column);
                    let week_str = format!("{:>width$}", date.format("%b %d"), width = col_width);
                    week_dates.push_str(&week_str);

                    let week_num = date.iso_week().week();
                    let wk_str = format!("{:>width$}", format!("W{}", week_num), width = col_width);
                    week_nums.push_str(&wk_str);
                }

                // Render week dates row
                let date_line = Line::from(vec![
                    Span::styled(&label_padding, Style::default()),
                    Span::styled(
                        week_dates,
                        Style::default().fg(theme.colors.foreground.to_color()),
                    ),
                ]);
                buf.set_line(area.x, area.y, &date_line, area.width);

                // Render week numbers row
                let week_line = Line::from(vec![
                    Span::styled(&label_padding, Style::default()),
                    Span::styled(
                        week_nums,
                        Style::default().fg(theme.colors.muted.to_color()),
                    ),
                ]);
                buf.set_line(area.x, area.y + 1, &week_line, area.width);
            }
        }

        // Highlight "today" column
        if let Some(today_col) = self.date_to_column(today) {
            let x = area.x + LABEL_WIDTH + (today_col as u16 * col_width as u16);
            for row in 0..2 {
                if x < area.x + area.width {
                    if let Some(cell) = buf.cell_mut((x, area.y + row)) {
                        cell.set_style(
                            Style::default()
                                .fg(theme.colors.accent.to_color())
                                .add_modifier(Modifier::BOLD),
                        );
                    }
                }
            }
        }
    }

    fn render_task_rows(
        &self,
        area: Rect,
        buf: &mut Buffer,
        today: NaiveDate,
        _viewport_days: usize,
    ) {
        let theme = self.theme;
        let state = &self.model.timeline_state;
        let tasks = self.timeline_tasks();
        let (num_columns, days_per_column) = self.zoom_params();

        if tasks.is_empty() {
            let empty = Paragraph::new("No tasks with dates. Press 'n' to create one.")
                .style(Style::default().fg(theme.colors.muted.to_color()));
            empty.render(area, buf);
            return;
        }

        // Calculate column width
        let date_area_width = area.width.saturating_sub(LABEL_WIDTH) as usize;
        let col_width = (date_area_width / num_columns).max(1);

        // Render block around task area
        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme.colors.border.to_color()));
        let inner = block.inner(area);
        block.render(area, buf);

        // Visible rows
        let visible_rows = inner.height as usize;
        let scroll_offset = state.task_scroll_offset;

        for (row_idx, task) in tasks
            .iter()
            .skip(scroll_offset)
            .take(visible_rows)
            .enumerate()
        {
            let y = inner.y + row_idx as u16;
            let is_selected = (row_idx + scroll_offset) == state.selected_task_index;

            // Task label (truncated)
            let max_label = (LABEL_WIDTH - 2) as usize;
            let label = if task.title.len() > max_label {
                format!("{}...", &task.title[..max_label - 3])
            } else {
                format!("{:<width$}", task.title, width = max_label)
            };

            let label_style = if is_selected {
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray)
            } else {
                Style::default().fg(theme.colors.foreground.to_color())
            };

            buf.set_string(inner.x, y, &label, label_style);
            buf.set_string(
                inner.x + LABEL_WIDTH - 2,
                y,
                "│ ",
                Style::default().fg(theme.colors.border.to_color()),
            );

            // Render task bar
            let (start, end) = Self::task_span(task);
            let bar_color = self.task_color(task);

            // Calculate bar position (in columns, accounting for zoom)
            let start_days = (start - state.viewport_start).num_days();
            let end_days = (end - state.viewport_start).num_days();

            // Convert days to columns based on zoom level
            let start_col = start_days / days_per_column;
            let end_col = end_days / days_per_column;

            let bar_start_col = start_col.max(0) as usize;
            let bar_end_col = end_col.min(num_columns as i64 - 1).max(0) as usize;

            // Only render if bar is visible
            if bar_start_col < num_columns {
                let bar_x = inner.x + LABEL_WIDTH + (bar_start_col * col_width) as u16;
                let bar_len = ((bar_end_col - bar_start_col + 1) * col_width).min(date_area_width);

                // Milestone (single day or within same column) uses diamond
                if start == end || start_col == end_col {
                    let milestone_char = if task.status.is_complete() {
                        "◆"
                    } else {
                        "◇"
                    };
                    buf.set_string(bar_x, y, milestone_char, Style::default().fg(bar_color));
                } else {
                    // Regular bar
                    let extends_left = start_col < 0;
                    let extends_right = end_col >= num_columns as i64;

                    let bar = self.build_bar_string(bar_len, extends_left, extends_right);
                    buf.set_string(bar_x, y, &bar, Style::default().fg(bar_color));
                }
            }

            // Highlight today column in task row
            if let Some(today_col) = self.date_to_column(today) {
                let x = inner.x + LABEL_WIDTH + (today_col as u16 * col_width as u16);
                if x < inner.x + inner.width {
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        // Add subtle background for today
                        if cell.symbol() == " " {
                            cell.set_char('│');
                            cell.set_style(Style::default().fg(theme.colors.muted.to_color()));
                        }
                    }
                }
            }
        }

        // Render dependency lines if enabled
        if state.show_dependencies {
            self.render_dependency_lines(
                inner,
                buf,
                &tasks,
                scroll_offset,
                visible_rows,
                col_width,
            );
        }
    }

    fn build_bar_string(&self, len: usize, extends_left: bool, extends_right: bool) -> String {
        if len == 0 {
            return String::new();
        }
        if len == 1 {
            return if extends_left && extends_right {
                "═".to_string()
            } else if extends_left {
                "╡".to_string()
            } else if extends_right {
                "╞".to_string()
            } else {
                "█".to_string()
            };
        }

        let start_char = if extends_left { '<' } else { '[' };
        let end_char = if extends_right { '>' } else { ']' };
        let fill = "=".repeat(len.saturating_sub(2));

        format!("{}{}{}", start_char, fill, end_char)
    }

    fn render_dependency_lines(
        &self,
        area: Rect,
        buf: &mut Buffer,
        tasks: &[&Task],
        scroll_offset: usize,
        visible_rows: usize,
        col_width: usize,
    ) {
        let theme = self.theme;

        // Build a map of task_id to row index
        let task_rows: std::collections::HashMap<_, _> =
            tasks.iter().enumerate().map(|(i, t)| (t.id, i)).collect();

        for (row_idx, task) in tasks
            .iter()
            .skip(scroll_offset)
            .take(visible_rows)
            .enumerate()
        {
            // Check for next_task_id (chain dependency)
            if let Some(next_id) = task.next_task_id {
                if let Some(&next_row) = task_rows.get(&next_id) {
                    // Draw connection if both are visible
                    let this_row_abs = row_idx + scroll_offset;
                    if next_row > this_row_abs && next_row < scroll_offset + visible_rows {
                        let (_, end) = Self::task_span(task);
                        if let Some(end_col) = self.date_to_column(end) {
                            let x =
                                area.x + LABEL_WIDTH + ((end_col as usize + 1) * col_width) as u16;
                            let y1 = area.y + row_idx as u16;
                            let y2 = area.y + (next_row - scroll_offset) as u16;

                            // Draw vertical line
                            for y in (y1 + 1)..y2 {
                                if x < area.x + area.width && y < area.y + area.height {
                                    buf.set_string(
                                        x,
                                        y,
                                        "│",
                                        Style::default().fg(theme.colors.muted.to_color()),
                                    );
                                }
                            }
                            // Arrow at end
                            if x < area.x + area.width && y2 < area.y + area.height {
                                buf.set_string(
                                    x,
                                    y2,
                                    "→",
                                    Style::default().fg(theme.colors.muted.to_color()),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_renders_without_panic() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let timeline = Timeline::new(&model, &theme);

        let area = Rect::new(0, 0, 120, 30);
        let mut buffer = Buffer::empty(area);
        timeline.render(area, &mut buffer);

        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_task_span_with_both_dates() {
        let mut task = Task::new("Test");
        task.scheduled_date = Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        task.due_date = Some(NaiveDate::from_ymd_opt(2024, 1, 5).unwrap());

        let (start, end) = Timeline::task_span(&task);
        assert_eq!(start, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2024, 1, 5).unwrap());
    }

    #[test]
    fn test_task_span_milestone() {
        let mut task = Task::new("Milestone");
        task.due_date = Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());

        let (start, end) = Timeline::task_span(&task);
        assert_eq!(start, end);
    }

    #[test]
    fn test_build_bar_string() {
        let model = Model::new();
        let theme = Theme::default();
        let timeline = Timeline::new(&model, &theme);

        assert_eq!(timeline.build_bar_string(5, false, false), "[===]");
        assert_eq!(timeline.build_bar_string(5, true, false), "<===]");
        assert_eq!(timeline.build_bar_string(5, false, true), "[===>");
        assert_eq!(timeline.build_bar_string(5, true, true), "<===>");
        assert_eq!(timeline.build_bar_string(1, false, false), "█");
    }
}
