//! Timeline rendering methods.

use chrono::{Datelike, Duration, NaiveDate};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget,
    },
};

use crate::app::TimelineZoom;
use crate::domain::Task;

use super::{Timeline, LABEL_WIDTH};

impl Timeline<'_> {
    pub(crate) fn render_title_bar(&self, area: Rect, buf: &mut Buffer, _today: NaiveDate) {
        let theme = self.theme;
        let state = &self.model.timeline_state;

        let viewport_end =
            state.viewport_start + Duration::days(i64::from(state.viewport_days) - 1);
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

    pub(crate) fn render_date_headers(&self, area: Rect, buf: &mut Buffer, today: NaiveDate) {
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
                    weekday_row.push_str(&format!("{weekday:>col_width$}"));
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

    pub(crate) fn render_task_rows(
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

            // Task label (truncated), with optional time prefix
            let max_label = (LABEL_WIDTH - 2) as usize;
            let display_text = if let Some(time_str) = task.scheduled_time_display() {
                format!("{} {}", time_str, task.title)
            } else {
                task.title.clone()
            };
            let label = if display_text.len() > max_label {
                format!("{}...", &display_text[..max_label - 3])
            } else {
                format!("{display_text:<max_label$}")
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

        // Render scrollbar if content exceeds viewport
        let total_tasks = tasks.len();
        if total_tasks > visible_rows {
            let mut scrollbar_state = ScrollbarState::new(total_tasks.saturating_sub(1))
                .position(state.selected_task_index);

            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .track_symbol(Some("│"))
                .thumb_symbol("█")
                .track_style(Style::default().fg(theme.colors.muted.to_color()))
                .thumb_style(Style::default().fg(theme.colors.accent.to_color()));

            StatefulWidget::render(scrollbar, area, buf, &mut scrollbar_state);
        }
    }

    #[allow(clippy::unused_self)] // Keep self for API consistency
    pub(crate) fn build_bar_string(
        &self,
        len: usize,
        extends_left: bool,
        extends_right: bool,
    ) -> String {
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

        format!("{start_char}{fill}{end_char}")
    }

    pub(crate) fn render_dependency_lines(
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
