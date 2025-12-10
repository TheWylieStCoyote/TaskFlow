//! Task operation messages.

use crate::domain::{Priority, ProjectId, TaskId, TaskStatus};

/// Task operation messages.
///
/// These messages handle creating, modifying, and deleting tasks.
///
/// # Examples
///
/// ```
/// use taskflow::app::{Model, Message, TaskMessage, update};
/// use taskflow::domain::{Priority, TaskStatus, TaskId};
///
/// let mut model = Model::new();
///
/// // Create a new task
/// update(&mut model, TaskMessage::Create("Buy groceries".to_string()).into());
///
/// // Toggle completion of selected task
/// update(&mut model, TaskMessage::ToggleComplete.into());
///
/// // Cycle through priorities
/// update(&mut model, TaskMessage::CyclePriority.into());
/// ```
#[derive(Debug, Clone)]
pub enum TaskMessage {
    /// Toggle completion status of selected task
    ToggleComplete,
    /// Set specific status for a task
    SetStatus(TaskId, TaskStatus),
    /// Set specific priority for a task
    SetPriority(TaskId, Priority),
    /// Cycle through priority levels (None → Low → Medium → High → Urgent)
    CyclePriority,
    /// Create a new task with given title
    Create(String),
    /// Delete a task by ID
    Delete(TaskId),
    /// Move task to a project (or remove from project with None)
    MoveToProject(TaskId, Option<ProjectId>),
    /// Duplicate the selected task with "Copy of" prefix
    Duplicate,
}
