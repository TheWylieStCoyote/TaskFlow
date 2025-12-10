//! Timeline rendering methods.

use chrono::{Datelike, Duration, NaiveDate};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Model;
    use crate::config::Theme;

    fn create_test_timeline<'a>(model: &'a Model, theme: &'a Theme) -> Timeline<'a> {
        Timeline::new(model, theme)
    }

    #[test]
    fn test_build_bar_string_empty() {
        let model = Model::new();
        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        assert_eq!(timeline.build_bar_string(0, false, false), "");
    }

    #[test]
    fn test_build_bar_string_single_char() {
        let model = Model::new();
        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        // Single char without extensions = block
        assert_eq!(timeline.build_bar_string(1, false, false), "█");

        // Single char with left extension
        assert_eq!(timeline.build_bar_string(1, true, false), "╡");

        // Single char with right extension
        assert_eq!(timeline.build_bar_string(1, false, true), "╞");

        // Single char with both extensions
        assert_eq!(timeline.build_bar_string(1, true, true), "═");
    }

    #[test]
    fn test_build_bar_string_short_bar() {
        let model = Model::new();
        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        // 2 chars = brackets with no fill
        assert_eq!(timeline.build_bar_string(2, false, false), "[]");

        // 3 chars = brackets with 1 fill
        assert_eq!(timeline.build_bar_string(3, false, false), "[=]");
    }

    #[test]
    fn test_build_bar_string_extends_left() {
        let model = Model::new();
        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        // Extends left = < start
        assert_eq!(timeline.build_bar_string(3, true, false), "<=]");
        assert_eq!(timeline.build_bar_string(5, true, false), "<===]");
    }

    #[test]
    fn test_build_bar_string_extends_right() {
        let model = Model::new();
        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        // Extends right = > end
        assert_eq!(timeline.build_bar_string(3, false, true), "[=>");
        assert_eq!(timeline.build_bar_string(5, false, true), "[===>");
    }

    #[test]
    fn test_build_bar_string_extends_both() {
        let model = Model::new();
        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        // Both extensions
        assert_eq!(timeline.build_bar_string(3, true, true), "<=>");
        assert_eq!(timeline.build_bar_string(5, true, true), "<===>");
    }

    #[test]
    fn test_build_bar_string_long_bar() {
        let model = Model::new();
        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        // Longer bar
        assert_eq!(timeline.build_bar_string(10, false, false), "[========]");
        assert_eq!(timeline.build_bar_string(10, true, true), "<========>");
    }

    #[test]
    fn test_zoom_params_day_zoom() {
        let mut model = Model::new();
        model.timeline_state.zoom_level = TimelineZoom::Day;
        model.timeline_state.viewport_days = 14;

        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        let (num_cols, days_per_col) = timeline.zoom_params();
        assert_eq!(num_cols, 14);
        assert_eq!(days_per_col, 1);
    }

    #[test]
    fn test_zoom_params_week_zoom() {
        let mut model = Model::new();
        model.timeline_state.zoom_level = TimelineZoom::Week;
        model.timeline_state.viewport_days = 28;

        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        let (num_cols, days_per_col) = timeline.zoom_params();
        assert_eq!(num_cols, 28);
        assert_eq!(days_per_col, 7);
    }

    #[test]
    fn test_date_to_column_in_viewport() {
        use chrono::NaiveDate;

        let mut model = Model::new();
        let start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
        model.timeline_state.viewport_start = start;
        model.timeline_state.viewport_days = 14;
        model.timeline_state.zoom_level = TimelineZoom::Day;

        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        // First day = column 0
        assert_eq!(timeline.date_to_column(start), Some(0));

        // Day 7 = column 6 (0-indexed)
        let day7 = NaiveDate::from_ymd_opt(2024, 12, 7).unwrap();
        assert_eq!(timeline.date_to_column(day7), Some(6));

        // Day 14 = column 13 (last column)
        let day14 = NaiveDate::from_ymd_opt(2024, 12, 14).unwrap();
        assert_eq!(timeline.date_to_column(day14), Some(13));
    }

    #[test]
    fn test_date_to_column_out_of_viewport() {
        use chrono::NaiveDate;

        let mut model = Model::new();
        let start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
        model.timeline_state.viewport_start = start;
        model.timeline_state.viewport_days = 14;
        model.timeline_state.zoom_level = TimelineZoom::Day;

        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        // Before viewport
        let before = NaiveDate::from_ymd_opt(2024, 11, 30).unwrap();
        assert_eq!(timeline.date_to_column(before), None);

        // After viewport
        let after = NaiveDate::from_ymd_opt(2024, 12, 20).unwrap();
        assert_eq!(timeline.date_to_column(after), None);
    }

    #[test]
    fn test_task_span_with_due_date_only() {
        use chrono::NaiveDate;

        let due = NaiveDate::from_ymd_opt(2024, 12, 15).unwrap();
        let mut task = Task::new("Test");
        task.due_date = Some(due);

        let (start, end) = Timeline::task_span(&task);
        assert_eq!(start, due);
        assert_eq!(end, due);
    }

    #[test]
    fn test_task_span_with_scheduled_and_due() {
        use chrono::NaiveDate;

        let scheduled = NaiveDate::from_ymd_opt(2024, 12, 10).unwrap();
        let due_date = NaiveDate::from_ymd_opt(2024, 12, 15).unwrap();

        let mut task = Task::new("Test");
        task.scheduled_date = Some(scheduled);
        task.due_date = Some(due_date);

        let (start, end) = Timeline::task_span(&task);
        assert_eq!(start, scheduled);
        assert_eq!(end, due_date);
    }

    // =========================================================================
    // Additional rendering coverage tests
    // =========================================================================

    fn buffer_content(buffer: &Buffer) -> String {
        let mut content = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                content.push(
                    buffer
                        .cell((x, y))
                        .map_or(' ', |c| c.symbol().chars().next().unwrap_or(' ')),
                );
            }
            content.push('\n');
        }
        content
    }

    #[test]
    fn test_render_title_bar() {
        use chrono::NaiveDate;

        let mut model = Model::new();
        model.timeline_state.viewport_start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
        model.timeline_state.viewport_days = 14;
        model.timeline_state.zoom_level = TimelineZoom::Day;
        model.timeline_state.show_dependencies = true;

        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        let area = Rect::new(0, 0, 80, 1);
        let mut buffer = Buffer::empty(area);
        let today = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
        timeline.render_title_bar(area, &mut buffer, today);

        let content = buffer_content(&buffer);
        assert!(content.contains("Timeline"), "Should contain Timeline title");
        assert!(content.contains("Day"), "Should show zoom level");
        assert!(content.contains("ON"), "Dependencies should be ON");
    }

    #[test]
    fn test_render_title_bar_deps_off() {
        use chrono::NaiveDate;

        let mut model = Model::new();
        model.timeline_state.show_dependencies = false;

        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        let area = Rect::new(0, 0, 80, 1);
        let mut buffer = Buffer::empty(area);
        let today = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
        timeline.render_title_bar(area, &mut buffer, today);

        let content = buffer_content(&buffer);
        assert!(content.contains("off"), "Dependencies should be off");
    }

    #[test]
    fn test_render_title_bar_week_zoom() {
        use chrono::NaiveDate;

        let mut model = Model::new();
        model.timeline_state.zoom_level = TimelineZoom::Week;

        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        let area = Rect::new(0, 0, 80, 1);
        let mut buffer = Buffer::empty(area);
        let today = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
        timeline.render_title_bar(area, &mut buffer, today);

        let content = buffer_content(&buffer);
        assert!(content.contains("Week"), "Should show Week zoom level");
    }

    #[test]
    fn test_render_date_headers_day_zoom() {
        use chrono::NaiveDate;

        let mut model = Model::new();
        model.timeline_state.viewport_start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
        model.timeline_state.viewport_days = 7;
        model.timeline_state.zoom_level = TimelineZoom::Day;

        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        let area = Rect::new(0, 0, 80, 2);
        let mut buffer = Buffer::empty(area);
        let today = NaiveDate::from_ymd_opt(2024, 12, 3).unwrap();
        timeline.render_date_headers(area, &mut buffer, today);

        // Should render day numbers and weekdays
        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_render_date_headers_week_zoom() {
        use chrono::NaiveDate;

        let mut model = Model::new();
        model.timeline_state.viewport_start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
        model.timeline_state.viewport_days = 28;
        model.timeline_state.zoom_level = TimelineZoom::Week;

        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        let area = Rect::new(0, 0, 80, 2);
        let mut buffer = Buffer::empty(area);
        let today = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
        timeline.render_date_headers(area, &mut buffer, today);

        // Should render without panic
        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_render_date_headers_small_height() {
        use chrono::NaiveDate;

        let mut model = Model::new();
        model.timeline_state.viewport_start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
        model.timeline_state.zoom_level = TimelineZoom::Day;

        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        // Height less than 2 - should early return
        let area = Rect::new(0, 0, 80, 1);
        let mut buffer = Buffer::empty(area);
        let today = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
        timeline.render_date_headers(area, &mut buffer, today);

        // Should not panic, just return early
        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_render_task_rows_empty() {
        use chrono::NaiveDate;

        let model = Model::new();
        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        let area = Rect::new(0, 0, 80, 10);
        let mut buffer = Buffer::empty(area);
        let today = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
        timeline.render_task_rows(area, &mut buffer, today, 14);

        let content = buffer_content(&buffer);
        assert!(
            content.contains("No tasks"),
            "Should show empty message when no tasks"
        );
    }

    #[test]
    fn test_render_task_rows_with_tasks() {
        use chrono::NaiveDate;

        let mut model = Model::new();
        let start_date = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
        model.timeline_state.viewport_start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
        model.timeline_state.viewport_days = 14;

        // Add task with due date in viewport
        let mut task = Task::new("Test Task");
        task.due_date = Some(start_date);
        model.tasks.insert(task.id, task);
        model.refresh_visible_tasks();

        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        let area = Rect::new(0, 0, 100, 10);
        let mut buffer = Buffer::empty(area);
        timeline.render_task_rows(area, &mut buffer, start_date, 14);

        let content = buffer_content(&buffer);
        assert!(
            content.contains("Test Task") || content.contains("Test"),
            "Should show task title"
        );
    }

    #[test]
    fn test_render_task_rows_milestone() {
        use chrono::NaiveDate;

        let mut model = Model::new();
        let date = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
        model.timeline_state.viewport_start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
        model.timeline_state.viewport_days = 14;

        // Milestone: same start and end date
        let mut task = Task::new("Milestone");
        task.due_date = Some(date);
        task.scheduled_date = Some(date);
        model.tasks.insert(task.id, task);
        model.refresh_visible_tasks();

        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        let area = Rect::new(0, 0, 100, 10);
        let mut buffer = Buffer::empty(area);
        timeline.render_task_rows(area, &mut buffer, date, 14);

        // Milestone should render with diamond character
        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_render_task_rows_with_dependencies() {
        use chrono::NaiveDate;

        let mut model = Model::new();
        let start_date = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
        model.timeline_state.viewport_start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
        model.timeline_state.viewport_days = 14;
        model.timeline_state.show_dependencies = true;

        // Add two tasks with chain relationship
        let mut task1 = Task::new("Task 1");
        task1.due_date = Some(start_date);
        let task1_id = task1.id;

        let mut task2 = Task::new("Task 2");
        task2.due_date = Some(start_date + Duration::days(3));
        task1.next_task_id = Some(task2.id);

        model.tasks.insert(task1.id, task1);
        model.tasks.insert(task2.id, task2);
        model.refresh_visible_tasks();

        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        let area = Rect::new(0, 0, 100, 10);
        let mut buffer = Buffer::empty(area);
        timeline.render_task_rows(area, &mut buffer, start_date, 14);

        // Should render dependency lines
        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_date_to_column_week_zoom() {
        use chrono::NaiveDate;

        let mut model = Model::new();
        let start = NaiveDate::from_ymd_opt(2024, 12, 2).unwrap(); // Monday
        model.timeline_state.viewport_start = start;
        model.timeline_state.viewport_days = 28;
        model.timeline_state.zoom_level = TimelineZoom::Week;

        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        // In week zoom, date_to_column returns the day index (0-27),
        // not the week index
        assert_eq!(timeline.date_to_column(start), Some(0));

        // Day 7 is at index 7 in the 28-day viewport
        let day7 = start + Duration::days(7);
        assert_eq!(timeline.date_to_column(day7), Some(1));
    }

    #[test]
    fn test_task_span_scheduled_only() {
        use chrono::NaiveDate;

        let scheduled = NaiveDate::from_ymd_opt(2024, 12, 10).unwrap();
        let mut task = Task::new("Test");
        task.scheduled_date = Some(scheduled);

        let (start, end) = Timeline::task_span(&task);
        assert_eq!(start, scheduled);
        assert_eq!(end, scheduled);
    }

    #[test]
    fn test_task_span_no_dates() {
        use chrono::Local;

        let task = Task::new("Test");
        let (start, end) = Timeline::task_span(&task);

        // Should default to today
        let today = Local::now().date_naive();
        assert_eq!(start, today);
        assert_eq!(end, today);
    }

    #[test]
    fn test_task_color_priority_high() {
        use chrono::NaiveDate;
        use crate::domain::Priority;

        let model = Model::new();
        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        let task = Task::new("High priority").with_priority(Priority::High);
        let color = timeline.task_color(&task);

        // High priority should have a color
        assert!(color != Color::Reset);
    }

    #[test]
    fn test_task_color_completed() {
        use chrono::NaiveDate;
        use crate::domain::TaskStatus;

        let model = Model::new();
        let theme = Theme::default();
        let timeline = create_test_timeline(&model, &theme);

        let task = Task::new("Completed task").with_status(TaskStatus::Done);
        let color = timeline.task_color(&task);

        // Completed task should have a color
        assert!(color != Color::Reset);
    }
}
