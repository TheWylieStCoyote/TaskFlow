//! Request/Response types for the pipe interface.

use serde::{Deserialize, Serialize};

/// Supported output formats for the pipe interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Json,
    Yaml,
    Csv,
}

impl OutputFormat {
    /// Parse a format string into an `OutputFormat`.
    #[allow(dead_code)]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "yaml" | "yml" => Some(Self::Yaml),
            "csv" => Some(Self::Csv),
            _ => None,
        }
    }
}

/// Entity types supported by the pipe interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Task,
    Project,
    TimeEntry,
    WorkLog,
    Habit,
    Goal,
    KeyResult,
    Tag,
    SavedFilter,
}

/// Operations supported by the pipe interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    List,
    Get,
    Create,
    Update,
    Delete,
    Export,
    Import,
}

/// Incoming request from stdin.
#[derive(Debug, Clone, Deserialize)]
pub struct PipeRequest {
    /// The operation to perform.
    pub operation: Operation,
    /// The entity type to operate on.
    pub entity: EntityType,
    /// The ID of the entity (for get/update/delete).
    #[serde(default)]
    pub id: Option<String>,
    /// Data payload (for create/update/import).
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    /// Filter parameters (for list operations).
    #[serde(default)]
    pub filters: Option<FilterParams>,
}

/// Filter parameters for list operations.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FilterParams {
    /// Filter by project ID.
    pub project_id: Option<String>,
    /// Filter by tags.
    pub tags: Option<Vec<String>>,
    /// Tag matching mode: "any" or "all" (default: "all").
    pub tags_mode: Option<String>,
    /// Filter by status.
    pub status: Option<Vec<String>>,
    /// Filter by priority.
    pub priority: Option<Vec<String>>,
    /// Search text in title and tags.
    pub search: Option<String>,
    /// Only show tasks due before this date (YYYY-MM-DD).
    pub due_before: Option<String>,
    /// Only show tasks due after this date (YYYY-MM-DD).
    pub due_after: Option<String>,
    /// Include completed tasks.
    pub include_completed: Option<bool>,
    /// Maximum number of results.
    pub limit: Option<usize>,
    /// Offset for pagination.
    pub offset: Option<usize>,
    /// Field to sort by.
    pub sort_by: Option<String>,
    /// Sort order: "asc" or "desc".
    pub sort_order: Option<String>,
}

/// Response written to stdout.
#[derive(Debug, Clone, Serialize)]
pub struct PipeResponse<T: Serialize> {
    /// Whether the operation succeeded.
    pub success: bool,
    /// The response data (if successful).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Error information (if failed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<PipeError>,
    /// Response metadata (pagination info, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResponseMetadata>,
}

/// Error information for failed operations.
#[derive(Debug, Clone, Serialize)]
pub struct PipeError {
    /// Error code (e.g., "NOT_FOUND", "INVALID_DATA").
    pub code: String,
    /// Human-readable error message.
    pub message: String,
    /// Additional error details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Response metadata for list operations.
#[derive(Debug, Clone, Serialize)]
pub struct ResponseMetadata {
    /// Total number of items (before pagination).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
    /// Current offset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// Number of items returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

impl<T: Serialize> PipeResponse<T> {
    /// Create a successful response with data.
    #[allow(dead_code)]
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            metadata: None,
        }
    }

    /// Create a successful response with data and metadata.
    #[allow(dead_code)]
    pub fn success_with_metadata(data: T, metadata: ResponseMetadata) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            metadata: Some(metadata),
        }
    }
}

impl PipeResponse<()> {
    /// Create an error response.
    #[allow(dead_code)]
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> PipeResponse<()> {
        PipeResponse {
            success: false,
            data: None,
            error: Some(PipeError {
                code: code.into(),
                message: message.into(),
                details: None,
            }),
            metadata: None,
        }
    }

    /// Create an error response with details.
    #[allow(dead_code)]
    pub fn error_with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> PipeResponse<()> {
        PipeResponse {
            success: false,
            data: None,
            error: Some(PipeError {
                code: code.into(),
                message: message.into(),
                details: Some(details),
            }),
            metadata: None,
        }
    }
}

impl PipeError {
    /// Create a new error.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Create a serialization error.
    pub fn serialization(err: impl std::fmt::Display) -> Self {
        Self {
            code: "SERIALIZATION_ERROR".to_string(),
            message: format!("Failed to serialize response: {err}"),
            details: None,
        }
    }
}

impl std::fmt::Display for PipeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for PipeError {}

// ============================================================================
// Input types for create/update operations
// ============================================================================

/// Input for creating/updating a task.
#[derive(Debug, Clone, Deserialize)]
pub struct TaskInput {
    /// Task title (required for create).
    pub title: Option<String>,
    /// Task description.
    #[serde(default)]
    pub description: Option<String>,
    /// Task status: "todo", "in_progress", "blocked", "done", "cancelled".
    #[serde(default)]
    pub status: Option<String>,
    /// Task priority: "none", "low", "medium", "high", "urgent".
    #[serde(default)]
    pub priority: Option<String>,
    /// Project ID to assign to.
    #[serde(default)]
    pub project_id: Option<String>,
    /// Tags to assign.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Due date (YYYY-MM-DD).
    #[serde(default)]
    pub due_date: Option<String>,
    /// Scheduled date (YYYY-MM-DD).
    #[serde(default)]
    pub scheduled_date: Option<String>,
    /// Estimated time in minutes.
    #[serde(default)]
    pub estimated_minutes: Option<u32>,
    /// Task dependencies (task IDs).
    #[serde(default)]
    pub dependencies: Option<Vec<String>>,
}

/// Input for creating/updating a project.
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectInput {
    /// Project name (required for create).
    pub name: Option<String>,
    /// Project description.
    #[serde(default)]
    pub description: Option<String>,
    /// Project status: "active", "on_hold", "completed", "archived".
    #[serde(default)]
    pub status: Option<String>,
    /// Parent project ID.
    #[serde(default)]
    pub parent_id: Option<String>,
    /// Project color (hex code).
    #[serde(default)]
    pub color: Option<String>,
}

/// Input for creating/updating a time entry.
#[derive(Debug, Clone, Deserialize)]
pub struct TimeEntryInput {
    /// Task ID (required for create).
    pub task_id: Option<String>,
    /// Start time (ISO 8601).
    #[serde(default)]
    pub started_at: Option<String>,
    /// End time (ISO 8601).
    #[serde(default)]
    pub ended_at: Option<String>,
    /// Duration in minutes (alternative to ended_at).
    #[serde(default)]
    pub duration_minutes: Option<u32>,
    /// Description of work done.
    #[serde(default)]
    pub description: Option<String>,
}

/// Input for creating/updating a habit.
#[derive(Debug, Clone, Deserialize)]
pub struct HabitInput {
    /// Habit name (required for create).
    pub name: Option<String>,
    /// Habit description.
    #[serde(default)]
    pub description: Option<String>,
    /// Frequency: "daily", "weekly", "monthly".
    #[serde(default)]
    pub frequency: Option<String>,
    /// Target count per period.
    #[serde(default)]
    pub target_count: Option<u32>,
}

/// Input for creating/updating a goal.
#[derive(Debug, Clone, Deserialize)]
pub struct GoalInput {
    /// Goal name (required for create).
    pub name: Option<String>,
    /// Goal description.
    #[serde(default)]
    pub description: Option<String>,
    /// Target date (YYYY-MM-DD).
    #[serde(default)]
    pub target_date: Option<String>,
}

/// Input for creating/updating a key result.
#[derive(Debug, Clone, Deserialize)]
pub struct KeyResultInput {
    /// Goal ID (required for create).
    pub goal_id: Option<String>,
    /// Key result name (required for create).
    pub name: Option<String>,
    /// Target value.
    #[serde(default)]
    pub target_value: Option<f64>,
    /// Current value.
    #[serde(default)]
    pub current_value: Option<f64>,
    /// Unit of measurement.
    #[serde(default)]
    pub unit: Option<String>,
}
