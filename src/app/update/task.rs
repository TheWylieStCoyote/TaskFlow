//! Task message handlers
//!
//! Handles all task-related messages including:
//! - Creating, deleting, and modifying tasks
//! - Toggling completion (with recurring task support)
//! - Priority cycling
//! - Moving tasks between projects

use crate::app::{Model, TaskMessage, UndoAction};
use crate::domain::{Priority, Recurrence, Task, TaskStatus};
use chrono::{Datelike, Duration, NaiveDate, Utc};

/// Handle task messages
#[allow(clippy::too_many_lines)]
pub fn handle_task(model: &mut Model, msg: TaskMessage) {
    match msg {
        TaskMessage::ToggleComplete => {
            // Get the task id first to avoid borrow issues
            let task_id = model.visible_tasks.get(model.selected_index).cloned();

            if let Some(id) = task_id {
                // Check if completing a recurring task
                let next_task = model.tasks.get(&id).and_then(|task| {
                    if task.status != TaskStatus::Done && task.recurrence.is_some() {
                        // Create next occurrence
                        Some(create_next_recurring_task(task))
                    } else {
                        None
                    }
                });

                // Check for task chain - if completing and has next_task_id, schedule it
                let chain_next_id = model.tasks.get(&id).and_then(|task| {
                    if task.status != TaskStatus::Done {
                        task.next_task_id.clone()
                    } else {
                        None
                    }
                });

                // Check if we're completing (not uncompleting) to cascade to descendants
                let is_completing = model
                    .tasks
                    .get(&id)
                    .is_some_and(|t| t.status != TaskStatus::Done);

                // Get all descendants BEFORE modifying the task
                let descendants = if is_completing {
                    model.get_all_descendants(&id)
                } else {
                    Vec::new()
                };

                if let Some(task) = model.tasks.get_mut(&id) {
                    let before = task.clone();
                    task.toggle_complete();
                    let after = task.clone();
                    model.sync_task(&after);
                    model.undo_stack.push(UndoAction::TaskModified {
                        before: Box::new(before),
                        after: Box::new(after),
                    });
                }

                // Auto-complete all descendants when completing a parent task
                for descendant_id in descendants {
                    // Only complete if not already complete
                    let needs_complete = model
                        .tasks
                        .get(&descendant_id)
                        .is_some_and(|t| !t.status.is_complete());
                    if needs_complete {
                        model.modify_task_with_undo(&descendant_id, |task| {
                            task.status = TaskStatus::Done;
                        });
                    }
                }

                // Add the next recurring task if one was created
                if let Some(new_task) = next_task {
                    model.sync_task(&new_task);
                    model
                        .undo_stack
                        .push(UndoAction::TaskCreated(Box::new(new_task.clone())));
                    model.tasks.insert(new_task.id.clone(), new_task);
                }

                // Auto-schedule the next task in chain for today
                if let Some(next_id) = chain_next_id {
                    model.modify_task_with_undo(&next_id, |task| {
                        task.scheduled_date = Some(chrono::Local::now().date_naive());
                    });
                }
            }
            model.refresh_visible_tasks();
        }
        TaskMessage::SetStatus(task_id, status) => {
            model.modify_task_with_undo(&task_id, |task| {
                task.status = status;
            });
            model.refresh_visible_tasks();
        }
        TaskMessage::SetPriority(task_id, priority) => {
            model.modify_task_with_undo(&task_id, |task| {
                task.priority = priority;
            });
            model.refresh_visible_tasks();
        }
        TaskMessage::CyclePriority => {
            if let Some(id) = model.visible_tasks.get(model.selected_index).cloned() {
                model.modify_task_with_undo(&id, |task| {
                    task.priority = match task.priority {
                        Priority::None => Priority::Low,
                        Priority::Low => Priority::Medium,
                        Priority::Medium => Priority::High,
                        Priority::High => Priority::Urgent,
                        Priority::Urgent => Priority::None,
                    };
                });
            }
            model.refresh_visible_tasks();
        }
        TaskMessage::Create(title) => {
            let task = Task::new(title).with_priority(model.default_priority);
            model.sync_task(&task);
            model
                .undo_stack
                .push(UndoAction::TaskCreated(Box::new(task.clone())));
            model.tasks.insert(task.id.clone(), task);
            model.refresh_visible_tasks();
        }
        TaskMessage::Delete(task_id) => {
            if let Some(task) = model.tasks.remove(&task_id) {
                // Collect time entries for this task before deleting
                let task_entries: Vec<_> = model
                    .time_entries
                    .values()
                    .filter(|e| e.task_id == task_id)
                    .cloned()
                    .collect();

                // Clear active time entry if it belongs to this task
                if model
                    .active_time_entry
                    .as_ref()
                    .and_then(|id| model.time_entries.get(id))
                    .is_some_and(|e| e.task_id == task_id)
                {
                    model.active_time_entry = None;
                }

                // Delete time entries (collect IDs first to avoid borrow issues)
                let entry_ids: Vec<_> = task_entries.iter().map(|e| e.id.clone()).collect();
                for entry_id in entry_ids {
                    model.delete_time_entry(&entry_id);
                }

                model.delete_task_from_storage(&task_id);
                model.undo_stack.push(UndoAction::TaskDeleted {
                    task: Box::new(task),
                    time_entries: task_entries,
                });
            }
            model.refresh_visible_tasks();
        }
        TaskMessage::MoveToProject(task_id, project_id) => {
            model.modify_task_with_undo(&task_id, |task| {
                task.project_id = project_id;
            });
            model.refresh_visible_tasks();
        }
    }
}

/// Create the next occurrence of a recurring task
pub fn create_next_recurring_task(task: &Task) -> Task {
    let today = Utc::now().date_naive();
    let base_date = task.due_date.unwrap_or(today);

    let next_due = match &task.recurrence {
        Some(Recurrence::Daily) => base_date + Duration::days(1),
        Some(Recurrence::Weekly { days }) => {
            if days.is_empty() {
                // Default: same day next week
                base_date + Duration::weeks(1)
            } else {
                // Find next day in the list after today
                let current_weekday = today.weekday();
                let mut min_days = 7i64;
                for day in days {
                    let days_until = (i64::from(day.num_days_from_monday())
                        - i64::from(current_weekday.num_days_from_monday())
                        + 7)
                        % 7;
                    let days_until = if days_until == 0 { 7 } else { days_until };
                    if days_until < min_days {
                        min_days = days_until;
                    }
                }
                today + Duration::days(min_days)
            }
        }
        Some(Recurrence::Monthly { day }) => {
            let next_month = if base_date.month() == 12 {
                NaiveDate::from_ymd_opt(base_date.year() + 1, 1, *day)
            } else {
                NaiveDate::from_ymd_opt(base_date.year(), base_date.month() + 1, *day)
            };
            // Handle invalid dates (e.g., Feb 30) by using last day of month
            next_month.unwrap_or_else(|| {
                let year = if base_date.month() == 12 {
                    base_date.year() + 1
                } else {
                    base_date.year()
                };
                let month = if base_date.month() == 12 {
                    1
                } else {
                    base_date.month() + 1
                };
                // Get last day of the target month (first of next month minus 1 day)
                NaiveDate::from_ymd_opt(
                    if month == 12 { year + 1 } else { year },
                    if month == 12 { 1 } else { month + 1 },
                    1,
                )
                .expect("day 1 of any month always exists")
                    - Duration::days(1)
            })
        }
        Some(Recurrence::Yearly { month, day }) => {
            let next_year = base_date.year() + 1;
            // Try exact date, fall back to 28th if invalid (e.g., Feb 30)
            NaiveDate::from_ymd_opt(next_year, *month, *day).unwrap_or_else(|| {
                NaiveDate::from_ymd_opt(next_year, *month, 28)
                    .expect("day 28 always exists in any month")
            })
        }
        None => today + Duration::days(1), // Shouldn't happen
    };

    Task::new(&task.title)
        .with_priority(task.priority)
        .with_due_date(next_due)
        .with_tags(task.tags.clone())
        .with_recurrence(task.recurrence.clone())
        .with_project_opt(task.project_id.clone())
        .with_description_opt(task.description.clone())
}
