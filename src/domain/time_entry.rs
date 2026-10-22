//! Time tracking entries for tasks.
//!
//! Time entries record how much time is spent on individual tasks.

use std::cmp::{Ordering, Reverse};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{LocalDate, TaskId};

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

/// The order time entries are listed in: days newest first, and within a
/// day entries in chronological order.
///
/// Days are local calendar dates of `started_at`. Equal start times fall back
/// to the entry id so the order is deterministic. The model's per-task entry
/// list and the time log editor's day grouping both sort with this, so an
/// index into one is valid in the other.
///
/// ```
/// use taskflow::domain::{time_entry_display_order, Task, TimeEntry};
/// use chrono::Duration;
///
/// let task = Task::new("Ordering");
/// let mut later = TimeEntry::start(task.id);
/// let mut earlier = later.clone();
/// earlier.started_at -= Duration::minutes(5);
/// // Same day: the earlier entry comes first.
/// assert!(time_entry_display_order(&earlier, &later).is_lt());
/// later.started_at -= Duration::days(1);
/// // A newer day comes first regardless of the clock time.
/// assert!(time_entry_display_order(&earlier, &later).is_lt());
/// ```
#[must_use]
pub fn time_entry_display_order(a: &TimeEntry, b: &TimeEntry) -> Ordering {
    let key = |e: &TimeEntry| (Reverse(e.started_at.local_date()), e.started_at, e.id.0);
    key(a).cmp(&key(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Local, TimeZone};

    /// A stopped entry starting at `h:m` local on today minus `days_ago`.
    fn entry_on(task_id: TaskId, days_ago: i64, h: u32, m: u32) -> TimeEntry {
        let date = crate::domain::today() - Duration::days(days_ago);
        let started_at = Local
            .from_local_datetime(&date.and_hms_opt(h, m, 0).unwrap())
            .single()
            .unwrap()
            .with_timezone(&Utc);
        let mut e = TimeEntry::start(task_id);
        e.started_at = started_at;
        e.ended_at = Some(started_at + Duration::minutes(30));
        e.duration_minutes = Some(30);
        e
    }

    #[test]
    fn display_order_is_days_newest_first_then_chronological() {
        let task_id = TaskId::new();
        let today_early = entry_on(task_id, 0, 9, 0);
        let today_late = entry_on(task_id, 0, 14, 0);
        let yesterday_late = entry_on(task_id, 1, 23, 30);
        let yesterday_early = entry_on(task_id, 1, 8, 0);

        let mut entries = [&yesterday_late, &today_late, &yesterday_early, &today_early];
        entries.sort_by(|a, b| time_entry_display_order(a, b));
        let ids: Vec<_> = entries.iter().map(|e| e.id).collect();
        assert_eq!(
            ids,
            vec![
                today_early.id,
                today_late.id,
                yesterday_early.id,
                yesterday_late.id
            ]
        );
    }

    #[test]
    fn display_order_breaks_equal_starts_by_id() {
        let task_id = TaskId::new();
        let a = entry_on(task_id, 0, 9, 0);
        let mut b = a.clone();
        b.id = TimeEntryId::new();
        assert_eq!(time_entry_display_order(&a, &a), Ordering::Equal);
        assert_eq!(
            time_entry_display_order(&a, &b),
            a.id.0.cmp(&b.id.0),
            "ties fall back to the id"
        );
        assert_eq!(
            time_entry_display_order(&a, &b),
            time_entry_display_order(&b, &a).reverse()
        );
    }

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

    #[test]
    fn test_time_entry_formatted_zero_minutes() {
        let task_id = TaskId::new();
        let mut entry = TimeEntry::start(task_id);
        entry.duration_minutes = Some(0);

        assert_eq!(entry.formatted_duration(), "0m");
    }

    #[test]
    fn test_time_entry_formatted_exact_hour() {
        let task_id = TaskId::new();
        let mut entry = TimeEntry::start(task_id);
        entry.duration_minutes = Some(60); // Exactly 1 hour

        assert_eq!(entry.formatted_duration(), "1h 0m");
    }

    #[test]
    fn test_time_entry_formatted_24_hours() {
        let task_id = TaskId::new();
        let mut entry = TimeEntry::start(task_id);
        entry.duration_minutes = Some(24 * 60); // 24 hours

        assert_eq!(entry.formatted_duration(), "24h 0m");
    }

    #[test]
    fn test_time_entry_manual_override_priority() {
        let task_id = TaskId::new();
        let mut entry = TimeEntry::start(task_id);

        // Set both ended_at (which would calculate 30 mins) and manual duration
        entry.ended_at = Some(entry.started_at + chrono::Duration::minutes(30));
        entry.duration_minutes = Some(45); // Manual override

        // Manual duration should take priority
        assert_eq!(entry.calculated_duration_minutes(), 45);
    }
}
