//! Calendar state types.

use chrono::{Datelike, Duration, NaiveDate, Utc};

/// State for the calendar view.
///
/// Tracks the currently displayed month and selected day.
///
/// # Examples
///
/// ```
/// use taskflow::app::CalendarState;
///
/// // Default is current month with today selected
/// let state = CalendarState::default();
/// assert!(state.selected_day.is_some());
/// ```
#[derive(Debug, Clone)]
pub struct CalendarState {
    /// The year being displayed
    pub year: i32,
    /// The month being displayed (1-12)
    pub month: u32,
    /// The selected day within the month (if any)
    pub selected_day: Option<u32>,
    /// Whether focus is on the task list (true) or calendar grid (false)
    pub focus_task_list: bool,
}

impl Default for CalendarState {
    fn default() -> Self {
        let today = Utc::now().date_naive();
        Self {
            year: today.year(),
            month: today.month(),
            selected_day: Some(today.day()),
            focus_task_list: false,
        }
    }
}

impl CalendarState {
    /// Returns the number of days in the current month.
    #[must_use]
    pub fn days_in_month(&self) -> u32 {
        // Get first day of next month, subtract one day
        let (next_year, next_month) = if self.month == 12 {
            (self.year + 1, 1)
        } else {
            (self.year, self.month + 1)
        };
        NaiveDate::from_ymd_opt(next_year, next_month, 1)
            .and_then(|d| d.checked_sub_signed(Duration::days(1)))
            .map_or(31, |d| d.day())
    }

    /// Validates and clamps the selected day to be within the valid range for the month.
    ///
    /// Returns the clamped day value (1 to days_in_month).
    #[must_use]
    pub fn validated_day(&self) -> Option<u32> {
        self.selected_day
            .map(|day| day.clamp(1, self.days_in_month()))
    }

    /// Returns true if the current state represents a valid calendar date.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        if !(1..=12).contains(&self.month) {
            return false;
        }
        if let Some(day) = self.selected_day {
            if day < 1 || day > self.days_in_month() {
                return false;
            }
        }
        true
    }

    /// Creates a NaiveDate from the current state, if valid.
    #[must_use]
    pub fn to_date(&self) -> Option<NaiveDate> {
        self.selected_day
            .and_then(|day| NaiveDate::from_ymd_opt(self.year, self.month, day))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calendar_state_days_in_month() {
        // Test various months
        let state = CalendarState {
            year: 2024,
            month: 1, // January
            ..Default::default()
        };
        assert_eq!(state.days_in_month(), 31);

        let state = CalendarState {
            year: 2024,
            month: 2, // February (leap year 2024)
            ..Default::default()
        };
        assert_eq!(state.days_in_month(), 29);

        let state = CalendarState {
            year: 2023,
            month: 2, // February (non-leap year)
            ..Default::default()
        };
        assert_eq!(state.days_in_month(), 28);

        let state = CalendarState {
            year: 2023,
            month: 4, // April
            ..Default::default()
        };
        assert_eq!(state.days_in_month(), 30);
    }

    #[test]
    fn test_calendar_state_is_valid() {
        let mut state = CalendarState::default();

        // Valid default state
        assert!(state.is_valid());

        // Invalid month
        state.month = 0;
        assert!(!state.is_valid());

        state.month = 13;
        assert!(!state.is_valid());

        // Invalid day for month
        state.month = 2;
        state.year = 2023;
        state.selected_day = Some(30); // Feb doesn't have 30 days
        assert!(!state.is_valid());

        // Valid day
        state.selected_day = Some(28);
        assert!(state.is_valid());

        // No day selected is valid
        state.selected_day = None;
        assert!(state.is_valid());
    }

    #[test]
    fn test_calendar_state_validated_day() {
        // Day within range
        let state = CalendarState {
            year: 2023,
            month: 2, // February with 28 days
            selected_day: Some(15),
            ..Default::default()
        };
        assert_eq!(state.validated_day(), Some(15));

        // Day too high gets clamped
        let state = CalendarState {
            year: 2023,
            month: 2,
            selected_day: Some(31),
            ..Default::default()
        };
        assert_eq!(state.validated_day(), Some(28));

        // Day too low gets clamped
        let state = CalendarState {
            year: 2023,
            month: 2,
            selected_day: Some(0),
            ..Default::default()
        };
        assert_eq!(state.validated_day(), Some(1));

        // None stays None
        let state = CalendarState {
            year: 2023,
            month: 2,
            selected_day: None,
            ..Default::default()
        };
        assert_eq!(state.validated_day(), None);
    }

    #[test]
    fn test_calendar_state_to_date() {
        let state = CalendarState {
            year: 2024,
            month: 3,
            selected_day: Some(15),
            ..Default::default()
        };

        let date = state.to_date();
        assert!(date.is_some());
        let date = date.unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 3);
        assert_eq!(date.day(), 15);

        // Invalid date returns None
        let state = CalendarState {
            year: 2024,
            month: 3,
            selected_day: Some(32),
            ..Default::default()
        };
        assert!(state.to_date().is_none());

        // No day returns None
        let state = CalendarState {
            year: 2024,
            month: 3,
            selected_day: None,
            ..Default::default()
        };
        assert!(state.to_date().is_none());
    }
}
