//! Tasks panel: every task with tracked time in the selected window, as a
//! scrollable table.
//!
//! Layout, top to bottom:
//!
//! 1. Header: `Tasks (last 30 days)  [w] window · j/k scroll`
//! 2. Table: `#`, Task, Project, Entries, Time, Share — most time first
//! 3. Footer: total for the window and the visible range
//!
//! The window is `model.reports_time_window` (shared with the Time panel);
//! the selected row is `model.reports_tasks_selected`, kept in view by
//! centring it once the table overflows.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
};

use crate::domain::analytics::{TaskTimeRow, TimeAnalytics};
use crate::domain::format_minutes;
use crate::domain::text::truncate_chars;

use super::super::ReportsView;
use super::time::{render_empty_note, titled_block};

/// Fixed column widths (including one leading space each).
const COL_INDEX: u16 = 4;
const COL_ENTRIES: u16 = 8;
const COL_TIME: u16 = 9;
const COL_SHARE: u16 = 7;
/// Project column: a quarter of the width, capped.
const MAX_PROJECT: u16 = 18;
const MIN_PROJECT: u16 = 8;
/// Below this width the project column is dropped.
const MIN_WIDTH_FOR_PROJECT: u16 = 60;

/// Column x-offsets and widths for a table `width` columns wide.
struct Columns {
    task: u16,
    project: Option<u16>,
}

impl Columns {
    fn for_width(width: u16) -> Self {
        let fixed = COL_INDEX + COL_ENTRIES + COL_TIME + COL_SHARE;
        let project = if width >= MIN_WIDTH_FOR_PROJECT {
            Some((width / 4).clamp(MIN_PROJECT, MAX_PROJECT))
        } else {
            None
        };
        let task = width
            .saturating_sub(fixed)
            .saturating_sub(project.unwrap_or(0))
            .max(1);
        Self { task, project }
    }
}

impl ReportsView<'_> {
    pub(crate) fn render_tasks(&self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        let window = self.model.reports_time_window;
        let data = self.time_window_data(window);
        let analytics: &TimeAnalytics = &data.analytics;
        let rows = analytics.task_rows();

        // Header line
        let header_style = Style::default()
            .fg(self.theme.colors.accent.to_color())
            .add_modifier(Modifier::BOLD);
        let header = format!(
            "Tasks (last {})  [w] window · j/k scroll · {} task{}",
            window.label(),
            rows.len(),
            if rows.len() == 1 { "" } else { "s" }
        );
        buf.set_string(
            area.x,
            area.y,
            truncate_chars(&header, area.width as usize, "…"),
            header_style,
        );

        let body = Rect::new(
            area.x,
            area.y + 1,
            area.width,
            area.height.saturating_sub(1),
        );
        let Some(inner) = titled_block(body, buf, " Time by Task ") else {
            return;
        };
        if rows.is_empty() {
            render_empty_note(inner, buf, self.theme);
            return;
        }
        if inner.height < 2 {
            return;
        }

        let cols = Columns::for_width(inner.width);
        let muted = Style::default().fg(self.theme.colors.muted.to_color());
        let text = Style::default().fg(self.theme.colors.foreground.to_color());
        let selected_style = Style::default()
            .fg(self.theme.colors.accent.to_color())
            .add_modifier(Modifier::BOLD | Modifier::REVERSED);

        // Column headings
        let heading = format_row(
            &cols,
            "#",
            "Task",
            cols.project.map(|_| "Project"),
            "Entries",
            "Time",
            "Share",
        );
        buf.set_string(
            inner.x,
            inner.y,
            &heading,
            muted.add_modifier(Modifier::BOLD),
        );

        // Footer takes the last row when there is room for it.
        let has_footer = inner.height >= 4;
        let visible = (inner.height - 1 - u16::from(has_footer)) as usize;
        if visible == 0 {
            return;
        }
        let selected = self.model.reports_tasks_selected.min(rows.len() - 1);
        let offset = scroll_offset(selected, rows.len(), visible);

        for (i, row) in rows.iter().enumerate().skip(offset).take(visible) {
            let y = inner.y + 1 + (i - offset) as u16;
            let project = cols.project.map(|_| self.project_label(row));
            let line = format_row(
                &cols,
                &format!("{}", i + 1),
                &row.title,
                project.as_deref(),
                &row.entries.to_string(),
                &format_minutes(row.minutes),
                &format!("{:.0}%", row.share),
            );
            let style = if i == selected { selected_style } else { text };
            // Pad to the full width so the highlight spans the row.
            let padded = format!("{line:<width$}", width = inner.width as usize);
            buf.set_string(
                inner.x,
                y,
                truncate_chars(&padded, inner.width as usize, ""),
                style,
            );
        }

        if has_footer {
            let first = offset + 1;
            let last = (offset + visible).min(rows.len());
            let footer = format!(
                "Total {} · showing {first}–{last} of {}",
                format_minutes(analytics.total_minutes),
                rows.len()
            );
            buf.set_string(
                inner.x,
                inner.y + inner.height - 1,
                truncate_chars(&footer, inner.width as usize, "…"),
                muted,
            );
        }
    }

    fn project_label(&self, row: &TaskTimeRow) -> String {
        row.project_id
            .and_then(|id| self.model.projects.get(&id))
            .map_or_else(|| "(no project)".to_string(), |p| p.name.clone())
    }
}

#[allow(clippy::too_many_arguments)]
fn format_row(
    cols: &Columns,
    index: &str,
    task: &str,
    project: Option<&str>,
    entries: &str,
    time: &str,
    share: &str,
) -> String {
    let mut line = String::new();
    line.push_str(&format!("{index:>w$}", w = COL_INDEX as usize - 1));
    line.push(' ');
    line.push_str(&format!(
        "{:<w$}",
        truncate_chars(task, cols.task as usize, "…"),
        w = cols.task as usize
    ));
    if let (Some(width), Some(project)) = (cols.project, project) {
        line.push(' ');
        line.push_str(&format!(
            "{:<w$}",
            truncate_chars(project, width as usize - 1, "…"),
            w = width as usize - 1
        ));
    }
    line.push_str(&format!("{entries:>w$}", w = COL_ENTRIES as usize));
    line.push_str(&format!("{time:>w$}", w = COL_TIME as usize));
    line.push_str(&format!("{share:>w$}", w = COL_SHARE as usize));
    line
}

/// First visible row so that `selected` is centred once the table overflows.
fn scroll_offset(selected: usize, total: usize, visible: usize) -> usize {
    if total <= visible {
        return 0;
    }
    let max_offset = total - visible;
    selected.saturating_sub(visible / 2).min(max_offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_offset_centres_selection_and_clamps() {
        assert_eq!(scroll_offset(0, 5, 10), 0, "fits: never scroll");
        assert_eq!(scroll_offset(0, 50, 10), 0);
        assert_eq!(scroll_offset(4, 50, 10), 0, "still in the first page");
        assert_eq!(scroll_offset(25, 50, 10), 20, "centred");
        assert_eq!(scroll_offset(49, 50, 10), 40, "clamped at the end");
    }

    #[test]
    fn columns_drop_project_on_narrow_tables() {
        let wide = Columns::for_width(100);
        assert_eq!(wide.project, Some(MAX_PROJECT));
        assert_eq!(
            wide.task,
            100 - (COL_INDEX + COL_ENTRIES + COL_TIME + COL_SHARE) - MAX_PROJECT
        );
        let narrow = Columns::for_width(50);
        assert_eq!(narrow.project, None);
        assert_eq!(
            narrow.task,
            50 - (COL_INDEX + COL_ENTRIES + COL_TIME + COL_SHARE)
        );
        let tiny = Columns::for_width(10);
        assert_eq!(tiny.task, 1, "never zero wide");
    }
}
