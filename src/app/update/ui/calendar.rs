//! Calendar navigation handlers

use crate::app::{Model, UiMessage, ViewId};

use super::super::navigation::days_in_month;

/// Handle calendar navigation messages
pub fn handle_ui_calendar(model: &mut Model, msg: UiMessage) {
    if model.current_view != ViewId::Calendar {
        return;
    }

    match msg {
        UiMessage::ScheduleTaskToDay => schedule_selected_task_to_day(model),
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

/// Schedule the selected task on the focused calendar day.
///
/// Sets `scheduled_date` (keeping any time block) with an undo entry, then
/// refreshes the day panel so the task moves into its time-blocked section
/// and stays selected. Does nothing when no day or task is selected, or when
/// the task is already scheduled on that day.
fn schedule_selected_task_to_day(model: &mut Model) {
    let Some(date) = model.calendar_state.to_date() else {
        return;
    };
    let Some(task_id) = model.selected_task_id() else {
        return;
    };
    if model
        .tasks
        .get(&task_id)
        .is_none_or(|t| t.scheduled_date == Some(date))
    {
        return;
    }

    model.modify_task_with_undo(&task_id, |task| {
        task.scheduled_date = Some(date);
    });
    model.alerts.status_message = Some(format!("Scheduled to {}", date.format("%a %-d %b")));
    model.refresh_visible_tasks();
    if let Some(index) = model.visible_tasks.iter().position(|id| *id == task_id) {
        model.selected_index = index;
    }
}
