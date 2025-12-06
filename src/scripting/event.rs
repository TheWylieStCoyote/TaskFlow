//! Hook events that can trigger script execution.

use crate::domain::{PomodoroPhase, Priority, Task, TaskStatus};

/// Events that can trigger hook execution.
#[derive(Debug, Clone)]
pub enum HookEvent {
    /// A new task was created.
    TaskCreated {
        /// The newly created task.
        task: Task,
    },

    /// A task was marked as completed.
    TaskCompleted {
        /// The completed task.
        task: Task,
    },

    /// A task's status changed.
    TaskStatusChanged {
        /// The task after the change.
        task: Task,
        /// The previous status.
        old_status: TaskStatus,
        /// The new status.
        new_status: TaskStatus,
    },

    /// A task's priority changed.
    TaskPriorityChanged {
        /// The task after the change.
        task: Task,
        /// The previous priority.
        old_priority: Priority,
        /// The new priority.
        new_priority: Priority,
    },

    /// Time tracking was started for a task.
    TimeTrackingStarted {
        /// The task being tracked.
        task: Task,
    },

    /// Time tracking was stopped for a task.
    TimeTrackingStopped {
        /// The task that was being tracked.
        task: Task,
        /// Duration in minutes.
        duration_mins: u32,
    },

    /// A Pomodoro phase was completed.
    PomodoroPhaseCompleted {
        /// The completed phase.
        phase: PomodoroPhase,
        /// The task being worked on.
        task: Task,
    },

    /// A task was deleted.
    TaskDeleted {
        /// The deleted task.
        task: Task,
    },

    /// A tag was added to a task.
    TagAdded {
        /// The task.
        task: Task,
        /// The tag that was added.
        tag: String,
    },

    /// A tag was removed from a task.
    TagRemoved {
        /// The task.
        task: Task,
        /// The tag that was removed.
        tag: String,
    },
}

impl HookEvent {
    /// Returns the hook name for this event type.
    #[must_use]
    pub fn hook_name(&self) -> &'static str {
        match self {
            Self::TaskCreated { .. } => "on_task_created",
            Self::TaskCompleted { .. } => "on_task_completed",
            Self::TaskStatusChanged { .. } => "on_task_status_changed",
            Self::TaskPriorityChanged { .. } => "on_task_priority_changed",
            Self::TimeTrackingStarted { .. } => "on_time_tracking_started",
            Self::TimeTrackingStopped { .. } => "on_time_tracking_stopped",
            Self::PomodoroPhaseCompleted { .. } => "on_pomodoro_phase_completed",
            Self::TaskDeleted { .. } => "on_task_deleted",
            Self::TagAdded { .. } => "on_tag_added",
            Self::TagRemoved { .. } => "on_tag_removed",
        }
    }

    /// Returns the task involved in this event, if any.
    #[must_use]
    pub fn task(&self) -> &Task {
        match self {
            Self::TaskCreated { task }
            | Self::TaskCompleted { task }
            | Self::TaskStatusChanged { task, .. }
            | Self::TaskPriorityChanged { task, .. }
            | Self::TimeTrackingStarted { task }
            | Self::TimeTrackingStopped { task, .. }
            | Self::PomodoroPhaseCompleted { task, .. }
            | Self::TaskDeleted { task }
            | Self::TagAdded { task, .. }
            | Self::TagRemoved { task, .. } => task,
        }
    }
}
