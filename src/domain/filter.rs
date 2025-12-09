//! Task filtering and sorting functionality.
//!
//! This module provides the [`Filter`] struct for querying tasks based on various
//! criteria, and [`SortSpec`] for ordering results. Filters can be saved as
//! [`SavedFilter`] for quick access.
//!
//! # Examples
//!
//! ```
//! use taskflow::domain::{Filter, SortField, SortOrder, SortSpec, TaskStatus, Priority};
//!
//! // Filter for high-priority incomplete tasks
//! let filter = Filter {
//!     priority: Some(vec![Priority::High, Priority::Urgent]),
//!     status: Some(vec![TaskStatus::Todo, TaskStatus::InProgress]),
//!     ..Default::default()
//! };
//!
//! // Sort by due date, then priority
//! let sort = SortSpec {
//!     field: SortField::DueDate,
//!     order: SortOrder::Ascending,
//! };
//! ```

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

#[cfg(test)]
mod tests {
    use super::*;

    // SavedFilterId tests

    #[test]
    fn test_saved_filter_id_uniqueness() {
        let id1 = SavedFilterId::new();
        let id2 = SavedFilterId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_saved_filter_id_default() {
        let id1 = SavedFilterId::default();
        let id2 = SavedFilterId::default();
        // Each default call creates a new unique ID
        assert_ne!(id1, id2);
    }

    // Filter tests

    #[test]
    fn test_filter_default() {
        let filter = Filter::default();
        assert!(filter.status.is_none());
        assert!(filter.priority.is_none());
        assert!(filter.project_id.is_none());
        assert!(filter.tags.is_none());
        assert!(filter.due_before.is_none());
        assert!(filter.due_after.is_none());
        assert!(filter.search_text.is_none());
        assert!(!filter.include_subtasks);
        assert!(!filter.include_completed);
    }

    #[test]
    fn test_filter_with_status() {
        let filter = Filter {
            status: Some(vec![TaskStatus::Todo, TaskStatus::InProgress]),
            ..Default::default()
        };
        assert_eq!(filter.status.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_filter_with_priority() {
        let filter = Filter {
            priority: Some(vec![Priority::High, Priority::Urgent]),
            ..Default::default()
        };
        assert_eq!(filter.priority.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_filter_with_date_range() {
        use chrono::NaiveDate;
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();

        let filter = Filter {
            due_after: Some(start),
            due_before: Some(end),
            ..Default::default()
        };

        assert_eq!(filter.due_after, Some(start));
        assert_eq!(filter.due_before, Some(end));
    }

    #[test]
    fn test_filter_with_search_text() {
        let filter = Filter {
            search_text: Some("urgent".to_string()),
            ..Default::default()
        };
        assert_eq!(filter.search_text, Some("urgent".to_string()));
    }

    // TagFilterMode tests

    #[test]
    fn test_tag_filter_mode_default() {
        let mode = TagFilterMode::default();
        assert!(matches!(mode, TagFilterMode::Any));
    }

    #[test]
    fn test_tag_filter_mode_variants() {
        let any = TagFilterMode::Any;
        let all = TagFilterMode::All;
        assert!(matches!(any, TagFilterMode::Any));
        assert!(matches!(all, TagFilterMode::All));
    }

    // SortField tests

    #[test]
    fn test_sort_field_default() {
        let field = SortField::default();
        assert!(matches!(field, SortField::CreatedAt));
    }

    #[test]
    fn test_sort_field_variants() {
        let fields = [
            SortField::CreatedAt,
            SortField::UpdatedAt,
            SortField::DueDate,
            SortField::Priority,
            SortField::Title,
            SortField::Status,
        ];
        assert_eq!(fields.len(), 6);
    }

    // SortOrder tests

    #[test]
    fn test_sort_order_default() {
        let order = SortOrder::default();
        assert!(matches!(order, SortOrder::Ascending));
    }

    #[test]
    fn test_sort_order_variants() {
        let asc = SortOrder::Ascending;
        let desc = SortOrder::Descending;
        assert!(matches!(asc, SortOrder::Ascending));
        assert!(matches!(desc, SortOrder::Descending));
    }

    // SortSpec tests

    #[test]
    fn test_sort_spec_default() {
        let spec = SortSpec::default();
        assert!(matches!(spec.field, SortField::CreatedAt));
        assert!(matches!(spec.order, SortOrder::Descending));
    }

    #[test]
    fn test_sort_spec_custom() {
        let spec = SortSpec {
            field: SortField::Priority,
            order: SortOrder::Ascending,
        };
        assert!(matches!(spec.field, SortField::Priority));
        assert!(matches!(spec.order, SortOrder::Ascending));
    }

    // SavedFilter tests

    #[test]
    fn test_saved_filter_new() {
        let filter = Filter::default();
        let sort = SortSpec::default();
        let saved = SavedFilter::new("My Filter", filter, sort);

        assert_eq!(saved.name, "My Filter");
        assert!(saved.icon.is_none());
    }

    #[test]
    fn test_saved_filter_with_icon() {
        let filter = Filter::default();
        let sort = SortSpec::default();
        let saved = SavedFilter::new("Urgent Tasks", filter, sort).with_icon("🔥");

        assert_eq!(saved.icon, Some("🔥".to_string()));
    }

    // Serialization tests

    #[test]
    fn test_filter_serialization_roundtrip() {
        let filter = Filter {
            status: Some(vec![TaskStatus::Todo]),
            priority: Some(vec![Priority::High]),
            search_text: Some("test".to_string()),
            include_completed: true,
            ..Default::default()
        };

        let json = serde_json::to_string(&filter).expect("Failed to serialize");
        let restored: Filter = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(restored.status, filter.status);
        assert_eq!(restored.priority, filter.priority);
        assert_eq!(restored.search_text, filter.search_text);
        assert_eq!(restored.include_completed, filter.include_completed);
    }

    #[test]
    fn test_sort_spec_serialization_roundtrip() {
        let spec = SortSpec {
            field: SortField::DueDate,
            order: SortOrder::Ascending,
        };

        let json = serde_json::to_string(&spec).expect("Failed to serialize");
        let restored: SortSpec = serde_json::from_str(&json).expect("Failed to deserialize");

        assert!(matches!(restored.field, SortField::DueDate));
        assert!(matches!(restored.order, SortOrder::Ascending));
    }

    #[test]
    fn test_saved_filter_serialization_roundtrip() {
        let filter = Filter {
            search_text: Some("urgent".to_string()),
            ..Default::default()
        };
        let sort = SortSpec {
            field: SortField::Priority,
            order: SortOrder::Descending,
        };
        let saved = SavedFilter::new("Urgent", filter, sort).with_icon("⚡");

        let json = serde_json::to_string(&saved).expect("Failed to serialize");
        let restored: SavedFilter = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(restored.name, "Urgent");
        assert_eq!(restored.icon, Some("⚡".to_string()));
        assert_eq!(restored.filter.search_text, Some("urgent".to_string()));
    }

    #[test]
    fn test_tag_filter_mode_serialization() {
        let any_json = serde_json::to_string(&TagFilterMode::Any).unwrap();
        let all_json = serde_json::to_string(&TagFilterMode::All).unwrap();

        assert_eq!(any_json, "\"any\"");
        assert_eq!(all_json, "\"all\"");

        let any: TagFilterMode = serde_json::from_str(&any_json).unwrap();
        let all: TagFilterMode = serde_json::from_str(&all_json).unwrap();

        assert!(matches!(any, TagFilterMode::Any));
        assert!(matches!(all, TagFilterMode::All));
    }
}
