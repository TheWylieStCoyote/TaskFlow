//! Time tracking entries for tasks.
//!
//! Time entries record how much time is spent on individual tasks.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::TaskId;

/// Unique identifier for time entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimeEntryId(pub Uuid);

impl TimeEntryId {
    /// Creates a new unique time entry identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TimeEntryId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TimeEntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A time tracking entry associated with a task.
///
/// Time entries record the duration spent working on a specific task.
/// They can be started and stopped, or have a duration manually set.
///
/// # Examples
///
/// ## Basic Time Tracking
///
/// ```
/// use taskflow::domain::{Task, TimeEntry};
///
/// let task = Task::new("Write documentation");
///
/// // Start tracking
/// let mut entry = TimeEntry::start(task.id);
/// assert!(entry.is_running());
///
/// // Do some work...
///
/// // Stop tracking
/// entry.stop();
/// assert!(!entry.is_running());
///
/// // Get the duration
/// println!("Time spent: {}", entry.formatted_duration());
/// ```
///
/// ## Manual Duration
///
/// ```
/// use taskflow::domain::{Task, TimeEntry};
///
/// let task = Task::new("Meeting");
/// let mut entry = TimeEntry::start(task.id);
///
/// // Set duration manually (e.g., for a 30-minute meeting)
/// entry.duration_minutes = Some(30);
/// assert_eq!(entry.calculated_duration_minutes(), 30);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: TimeEntryId,
    pub task_id: TaskId,
    pub description: Option<String>,

    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,

    /// Duration in minutes (calculated or manual)
    pub duration_minutes: Option<u32>,
}

impl TimeEntry {
    #[must_use]
    pub fn start(task_id: TaskId) -> Self {
        Self {
            id: TimeEntryId::new(),
            task_id,
            description: None,
            started_at: Utc::now(),
            ended_at: None,
            duration_minutes: None,
        }
    }

    pub fn stop(&mut self) {
        let end = Utc::now();
        self.ended_at = Some(end);
        self.duration_minutes = Some((end - self.started_at).num_minutes().max(0) as u32);
    }

    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.ended_at.is_none()
    }

    pub fn calculated_duration_minutes(&self) -> u32 {
        if let Some(duration) = self.duration_minutes {
            duration
        } else {
            let end = self.ended_at.unwrap_or_else(Utc::now);
            (end - self.started_at).num_minutes().max(0) as u32
        }
    }

    #[must_use]
    pub fn formatted_duration(&self) -> String {
        let minutes = self.calculated_duration_minutes();
        let hours = minutes / 60;
        let mins = minutes % 60;
        if hours > 0 {
            format!("{hours}h {mins}m")
        } else {
            format!("{mins}m")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_entry_start() {
        let task_id = TaskId::new();
        let entry = TimeEntry::start(task_id);

        assert_eq!(entry.task_id, task_id);
        assert!(entry.ended_at.is_none());
        assert!(entry.duration_minutes.is_none());
        assert!(entry.is_running());
    }

    #[test]
    fn test_time_entry_stop() {
        let task_id = TaskId::new();
        let mut entry = TimeEntry::start(task_id);

        entry.stop();

        assert!(entry.ended_at.is_some());
        assert!(entry.duration_minutes.is_some());
        assert!(!entry.is_running());
    }

    #[test]
    fn test_time_entry_is_running() {
        let task_id = TaskId::new();
        let mut entry = TimeEntry::start(task_id);

        assert!(entry.is_running());

        entry.stop();

        assert!(!entry.is_running());
    }

    #[test]
    fn test_time_entry_calculated_duration() {
        let task_id = TaskId::new();
        let mut entry = TimeEntry::start(task_id);

        // Set explicit duration
        entry.duration_minutes = Some(45);
        assert_eq!(entry.calculated_duration_minutes(), 45);

        // When duration_minutes is None but ended_at is set, it calculates
        entry.duration_minutes = None;
        entry.ended_at = Some(entry.started_at + chrono::Duration::minutes(30));
        assert_eq!(entry.calculated_duration_minutes(), 30);
    }

    #[test]
    fn test_time_entry_formatted_duration_hours() {
        let task_id = TaskId::new();
        let mut entry = TimeEntry::start(task_id);
        entry.duration_minutes = Some(90); // 1h 30m

        assert_eq!(entry.formatted_duration(), "1h 30m");
    }

    #[test]
    fn test_time_entry_formatted_duration_minutes_only() {
        let task_id = TaskId::new();
        let mut entry = TimeEntry::start(task_id);
        entry.duration_minutes = Some(45);

        assert_eq!(entry.formatted_duration(), "45m");
    }

    #[test]
    fn test_time_entry_duration_never_negative() {
        let task_id = TaskId::new();
        let mut entry = TimeEntry::start(task_id);

        // Set ended_at before started_at (edge case)
        entry.ended_at = Some(entry.started_at - chrono::Duration::minutes(10));
        entry.duration_minutes = None;

        // Should return 0, not negative
        assert_eq!(entry.calculated_duration_minutes(), 0);
    }
}
