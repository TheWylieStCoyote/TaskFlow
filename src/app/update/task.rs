//! Task message handlers
//!
//! Handles all task-related messages including:
//! - Creating, deleting, and modifying tasks
//! - Toggling completion (with recurring task support)
//! - Priority cycling
//! - Moving tasks between projects

use crate::app::{Model, TaskMessage, UndoAction};
use crate::domain::{Priority, Recurrence, Task, TaskId, TaskStatus};
use chrono::{Datelike, Duration, NaiveDate, Utc};

// ============================================================================
// Toggle Completion Data Structures
// ============================================================================

/// Data gathered before mutating tasks during toggle completion.
///
/// This struct holds all the information needed to handle the complex
/// toggle completion operation, gathered upfront to avoid borrow conflicts.
struct ToggleData {
    /// The next recurring task to create (if completing a recurring task)
    next_recurring_task: Option<Task>,
    /// The ID of the next task in a chain (if completing a chained task)
    chain_next_id: Option<TaskId>,
    /// All descendant task IDs to cascade complete
    descendants_to_complete: Vec<TaskId>,
    /// Whether we're completing (vs uncompleting)
    is_completing: bool,
}

impl ToggleData {
    /// Gather all data needed for toggle completion before any mutations.
    fn gather(model: &Model, task_id: &TaskId) -> Option<Self> {
        let task = model.tasks.get(task_id)?;

        // Determine direction first (completing vs uncompleting)
        let is_completing = task.status != TaskStatus::Done;

        // For recurring tasks: prepare the next occurrence if completing
        let next_recurring_task = if is_completing && task.recurrence.is_some() {
            Some(create_next_recurring_task(task))
        } else {
            None
        };

        // For task chains: find the next task to schedule if completing
        let chain_next_id = task.next_task_id.filter(|_| is_completing);

        // For cascade completion: gather descendants if completing
        let descendants_to_complete = if is_completing {
            model.get_all_descendants(task_id)
        } else {
            Vec::new()
        };

        Some(Self {
            next_recurring_task,
            chain_next_id,
            descendants_to_complete,
            is_completing,
        })
    }
}

// ============================================================================
// Toggle Completion Helpers
// ============================================================================

/// Cascade completion to all descendants of a task.
///
/// Each descendant gets its own undo action so undoing reverses the entire cascade.
fn cascade_complete_descendants(model: &mut Model, descendants: &[TaskId]) {
    for descendant_id in descendants {
        let needs_complete = model
            .tasks
            .get(descendant_id)
            .is_some_and(|t| !t.status.is_complete());
        if needs_complete {
            model.modify_task_with_undo(descendant_id, |task| {
                task.status = TaskStatus::Done;
            });
        }
    }
}

/// Add the next occurrence of a recurring task.
fn add_next_recurring_task(model: &mut Model, new_task: Task) {
    model.sync_task(&new_task);
    model
        .undo_stack
        .push(UndoAction::TaskCreated(Box::new(new_task.clone())));
    model.tasks.insert(new_task.id, new_task);
}

/// Schedule the next task in a chain for today.
fn schedule_chained_task(model: &mut Model, next_id: &TaskId) {
    model.modify_task_with_undo(next_id, |task| {
        task.scheduled_date = Some(chrono::Local::now().date_naive());
    });
}

// ============================================================================
// Main Handler
// ============================================================================

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
            if let Some(id) = model.selected_task_id() {
                // Phase 1: Gather all data BEFORE any mutations (avoids borrow conflicts)
                if let Some(data) = ToggleData::gather(model, &id) {
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
                    if data.is_completing {
                        cascade_complete_descendants(model, &data.descendants_to_complete);
                    }

                    // Phase 4: Handle recurring task - add next occurrence
                    if let Some(new_task) = data.next_recurring_task {
                        add_next_recurring_task(model, new_task);
                    }

                    // Phase 5: Handle task chain - schedule next task for today
                    if let Some(ref next_id) = data.chain_next_id {
                        schedule_chained_task(model, next_id);
                    }
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
            if let Some(id) = model.selected_task_id() {
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
            if let Some(id) = model.selected_task_id() {
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

    // Invalidate report cache since task data has changed
    model.invalidate_report_cache();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Recurrence;
    use chrono::Weekday;

    #[test]
    fn test_create_next_recurring_task_daily() {
        let due = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
        let task = Task::new("Daily task")
            .with_due_date(due)
            .with_recurrence(Some(Recurrence::Daily));

        let next = create_next_recurring_task(&task);

        assert_eq!(next.title, "Daily task");
        assert_eq!(next.due_date, Some(due + Duration::days(1)));
        assert!(matches!(next.recurrence, Some(Recurrence::Daily)));
    }

    #[test]
    fn test_create_next_recurring_task_weekly_empty_days() {
        let due = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap(); // Saturday
        let task = Task::new("Weekly task")
            .with_due_date(due)
            .with_recurrence(Some(Recurrence::Weekly { days: vec![] }));

        let next = create_next_recurring_task(&task);

        // Empty days = same day next week
        assert_eq!(next.due_date, Some(due + Duration::weeks(1)));
    }

    #[test]
    fn test_create_next_recurring_task_weekly_specific_days() {
        let task = Task::new("Weekly task")
            .with_due_date(NaiveDate::from_ymd_opt(2025, 3, 15).unwrap())
            .with_recurrence(Some(Recurrence::Weekly {
                days: vec![Weekday::Mon, Weekday::Wed, Weekday::Fri],
            }));

        let next = create_next_recurring_task(&task);

        // Next occurrence should be on one of the specified days
        let next_day = next.due_date.unwrap().weekday();
        assert!(
            next_day == Weekday::Mon || next_day == Weekday::Wed || next_day == Weekday::Fri,
            "Expected Mon/Wed/Fri but got {next_day:?}"
        );
    }

    #[test]
    fn test_create_next_recurring_task_monthly() {
        let due = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
        let task = Task::new("Monthly task")
            .with_due_date(due)
            .with_recurrence(Some(Recurrence::Monthly { day: 15 }));

        let next = create_next_recurring_task(&task);

        // Next month, same day
        assert_eq!(
            next.due_date,
            Some(NaiveDate::from_ymd_opt(2025, 4, 15).unwrap())
        );
    }

    #[test]
    fn test_create_next_recurring_task_monthly_december_rollover() {
        let due = NaiveDate::from_ymd_opt(2025, 12, 10).unwrap();
        let task = Task::new("Monthly task")
            .with_due_date(due)
            .with_recurrence(Some(Recurrence::Monthly { day: 10 }));

        let next = create_next_recurring_task(&task);

        // December -> January next year
        assert_eq!(
            next.due_date,
            Some(NaiveDate::from_ymd_opt(2026, 1, 10).unwrap())
        );
    }

    #[test]
    fn test_create_next_recurring_task_monthly_invalid_day() {
        // Feb 30 doesn't exist - should fall back to last day of Feb
        let due = NaiveDate::from_ymd_opt(2025, 1, 30).unwrap();
        let task = Task::new("Monthly task")
            .with_due_date(due)
            .with_recurrence(Some(Recurrence::Monthly { day: 30 }));

        let next = create_next_recurring_task(&task);

        // Feb 30 invalid -> Feb 28 (2025 is not a leap year)
        assert_eq!(
            next.due_date,
            Some(NaiveDate::from_ymd_opt(2025, 2, 28).unwrap())
        );
    }

    #[test]
    fn test_create_next_recurring_task_yearly() {
        let due = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let task = Task::new("Yearly task")
            .with_due_date(due)
            .with_recurrence(Some(Recurrence::Yearly { month: 6, day: 15 }));

        let next = create_next_recurring_task(&task);

        // Same date next year
        assert_eq!(
            next.due_date,
            Some(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap())
        );
    }

    #[test]
    fn test_create_next_recurring_task_yearly_feb_29_non_leap() {
        // Feb 29 in a leap year -> Feb 28 in non-leap year
        let due = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(); // 2024 is leap year
        let task = Task::new("Leap year task")
            .with_due_date(due)
            .with_recurrence(Some(Recurrence::Yearly { month: 2, day: 29 }));

        let next = create_next_recurring_task(&task);

        // 2025 is not a leap year, so Feb 29 -> Feb 28
        assert_eq!(
            next.due_date,
            Some(NaiveDate::from_ymd_opt(2025, 2, 28).unwrap())
        );
    }

    #[test]
    fn test_create_next_recurring_task_preserves_fields() {
        let due = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
        let task = Task::new("Recurring task")
            .with_due_date(due)
            .with_priority(Priority::High)
            .with_tags(vec!["work".to_string(), "important".to_string()])
            .with_description("A recurring task description")
            .with_recurrence(Some(Recurrence::Daily));

        let next = create_next_recurring_task(&task);

        assert_eq!(next.priority, Priority::High);
        assert_eq!(next.tags, vec!["work".to_string(), "important".to_string()]);
        assert_eq!(
            next.description,
            Some("A recurring task description".to_string())
        );
        assert!(matches!(next.recurrence, Some(Recurrence::Daily)));
    }

    #[test]
    fn test_create_next_recurring_task_no_due_date_uses_today() {
        let task = Task::new("No due date").with_recurrence(Some(Recurrence::Daily));

        let next = create_next_recurring_task(&task);
        let today = Utc::now().date_naive();

        // Should use today as base, so next is tomorrow
        assert_eq!(next.due_date, Some(today + Duration::days(1)));
    }

    #[test]
    fn test_create_next_recurring_task_none_recurrence() {
        // Edge case: called with no recurrence (shouldn't happen but code handles it)
        let due = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
        let task = Task::new("No recurrence").with_due_date(due);

        let next = create_next_recurring_task(&task);
        let today = Utc::now().date_naive();

        // Falls through to default: today + 1 day
        assert_eq!(next.due_date, Some(today + Duration::days(1)));
    }

    #[test]
    fn test_create_next_recurring_task_yearly_december() {
        // Test yearly recurrence in December
        let due = NaiveDate::from_ymd_opt(2025, 12, 25).unwrap();
        let task = Task::new("Christmas task")
            .with_due_date(due)
            .with_recurrence(Some(Recurrence::Yearly { month: 12, day: 25 }));

        let next = create_next_recurring_task(&task);

        assert_eq!(
            next.due_date,
            Some(NaiveDate::from_ymd_opt(2026, 12, 25).unwrap())
        );
    }

    // ========================================================================
    // Additional Edge Cases - Short Months and Leap Years
    // ========================================================================

    #[test]
    fn test_create_next_recurring_task_monthly_31st_to_30_day_month() {
        // Mar 31 -> Apr 30 (April only has 30 days)
        let due = NaiveDate::from_ymd_opt(2025, 3, 31).unwrap();
        let task = Task::new("Monthly task")
            .with_due_date(due)
            .with_recurrence(Some(Recurrence::Monthly { day: 31 }));

        let next = create_next_recurring_task(&task);

        // April 31 doesn't exist -> April 30
        assert_eq!(
            next.due_date,
            Some(NaiveDate::from_ymd_opt(2025, 4, 30).unwrap())
        );
    }

    #[test]
    fn test_create_next_recurring_task_monthly_31st_to_feb_leap_year() {
        // Jan 31, 2024 -> Feb (2024 is a leap year, so Feb has 29 days)
        let due = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let task = Task::new("Monthly task")
            .with_due_date(due)
            .with_recurrence(Some(Recurrence::Monthly { day: 31 }));

        let next = create_next_recurring_task(&task);

        // Feb 31 doesn't exist -> Feb 29 (leap year)
        assert_eq!(
            next.due_date,
            Some(NaiveDate::from_ymd_opt(2024, 2, 29).unwrap())
        );
    }

    #[test]
    fn test_create_next_recurring_task_yearly_feb_29_leap_to_leap() {
        // Feb 29, 2024 (leap year) -> Feb 29, 2028 (next leap year)
        // This tests the less common case: year 2024 -> 2025 (non-leap) already covered
        // Here we test what happens after multiple non-leap years

        // Note: The implementation creates next occurrence for the immediate next year,
        // not skipping to the next leap year. So Feb 29, 2024 -> Feb 28, 2025.
        // This test documents that behavior.
        let due = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        let task = Task::new("Leap year task")
            .with_due_date(due)
            .with_recurrence(Some(Recurrence::Yearly { month: 2, day: 29 }));

        let next = create_next_recurring_task(&task);

        // 2025 is not a leap year, so Feb 29 -> Feb 28
        assert_eq!(
            next.due_date,
            Some(NaiveDate::from_ymd_opt(2025, 2, 28).unwrap())
        );
    }

    #[test]
    fn test_create_next_recurring_task_monthly_30th_to_feb() {
        // Jan 30 -> Feb (Feb only has 28/29 days)
        let due = NaiveDate::from_ymd_opt(2025, 1, 30).unwrap();
        let task = Task::new("Monthly task")
            .with_due_date(due)
            .with_recurrence(Some(Recurrence::Monthly { day: 30 }));

        let next = create_next_recurring_task(&task);

        // Feb 30 doesn't exist, 2025 is not a leap year -> Feb 28
        assert_eq!(
            next.due_date,
            Some(NaiveDate::from_ymd_opt(2025, 2, 28).unwrap())
        );
    }

    #[test]
    fn test_create_next_recurring_task_yearly_invalid_date() {
        // Test yearly recurrence with invalid date (Apr 31)
        let due = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
        let task = Task::new("Invalid date task")
            .with_due_date(due)
            .with_recurrence(Some(Recurrence::Yearly { month: 4, day: 31 }));

        let next = create_next_recurring_task(&task);

        // April 31 doesn't exist -> April 30
        assert_eq!(
            next.due_date,
            Some(NaiveDate::from_ymd_opt(2026, 4, 30).unwrap())
        );
    }
}
