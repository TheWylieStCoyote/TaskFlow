//! Actions that scripts can request.
//!
//! Scripts don't directly modify application state. Instead, they return
//! actions that the application processes through the normal message system.

use crate::domain::{Priority, TaskId, TaskStatus};

/// Actions that a script can request the application to perform.
#[derive(Debug, Clone)]
pub enum ScriptAction {
    /// Create a new task.
    CreateTask {
        /// Task title.
        title: String,
        /// Optional priority.
        priority: Option<Priority>,
        /// Optional due date (days from today, 0 = today, 1 = tomorrow).
        due_in_days: Option<i32>,
        /// Tags to add.
        tags: Vec<String>,
        /// Optional project name to assign.
        project_name: Option<String>,
    },

    /// Complete a task.
    CompleteTask {
        /// Task ID to complete.
        task_id: TaskId,
    },

    /// Set task status.
    SetTaskStatus {
        /// Task ID.
        task_id: TaskId,
        /// New status.
        status: TaskStatus,
    },

    /// Set task priority.
    SetTaskPriority {
        /// Task ID.
        task_id: TaskId,
        /// New priority.
        priority: Priority,
    },

    /// Add a tag to a task.
    AddTag {
        /// Task ID.
        task_id: TaskId,
        /// Tag to add.
        tag: String,
    },

    /// Remove a tag from a task.
    RemoveTag {
        /// Task ID.
        task_id: TaskId,
        /// Tag to remove.
        tag: String,
    },

    /// Start time tracking for a task.
    StartTracking {
        /// Task ID.
        task_id: TaskId,
    },

    /// Stop time tracking.
    StopTracking,

    /// Log a message (for debugging).
    Log {
        /// Message to log.
        message: String,
    },

    /// Show a notification to the user.
    Notify {
        /// Notification message.
        message: String,
    },
}
