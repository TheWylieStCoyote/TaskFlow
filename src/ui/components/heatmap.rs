//! Heatmap view component - GitHub-style contribution graph.
//!
//! Displays task completion activity over time as a color-coded grid,
//! similar to GitHub's contribution graph.

use chrono::{Datelike, Duration, Local, NaiveDate};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::Model;
use crate::config::Theme;

/// Number of weeks to display in the heatmap
const WEEKS_TO_DISPLAY: usize = 52;

/// Heatmap view widget showing task completion activity
pub struct Heatmap<'a> {
    model: &'a Model,
    theme: &'a Theme,
}

impl<'a> Heatmap<'a> {
    /// Create a new heatmap widget
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }

    /// Calculate completion counts per day for the past year
    fn get_completion_data(&self) -> std::collections::HashMap<NaiveDate, usize> {
        let mut counts = std::collections::HashMap::new();
        let today = Local::now().date_naive();
        let start_date = today - Duration::days(365);

        for task in self.model.tasks.values() {
            if task.status.is_complete() {
                if let Some(completed_at) = task.completed_at {
                    let date = completed_at.date_naive();
                    if date >= start_date && date <= today {
                        *counts.entry(date).or_insert(0) += 1;
                    }
                }
            }
        }

        counts
    }

    /// Get the color intensity for a completion count
    fn get_intensity_color(count: usize) -> Color {
        match count {
            0 => Color::Rgb(22, 27, 34),      // Empty - dark gray
            1 => Color::Rgb(14, 68, 41),      // Light green
            2..=3 => Color::Rgb(0, 109, 50),  // Medium green
            4..=6 => Color::Rgb(38, 166, 65), // Bright green
            _ => Color::Rgb(57, 211, 83),     // Intense green
        }
    }

    /// Render the month labels
    fn render_month_labels(&self, area: Rect, buf: &mut Buffer) {
        let today = Local::now().date_naive();
        let mut months = Vec::new();
        let mut current_month = None;

        // Calculate which months appear in each week column
        for week in 0..WEEKS_TO_DISPLAY {
            let week_start = today - Duration::days((WEEKS_TO_DISPLAY - 1 - week) as i64 * 7);
            let month = week_start.month();

            if current_month != Some(month) {
                current_month = Some(month);
                let month_name = match month {
                    1 => "Jan",
                    2 => "Feb",
                    3 => "Mar",
                    4 => "Apr",
                    5 => "May",
                    6 => "Jun",
                    7 => "Jul",
                    8 => "Aug",
                    9 => "Sep",
                    10 => "Oct",
                    11 => "Nov",
                    12 => "Dec",
                    _ => "",
                };
                months.push((week, month_name));
            }
        }

        // Render month labels
        let style = Style::default().fg(self.theme.colors.muted.to_color());
        for (week, name) in months {
            if week < area.width as usize {
                let x = area.x + week as u16;
                if x < area.x + area.width {
                    buf.set_string(x, area.y, name, style);
                }
            }
        }
    }

    /// Render the day labels (Mon, Wed, Fri)
    fn render_day_labels(&self, area: Rect, buf: &mut Buffer) {
        let style = Style::default().fg(self.theme.colors.muted.to_color());
        let labels = ["", "Mon", "", "Wed", "", "Fri", ""];

        for (i, label) in labels.iter().enumerate() {
            if i < area.height as usize {
                buf.set_string(area.x, area.y + i as u16, *label, style);
            }
        }
    }

    /// Render the heatmap grid
    fn render_grid(&self, area: Rect, buf: &mut Buffer) {
        let today = Local::now().date_naive();
        let data = self.get_completion_data();

        // Each cell is 1 character wide, 1 row tall
        for week in 0..WEEKS_TO_DISPLAY.min(area.width as usize) {
            for day in 0..7 {
                if day >= area.height as usize {
                    break;
                }

                // Calculate the date for this cell
                let days_ago = (WEEKS_TO_DISPLAY - 1 - week) * 7 + (6 - day);
                let date = today - Duration::days(days_ago as i64);

                // Skip future dates
                if date > today {
                    continue;
                }

                let count = data.get(&date).copied().unwrap_or(0);
                let color = Self::get_intensity_color(count);

                let x = area.x + week as u16;
                let y = area.y + day as u16;

                if x < area.x + area.width && y < area.y + area.height {
                    buf.set_string(x, y, "█", Style::default().fg(color));
                }
            }
        }
    }

    /// Render the legend
    fn render_legend(&self, area: Rect, buf: &mut Buffer) {
        let style = Style::default().fg(self.theme.colors.muted.to_color());
        buf.set_string(area.x, area.y, "Less ", style);

        let levels = [0, 1, 2, 4, 7];
        let mut x = area.x + 5;
        for &level in &levels {
            let color = Self::get_intensity_color(level);
            buf.set_string(x, area.y, "█", Style::default().fg(color));
            x += 1;
        }

        buf.set_string(x + 1, area.y, " More", style);
    }

    /// Render the stats summary
    fn render_stats(&self, area: Rect, buf: &mut Buffer) {
        let data = self.get_completion_data();
        let total: usize = data.values().sum();
        let today = Local::now().date_naive();

        // Calculate current streak
        let mut streak = 0;
        let mut check_date = today;
        while data.get(&check_date).copied().unwrap_or(0) > 0 {
            streak += 1;
            check_date -= Duration::days(1);
        }

        // Calculate longest streak
        let mut longest_streak = 0;
        let mut current_streak = 0;
        let start_date = today - Duration::days(365);
        let mut date = start_date;
        while date <= today {
            if data.get(&date).copied().unwrap_or(0) > 0 {
                current_streak += 1;
                longest_streak = longest_streak.max(current_streak);
            } else {
                current_streak = 0;
            }
            date += Duration::days(1);
        }

        // Calculate busiest day
        let busiest = data.values().max().copied().unwrap_or(0);

        let stats = vec![
            Line::from(vec![
                Span::styled(
                    format!("{total} "),
                    Style::default()
                        .fg(self.theme.colors.accent.to_color())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "tasks completed in the last year",
                    Style::default().fg(self.theme.colors.foreground.to_color()),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Current streak: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{streak} days"),
                    Style::default().fg(self.theme.colors.foreground.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Longest streak: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{longest_streak} days"),
                    Style::default().fg(self.theme.colors.foreground.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Best day: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{busiest} tasks"),
                    Style::default().fg(self.theme.colors.foreground.to_color()),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(stats);
        paragraph.render(area, buf);
    }
}

impl Widget for Heatmap<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Heatmap - Task Completion Activity ")
            .title_style(
                Style::default()
                    .fg(self.theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.colors.border.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 20 || inner.height < 10 {
            return;
        }

        // Layout: month labels (1 row), day labels (3 chars) + grid, legend, stats
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Month labels
                Constraint::Length(7), // Grid (7 days)
                Constraint::Length(1), // Spacing
                Constraint::Length(1), // Legend
                Constraint::Length(1), // Spacing
                Constraint::Min(5),    // Stats
            ])
            .split(inner);

        // Day labels area
        let grid_with_labels = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(4), Constraint::Min(1)])
            .split(chunks[1]);

        self.render_month_labels(
            Rect::new(
                chunks[0].x + 4,
                chunks[0].y,
                chunks[0].width.saturating_sub(4),
                1,
            ),
            buf,
        );
        self.render_day_labels(grid_with_labels[0], buf);
        self.render_grid(grid_with_labels[1], buf);
        self.render_legend(chunks[3], buf);
        self.render_stats(chunks[5], buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heatmap_intensity_levels() {
        // Different counts should give different colors
        let color0 = Heatmap::get_intensity_color(0);
        let color1 = Heatmap::get_intensity_color(1);
        let color5 = Heatmap::get_intensity_color(5);
        let color10 = Heatmap::get_intensity_color(10);

        assert_ne!(color0, color1);
        assert_ne!(color1, color5);
        assert_ne!(color5, color10);
    }
}
