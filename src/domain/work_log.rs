//! Work log entries for tasks.
//!
//! Work log entries record notes, updates, and progress information
//! for individual tasks over time, similar to commit history.
//!
//! # Usage Pattern
//!
//! Work logs are designed for journaling progress on tasks. Unlike [`TimeEntry`]
//! which tracks time spent, work logs capture *what* was done in text form.
//!
//! ```
//! use taskflow::domain::{Task, WorkLogEntry};
//!
//! // Create a task and log work on it
//! let task = Task::new("Implement search feature");
//!
//! // Log initial research
//! let entry1 = WorkLogEntry::new(task.id, "Researched full-text search options:\n- Tantivy (Rust native)\n- MeiliSearch\n- SQLite FTS5");
//!
//! // Log implementation progress
//! let entry2 = WorkLogEntry::new(task.id, "Implemented basic search with SQLite FTS5.\nStill need to add highlighting.");
//!
//! // Get a quick summary for list views
//! assert_eq!(entry1.summary(), "Researched full-text search options:");
//!
//! // Show relative time in UI
//! println!("Logged {}", entry1.relative_time()); // "just now", "2 hours ago", etc.
//! ```
//!
//! # Querying Work Logs
//!
//! Work logs are typically queried by task ID to show a task's history:
//!
//! ```ignore
//! // In the app model, work logs are stored in a HashMap:
//! let logs_for_task: Vec<_> = model.work_logs.values()
//!     .filter(|log| log.task_id == task_id)
//!     .collect();
//! ```
//!
//! [`TimeEntry`]: crate::domain::TimeEntry

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::TaskId;

/// Unique identifier for work log entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkLogEntryId(pub Uuid);

impl WorkLogEntryId {
    /// Creates a new unique work log entry identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for WorkLogEntryId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkLogEntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A work log entry associated with a task.
///
/// Work log entries record notes, updates, and progress information
/// for a specific task. They form a chronological journal of work
/// done on the task, similar to git commits.
///
/// # Examples
///
/// ## Creating a Work Log Entry
///
/// ```
/// use taskflow::domain::{Task, WorkLogEntry};
///
/// let task = Task::new("Implement login feature");
///
/// // Create a new entry
/// let entry = WorkLogEntry::new(
///     task.id,
///     "Started implementing OAuth2 flow.\nIntegrated with Google provider."
/// );
///
/// assert_eq!(entry.task_id, task.id);
/// assert!(entry.content.contains("OAuth2"));
/// ```
///
/// ## Multi-line Content
///
/// ```
/// use taskflow::domain::{Task, WorkLogEntry};
///
/// let task = Task::new("Debug performance issue");
/// let entry = WorkLogEntry::new(
///     task.id,
///     "Investigation findings:\n- Memory leak in cache\n- Fixed by clearing stale entries"
/// );
///
/// assert_eq!(entry.content.lines().count(), 3);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkLogEntry {
    pub id: WorkLogEntryId,
    pub task_id: TaskId,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WorkLogEntry {
    /// Creates a new work log entry for the given task.
    #[must_use]
    pub fn new(task_id: TaskId, content: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: WorkLogEntryId::new(),
            task_id,
            content: content.into(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Updates the content and refreshes the updated_at timestamp.
    pub fn update_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
        self.updated_at = Utc::now();
    }

    /// Returns the first line of content as a summary.
    #[must_use]
    pub fn summary(&self) -> &str {
        self.content.lines().next().unwrap_or(&self.content)
    }

    /// Returns the number of lines in the content.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.content.lines().count().max(1)
    }

    /// Returns a formatted timestamp for display.
    #[must_use]
    pub fn formatted_timestamp(&self) -> String {
        self.created_at.format("%Y-%m-%d %H:%M").to_string()
    }

    /// Returns a relative time description (e.g., "2 hours ago").
    #[must_use]
    pub fn relative_time(&self) -> String {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.created_at);

        if duration.num_days() > 30 {
            format!("{} months ago", duration.num_days() / 30)
        } else if duration.num_days() > 0 {
            let days = duration.num_days();
            if days == 1 {
                "yesterday".to_string()
            } else {
                format!("{days} days ago")
            }
        } else if duration.num_hours() > 0 {
            let hours = duration.num_hours();
            if hours == 1 {
                "1 hour ago".to_string()
            } else {
                format!("{hours} hours ago")
            }
        } else if duration.num_minutes() > 0 {
            let mins = duration.num_minutes();
            if mins == 1 {
                "1 minute ago".to_string()
            } else {
                format!("{mins} minutes ago")
            }
        } else {
            "just now".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_log_entry_new() {
        let task_id = TaskId::new();
        let entry = WorkLogEntry::new(task_id, "Test content");

        assert_eq!(entry.task_id, task_id);
        assert_eq!(entry.content, "Test content");
        assert!(entry.created_at <= Utc::now());
        assert_eq!(entry.created_at, entry.updated_at);
    }

    #[test]
    fn test_work_log_entry_update_content() {
        let task_id = TaskId::new();
        let mut entry = WorkLogEntry::new(task_id, "Original content");
        let original_created = entry.created_at;

        entry.update_content("Updated content");

        assert_eq!(entry.content, "Updated content");
        // created_at should be unchanged
        assert_eq!(entry.created_at, original_created);
        // updated_at should be >= created_at (may be equal on fast systems)
        assert!(entry.updated_at >= entry.created_at);
    }

    #[test]
    fn test_work_log_entry_summary() {
        let task_id = TaskId::new();
        let entry = WorkLogEntry::new(task_id, "First line\nSecond line\nThird line");

        assert_eq!(entry.summary(), "First line");
    }

    #[test]
    fn test_work_log_entry_summary_single_line() {
        let task_id = TaskId::new();
        let entry = WorkLogEntry::new(task_id, "Only one line");

        assert_eq!(entry.summary(), "Only one line");
    }

    #[test]
    fn test_work_log_entry_line_count() {
        let task_id = TaskId::new();

        let single = WorkLogEntry::new(task_id, "One line");
        assert_eq!(single.line_count(), 1);

        let multi = WorkLogEntry::new(task_id, "Line 1\nLine 2\nLine 3");
        assert_eq!(multi.line_count(), 3);
    }

    #[test]
    fn test_work_log_entry_formatted_timestamp() {
        let task_id = TaskId::new();
        let entry = WorkLogEntry::new(task_id, "Test");

        let timestamp = entry.formatted_timestamp();
        // Should be in YYYY-MM-DD HH:MM format
        assert!(timestamp.len() == 16);
        assert!(timestamp.contains('-'));
        assert!(timestamp.contains(':'));
    }

    #[test]
    fn test_work_log_entry_relative_time_just_now() {
        let task_id = TaskId::new();
        let entry = WorkLogEntry::new(task_id, "Test");

        assert_eq!(entry.relative_time(), "just now");
    }

    #[test]
    fn test_work_log_entry_id_uniqueness() {
        let id1 = WorkLogEntryId::new();
        let id2 = WorkLogEntryId::new();

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_work_log_entry_serialization() {
        let task_id = TaskId::new();
        let entry = WorkLogEntry::new(task_id, "Test content\nWith multiple lines");

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: WorkLogEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(entry.id, deserialized.id);
        assert_eq!(entry.task_id, deserialized.task_id);
        assert_eq!(entry.content, deserialized.content);
    }
}
