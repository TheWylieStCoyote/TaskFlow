//! Forecast view component - workload projection into future weeks.
//!
//! Displays upcoming workload based on due dates and estimated times,
//! helping users plan capacity and identify overloaded periods.

mod render;

#[cfg(test)]
mod tests;

use chrono::{Datelike, Duration, Local, NaiveDate};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Widget},
};

use crate::app::Model;
use crate::config::Theme;

/// Number of weeks to forecast
pub(crate) const FORECAST_WEEKS: usize = 8;

/// Standard work day capacity in hours
pub(crate) const DAILY_CAPACITY_HOURS: u32 = 8;

/// Forecast view widget showing workload projection
pub struct Forecast<'a> {
    pub(crate) model: &'a Model,
    pub(crate) theme: &'a Theme,
}

impl<'a> Forecast<'a> {
    /// Create a new forecast widget
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }

    /// Get the start of the week (Monday) for a given date
    pub(crate) fn week_start(date: NaiveDate) -> NaiveDate {
        let days_from_monday = date.weekday().num_days_from_monday();
        date - Duration::days(i64::from(days_from_monday))
    }

    /// Calculate workload per week (task count and estimated minutes)
    pub(crate) fn get_weekly_workload(&self) -> Vec<(NaiveDate, usize, u32)> {
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
    pub(crate) fn get_daily_workload(&self) -> Vec<(NaiveDate, usize, u32)> {
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
