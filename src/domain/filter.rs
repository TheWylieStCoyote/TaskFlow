use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Priority, ProjectId, TaskStatus};

/// Unique identifier for a saved filter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SavedFilterId(pub String);

impl SavedFilterId {
    /// Generate a new unique ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for SavedFilterId {
    fn default() -> Self {
        Self::new()
    }
}

/// A saved/named filter that can be quickly applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedFilter {
    pub id: SavedFilterId,
    pub name: String,
    pub filter: Filter,
    pub sort: SortSpec,
    /// Optional icon/emoji for display
    pub icon: Option<String>,
}

impl SavedFilter {
    /// Create a new saved filter with the given name.
    #[must_use]
    pub fn new(name: impl Into<String>, filter: Filter, sort: SortSpec) -> Self {
        Self {
            id: SavedFilterId::new(),
            name: name.into(),
            filter,
            sort,
            icon: None,
        }
    }

    /// Set the icon for this filter.
    #[must_use]
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

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
