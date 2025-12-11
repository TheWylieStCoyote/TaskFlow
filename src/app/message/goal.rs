//! Goal/OKR tracking messages.

use chrono::NaiveDate;

use crate::domain::{GoalId, GoalStatus, KeyResultId, KeyResultStatus, ProjectId, Quarter, TaskId};

/// Goal/OKR tracking messages.
///
/// These messages handle creating, modifying, and tracking goals and key results.
#[derive(Debug, Clone)]
pub enum GoalMessage {
    // ==================== Goal CRUD ====================
    /// Create a new goal with the given name
    Create(String),
    /// Update a goal's name
    UpdateName {
        /// The goal to update
        id: GoalId,
        /// The new name
        name: String,
    },
    /// Update a goal's description
    UpdateDescription {
        /// The goal to update
        id: GoalId,
        /// The new description (None to clear)
        description: Option<String>,
    },
    /// Set a goal's status
    SetStatus {
        /// The goal to update
        id: GoalId,
        /// The new status
        status: GoalStatus,
    },
    /// Set custom dates for a goal
    SetDates {
        /// The goal to update
        id: GoalId,
        /// Start date (None to clear)
        start: Option<NaiveDate>,
        /// End/due date (None to clear)
        end: Option<NaiveDate>,
    },
    /// Set the quarter for a goal (OKR-style)
    SetQuarter {
        /// The goal to update
        id: GoalId,
        /// The quarter (None to clear)
        quarter: Option<(i32, Quarter)>,
    },
    /// Set manual progress override for a goal
    SetManualProgress {
        /// The goal to update
        id: GoalId,
        /// Progress 0-100 (None to use auto-calculation)
        progress: Option<u8>,
    },
    /// Delete a goal and its key results
    Delete(GoalId),

    // ==================== Key Result CRUD ====================
    /// Create a new key result for a goal
    CreateKeyResult {
        /// Parent goal
        goal_id: GoalId,
        /// Key result name
        name: String,
    },
    /// Update a key result's name
    UpdateKeyResultName {
        /// The key result to update
        id: KeyResultId,
        /// The new name
        name: String,
    },
    /// Set key result status
    SetKeyResultStatus {
        /// The key result to update
        id: KeyResultId,
        /// The new status
        status: KeyResultStatus,
    },
    /// Set target value for a key result
    SetKeyResultTarget {
        /// The key result to update
        id: KeyResultId,
        /// Target value to achieve
        target: f64,
        /// Optional unit (e.g., "users", "%", "$")
        unit: Option<String>,
    },
    /// Set current value for a key result
    SetKeyResultValue {
        /// The key result to update
        id: KeyResultId,
        /// Current progress value
        value: f64,
    },
    /// Set manual progress override for a key result
    SetKeyResultManualProgress {
        /// The key result to update
        id: KeyResultId,
        /// Progress 0-100 (None to use auto-calculation)
        progress: Option<u8>,
    },
    /// Link a project to a key result
    LinkProject {
        /// The key result
        kr_id: KeyResultId,
        /// The project to link
        project_id: ProjectId,
    },
    /// Unlink a project from a key result
    UnlinkProject {
        /// The key result
        kr_id: KeyResultId,
        /// The project to unlink
        project_id: ProjectId,
    },
    /// Link a task to a key result
    LinkTask {
        /// The key result
        kr_id: KeyResultId,
        /// The task to link
        task_id: TaskId,
    },
    /// Unlink a task from a key result
    UnlinkTask {
        /// The key result
        kr_id: KeyResultId,
        /// The task to unlink
        task_id: TaskId,
    },
    /// Delete a key result
    DeleteKeyResult(KeyResultId),

    // ==================== View Navigation ====================
    /// Expand a goal to show its key results
    ExpandGoal(GoalId),
    /// Collapse the expanded goal
    CollapseGoal,
    /// Toggle showing archived/completed goals
    ToggleArchived,
    /// Filter by a specific quarter
    FilterByQuarter(Option<(i32, Quarter)>),
    /// Navigate up in the goal list
    NavigateUp,
    /// Navigate down in the goal list
    NavigateDown,
    /// Navigate into key results of selected goal
    NavigateInto,
    /// Navigate back to goals from key results
    NavigateBack,
}
