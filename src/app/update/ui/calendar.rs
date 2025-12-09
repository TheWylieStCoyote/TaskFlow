//! Calendar navigation handlers

use crate::app::{Model, UiMessage, ViewId};

use super::super::navigation::days_in_month;

/// Handle calendar navigation messages
pub fn handle_ui_calendar(model: &mut Model, msg: UiMessage) {
    if model.current_view != ViewId::Calendar {
        return;
    }

    match msg {
        UiMessage::CalendarPrevDay => {
            if let Some(day) = model.calendar_state.selected_day {
                if day > 1 {
                    model.calendar_state.selected_day = Some(day - 1);
                } else {
                    // Go to previous month's last day
                    if model.calendar_state.month == 1 {
                        model.calendar_state.month = 12;
                        model.calendar_state.year -= 1;
                    } else {
                        model.calendar_state.month -= 1;
                    }
                    let days = days_in_month(model.calendar_state.year, model.calendar_state.month);
                    model.calendar_state.selected_day = Some(days);
                }
                model.selected_index = 0;
                model.refresh_visible_tasks();
            }
        }
        UiMessage::CalendarNextDay => {
            if let Some(day) = model.calendar_state.selected_day {
                let days = days_in_month(model.calendar_state.year, model.calendar_state.month);
                if day < days {
                    model.calendar_state.selected_day = Some(day + 1);
                } else {
                    // Go to next month's first day
                    if model.calendar_state.month == 12 {
                        model.calendar_state.month = 1;
                        model.calendar_state.year += 1;
                    } else {
                        model.calendar_state.month += 1;
                    }
                    model.calendar_state.selected_day = Some(1);
                }
                model.selected_index = 0;
                model.refresh_visible_tasks();
            }
        }
        _ => {}
    }
}
