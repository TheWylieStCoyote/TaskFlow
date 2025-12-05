use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::ProjectId;

/// Unique identifier for tasks
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);

impl TaskId {
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

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    #[default]
    None,
    Low,
    Medium,
    High,
    Urgent,
}

impl Priority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Priority::None => "none",
            Priority::Low => "low",
            Priority::Medium => "medium",
            Priority::High => "high",
            Priority::Urgent => "urgent",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "none" => Some(Priority::None),
            "low" => Some(Priority::Low),
            "medium" | "med" => Some(Priority::Medium),
            "high" => Some(Priority::High),
            "urgent" => Some(Priority::Urgent),
            _ => None,
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            Priority::None => " ",
            Priority::Low => "!",
            Priority::Medium => "!!",
            Priority::High => "!!!",
            Priority::Urgent => "!!!!",
        }
    }
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    #[default]
    Todo,
    InProgress,
    Blocked,
    Done,
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Todo => "todo",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Blocked => "blocked",
            TaskStatus::Done => "done",
            TaskStatus::Cancelled => "cancelled",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            TaskStatus::Todo => "[ ]",
            TaskStatus::InProgress => "[~]",
            TaskStatus::Blocked => "[!]",
            TaskStatus::Done => "[x]",
            TaskStatus::Cancelled => "[-]",
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, TaskStatus::Done | TaskStatus::Cancelled)
    }
}

/// Recurrence pattern for repeating tasks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Recurrence {
    Daily,
    Weekly { days: Vec<chrono::Weekday> },
    Monthly { day: u32 },
    Yearly { month: u32, day: u32 },
}

/// Core task entity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
            custom_fields: HashMap::new(),
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_status(mut self, status: TaskStatus) -> Self {
        self.status = status;
        if status.is_complete() && self.completed_at.is_none() {
            self.completed_at = Some(Utc::now());
        }
        self
    }

    pub fn with_due_date(mut self, date: NaiveDate) -> Self {
        self.due_date = Some(date);
        self
    }

    pub fn with_project(mut self, project_id: ProjectId) -> Self {
        self.project_id = Some(project_id);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_parent(mut self, parent_id: TaskId) -> Self {
        self.parent_task_id = Some(parent_id);
        self
    }

    pub fn with_recurrence(mut self, recurrence: Option<Recurrence>) -> Self {
        self.recurrence = recurrence;
        self
    }

    pub fn with_project_opt(mut self, project_id: Option<super::ProjectId>) -> Self {
        self.project_id = project_id;
        self
    }

    pub fn with_description_opt(mut self, description: Option<String>) -> Self {
        self.description = description;
        self
    }

    pub fn is_overdue(&self) -> bool {
        if let Some(due) = self.due_date {
            return due < Utc::now().date_naive() && !self.status.is_complete();
        }
        false
    }

    pub fn is_due_today(&self) -> bool {
        if let Some(due) = self.due_date {
            return due == Utc::now().date_naive();
        }
        false
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
}
