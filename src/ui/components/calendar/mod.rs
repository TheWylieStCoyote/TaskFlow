//! Calendar view component.
//!
//! Displays a monthly calendar grid with task indicators. Users can navigate
//! between months, select days, and view tasks due on each date.
//!
//! # Features
//!
//! - Month/year navigation with arrow keys
//! - Visual indicators for days with tasks
//! - Highlighting for today, selected day, and overdue tasks
//! - Task list panel showing tasks for the selected day

mod grid;
mod tasks;

#[cfg(test)]
mod tests;

use chrono::{Datelike, NaiveDate, Utc};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Widget,
};

use crate::app::Model;
use crate::config::Theme;

/// Calendar view widget showing a month grid with tasks
pub struct Calendar<'a> {
    pub(crate) model: &'a Model,
    pub(crate) theme: &'a Theme,
}

impl<'a> Calendar<'a> {
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }

    /// Get the first day of the month
    pub(crate) fn first_day_of_month(&self) -> NaiveDate {
        NaiveDate::from_ymd_opt(
            self.model.calendar_state.year,
            self.model.calendar_state.month,
            1,
        )
        .unwrap_or_else(|| Utc::now().date_naive())
    }

    /// Get the number of days in the current month
    pub(crate) fn days_in_month(&self) -> u32 {
        let year = self.model.calendar_state.year;
        let month = self.model.calendar_state.month;

        // Get the first day of next month, then subtract one day
        if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .and_then(|d| d.pred_opt())
        .map_or(28, |d| d.day())
    }

    /// Get the weekday of the first day (0=Mon, 6=Sun)
    pub(crate) fn first_weekday(&self) -> u32 {
        self.first_day_of_month().weekday().num_days_from_monday()
    }

    /// Get month name
    pub(crate) const fn month_name(&self) -> &'static str {
        match self.model.calendar_state.month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "Unknown",
        }
    }
}

impl Widget for Calendar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let today = Utc::now().date_naive();

        // Split into calendar grid and task list
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(24), // Calendar grid (at least 24 chars: 7 days * 3 chars + borders)
                Constraint::Length(30), // Task list panel
            ])
            .split(area);

        // Render calendar grid
        self.render_calendar_grid(chunks[0], buf, today, theme);

        // Render task list for selected day
        self.render_task_list(chunks[1], buf, theme);
    }
}
