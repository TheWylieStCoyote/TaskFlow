//! Weekly planner view component.
//!
//! Displays tasks organized by day of the week with columns for each day.

mod render;

#[cfg(test)]
mod tests;

use chrono::{Datelike, NaiveDate, Utc};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::app::Model;
use crate::config::Theme;
use crate::domain::Task;

/// Parameters for rendering a day column in the weekly planner.
pub(crate) struct DayColumnParams<'a> {
    pub(crate) area: Rect,
    pub(crate) date: NaiveDate,
    pub(crate) day_name: &'a str,
    pub(crate) tasks: Vec<&'a Task>,
    pub(crate) is_today: bool,
    pub(crate) is_past: bool,
    pub(crate) is_selected: bool,
    pub(crate) selected_task_index: Option<usize>,
}

/// Weekly planner widget showing tasks organized by day.
pub struct WeeklyPlanner<'a> {
    pub(crate) model: &'a Model,
    pub(crate) theme: &'a Theme,
}

impl<'a> WeeklyPlanner<'a> {
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }

    /// Get the start of the current week (Monday).
    pub(crate) fn week_start() -> NaiveDate {
        let today = Utc::now().date_naive();
        today - chrono::Duration::days(today.weekday().num_days_from_monday().into())
    }

    /// Get tasks for a specific date (by due_date or scheduled_date).
    pub(crate) fn tasks_for_date(&self, date: NaiveDate) -> Vec<&Task> {
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
        let selected_task_index = self.model.view_selection.weekly_planner_task_index;
        for (i, (date, day_name)) in days.iter().enumerate() {
            let is_selected = i == selected_day;
            let params = DayColumnParams {
                area: day_columns[i],
                date: *date,
                day_name,
                tasks: self.tasks_for_date(*date),
                is_today: *date == today,
                is_past: *date < today,
                is_selected,
                selected_task_index: if is_selected {
                    Some(selected_task_index)
                } else {
                    None
                },
            };
            self.render_day_column(buf, params);
        }
    }
}
