//! Task entity and related types.
//!
//! This module contains the core [`Task`] type along with supporting
//! types like [`TaskId`], [`Priority`], [`TaskStatus`], and [`Recurrence`].

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::ProjectId;

/// Unique identifier for tasks.
///
/// Each task has a UUID-based identifier that remains stable across
/// serialization and storage operations.
///
/// # Examples
///
/// ```
/// use taskflow::domain::TaskId;
///
/// let id1 = TaskId::new();
/// let id2 = TaskId::new();
/// assert_ne!(id1, id2); // Each ID is unique
///
/// // Can be displayed as a string
/// println!("Task ID: {}", id1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);

impl TaskId {
    /// Creates a new unique task identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Task priority levels from lowest to highest urgency.
///
/// Priority helps organize tasks by importance. Each level has an
/// associated symbol displayed in the UI.
///
/// # Examples
///
/// ```
/// use taskflow::domain::Priority;
///
/// let priority = Priority::High;
/// assert_eq!(priority.symbol(), "!!!");
/// assert_eq!(priority.as_str(), "high");
///
/// // Parse from string (case-insensitive)
/// assert_eq!(Priority::parse("HIGH"), Some(Priority::High));
/// assert_eq!(Priority::parse("med"), Some(Priority::Medium));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    /// No priority assigned (default)
    #[default]
    None,
    /// Low priority - nice to have, backlog items
    Low,
    /// Medium priority - standard work items
    Medium,
    /// High priority - important features, upcoming deadlines
    High,
    /// Urgent priority - critical issues, production bugs
    Urgent,
}

impl Priority {
    /// Returns the priority as a lowercase string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }

    /// Parses a priority from a string (case-insensitive).
    ///
    /// Accepts "med" as a shorthand for "medium".
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "none" => Some(Self::None),
            "low" => Some(Self::Low),
            "medium" | "med" => Some(Self::Medium),
            "high" => Some(Self::High),
            "urgent" => Some(Self::Urgent),
            _ => None,
        }
    }

    /// Returns the visual symbol for this priority level.
    ///
    /// Used in the UI to show priority at a glance.
    #[must_use]
    pub const fn symbol(&self) -> &'static str {
        match self {
            Self::None => " ",
            Self::Low => "!",
            Self::Medium => "!!",
            Self::High => "!!!",
            Self::Urgent => "!!!!",
        }
    }
}

/// Task completion status.
///
/// Represents the current state of a task in its lifecycle.
///
/// # Examples
///
/// ```
/// use taskflow::domain::TaskStatus;
///
/// let status = TaskStatus::InProgress;
/// assert_eq!(status.symbol(), "[~]");
/// assert!(!status.is_complete());
///
/// let done = TaskStatus::Done;
/// assert!(done.is_complete());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Task has not been started (default)
    #[default]
    Todo,
    /// Task is currently being worked on
    InProgress,
    /// Task is waiting on something else
    Blocked,
    /// Task has been completed successfully
    Done,
    /// Task was cancelled and won't be done
    Cancelled,
}

impl TaskStatus {
    /// Returns the status as a lowercase string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        }
    }

    /// Returns the visual symbol for this status.
    ///
    /// Used in the UI to show status at a glance.
    #[must_use]
    pub const fn symbol(&self) -> &'static str {
        match self {
            Self::Todo => "[ ]",
            Self::InProgress => "[~]",
            Self::Blocked => "[!]",
            Self::Done => "[x]",
            Self::Cancelled => "[-]",
        }
    }

    /// Returns true if the task is in a terminal state (Done or Cancelled).
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        matches!(self, Self::Done | Self::Cancelled)
    }
}

/// Recurrence pattern for repeating tasks.
///
/// When a recurring task is completed, a new instance is automatically
/// created based on the recurrence pattern.
///
/// # Examples
///
/// ```
/// use taskflow::domain::Recurrence;
/// use chrono::Weekday;
///
/// // Daily tasks (e.g., standup)
/// let daily = Recurrence::Daily;
///
/// // Weekly on specific days (e.g., team sync on Mon/Wed/Fri)
/// let weekly = Recurrence::Weekly {
///     days: vec![Weekday::Mon, Weekday::Wed, Weekday::Fri]
/// };
///
/// // Monthly on a specific day (e.g., monthly report on the 15th)
/// let monthly = Recurrence::Monthly { day: 15 };
///
/// // Yearly (e.g., annual review on March 1st)
/// let yearly = Recurrence::Yearly { month: 3, day: 1 };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Recurrence {
    /// Repeats every day
    Daily,
    /// Repeats on specific days of the week
    Weekly {
        /// Days of the week when the task recurs
        days: Vec<chrono::Weekday>,
    },
    /// Repeats on a specific day each month
    Monthly {
        /// Day of the month (1-31)
        day: u32,
    },
    /// Repeats on a specific date each year
    Yearly {
        /// Month (1-12)
        month: u32,
        /// Day of the month (1-31)
        day: u32,
    },
}

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
///     .with_parent(parent.id.clone());
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
                    format!("+{}h {}m over", hours, mins)
                } else {
                    format!("+{}m over", mins)
                }
            } else {
                let abs = variance.abs();
                let hours = abs / 60;
                let mins = abs % 60;
                if hours > 0 {
                    format!("-{}h {}m under", hours, mins)
                } else {
                    format!("-{}m under", mins)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_new_creates_unique_id() {
        let task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");
        assert_ne!(task1.id, task2.id);
    }

    #[test]
    fn test_task_new_sets_defaults() {
        let task = Task::new("Test task");
        assert_eq!(task.title, "Test task");
        assert_eq!(task.status, TaskStatus::Todo);
        assert_eq!(task.priority, Priority::None);
        assert!(task.description.is_none());
        assert!(task.due_date.is_none());
        assert!(task.completed_at.is_none());
        assert!(task.tags.is_empty());
    }

    #[test]
    fn test_task_with_priority() {
        let task = Task::new("Test").with_priority(Priority::High);
        assert_eq!(task.priority, Priority::High);
    }

    #[test]
    fn test_task_with_status_sets_completion() {
        let task = Task::new("Test").with_status(TaskStatus::Done);
        assert_eq!(task.status, TaskStatus::Done);
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_task_with_due_date() {
        let date = NaiveDate::from_ymd_opt(2025, 12, 25).unwrap();
        let task = Task::new("Test").with_due_date(date);
        assert_eq!(task.due_date, Some(date));
    }

    #[test]
    fn test_task_toggle_complete_todo_to_done() {
        let mut task = Task::new("Test");
        assert_eq!(task.status, TaskStatus::Todo);
        assert!(task.completed_at.is_none());

        task.toggle_complete();

        assert_eq!(task.status, TaskStatus::Done);
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_task_toggle_complete_done_to_todo() {
        let mut task = Task::new("Test").with_status(TaskStatus::Done);
        assert_eq!(task.status, TaskStatus::Done);
        assert!(task.completed_at.is_some());

        task.toggle_complete();

        assert_eq!(task.status, TaskStatus::Todo);
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn test_task_is_overdue_no_due_date() {
        let task = Task::new("Test");
        assert!(!task.is_overdue());
    }

    #[test]
    fn test_task_is_overdue_past_date() {
        let past_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let task = Task::new("Test").with_due_date(past_date);
        assert!(task.is_overdue());
    }

    #[test]
    fn test_task_is_overdue_completed() {
        let past_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let task = Task::new("Test")
            .with_due_date(past_date)
            .with_status(TaskStatus::Done);
        assert!(!task.is_overdue());
    }

    #[test]
    fn test_task_is_due_today() {
        let today = Utc::now().date_naive();
        let task = Task::new("Test").with_due_date(today);
        assert!(task.is_due_today());

        let yesterday = today - chrono::Duration::days(1);
        let task2 = Task::new("Test").with_due_date(yesterday);
        assert!(!task2.is_due_today());
    }

    #[test]
    fn test_priority_as_str() {
        assert_eq!(Priority::None.as_str(), "none");
        assert_eq!(Priority::Low.as_str(), "low");
        assert_eq!(Priority::Medium.as_str(), "medium");
        assert_eq!(Priority::High.as_str(), "high");
        assert_eq!(Priority::Urgent.as_str(), "urgent");
    }

    #[test]
    fn test_priority_symbol() {
        assert_eq!(Priority::None.symbol(), " ");
        assert_eq!(Priority::Low.symbol(), "!");
        assert_eq!(Priority::Medium.symbol(), "!!");
        assert_eq!(Priority::High.symbol(), "!!!");
        assert_eq!(Priority::Urgent.symbol(), "!!!!");
    }

    #[test]
    fn test_priority_parse() {
        assert_eq!(Priority::parse("none"), Some(Priority::None));
        assert_eq!(Priority::parse("low"), Some(Priority::Low));
        assert_eq!(Priority::parse("medium"), Some(Priority::Medium));
        assert_eq!(Priority::parse("med"), Some(Priority::Medium));
        assert_eq!(Priority::parse("high"), Some(Priority::High));
        assert_eq!(Priority::parse("urgent"), Some(Priority::Urgent));
        // Case insensitive
        assert_eq!(Priority::parse("HIGH"), Some(Priority::High));
        assert_eq!(Priority::parse("Low"), Some(Priority::Low));
        // Invalid
        assert_eq!(Priority::parse("invalid"), None);
        assert_eq!(Priority::parse(""), None);
    }

    #[test]
    fn test_task_status_as_str() {
        assert_eq!(TaskStatus::Todo.as_str(), "todo");
        assert_eq!(TaskStatus::InProgress.as_str(), "in_progress");
        assert_eq!(TaskStatus::Blocked.as_str(), "blocked");
        assert_eq!(TaskStatus::Done.as_str(), "done");
        assert_eq!(TaskStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn test_task_status_symbol() {
        assert_eq!(TaskStatus::Todo.symbol(), "[ ]");
        assert_eq!(TaskStatus::InProgress.symbol(), "[~]");
        assert_eq!(TaskStatus::Blocked.symbol(), "[!]");
        assert_eq!(TaskStatus::Done.symbol(), "[x]");
        assert_eq!(TaskStatus::Cancelled.symbol(), "[-]");
    }

    #[test]
    fn test_task_status_is_complete() {
        assert!(!TaskStatus::Todo.is_complete());
        assert!(!TaskStatus::InProgress.is_complete());
        assert!(!TaskStatus::Blocked.is_complete());
        assert!(TaskStatus::Done.is_complete());
        assert!(TaskStatus::Cancelled.is_complete());
    }

    #[test]
    fn test_time_variance_over_estimate() {
        let mut task = Task::new("Test");
        task.estimated_minutes = Some(60);
        task.actual_minutes = 90;
        assert_eq!(task.time_variance(), Some(30));
        assert_eq!(task.time_variance_display(), Some("+30m over".to_string()));
    }

    #[test]
    fn test_time_variance_under_estimate() {
        let mut task = Task::new("Test");
        task.estimated_minutes = Some(60);
        task.actual_minutes = 45;
        assert_eq!(task.time_variance(), Some(-15));
        assert_eq!(task.time_variance_display(), Some("-15m under".to_string()));
    }

    #[test]
    fn test_time_variance_on_target() {
        let mut task = Task::new("Test");
        task.estimated_minutes = Some(60);
        task.actual_minutes = 60;
        assert_eq!(task.time_variance(), Some(0));
        assert_eq!(task.time_variance_display(), Some("on target".to_string()));
    }

    #[test]
    fn test_time_variance_no_estimate() {
        let task = Task::new("Test");
        assert_eq!(task.time_variance(), None);
        assert_eq!(task.time_variance_display(), None);
    }

    #[test]
    fn test_time_variance_display_hours() {
        let mut task = Task::new("Test");
        task.estimated_minutes = Some(60);
        task.actual_minutes = 150; // 90 minutes over
        assert_eq!(
            task.time_variance_display(),
            Some("+1h 30m over".to_string())
        );

        task.actual_minutes = 0; // 60 minutes under
        assert_eq!(
            task.time_variance_display(),
            Some("-1h 0m under".to_string())
        );
    }

    #[test]
    fn test_estimation_accuracy() {
        let mut task = Task::new("Test");
        task.estimated_minutes = Some(100);
        task.actual_minutes = 100;
        assert!((task.estimation_accuracy().unwrap() - 100.0).abs() < 0.01);

        task.actual_minutes = 50;
        assert!((task.estimation_accuracy().unwrap() - 50.0).abs() < 0.01);

        task.actual_minutes = 150;
        assert!((task.estimation_accuracy().unwrap() - 150.0).abs() < 0.01);
    }

    #[test]
    fn test_estimation_accuracy_zero_estimate() {
        let mut task = Task::new("Test");
        task.estimated_minutes = Some(0);
        task.actual_minutes = 100;
        assert_eq!(task.estimation_accuracy(), None);
    }

    #[test]
    fn test_estimation_accuracy_no_estimate() {
        let task = Task::new("Test");
        assert_eq!(task.estimation_accuracy(), None);
    }

    // Snooze functionality tests

    #[test]
    fn test_task_not_snoozed_by_default() {
        let task = Task::new("Test");
        assert!(!task.is_snoozed());
        assert!(task.snooze_until.is_none());
    }

    #[test]
    fn test_task_snooze_until_future_date() {
        let mut task = Task::new("Test");
        let future = Utc::now().date_naive() + chrono::Duration::days(7);
        task.snooze_until_date(future);

        assert!(task.is_snoozed());
        assert_eq!(task.snooze_until, Some(future));
    }

    #[test]
    fn test_task_snooze_until_past_date_not_snoozed() {
        let mut task = Task::new("Test");
        let past = Utc::now().date_naive() - chrono::Duration::days(1);
        task.snooze_until_date(past);

        // Snooze date is set, but is_snoozed returns false for past dates
        assert!(!task.is_snoozed());
        assert_eq!(task.snooze_until, Some(past));
    }

    #[test]
    fn test_task_snooze_until_today_not_snoozed() {
        let mut task = Task::new("Test");
        let today = Utc::now().date_naive();
        task.snooze_until_date(today);

        // Today is not > today, so not snoozed
        assert!(!task.is_snoozed());
    }

    #[test]
    fn test_task_clear_snooze() {
        let mut task = Task::new("Test");
        let future = Utc::now().date_naive() + chrono::Duration::days(7);
        task.snooze_until_date(future);
        assert!(task.is_snoozed());

        task.clear_snooze();
        assert!(!task.is_snoozed());
        assert!(task.snooze_until.is_none());
    }

    #[test]
    fn test_snooze_updates_updated_at() {
        let mut task = Task::new("Test");
        let original_updated = task.updated_at;

        // Small delay to ensure time difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        let future = Utc::now().date_naive() + chrono::Duration::days(1);
        task.snooze_until_date(future);

        assert!(task.updated_at > original_updated);
    }

    #[test]
    fn test_clear_snooze_updates_updated_at() {
        let mut task = Task::new("Test");
        let future = Utc::now().date_naive() + chrono::Duration::days(1);
        task.snooze_until_date(future);
        let after_snooze = task.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));

        task.clear_snooze();
        assert!(task.updated_at > after_snooze);
    }

    // Scheduled date tests

    #[test]
    fn test_task_scheduled_date() {
        let date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let mut task = Task::new("Test");
        task.scheduled_date = Some(date);

        assert_eq!(task.scheduled_date, Some(date));
    }

    #[test]
    fn test_task_scheduled_vs_due_dates_independent() {
        let scheduled = NaiveDate::from_ymd_opt(2025, 6, 10).unwrap();
        let due = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

        let task = Task::new("Test").with_due_date(due);
        let mut task = task;
        task.scheduled_date = Some(scheduled);

        assert_eq!(task.scheduled_date, Some(scheduled));
        assert_eq!(task.due_date, Some(due));
    }

    // Custom fields tests

    #[test]
    fn test_task_custom_fields_empty_by_default() {
        let task = Task::new("Test");
        assert!(task.custom_fields.is_empty());
    }

    #[test]
    fn test_task_custom_fields_string() {
        let mut task = Task::new("Test");
        task.custom_fields
            .insert("client".to_string(), serde_json::json!("Acme Corp"));

        assert_eq!(
            task.custom_fields.get("client"),
            Some(&serde_json::json!("Acme Corp"))
        );
    }

    #[test]
    fn test_task_custom_fields_number() {
        let mut task = Task::new("Test");
        task.custom_fields
            .insert("story_points".to_string(), serde_json::json!(5));

        assert_eq!(
            task.custom_fields.get("story_points"),
            Some(&serde_json::json!(5))
        );
    }

    #[test]
    fn test_task_custom_fields_complex() {
        let mut task = Task::new("Test");
        task.custom_fields.insert(
            "metadata".to_string(),
            serde_json::json!({"reviewed": true, "reviewer": "alice"}),
        );

        let metadata = task.custom_fields.get("metadata").unwrap();
        assert_eq!(metadata["reviewed"], serde_json::json!(true));
        assert_eq!(metadata["reviewer"], serde_json::json!("alice"));
    }

    // Dependencies tests

    #[test]
    fn test_task_dependencies_empty_by_default() {
        let task = Task::new("Test");
        assert!(task.dependencies.is_empty());
    }

    #[test]
    fn test_task_dependencies() {
        let task1 = Task::new("Blocker");
        let mut task2 = Task::new("Blocked task");
        task2.dependencies.push(task1.id);

        assert_eq!(task2.dependencies.len(), 1);
        assert_eq!(task2.dependencies[0], task1.id);
    }

    // Task chain tests

    #[test]
    fn test_task_chain_next_task() {
        let task1 = Task::new("First");
        let mut task2 = Task::new("Second");
        task2.next_task_id = Some(task1.id);

        assert_eq!(task2.next_task_id, Some(task1.id));
    }

    #[test]
    fn test_task_chain_none_by_default() {
        let task = Task::new("Test");
        assert!(task.next_task_id.is_none());
    }

    // Sort order tests

    #[test]
    fn test_task_sort_order() {
        let mut task = Task::new("Test");
        task.sort_order = Some(100);

        assert_eq!(task.sort_order, Some(100));
    }

    #[test]
    fn test_task_sort_order_negative() {
        let mut task = Task::new("Test");
        task.sort_order = Some(-50);

        assert_eq!(task.sort_order, Some(-50));
    }

    // Serialization tests

    #[test]
    fn test_task_serialization_roundtrip() {
        let task = Task::new("Roundtrip test")
            .with_priority(Priority::High)
            .with_status(TaskStatus::InProgress)
            .with_tags(vec!["tag1".to_string(), "tag2".to_string()])
            .with_description("A description");

        let json = serde_json::to_string(&task).expect("Failed to serialize");
        let restored: Task = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(restored.id, task.id);
        assert_eq!(restored.title, task.title);
        assert_eq!(restored.priority, task.priority);
        assert_eq!(restored.status, task.status);
        assert_eq!(restored.tags, task.tags);
        assert_eq!(restored.description, task.description);
    }

    #[test]
    fn test_task_serialization_with_snooze() {
        let mut task = Task::new("Snoozed task");
        let future = NaiveDate::from_ymd_opt(2025, 12, 25).unwrap();
        task.snooze_until = Some(future);

        let json = serde_json::to_string(&task).expect("Failed to serialize");
        let restored: Task = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(restored.snooze_until, Some(future));
    }

    #[test]
    fn test_task_serialization_with_custom_fields() {
        let mut task = Task::new("Task with fields");
        task.custom_fields
            .insert("key".to_string(), serde_json::json!("value"));

        let json = serde_json::to_string(&task).expect("Failed to serialize");
        let restored: Task = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(
            restored.custom_fields.get("key"),
            Some(&serde_json::json!("value"))
        );
    }
}
