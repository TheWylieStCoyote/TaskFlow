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
