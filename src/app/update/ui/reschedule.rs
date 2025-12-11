//! Task rescheduling and snooze handlers.
//!
//! Handles quick rescheduling operations:
//! - Snooze task
//! - Reschedule to tomorrow
//! - Reschedule to next week
//! - Reschedule to next Monday

use chrono::{Datelike, Duration};

use crate::app::{Model, UiMessage};

use super::input::start_input;
use crate::ui::InputTarget;

/// Handle task rescheduling and snooze messages.
pub fn handle_ui_reschedule(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::StartSnoozeTask => {
            if let Some(task_id) = model.selected_task_id() {
                start_input(model, InputTarget::SnoozeTask(task_id), None);
            }
        }
        UiMessage::ClearSnooze => {
            if let Some(task_id) = model.selected_task_id() {
                if let Some(task) = model.tasks.get_mut(&task_id) {
                    task.clear_snooze();
                }
                model.sync_task_by_id(&task_id);
                model.alerts.status_message = Some("Snooze cleared".to_string());
                model.refresh_visible_tasks();
            }
        }
        UiMessage::RescheduleTomorrow => {
            if let Some(task_id) = model.selected_task_id() {
                let tomorrow = chrono::Local::now().date_naive() + Duration::days(1);
                model.modify_task_with_undo(&task_id, |task| {
                    task.due_date = Some(tomorrow);
                });
                model.alerts.status_message =
                    Some(format!("Rescheduled to {}", tomorrow.format("%b %d")));
                model.refresh_visible_tasks();
            }
        }
        UiMessage::RescheduleNextWeek => {
            if let Some(task_id) = model.selected_task_id() {
                let next_week = chrono::Local::now().date_naive() + Duration::days(7);
                model.modify_task_with_undo(&task_id, |task| {
                    task.due_date = Some(next_week);
                });
                model.alerts.status_message =
                    Some(format!("Rescheduled to {}", next_week.format("%b %d")));
                model.refresh_visible_tasks();
            }
        }
        UiMessage::RescheduleNextMonday => {
            if let Some(task_id) = model.selected_task_id() {
                let today = chrono::Local::now().date_naive();
                // num_days_from_monday: Mon=0, Tue=1, ..., Sun=6
                // To get next Monday: (7 - current_weekday) % 7, but if 0 use 7
                let days_from_monday = today.weekday().num_days_from_monday();
                let days_until_monday = (7 - days_from_monday) % 7;
                let days_until_monday = if days_until_monday == 0 {
                    7
                } else {
                    days_until_monday
                };
                let next_monday = today + Duration::days(days_until_monday.into());
                model.modify_task_with_undo(&task_id, |task| {
                    task.due_date = Some(next_monday);
                });
                model.alerts.status_message =
                    Some(format!("Rescheduled to {}", next_monday.format("%b %d")));
                model.refresh_visible_tasks();
            }
        }
        _ => {}
    }
}
