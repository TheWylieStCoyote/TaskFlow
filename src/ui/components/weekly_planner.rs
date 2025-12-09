//! Weekly planner view component.
//!
//! Displays tasks organized by day of the week with columns for each day.

use chrono::{Datelike, NaiveDate, Utc};

#[cfg(test)]
use chrono::Weekday;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

use crate::app::Model;
use crate::config::Theme;
use crate::domain::Task;

/// Parameters for rendering a day column in the weekly planner.
struct DayColumnParams<'a> {
    area: Rect,
    date: NaiveDate,
    day_name: &'a str,
    tasks: Vec<&'a Task>,
    is_today: bool,
    is_past: bool,
    is_selected: bool,
}

/// Weekly planner widget showing tasks organized by day.
pub struct WeeklyPlanner<'a> {
    model: &'a Model,
    theme: &'a Theme,
}

impl<'a> WeeklyPlanner<'a> {
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }

    /// Get the start of the current week (Monday).
    fn week_start() -> NaiveDate {
        let today = Utc::now().date_naive();
        today - chrono::Duration::days(today.weekday().num_days_from_monday().into())
    }

    /// Get tasks for a specific date (by due_date or scheduled_date).
    fn tasks_for_date(&self, date: NaiveDate) -> Vec<&Task> {
        self.model
            .visible_tasks
            .iter()
            .filter_map(|id| self.model.tasks.get(id))
            .filter(|t| t.due_date == Some(date) || t.scheduled_date == Some(date))
            .collect()
    }
}

impl Widget for WeeklyPlanner<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let week_start = Self::week_start();
        let today = Utc::now().date_naive();

        // Days of the week
        let days: Vec<(NaiveDate, &str)> = vec![
            (week_start, "Mon"),
            (week_start + chrono::Duration::days(1), "Tue"),
            (week_start + chrono::Duration::days(2), "Wed"),
            (week_start + chrono::Duration::days(3), "Thu"),
            (week_start + chrono::Duration::days(4), "Fri"),
            (week_start + chrono::Duration::days(5), "Sat"),
            (week_start + chrono::Duration::days(6), "Sun"),
        ];

        // Create header with week info
        let week_num = week_start.iso_week().week();
        let header = format!(
            " Week {} • {} - {} ",
            week_num,
            week_start.format("%b %d"),
            (week_start + chrono::Duration::days(6)).format("%b %d, %Y")
        );

        // Split area: header row + day columns
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);

        // Render week header
        let header_line = Line::from(Span::styled(
            header,
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        ));
        buf.set_line(
            main_chunks[0].x,
            main_chunks[0].y,
            &header_line,
            main_chunks[0].width,
        );

        // Split into 7 columns for days
        let day_columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                days.iter()
                    .map(|_| Constraint::Ratio(1, 7))
                    .collect::<Vec<_>>(),
            )
            .split(main_chunks[1]);

        // Render each day column
        let selected_day = self.model.view_selection.weekly_planner_day;
        for (i, (date, day_name)) in days.iter().enumerate() {
            let params = DayColumnParams {
                area: day_columns[i],
                date: *date,
                day_name,
                tasks: self.tasks_for_date(*date),
                is_today: *date == today,
                is_past: *date < today,
                is_selected: i == selected_day,
            };
            self.render_day_column(buf, params);
        }
    }
}

impl WeeklyPlanner<'_> {
    fn render_day_column(&self, buf: &mut Buffer, params: DayColumnParams<'_>) {
        let DayColumnParams {
            area,
            date,
            day_name,
            tasks,
            is_today,
            is_past,
            is_selected,
        } = params;
        let theme = self.theme;

        // Determine title color based on day status
        let title_color = if is_today {
            theme.colors.accent.to_color()
        } else if is_past {
            theme.colors.muted.to_color()
        } else {
            theme.colors.foreground.to_color()
        };

        // Format title with day name and date
        let title = format!(" {} {} ({}) ", day_name, date.day(), tasks.len());

        // Selection takes precedence over today highlight for border
        let border_color = if is_selected {
            theme.colors.accent.to_color()
        } else if is_today {
            theme.colors.success.to_color()
        } else {
            theme.colors.border.to_color()
        };

        let block = Block::default()
            .title(title)
            .title_style(Style::default().fg(title_color).add_modifier(if is_today {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        block.render(area, buf);

        if tasks.is_empty() {
            // Show empty message
            let msg = if is_past { "—" } else { "No tasks" };
            let empty_msg =
                Paragraph::new(msg).style(Style::default().fg(theme.colors.muted.to_color()));
            empty_msg.render(inner, buf);
            return;
        }

        // Create list items for tasks
        let items: Vec<ListItem<'_>> = tasks
            .iter()
            .take(inner.height as usize)
            .map(|task| {
                // Status indicator
                let status_style = if task.status.is_complete() {
                    Style::default().fg(theme.colors.success.to_color())
                } else if is_past && !task.status.is_complete() {
                    Style::default().fg(theme.colors.danger.to_color())
                } else {
                    Style::default().fg(theme.colors.foreground.to_color())
                };

                let checkbox = if task.status.is_complete() {
                    "✓ "
                } else {
                    "◇ "
                };

                // Priority indicator (compact)
                let priority = match task.priority {
                    crate::domain::Priority::Urgent => "!",
                    crate::domain::Priority::High => "!",
                    _ => "",
                };

                let priority_style = match task.priority {
                    crate::domain::Priority::Urgent => {
                        Style::default().fg(theme.colors.danger.to_color())
                    }
                    crate::domain::Priority::High => {
                        Style::default().fg(theme.colors.warning.to_color())
                    }
                    _ => Style::default(),
                };

                // Truncate title to fit
                let max_len = inner.width.saturating_sub(4) as usize;
                let title = if task.title.len() > max_len {
                    format!("{}…", &task.title[..max_len.saturating_sub(1)])
                } else {
                    task.title.clone()
                };

                // Indicator for scheduled vs due
                let type_indicator =
                    if task.scheduled_date == Some(date) && task.due_date != Some(date) {
                        "○" // Scheduled only
                    } else if task.due_date == Some(date) {
                        "●" // Due date
                    } else {
                        ""
                    };

                let mut spans = vec![
                    Span::styled(checkbox, status_style),
                    Span::styled(priority, priority_style),
                ];

                if !type_indicator.is_empty() {
                    spans.push(Span::styled(
                        format!("{} ", type_indicator),
                        Style::default().fg(theme.colors.muted.to_color()),
                    ));
                }

                spans.push(Span::styled(title, status_style));

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items);
        list.render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weekly_planner_renders_without_panic() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let planner = WeeklyPlanner::new(&model, &theme);

        let area = Rect::new(0, 0, 140, 30);
        let mut buffer = Buffer::empty(area);
        planner.render(area, &mut buffer);

        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_week_start_is_monday() {
        let week_start = WeeklyPlanner::week_start();
        assert_eq!(week_start.weekday(), Weekday::Mon);
    }
}
