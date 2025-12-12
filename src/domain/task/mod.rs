//! Task entity and related types.
//!
//! This module contains the core [`Task`] type along with supporting
//! types like [`TaskId`], [`Priority`], [`TaskStatus`], and [`Recurrence`].

mod priority;
mod recurrence;
mod status;
mod task_id;

#[cfg(test)]
mod tests;

pub use priority::Priority;
pub use recurrence::Recurrence;
pub use status::TaskStatus;
pub use task_id::TaskId;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ProjectId;

/// A task represents a unit of work to be done.
///
/// Tasks are the core entity in `TaskFlow`. They have a title, status,
/// priority, and various optional metadata like due dates, tags,
/// descriptions, and time tracking.
///
/// # Examples
///
/// ## Creating Tasks
///
/// ```
/// use taskflow::domain::{Task, Priority, TaskStatus};
/// use chrono::Utc;
///
/// // Simple task
/// let task = Task::new("Buy groceries");
///
/// // Task with builder pattern
/// let today = Utc::now().date_naive();
/// let task = Task::new("Fix login bug")
///     .with_priority(Priority::Urgent)
///     .with_due_date(today)
///     .with_tags(vec!["bug".into(), "auth".into()])
///     .with_description("Users can't login with SSO".to_string());
///
/// assert_eq!(task.priority, Priority::Urgent);
/// assert!(task.is_due_today());
/// ```
///
/// ## Task Completion
///
/// ```
/// use taskflow::domain::{Task, TaskStatus};
///
/// let mut task = Task::new("Review PR");
/// assert_eq!(task.status, TaskStatus::Todo);
///
/// task.toggle_complete();
/// assert_eq!(task.status, TaskStatus::Done);
/// assert!(task.completed_at.is_some());
///
/// // Toggle back to incomplete
/// task.toggle_complete();
/// assert_eq!(task.status, TaskStatus::Todo);
/// ```
///
/// ## Subtasks
///
/// ```
/// use taskflow::domain::Task;
///
/// let parent = Task::new("Release v2.0");
/// let subtask = Task::new("Write changelog")
///     .with_parent(parent.id);
///
/// assert_eq!(subtask.parent_task_id, Some(parent.id));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: Priority,

    // Relationships
    pub project_id: Option<ProjectId>,
    pub parent_task_id: Option<TaskId>,
    pub tags: Vec<String>,
    pub dependencies: Vec<TaskId>,

    // Dates
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub due_date: Option<NaiveDate>,
    pub scheduled_date: Option<NaiveDate>,
    pub completed_at: Option<DateTime<Utc>>,

    // Recurrence
    pub recurrence: Option<Recurrence>,

    // Time tracking
    pub estimated_minutes: Option<u32>,
    pub actual_minutes: u32,

    // Manual ordering (lower values appear first)
    #[serde(default)]
    pub sort_order: Option<i32>,

    // Task chains - link to next task in sequence
    #[serde(default)]
    pub next_task_id: Option<TaskId>,

    // Snooze - hide task until this date
    #[serde(default)]
    pub snooze_until: Option<NaiveDate>,

    // Custom fields for extensibility
    #[serde(default)]
    pub custom_fields: HashMap<String, serde_json::Value>,

    // Git integration - linked branch
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<super::git::GitRef>,
}

impl Task {
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: TaskId::new(),
            title: title.into(),
            description: None,
            status: TaskStatus::default(),
            priority: Priority::default(),
            project_id: None,
            parent_task_id: None,
            tags: Vec::new(),
            dependencies: Vec::new(),
            created_at: now,
            updated_at: now,
            due_date: None,
            scheduled_date: None,
            completed_at: None,
            recurrence: None,
            estimated_minutes: None,
            actual_minutes: 0,
            sort_order: None,
            next_task_id: None,
            snooze_until: None,
            custom_fields: HashMap::new(),
            git_ref: None,
        }
    }

    #[must_use]
    pub const fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    #[must_use]
    pub fn with_status(mut self, status: TaskStatus) -> Self {
        self.status = status;
        if status.is_complete() && self.completed_at.is_none() {
            self.completed_at = Some(Utc::now());
        }
        self
    }

    #[must_use]
    pub const fn with_due_date(mut self, date: NaiveDate) -> Self {
        self.due_date = Some(date);
        self
    }

    #[must_use]
    pub const fn with_project(mut self, project_id: ProjectId) -> Self {
        self.project_id = Some(project_id);
        self
    }

    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    #[must_use]
    pub const fn with_parent(mut self, parent_id: TaskId) -> Self {
        self.parent_task_id = Some(parent_id);
        self
    }

    #[must_use]
    pub fn with_recurrence(mut self, recurrence: Option<Recurrence>) -> Self {
        self.recurrence = recurrence;
        self
    }

    #[must_use]
    pub const fn with_project_opt(mut self, project_id: Option<super::ProjectId>) -> Self {
        self.project_id = project_id;
        self
    }

    #[must_use]
    pub fn with_description_opt(mut self, description: Option<String>) -> Self {
        self.description = description;
        self
    }

    /// Sets a custom completion timestamp.
    ///
    /// This is useful for sample data and testing. For normal usage,
    /// prefer `with_status(TaskStatus::Done)` which auto-sets the timestamp.
    #[must_use]
    pub fn with_completed_at(mut self, completed_at: DateTime<Utc>) -> Self {
        self.completed_at = Some(completed_at);
        self
    }

    /// Sets a custom estimated time in minutes.
    #[must_use]
    pub const fn with_estimated_minutes(mut self, minutes: u32) -> Self {
        self.estimated_minutes = Some(minutes);
        self
    }

    /// Links the task to a git branch.
    #[must_use]
    pub fn with_git_ref(mut self, git_ref: super::git::GitRef) -> Self {
        self.git_ref = Some(git_ref);
        self
    }

    #[must_use]
    pub fn is_overdue(&self) -> bool {
        if let Some(due) = self.due_date {
            return due < Utc::now().date_naive() && !self.status.is_complete();
        }
        false
    }

    #[must_use]
    pub fn is_due_today(&self) -> bool {
        if let Some(due) = self.due_date {
            return due == Utc::now().date_naive();
        }
        false
    }

    /// Returns true if the task is currently snoozed (hidden until a future date).
    #[must_use]
    pub fn is_snoozed(&self) -> bool {
        self.snooze_until
            .is_some_and(|date| date > Utc::now().date_naive())
    }

    /// Snooze the task until a specific date.
    pub fn snooze_until_date(&mut self, date: NaiveDate) {
        self.snooze_until = Some(date);
        self.updated_at = Utc::now();
    }

    /// Clear the snooze on this task.
    pub fn clear_snooze(&mut self) {
        self.snooze_until = None;
        self.updated_at = Utc::now();
    }

    /// Returns time variance in minutes (positive = over estimate, negative = under estimate).
    /// Returns None if no estimated time is set.
    #[must_use]
    pub fn time_variance(&self) -> Option<i32> {
        self.estimated_minutes.and_then(|est| {
            let actual = i32::try_from(self.actual_minutes).ok()?;
            let estimate = i32::try_from(est).ok()?;
            Some(actual - estimate)
        })
    }

    /// Returns formatted variance string: "+30m over", "-15m under", "on target".
    /// Returns None if no estimated time is set.
    #[must_use]
    pub fn time_variance_display(&self) -> Option<String> {
        self.time_variance().map(|variance| {
            if variance == 0 {
                "on target".to_string()
            } else if variance > 0 {
                let hours = variance / 60;
                let mins = variance % 60;
                if hours > 0 {
                    format!("+{hours}h {mins}m over")
                } else {
                    format!("+{mins}m over")
                }
            } else {
                let abs = variance.abs();
                let hours = abs / 60;
                let mins = abs % 60;
                if hours > 0 {
                    format!("-{hours}h {mins}m under")
                } else {
                    format!("-{mins}m under")
                }
            }
        })
    }

    /// Returns estimation accuracy as a percentage (actual / estimated * 100).
    /// Returns None if no estimated time is set or if estimated time is zero.
    #[must_use]
    pub fn estimation_accuracy(&self) -> Option<f64> {
        self.estimated_minutes.and_then(|est| {
            if est == 0 {
                None
            } else {
                Some(f64::from(self.actual_minutes) / f64::from(est) * 100.0)
            }
        })
    }

    pub fn toggle_complete(&mut self) {
        if self.status == TaskStatus::Done {
            self.status = TaskStatus::Todo;
            self.completed_at = None;
        } else {
            self.status = TaskStatus::Done;
            self.completed_at = Some(Utc::now());
        }
        self.updated_at = Utc::now();
    }
}
