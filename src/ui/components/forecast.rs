//! Forecast view component - workload projection into future weeks.
//!
//! Displays upcoming workload based on due dates and estimated times,
//! helping users plan capacity and identify overloaded periods.

use chrono::{Datelike, Duration, Local, NaiveDate};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Paragraph, Widget},
};

use crate::app::Model;
use crate::config::Theme;

/// Number of weeks to forecast
const FORECAST_WEEKS: usize = 8;

/// Standard work day capacity in hours
const DAILY_CAPACITY_HOURS: u32 = 8;

/// Forecast view widget showing workload projection
pub struct Forecast<'a> {
    model: &'a Model,
    theme: &'a Theme,
}

impl<'a> Forecast<'a> {
    /// Create a new forecast widget
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }

    /// Get the start of the week (Monday) for a given date
    fn week_start(date: NaiveDate) -> NaiveDate {
        let days_from_monday = date.weekday().num_days_from_monday();
        date - Duration::days(i64::from(days_from_monday))
    }

    /// Calculate workload per week (task count and estimated minutes)
    fn get_weekly_workload(&self) -> Vec<(NaiveDate, usize, u32)> {
        let today = Local::now().date_naive();
        let current_week_start = Self::week_start(today);

        let mut weeks: Vec<(NaiveDate, usize, u32)> = (0..FORECAST_WEEKS)
            .map(|i| {
                let week_start = current_week_start + Duration::weeks(i as i64);
                (week_start, 0, 0)
            })
            .collect();

        for task in self.model.tasks.values() {
            if task.status.is_complete() {
                continue;
            }

            if let Some(due) = task.due_date {
                let week_start = Self::week_start(due);
                if let Some(entry) = weeks.iter_mut().find(|(ws, _, _)| *ws == week_start) {
                    entry.1 += 1;
                    entry.2 += task.estimated_minutes.unwrap_or(30);
                }
            }
        }

        weeks
    }

    /// Calculate workload per day for the next 7 days (task count and estimated minutes)
    fn get_daily_workload(&self) -> Vec<(NaiveDate, usize, u32)> {
        let today = Local::now().date_naive();

        let mut days: Vec<(NaiveDate, usize, u32)> = (0..7)
            .map(|i| {
                let date = today + Duration::days(i);
                (date, 0, 0)
            })
            .collect();

        for task in self.model.tasks.values() {
            if task.status.is_complete() {
                continue;
            }

            if let Some(due) = task.due_date {
                if let Some(entry) = days.iter_mut().find(|(d, _, _)| *d == due) {
                    entry.1 += 1;
                    entry.2 += task.estimated_minutes.unwrap_or(30);
                }
            }
        }

        days
    }

    /// Render daily capacity breakdown for the week
    fn render_daily_capacity(&self, area: Rect, buf: &mut Buffer) {
        let daily_workload = self.get_daily_workload();

        let lines: Vec<Line<'_>> = daily_workload
            .iter()
            .map(|(date, task_count, minutes)| {
                let hours = *minutes / 60;
                let mins = *minutes % 60;
                let day_name = date.format("%a").to_string();
                let date_str = date.format("%m/%d").to_string();

                // Capacity warning colors based on hours
                let (indicator, color) = if hours >= DAILY_CAPACITY_HOURS {
                    ("●", Color::Red) // Overloaded
                } else if hours >= DAILY_CAPACITY_HOURS / 2 {
                    ("◐", Color::Yellow) // Half capacity or more
                } else if *task_count > 0 {
                    ("○", Color::Green) // Has work but not overloaded
                } else {
                    ("·", self.theme.colors.muted.to_color()) // No work
                };

                let hours_str = if hours > 0 || mins > 0 {
                    format!("{hours}h{mins:02}m")
                } else {
                    "—".to_string()
                };

                Line::from(vec![
                    Span::styled(format!("{indicator} "), Style::default().fg(color)),
                    Span::styled(
                        format!("{day_name} {date_str} "),
                        Style::default().fg(self.theme.colors.muted.to_color()),
                    ),
                    Span::styled(format!("{hours_str:>7}"), Style::default().fg(color)),
                    Span::styled(
                        format!(" ({task_count} tasks)"),
                        Style::default().fg(self.theme.colors.muted.to_color()),
                    ),
                ])
            })
            .collect();

        let block = Block::default()
            .title(" Daily Capacity (Next 7 Days) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.colors.border.to_color()));

        // Check for overloaded days
        let overloaded_days = daily_workload
            .iter()
            .filter(|(_, _, mins)| *mins / 60 >= DAILY_CAPACITY_HOURS)
            .count();

        let mut all_lines = lines;
        if overloaded_days > 0 {
            all_lines.push(Line::from(""));
            all_lines.push(Line::from(vec![
                Span::styled("⚠ ", Style::default().fg(Color::Red)),
                Span::styled(
                    format!("{overloaded_days} day(s) over {DAILY_CAPACITY_HOURS}h capacity"),
                    Style::default().fg(Color::Red),
                ),
            ]));
        }

        let paragraph = Paragraph::new(all_lines).block(block);
        paragraph.render(area, buf);
    }

    /// Render the weekly bar chart
    fn render_chart(&self, area: Rect, buf: &mut Buffer, workload: &[(NaiveDate, usize, u32)]) {
        let max_tasks = workload
            .iter()
            .map(|(_, count, _)| *count)
            .max()
            .unwrap_or(1)
            .max(1);

        let bars: Vec<Bar<'_>> = workload
            .iter()
            .map(|(date, count, _)| {
                let label = format!("{}/{}", date.month(), date.day());
                let color = if *count > 10 {
                    Color::Red
                } else if *count > 5 {
                    Color::Yellow
                } else {
                    Color::Green
                };

                Bar::default()
                    .value(*count as u64)
                    .label(Line::from(label))
                    .style(Style::default().fg(color))
            })
            .collect();

        let chart = BarChart::default()
            .block(
                Block::default()
                    .title(" Tasks Due Per Week ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.colors.border.to_color())),
            )
            .data(BarGroup::default().bars(&bars))
            .bar_width(7)
            .bar_gap(1)
            .max(max_tasks as u64);

        chart.render(area, buf);
    }

    /// Render workload summary
    fn render_summary(&self, area: Rect, buf: &mut Buffer, workload: &[(NaiveDate, usize, u32)]) {
        let total_tasks: usize = workload.iter().map(|(_, count, _)| *count).sum();
        let total_minutes: u32 = workload.iter().map(|(_, _, mins)| *mins).sum();
        let total_hours = total_minutes / 60;
        let remaining_mins = total_minutes % 60;

        let overloaded_weeks = workload.iter().filter(|(_, count, _)| *count > 10).count();

        let this_week = workload.first().map_or(0, |(_, count, _)| *count);
        let next_week = workload.get(1).map_or(0, |(_, count, _)| *count);

        let lines = vec![
            Line::from(vec![
                Span::styled(
                    "Total tasks: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{total_tasks}"),
                    Style::default()
                        .fg(self.theme.colors.accent.to_color())
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Estimated time: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{total_hours}h {remaining_mins}m"),
                    Style::default().fg(self.theme.colors.foreground.to_color()),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "This week: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{this_week} tasks"),
                    Style::default().fg(if this_week > 10 {
                        Color::Red
                    } else {
                        self.theme.colors.foreground.to_color()
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Next week: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{next_week} tasks"),
                    Style::default().fg(if next_week > 10 {
                        Color::Red
                    } else {
                        self.theme.colors.foreground.to_color()
                    }),
                ),
            ]),
            Line::from(""),
            if overloaded_weeks > 0 {
                Line::from(vec![
                    Span::styled("⚠ ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("{overloaded_weeks} overloaded week(s)"),
                        Style::default().fg(Color::Yellow),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled("✓ ", Style::default().fg(Color::Green)),
                    Span::styled(
                        "Workload looks manageable",
                        Style::default().fg(Color::Green),
                    ),
                ])
            },
        ];

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .title(" Summary ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.theme.colors.border.to_color())),
        );
        paragraph.render(area, buf);
    }

    /// Render upcoming deadlines
    fn render_deadlines(&self, area: Rect, buf: &mut Buffer) {
        let today = Local::now().date_naive();
        let mut upcoming: Vec<_> = self
            .model
            .tasks
            .values()
            .filter(|t| !t.status.is_complete() && t.due_date.is_some())
            .collect();

        upcoming.sort_by_key(|t| t.due_date);
        upcoming.truncate(10);

        let lines: Vec<Line<'_>> = upcoming
            .iter()
            .map(|task| {
                let due = task.due_date.unwrap();
                let days_until = (due - today).num_days();
                let date_str = if days_until == 0 {
                    "Today".to_string()
                } else if days_until == 1 {
                    "Tomorrow".to_string()
                } else if days_until < 7 {
                    format!("in {days_until} days")
                } else {
                    format!("{}/{}", due.month(), due.day())
                };

                let color = if days_until < 0 {
                    Color::Red
                } else if days_until == 0 {
                    Color::Yellow
                } else {
                    self.theme.colors.foreground.to_color()
                };

                Line::from(vec![
                    Span::styled(
                        format!("{date_str:>10} "),
                        Style::default().fg(self.theme.colors.muted.to_color()),
                    ),
                    Span::styled(
                        task.title.chars().take(30).collect::<String>(),
                        Style::default().fg(color),
                    ),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .title(" Upcoming Deadlines ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.theme.colors.border.to_color())),
        );
        paragraph.render(area, buf);
    }
}

impl Widget for Forecast<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Forecast - Workload Projection ")
            .title_style(
                Style::default()
                    .fg(self.theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.colors.border.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 40 || inner.height < 15 {
            return;
        }

        let workload = self.get_weekly_workload();

        // Layout: chart on top, three panels below (daily capacity, summary, deadlines)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(12), Constraint::Min(12)])
            .split(inner);

        let bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(35),
                Constraint::Percentage(25),
                Constraint::Percentage(40),
            ])
            .split(chunks[1]);

        self.render_chart(chunks[0], buf, &workload);
        self.render_daily_capacity(bottom[0], buf);
        self.render_summary(bottom[1], buf, &workload);
        self.render_deadlines(bottom[2], buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Model;
    use crate::config::Theme;
    use chrono::Weekday;

    #[test]
    fn test_week_start_calculation() {
        // Test that week_start returns Monday
        let wednesday = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap(); // A Wednesday
        let monday = Forecast::week_start(wednesday);
        assert_eq!(monday.weekday(), Weekday::Mon);
        assert_eq!(monday, NaiveDate::from_ymd_opt(2024, 1, 8).unwrap());
    }

    #[test]
    fn test_forecast_renders_without_panic() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let forecast = Forecast::new(&model, &theme);

        let area = Rect::new(0, 0, 120, 30);
        let mut buffer = Buffer::empty(area);
        forecast.render(area, &mut buffer);

        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_daily_capacity_constant() {
        // Ensure daily capacity is 8 hours as documented
        assert_eq!(DAILY_CAPACITY_HOURS, 8);
    }
}
