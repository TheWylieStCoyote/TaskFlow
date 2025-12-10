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

/// Handles task-related messages.
///
/// This is the main entry point for all task operations. Each message type
/// is processed with proper undo support and storage synchronization.
#[allow(clippy::too_many_lines)]
pub fn handle_task(model: &mut Model, msg: TaskMessage) {
    match msg {
        // Toggle completion is complex because it handles three special cases:
        // 1. Cascade completion: completing a parent auto-completes all descendants
        // 2. Recurring tasks: completing creates the next occurrence automatically
        // 3. Task chains: completing auto-schedules the next linked task for today
        TaskMessage::ToggleComplete => {
            let task_id = model.visible_tasks.get(model.selected_index).copied();

            if let Some(id) = task_id {
                // Phase 1: Gather data BEFORE any mutations (to avoid borrow conflicts)

                // For recurring tasks: prepare the next occurrence if completing
                let next_task = model.tasks.get(&id).and_then(|task| {
                    if task.status != TaskStatus::Done && task.recurrence.is_some() {
                        Some(create_next_recurring_task(task))
                    } else {
                        None
                    }
                });

                // For task chains: find the next task to schedule if completing
                let chain_next_id = model.tasks.get(&id).and_then(|task| {
                    if task.status == TaskStatus::Done {
                        None
                    } else {
                        task.next_task_id
                    }
                });

                // For cascade completion: check direction and gather descendants
                let is_completing = model
                    .tasks
                    .get(&id)
                    .is_some_and(|t| t.status != TaskStatus::Done);
                let descendants = if is_completing {
                    model.get_all_descendants(&id)
                } else {
                    Vec::new()
                };

                // Phase 2: Toggle the task itself
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

                // Phase 3: Cascade - auto-complete all descendants
                // Each gets its own undo action so undo reverses the entire cascade
                for descendant_id in descendants {
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

                // Phase 4: Handle recurring task - add next occurrence
                if let Some(new_task) = next_task {
                    model.sync_task(&new_task);
                    model
                        .undo_stack
                        .push(UndoAction::TaskCreated(Box::new(new_task.clone())));
                    model.tasks.insert(new_task.id, new_task);
                }

                // Phase 5: Handle task chain - schedule next task for today
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
            if let Some(id) = model.visible_tasks.get(model.selected_index).copied() {
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
            model.tasks.insert(task.id, task);
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
                let entry_ids: Vec<_> = task_entries.iter().map(|e| e.id).collect();
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
        TaskMessage::Duplicate => {
            if let Some(id) = model.visible_tasks.get(model.selected_index).copied() {
                if let Some(original) = model.tasks.get(&id) {
                    // Create duplicate with "Copy of" prefix
                    let mut new_task = Task::new(format!("Copy of {}", original.title))
                        .with_priority(original.priority)
                        .with_tags(original.tags.clone())
                        .with_project_opt(original.project_id)
                        .with_description_opt(original.description.clone())
                        .with_recurrence(original.recurrence.clone());

                    // Copy optional fields
                    new_task.due_date = original.due_date;
                    new_task.scheduled_date = original.scheduled_date;
                    new_task.estimated_minutes = original.estimated_minutes;

                    model.sync_task(&new_task);
                    model
                        .undo_stack
                        .push(UndoAction::TaskCreated(Box::new(new_task.clone())));
                    model.tasks.insert(new_task.id, new_task);
                    model.refresh_visible_tasks();
                }
            }
        }
    }
}

/// Create the next occurrence of a recurring task
#[must_use]
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
            // Try exact date, fall back to last day of month if invalid (e.g., Feb 29 in non-leap year)
            NaiveDate::from_ymd_opt(next_year, *month, *day).unwrap_or_else(|| {
                // Get last day of the target month (first of next month minus 1 day)
                NaiveDate::from_ymd_opt(
                    if *month == 12 {
                        next_year + 1
                    } else {
                        next_year
                    },
                    if *month == 12 { 1 } else { month + 1 },
                    1,
                )
                .expect("day 1 of any month always exists")
                    - Duration::days(1)
            })
        }
        None => today + Duration::days(1), // Shouldn't happen
    };

    Task::new(&task.title)
        .with_priority(task.priority)
        .with_due_date(next_due)
        .with_tags(task.tags.clone())
        .with_recurrence(task.recurrence.clone())
        .with_project_opt(task.project_id)
        .with_description_opt(task.description.clone())
}
