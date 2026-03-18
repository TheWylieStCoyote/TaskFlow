//! Audit log for task mutations.
//!
//! Records every create, modify, complete, and delete event on tasks with
//! before/after field values.  Surfaced in the task detail panel as a "History" tab.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::TaskId;

/// Unique identifier for audit log entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuditLogEntryId(pub Uuid);

impl AuditLogEntryId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AuditLogEntryId {
    fn default() -> Self {
        Self::new()
    }
}

/// What kind of mutation happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditAction {
    /// Task was created.
    Created,
    /// One or more fields on the task were modified.
    Modified,
    /// Task was marked complete.
    Completed,
    /// Task was un-marked complete.
    Uncompleted,
    /// Task was deleted.
    Deleted,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Modified => write!(f, "modified"),
            Self::Completed => write!(f, "completed"),
            Self::Uncompleted => write!(f, "uncompleted"),
            Self::Deleted => write!(f, "deleted"),
        }
    }
}

/// A single field-level change captured in an audit entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    /// Name of the changed field (e.g., `"title"`, `"priority"`, `"due_date"`).
    pub field: String,
    /// Value before the change, as a display string.
    pub old_value: String,
    /// Value after the change, as a display string.
    pub new_value: String,
}

/// One record in the task audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: AuditLogEntryId,
    /// The task this entry refers to.
    pub task_id: TaskId,
    /// When the mutation happened.
    pub changed_at: DateTime<Utc>,
    /// What type of mutation.
    pub action: AuditAction,
    /// Field-level diffs (empty for `Created` / `Deleted`).
    #[serde(default)]
    pub changes: Vec<FieldChange>,
}

impl AuditLogEntry {
    /// Create a new entry.
    #[must_use]
    pub fn new(task_id: TaskId, action: AuditAction, changes: Vec<FieldChange>) -> Self {
        Self {
            id: AuditLogEntryId::new(),
            task_id,
            changed_at: Utc::now(),
            action,
            changes,
        }
    }

    /// Human-readable relative time, e.g. "2 hours ago".
    #[must_use]
    pub fn relative_time(&self) -> String {
        let now = Utc::now();
        let secs = (now - self.changed_at).num_seconds().max(0);
        match secs {
            0..=59 => "just now".to_string(),
            60..=3599 => format!("{} min ago", secs / 60),
            3600..=86399 => format!("{} hr ago", secs / 3600),
            _ => format!("{} days ago", secs / 86400),
        }
    }

    /// Formatted timestamp for display.
    #[must_use]
    pub fn formatted_timestamp(&self) -> String {
        self.changed_at.format("%Y-%m-%d %H:%M").to_string()
    }
}

// ============================================================================
// Field diff helper
// ============================================================================

use super::Task;

/// Compare two `Task` snapshots and return a list of changed fields.
#[must_use]
pub fn diff_tasks(before: &Task, after: &Task) -> Vec<FieldChange> {
    let mut changes = Vec::new();

    // Title
    if before.title != after.title {
        changes.push(FieldChange {
            field: "title".to_string(),
            old_value: before.title.clone(),
            new_value: after.title.clone(),
        });
    }

    // Status
    if before.status != after.status {
        changes.push(FieldChange {
            field: "status".to_string(),
            old_value: before.status.as_str().to_string(),
            new_value: after.status.as_str().to_string(),
        });
    }

    // Priority
    if before.priority != after.priority {
        changes.push(FieldChange {
            field: "priority".to_string(),
            old_value: format!("{:?}", before.priority),
            new_value: format!("{:?}", after.priority),
        });
    }

    // Due date
    if before.due_date != after.due_date {
        changes.push(FieldChange {
            field: "due_date".to_string(),
            old_value: before
                .due_date
                .map_or_else(|| "none".to_string(), |d| d.to_string()),
            new_value: after
                .due_date
                .map_or_else(|| "none".to_string(), |d| d.to_string()),
        });
    }

    // Tags
    if before.tags != after.tags {
        changes.push(FieldChange {
            field: "tags".to_string(),
            old_value: before.tags.join(", "),
            new_value: after.tags.join(", "),
        });
    }

    // Description
    if before.description != after.description {
        changes.push(FieldChange {
            field: "description".to_string(),
            old_value: before.description.clone().unwrap_or_default(),
            new_value: after.description.clone().unwrap_or_default(),
        });
    }

    // Project
    if before.project_id != after.project_id {
        changes.push(FieldChange {
            field: "project".to_string(),
            old_value: before
                .project_id
                .map_or_else(|| "none".to_string(), |id| id.0.to_string()),
            new_value: after
                .project_id
                .map_or_else(|| "none".to_string(), |id| id.0.to_string()),
        });
    }

    // Estimated time
    if before.estimated_minutes != after.estimated_minutes {
        changes.push(FieldChange {
            field: "estimate".to_string(),
            old_value: before
                .estimated_minutes
                .map_or_else(|| "none".to_string(), |m| format!("{m}m")),
            new_value: after
                .estimated_minutes
                .map_or_else(|| "none".to_string(), |m| format!("{m}m")),
        });
    }

    // Recurrence
    if before.recurrence != after.recurrence {
        changes.push(FieldChange {
            field: "recurrence".to_string(),
            old_value: before
                .recurrence
                .as_ref()
                .map_or_else(|| "none".to_string(), std::string::ToString::to_string),
            new_value: after
                .recurrence
                .as_ref()
                .map_or_else(|| "none".to_string(), std::string::ToString::to_string),
        });
    }

    changes
}
