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
use crate::domain::{today, Task};
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
                model.modify_task_with_undo(&task_id, Task::clear_snooze);
                model.alerts.status_message = Some("Snooze cleared".to_string());
                model.refresh_visible_tasks();
            }
        }
        UiMessage::RescheduleTomorrow => {
            if let Some(task_id) = model.selected_task_id() {
                let tomorrow = today() + Duration::days(1);
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
                let next_week = today() + Duration::days(7);
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
                let today = today();
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

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use crate::app::{update::update, Message, Model, SystemMessage, UiMessage};
    use crate::domain::{today, Task, TaskId};
    use crate::ui::{InputMode, InputTarget};

    fn model_with_tasks(n: usize) -> (Model, Vec<TaskId>) {
        let mut model = Model::new();
        let ids: Vec<TaskId> = (0..n)
            .map(|i| {
                let task = Task::new(format!("Task {i}"));
                let id = task.id;
                model.tasks.insert(id, task);
                id
            })
            .collect();
        model.refresh_visible_tasks();
        model.selected_index = 0;
        (model, ids)
    }

    fn submit(model: &mut Model, target: InputTarget, text: &str) {
        model.input.mode = InputMode::Editing;
        model.input.target = target;
        model.input.buffer = text.to_string();
        model.input.cursor = text.len();
        update(model, Message::Ui(UiMessage::SubmitInput));
    }

    fn undo(model: &mut Model) {
        update(model, Message::System(SystemMessage::Undo));
    }

    fn redo(model: &mut Model) {
        update(model, Message::System(SystemMessage::Redo));
    }

    fn snooze_of(model: &Model, id: TaskId) -> Option<chrono::NaiveDate> {
        model.tasks[&id].snooze_until
    }

    #[test]
    fn clear_snooze_is_undoable_and_redoable() {
        let (mut model, ids) = model_with_tasks(1);
        let date = today() + Duration::days(3);
        model
            .tasks
            .get_mut(&ids[0])
            .unwrap()
            .snooze_until_date(date);
        // Snoozed tasks are hidden from the list; point the selection at it directly.
        model.visible_tasks = vec![ids[0]];
        model.selected_index = 0;
        assert!(model.undo_stack.is_empty());

        update(&mut model, Message::Ui(UiMessage::ClearSnooze));
        assert_eq!(snooze_of(&model, ids[0]), None);
        assert_eq!(model.undo_stack.len(), 1, "clearing snooze is recorded");

        undo(&mut model);
        assert_eq!(snooze_of(&model, ids[0]), Some(date));

        redo(&mut model);
        assert_eq!(snooze_of(&model, ids[0]), None);
    }

    #[test]
    fn snooze_input_set_and_clear_are_undoable() {
        let (mut model, ids) = model_with_tasks(1);
        let date = today() + Duration::days(5);

        submit(
            &mut model,
            InputTarget::SnoozeTask(ids[0]),
            &date.format("%Y-%m-%d").to_string(),
        );
        assert_eq!(snooze_of(&model, ids[0]), Some(date));
        assert_eq!(model.undo_stack.len(), 1);

        undo(&mut model);
        assert_eq!(snooze_of(&model, ids[0]), None);
        redo(&mut model);
        assert_eq!(snooze_of(&model, ids[0]), Some(date));

        // Empty input clears the snooze; that is undoable too.
        submit(&mut model, InputTarget::SnoozeTask(ids[0]), "");
        assert_eq!(snooze_of(&model, ids[0]), None);
        undo(&mut model);
        assert_eq!(snooze_of(&model, ids[0]), Some(date));
    }

    #[test]
    fn snooze_input_with_invalid_date_records_nothing() {
        let (mut model, ids) = model_with_tasks(1);
        submit(&mut model, InputTarget::SnoozeTask(ids[0]), "not a date");
        assert_eq!(snooze_of(&model, ids[0]), None);
        assert!(model.undo_stack.is_empty());
    }

    #[test]
    fn bulk_snooze_set_and_clear_are_undoable_per_task() {
        let (mut model, ids) = model_with_tasks(3);
        let date = today() + Duration::days(2);
        model.multi_select.mode = true;
        model.multi_select.selected.insert(ids[0]);
        model.multi_select.selected.insert(ids[1]);

        submit(
            &mut model,
            InputTarget::BulkSnooze,
            &date.format("%Y-%m-%d").to_string(),
        );
        assert_eq!(snooze_of(&model, ids[0]), Some(date));
        assert_eq!(snooze_of(&model, ids[1]), Some(date));
        assert_eq!(snooze_of(&model, ids[2]), None, "unselected task untouched");
        assert_eq!(model.undo_stack.len(), 2, "one undo entry per task");

        undo(&mut model);
        undo(&mut model);
        assert_eq!(snooze_of(&model, ids[0]), None);
        assert_eq!(snooze_of(&model, ids[1]), None);

        redo(&mut model);
        redo(&mut model);
        assert_eq!(snooze_of(&model, ids[0]), Some(date));
        assert_eq!(snooze_of(&model, ids[1]), Some(date));

        // Bulk clear (empty input) is undoable as well.
        model.multi_select.mode = true;
        model.multi_select.selected.insert(ids[0]);
        model.multi_select.selected.insert(ids[1]);
        submit(&mut model, InputTarget::BulkSnooze, "");
        assert_eq!(snooze_of(&model, ids[0]), None);
        assert_eq!(snooze_of(&model, ids[1]), None);
        undo(&mut model);
        undo(&mut model);
        assert_eq!(snooze_of(&model, ids[0]), Some(date));
        assert_eq!(snooze_of(&model, ids[1]), Some(date));
    }
}
