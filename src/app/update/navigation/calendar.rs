//! Calendar view navigation handlers.

use crate::app::{Model, NavigationMessage, ViewId};

/// Handle calendar-specific navigation messages.
pub fn handle_calendar_navigation(model: &mut Model, msg: NavigationMessage) {
    match msg {
        NavigationMessage::CalendarPrevMonth => {
            if model.calendar_state.month == 1 {
                model.calendar_state.month = 12;
                model.calendar_state.year -= 1;
            } else {
                model.calendar_state.month -= 1;
            }
            // Adjust selected day if it exceeds days in new month
            let days = days_in_month(model.calendar_state.year, model.calendar_state.month);
            if let Some(day) = model.calendar_state.selected_day {
                if day > days {
                    model.calendar_state.selected_day = Some(days);
                }
            }
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        NavigationMessage::CalendarNextMonth => {
            if model.calendar_state.month == 12 {
                model.calendar_state.month = 1;
                model.calendar_state.year += 1;
            } else {
                model.calendar_state.month += 1;
            }
            // Adjust selected day if it exceeds days in new month
            let days = days_in_month(model.calendar_state.year, model.calendar_state.month);
            if let Some(day) = model.calendar_state.selected_day {
                if day > days {
                    model.calendar_state.selected_day = Some(days);
                }
            }
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        NavigationMessage::CalendarSelectDay(day) => {
            model.calendar_state.selected_day = Some(day);
            model.calendar_state.focus_task_list = false; // Reset focus to grid
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        NavigationMessage::CalendarFocusTaskList => {
            if model.current_view == ViewId::Calendar && !model.tasks_for_selected_day().is_empty()
            {
                model.calendar_state.focus_task_list = true;
                model.selected_index = 0;
            }
        }
        NavigationMessage::CalendarFocusGrid => {
            model.calendar_state.focus_task_list = false;
        }
        _ => {}
    }
}

/// Handle calendar up navigation (move to previous week).
pub fn handle_calendar_up(model: &mut Model) {
    if let Some(day) = model.calendar_state.selected_day {
        if day > 7 {
            model.calendar_state.selected_day = Some(day - 7);
        } else {
            // Move to previous month, last row
            if model.calendar_state.month == 1 {
                model.calendar_state.month = 12;
                model.calendar_state.year -= 1;
            } else {
                model.calendar_state.month -= 1;
            }
            let days = days_in_month(model.calendar_state.year, model.calendar_state.month);
            // Try to land on same weekday in last week
            let new_day = days - (7 - day);
            model.calendar_state.selected_day = Some(new_day.max(1));
        }
        model.calendar_state.focus_task_list = false; // Reset focus to grid
        model.selected_index = 0;
        model.refresh_visible_tasks();
    }
}

/// Handle calendar down navigation (move to next week).
pub fn handle_calendar_down(model: &mut Model) {
    if let Some(day) = model.calendar_state.selected_day {
        let days = days_in_month(model.calendar_state.year, model.calendar_state.month);
        if day + 7 <= days {
            model.calendar_state.selected_day = Some(day + 7);
        } else {
            // Move to next month, first row
            if model.calendar_state.month == 12 {
                model.calendar_state.month = 1;
                model.calendar_state.year += 1;
            } else {
                model.calendar_state.month += 1;
            }
            // Try to land on same weekday in first week
            let new_day = (day + 7) - days;
            model.calendar_state.selected_day = Some(new_day.min(7));
        }
        model.calendar_state.focus_task_list = false; // Reset focus to grid
        model.selected_index = 0;
        model.refresh_visible_tasks();
    }
}

/// Helper to get days in a month.
#[must_use]
pub fn days_in_month(year: i32, month: u32) -> u32 {
    use chrono::{Datelike, NaiveDate};
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .and_then(|d| d.pred_opt())
    .map_or(28, |d| d.day())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_days_in_month() {
        // January has 31 days
        assert_eq!(days_in_month(2024, 1), 31);
        // February 2024 (leap year) has 29 days
        assert_eq!(days_in_month(2024, 2), 29);
        // February 2023 (non-leap year) has 28 days
        assert_eq!(days_in_month(2023, 2), 28);
        // April has 30 days
        assert_eq!(days_in_month(2024, 4), 30);
        // December has 31 days
        assert_eq!(days_in_month(2024, 12), 31);
    }
}
