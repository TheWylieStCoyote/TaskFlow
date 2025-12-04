use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::{Priority, ProjectId, TaskStatus};

/// Filter criteria for querying tasks
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Filter {
    pub status: Option<Vec<TaskStatus>>,
    pub priority: Option<Vec<Priority>>,
    pub project_id: Option<ProjectId>,
    pub tags: Option<Vec<String>>,
    pub tags_mode: TagFilterMode,

    pub due_before: Option<NaiveDate>,
    pub due_after: Option<NaiveDate>,

    pub search_text: Option<String>,

    pub include_subtasks: bool,
    pub include_completed: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TagFilterMode {
    #[default]
    Any,
    All,
}

/// Sort options for task lists
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortField {
    #[default]
    CreatedAt,
    UpdatedAt,
    DueDate,
    Priority,
    Title,
    Status,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortSpec {
    pub field: SortField,
    pub order: SortOrder,
}

impl Default for SortSpec {
    fn default() -> Self {
        Self {
            field: SortField::CreatedAt,
            order: SortOrder::Descending,
        }
    }
}
