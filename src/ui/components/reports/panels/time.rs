//! Time panel rendering.
//!
//! Layout, top to bottom (lower sections are dropped first when the area is
//! short, nothing is ever drawn outside its section):
//!
//! 1. Header: `Time (last 30 days)  [w] window`
//! 2. Stat boxes: Total (window), Today, This Week, Avg/active day, Tracking
//! 3. By Project (top 8, hours + share of total) | Daily (a bar per day for
//!    the 7-day window, a sparkline for 30/90 days)
//! 4. By Hour (24 local-hour columns) | Top Tasks (top 5 by minutes)
//! 5. Minutes by Day of Week
//!
//! The window (7/30/90 days) comes from `model.reports_time_window`; its
//! analytics come from the matching cached report, or are computed on the
//! fly when the cache is cold. Today / This Week are always summed straight
//! from `model.time_entries` so they do not depend on the window.

use std::borrow::Cow;

use chrono::NaiveDate;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Widget},
};

use crate::app::analytics::AnalyticsEngine;
use crate::app::TimeWindow;
use crate::domain::analytics::{ReportConfig, TimeAnalytics};
use crate::domain::duration::format_hours_decimal;
use crate::domain::text::truncate_chars;
use crate::domain::{format_hms, format_minutes};
use crate::ui::components::charts::{BarChart, Sparkline, StatBox, SPARKLINE_CHARS};
use crate::ui::components::dashboard::stats::DashboardStats;

use super::super::ReportsView;

/// Minimum heights (rows, borders included) at which a section is worth drawing.
const STATS_HEIGHT: u16 = 4;
const MIN_MIDDLE: u16 = 5;
const PREF_MIDDLE: u16 = 10; // 8 project rows + borders
const MIN_LOWER: u16 = 6;
const PREF_LOWER: u16 = 7; // 5 task rows + borders
const MIN_DOW: u16 = 5;
const PREF_DOW: u16 = 9; // 7 weekday rows + borders

/// Below this width the five stat boxes cannot render; fall back to text.
const MIN_STAT_BOX_WIDTH: u16 = 45;

/// Time analytics for the selected window plus the dates it covers.
pub(super) struct WindowData<'a> {
    pub(super) analytics: Cow<'a, TimeAnalytics>,
    #[allow(dead_code)]
    pub(super) start: NaiveDate,
    #[allow(dead_code)]
    pub(super) end: NaiveDate,
}

/// One row of a horizontal bar chart: label, bar value, right-aligned text.
struct BarRow {
    label: String,
    value: u32,
    text: String,
}

impl ReportsView<'_> {
    pub(crate) fn render_time(&self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        let window = self.model.reports_time_window;
        let data = self.time_window_data(window);
        let analytics: &TimeAnalytics = &data.analytics;

        // Header row
        self.render_time_header(area, buf, window);
        let body = Rect::new(
            area.x,
            area.y + 1,
            area.width,
            area.height.saturating_sub(1),
        );

        // Allocate sections from the top; whatever does not fit is dropped.
        let mut constraints = vec![Constraint::Length(STATS_HEIGHT.min(body.height))];
        let remaining = body.height.saturating_sub(STATS_HEIGHT);
        let (middle, lower, dow) = section_heights(remaining);
        for height in [middle, lower, dow] {
            if height > 0 {
                constraints.push(Constraint::Length(height));
            }
        }
        constraints.push(Constraint::Min(0));
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(body);

        self.render_time_stats(chunks[0], buf, analytics);

        let mut next = 1;
        if middle > 0 {
            let halves = split_halves(chunks[next]);
            self.render_by_project(halves[0], buf, analytics);
            self.render_daily(halves[1], buf, &data, window);
            next += 1;
        }
        if lower > 0 {
            let halves = split_halves(chunks[next]);
            self.render_by_hour(halves[0], buf, analytics);
            self.render_top_tasks(halves[1], buf, analytics);
            next += 1;
        }
        if dow > 0 {
            render_day_of_week(chunks[next], buf, analytics);
        }
    }

    /// Analytics for `window`: the cached report when present, otherwise just
    /// the time breakdown computed now (cheaper than a whole report).
    pub(super) fn time_window_data(&self, window: TimeWindow) -> WindowData<'_> {
        if let Some(report) = self.model.report_cache.report_for_window(window) {
            return WindowData {
                analytics: Cow::Borrowed(&report.time_analytics),
                start: report.config.start_date,
                end: report.config.end_date,
            };
        }
        let config = ReportConfig::last_n_days(window.days());
        let analytics = AnalyticsEngine::new(self.model)
            .compute_time_analytics(config.start_date, config.end_date);
        WindowData {
            analytics: Cow::Owned(analytics),
            start: config.start_date,
            end: config.end_date,
        }
    }

    fn render_time_header(&self, area: Rect, buf: &mut Buffer, window: TimeWindow) {
        let title = format!("Time (last {})", window.label());
        let title_style = Style::default()
            .fg(self.theme.colors.accent.to_color())
            .add_modifier(Modifier::BOLD);
        let hint = "  [w] window";
        let line = truncate_chars(&format!("{title}{hint}"), area.width as usize, "");
        buf.set_string(area.x, area.y, &line, title_style);
        // Re-colour the hint part muted when it fully fits.
        if line.chars().count() == title.chars().count() + hint.chars().count() {
            buf.set_string(
                area.x + title.chars().count() as u16,
                area.y,
                hint,
                Style::default().fg(self.theme.colors.muted.to_color()),
            );
        }
    }

    /// Tracking readout: `HH:MM:SS <title>` while a timer runs, else `idle`.
    fn tracking_value(&self, max_chars: usize) -> String {
        match self.model.active_tracking() {
            Some((title, secs)) => {
                let clock = format_hms(secs);
                let room = max_chars.saturating_sub(clock.len() + 1);
                if room == 0 {
                    clock
                } else {
                    format!("{clock} {}", truncate_chars(&title, room, "…"))
                }
            }
            None => "idle".to_string(),
        }
    }

    fn render_time_stats(&self, area: Rect, buf: &mut Buffer, analytics: &TimeAnalytics) {
        if area.height == 0 {
            return;
        }
        let stats = DashboardStats::new(self.model);
        let total = format_minutes(analytics.total_minutes);
        let today = format_minutes(stats.minutes_tracked_today());
        let week = format_minutes(stats.minutes_tracked_this_week());
        let avg = format_minutes(analytics.avg_minutes_per_active_day());

        if area.width < MIN_STAT_BOX_WIDTH || area.height < 3 {
            // Compact text fallback for narrow or very short areas.
            let muted = Style::default().fg(self.theme.colors.muted.to_color());
            let lines = [
                format!("Total {total} · Today {today} · Week {week}"),
                format!(
                    "Avg/day {avg} · Tracking {}",
                    self.tracking_value(area.width as usize)
                ),
            ];
            for (i, line) in lines.iter().enumerate() {
                if (i as u16) < area.height {
                    let text = truncate_chars(line, area.width as usize, "…");
                    buf.set_string(area.x, area.y + i as u16, &text, muted);
                }
            }
            return;
        }

        let boxes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(18),
                Constraint::Percentage(18),
                Constraint::Percentage(18),
                Constraint::Percentage(18),
                Constraint::Percentage(28),
            ])
            .split(area);

        StatBox::new("Total", &total).render(boxes[0], buf);
        StatBox::new("Today", &today).render(boxes[1], buf);
        StatBox::new("This Week", &week).render(boxes[2], buf);
        // "Avg/active day" needs 16 columns including the borders
        let avg_label = if boxes[3].width >= 16 {
            "Avg/active day"
        } else {
            "Avg/day"
        };
        StatBox::new(avg_label, &avg).render(boxes[3], buf);
        let tracking = self.tracking_value(boxes[4].width.saturating_sub(2) as usize);
        StatBox::new("Tracking", &tracking).render(boxes[4], buf);
    }

    fn render_by_project(&self, area: Rect, buf: &mut Buffer, analytics: &TimeAnalytics) {
        let total = analytics.total_minutes.max(1);
        let rows: Vec<BarRow> = analytics
            .projects_by_minutes()
            .into_iter()
            .filter(|(_, minutes)| *minutes > 0)
            .take(8)
            .map(|(project_id, minutes)| {
                let label = match project_id {
                    None => "No project".to_string(),
                    Some(id) => self
                        .model
                        .projects
                        .get(&id)
                        .map_or_else(|| "(deleted project)".to_string(), |p| p.name.clone()),
                };
                let percent = (u64::from(minutes) * 100 + u64::from(total) / 2) / u64::from(total);
                BarRow {
                    label,
                    value: minutes,
                    text: format!("{} {percent:>3}%", format_hours_decimal(minutes)),
                }
            })
            .collect();
        render_hbar_chart(area, buf, "By Project", &rows, Color::Cyan, self.theme);
    }

    fn render_daily(
        &self,
        area: Rect,
        buf: &mut Buffer,
        data: &WindowData<'_>,
        window: TimeWindow,
    ) {
        let series = data.analytics.daily_series(data.start, data.end);

        if window == TimeWindow::Days7 {
            let rows: Vec<BarRow> = series
                .iter()
                .map(|(date, minutes)| BarRow {
                    label: date.format("%a %d").to_string(),
                    value: *minutes,
                    text: format_minutes(*minutes),
                })
                .collect();
            render_hbar_chart(area, buf, "Daily", &rows, Color::Green, self.theme);
            return;
        }

        let Some(inner) = titled_block(area, buf, " Daily ") else {
            return;
        };
        let Some((max_date, max_minutes)) = series
            .iter()
            .copied()
            .filter(|(_, m)| *m > 0)
            .max_by_key(|(_, m)| *m)
        else {
            render_empty_note(inner, buf, self.theme);
            return;
        };

        let values: Vec<f64> = series.iter().map(|(_, m)| f64::from(*m)).collect();
        let title = format!(
            "max {} on {}",
            format_minutes(max_minutes),
            max_date.format("%b %d")
        );
        let spark_area = Rect::new(inner.x, inner.y, inner.width, inner.height.min(2));
        Sparkline::new(&title, &values)
            .line_color(Color::Green)
            .render(spark_area, buf);

        if inner.height >= 3 {
            let muted = Style::default().fg(self.theme.colors.muted.to_color());
            let first = data.start.format("%b %d").to_string();
            let last = data.end.format("%b %d").to_string();
            let y = inner.y + 2;
            if inner.width as usize > first.len() + last.len() {
                buf.set_string(inner.x, y, &first, muted);
                buf.set_string(inner.x + inner.width - last.len() as u16, y, &last, muted);
            }
        }
    }

    fn render_by_hour(&self, area: Rect, buf: &mut Buffer, analytics: &TimeAnalytics) {
        let title = analytics.peak_hour().map_or_else(
            || " By Hour ".to_string(),
            |h| format!(" By Hour (peak {h:02}:00) "),
        );
        let Some(inner) = titled_block(area, buf, &title) else {
            return;
        };
        if inner.width < 24 || inner.height < 2 {
            return;
        }

        let col_w = (inner.width / 24).clamp(1, 2);
        let chart_w = col_w * 24;
        let x0 = inner.x + (inner.width - chart_w) / 2;
        let rows = inner.height - 1; // last row holds the hour labels
        let max = analytics.by_hour.iter().copied().max().unwrap_or(0);

        if max == 0 {
            render_empty_note(
                Rect::new(inner.x, inner.y, inner.width, rows),
                buf,
                self.theme,
            );
        } else {
            let bar_style = Style::default().fg(Color::Magenta);
            for (hour, &minutes) in analytics.by_hour.iter().enumerate() {
                // Height in eighths of a cell, rounded to nearest.
                let eighths = (u64::from(minutes) * u64::from(rows) * 8 + u64::from(max) / 2)
                    / u64::from(max);
                let x = x0 + hour as u16 * col_w;
                for from_bottom in 0..u64::from(rows) {
                    let filled = eighths.saturating_sub(from_bottom * 8).min(8);
                    if filled == 0 {
                        break;
                    }
                    let y = inner.y + rows - 1 - from_bottom as u16;
                    let glyph = SPARKLINE_CHARS[(filled - 1) as usize];
                    buf.set_string(x, y, glyph.to_string(), bar_style);
                }
            }
        }

        // Label every third hour
        let muted = Style::default().fg(self.theme.colors.muted.to_color());
        let label_y = inner.y + rows;
        for hour in (0..24u16).step_by(3) {
            let label = hour.to_string();
            let x = x0 + hour * col_w;
            if x + label.len() as u16 <= inner.x + inner.width {
                buf.set_string(x, label_y, &label, muted);
            }
        }
    }

    fn render_top_tasks(&self, area: Rect, buf: &mut Buffer, analytics: &TimeAnalytics) {
        let Some(inner) = titled_block(area, buf, " Top Tasks ") else {
            return;
        };
        let top: Vec<(String, u32)> = analytics
            .tasks_by_minutes()
            .into_iter()
            .filter(|(_, m)| *m > 0)
            .take(5)
            .map(|(task_id, minutes)| {
                let title = self
                    .model
                    .tasks
                    .get(&task_id)
                    .map_or_else(|| "(deleted task)".to_string(), |t| t.title.clone());
                (title, minutes)
            })
            .collect();
        if top.is_empty() {
            render_empty_note(inner, buf, self.theme);
            return;
        }

        let muted = Style::default().fg(self.theme.colors.muted.to_color());
        let text_style = Style::default().fg(self.theme.colors.foreground.to_color());
        for (i, (title, minutes)) in top.iter().enumerate() {
            if i as u16 >= inner.height {
                break;
            }
            let y = inner.y + i as u16;
            let duration = format_minutes(*minutes);
            let room = (inner.width as usize).saturating_sub(duration.len() + 1);
            if room > 0 {
                let shown = truncate_chars(title, room, "…");
                buf.set_string(inner.x, y, &shown, text_style);
            }
            if duration.len() as u16 <= inner.width {
                buf.set_string(
                    inner.x + inner.width - duration.len() as u16,
                    y,
                    &duration,
                    muted,
                );
            }
        }
    }
}

fn render_day_of_week(area: Rect, buf: &mut Buffer, analytics: &TimeAnalytics) {
    let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let day_data: Vec<(String, u32)> = analytics
        .by_day_of_week
        .iter()
        .enumerate()
        .map(|(i, &v)| (day_names[i].to_string(), v))
        .collect();
    BarChart::new("Minutes by Day of Week", &day_data)
        .bar_color(Color::Yellow)
        .render(area, buf);
}

/// Decide how tall the middle (projects/daily), lower (hour/tasks) and
/// day-of-week sections get for `remaining` rows below the stat boxes.
/// A zero means the section is dropped.
fn section_heights(remaining: u16) -> (u16, u16, u16) {
    if remaining < MIN_MIDDLE {
        (0, 0, 0)
    } else if remaining < MIN_MIDDLE + MIN_LOWER {
        (remaining.min(PREF_MIDDLE), 0, 0)
    } else if remaining < PREF_MIDDLE + MIN_LOWER {
        (remaining - MIN_LOWER, MIN_LOWER, 0)
    } else if remaining < PREF_MIDDLE + PREF_LOWER + MIN_DOW {
        (PREF_MIDDLE, (remaining - PREF_MIDDLE).min(PREF_LOWER), 0)
    } else {
        (
            PREF_MIDDLE,
            PREF_LOWER,
            (remaining - PREF_MIDDLE - PREF_LOWER).min(PREF_DOW),
        )
    }
}

fn split_halves(area: Rect) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area)
}

/// Draw a dark-grey bordered block with `title`; returns the inner area, or
/// `None` when the area is too small to hold anything.
pub(super) fn titled_block(area: Rect, buf: &mut Buffer, title: &str) -> Option<Rect> {
    if area.width < 10 || area.height < 3 {
        return None;
    }
    let block = Block::default()
        .title(title.to_string())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    block.render(area, buf);
    Some(inner)
}

/// "No time tracked in this window" placeholder for an empty section.
pub(super) fn render_empty_note(inner: Rect, buf: &mut Buffer, theme: &crate::config::Theme) {
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let note = truncate_chars("No time tracked in this window", inner.width as usize, "…");
    buf.set_string(
        inner.x,
        inner.y,
        &note,
        Style::default().fg(theme.colors.muted.to_color()),
    );
}

/// Horizontal bar chart with a right-aligned text value per row.
///
/// Unlike [`BarChart`] this is Unicode-safe for labels and lets each row
/// carry its own value text (`1.5h 43%`, `2h 10m`, ...).
fn render_hbar_chart(
    area: Rect,
    buf: &mut Buffer,
    title: &str,
    rows: &[BarRow],
    color: Color,
    theme: &crate::config::Theme,
) {
    let Some(inner) = titled_block(area, buf, &format!(" {title} ")) else {
        return;
    };
    if rows.is_empty() {
        render_empty_note(inner, buf, theme);
        return;
    }

    let width = inner.width as usize;
    let text_w = rows
        .iter()
        .map(|r| r.text.chars().count())
        .max()
        .unwrap_or(0);
    let mut label_w = rows
        .iter()
        .map(|r| r.label.chars().count())
        .max()
        .unwrap_or(0)
        .min(16)
        .min(width / 3);
    if width < label_w + text_w + 2 {
        label_w = width.saturating_sub(text_w + 2);
    }
    let bar_w = width.saturating_sub(label_w + text_w + 2);
    let max = rows.iter().map(|r| r.value).max().unwrap_or(0).max(1);

    let label_style = Style::default().fg(theme.colors.foreground.to_color());
    let bar_style = Style::default().fg(color);
    let text_style = Style::default().fg(theme.colors.muted.to_color());

    for (i, row) in rows.iter().enumerate() {
        if i as u16 >= inner.height {
            break;
        }
        let y = inner.y + i as u16;
        if label_w > 0 {
            let label = truncate_chars(&row.label, label_w, "…");
            buf.set_string(inner.x, y, format!("{label:>label_w$}"), label_style);
        }
        let filled = (u64::from(row.value) * bar_w as u64 / u64::from(max)) as usize;
        if filled > 0 {
            let bar: String = "█".repeat(filled);
            buf.set_string(inner.x + label_w as u16 + 1, y, &bar, bar_style);
        }
        if text_w > 0 && text_w <= width {
            let text = format!("{:>text_w$}", row.text);
            buf.set_string(inner.x + (width - text_w) as u16, y, &text, text_style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::section_heights;

    #[test]
    fn section_heights_drop_lower_sections_first() {
        assert_eq!(section_heights(0), (0, 0, 0));
        assert_eq!(section_heights(4), (0, 0, 0));
        assert_eq!(section_heights(5), (5, 0, 0), "only the middle row fits");
        assert_eq!(section_heights(10), (10, 0, 0));
        assert_eq!(
            section_heights(11),
            (5, 6, 0),
            "lower row appears at its minimum"
        );
        assert_eq!(section_heights(14), (8, 6, 0), "80x24 case: 6 project rows");
        assert_eq!(section_heights(16), (10, 6, 0));
        assert_eq!(section_heights(17), (10, 7, 0));
        assert_eq!(section_heights(21), (10, 7, 0), "day-of-week needs 5 rows");
        assert_eq!(section_heights(22), (10, 7, 5));
        assert_eq!(
            section_heights(40),
            (10, 7, 9),
            "day-of-week caps at 7 rows"
        );
    }
}
