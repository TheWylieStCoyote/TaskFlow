//! Task completion status.

use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_default() {
        assert_eq!(TaskStatus::default(), TaskStatus::Todo);
    }

    #[test]
    fn test_status_as_str() {
        assert_eq!(TaskStatus::Todo.as_str(), "todo");
        assert_eq!(TaskStatus::InProgress.as_str(), "in_progress");
        assert_eq!(TaskStatus::Blocked.as_str(), "blocked");
        assert_eq!(TaskStatus::Done.as_str(), "done");
        assert_eq!(TaskStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn test_status_symbol() {
        assert_eq!(TaskStatus::Todo.symbol(), "[ ]");
        assert_eq!(TaskStatus::InProgress.symbol(), "[~]");
        assert_eq!(TaskStatus::Blocked.symbol(), "[!]");
        assert_eq!(TaskStatus::Done.symbol(), "[x]");
        assert_eq!(TaskStatus::Cancelled.symbol(), "[-]");
    }

    #[test]
    fn test_is_complete_done() {
        assert!(TaskStatus::Done.is_complete());
    }

    #[test]
    fn test_is_complete_cancelled() {
        assert!(TaskStatus::Cancelled.is_complete());
    }

    #[test]
    fn test_is_complete_not_complete() {
        assert!(!TaskStatus::Todo.is_complete());
        assert!(!TaskStatus::InProgress.is_complete());
        assert!(!TaskStatus::Blocked.is_complete());
    }

    #[test]
    fn test_status_serialization() {
        for status in [
            TaskStatus::Todo,
            TaskStatus::InProgress,
            TaskStatus::Blocked,
            TaskStatus::Done,
            TaskStatus::Cancelled,
        ] {
            let json = serde_json::to_string(&status).expect("serialize");
            let restored: TaskStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(status, restored);
        }
    }

    #[test]
    fn test_status_equality() {
        assert_eq!(TaskStatus::Todo, TaskStatus::Todo);
        assert_ne!(TaskStatus::Todo, TaskStatus::Done);
        assert_ne!(TaskStatus::InProgress, TaskStatus::Blocked);
    }
}
