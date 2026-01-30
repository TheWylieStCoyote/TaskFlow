//! Project entity and related types.
//!
//! Projects provide a way to organize related tasks into logical groups.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for projects.
///
/// Each project has a UUID-based identifier that remains stable across
/// serialization and storage operations.
///
/// # Examples
///
/// ```
/// use taskflow::domain::ProjectId;
///
/// let id = ProjectId::new();
/// println!("Project ID: {}", id);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(pub Uuid);

impl ProjectId {
    /// Creates a new unique project identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Project lifecycle status.
///
/// # Examples
///
/// ```
/// use taskflow::domain::ProjectStatus;
///
/// let status = ProjectStatus::Active;
/// assert_eq!(status.as_str(), "active");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    /// Project is actively being worked on (default)
    #[default]
    Active,
    /// Project is temporarily paused
    OnHold,
    /// Project has been finished
    Completed,
    /// Project is no longer active but kept for reference
    Archived,
}

impl ProjectStatus {
    /// Returns the status as a lowercase string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::OnHold => "on_hold",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }

    /// Parses a status from a string, returning `Active` for unknown values.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "on_hold" => Self::OnHold,
            "completed" => Self::Completed,
            "archived" => Self::Archived,
            _ => Self::Active,
        }
    }
}

/// A project groups related tasks together.
///
/// Projects help organize work by providing a container for tasks
/// that share a common goal or context.
///
/// # Examples
///
/// ## Creating Projects
///
/// ```
/// use taskflow::domain::Project;
///
/// // Simple project
/// let project = Project::new("Backend API");
///
/// // Project with color and metadata
/// let project = Project::new("Frontend UI")
///     .with_color("#3498db");
///
/// assert!(project.is_active());
/// ```
///
/// ## Project Hierarchy
///
/// ```
/// use taskflow::domain::Project;
///
/// let parent = Project::new("Engineering");
/// let child = Project::new("Backend Team")
///     .with_parent(parent.id);
///
/// assert_eq!(child.parent_id, Some(parent.id));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,

    // Hierarchy
    pub parent_id: Option<ProjectId>,

    // Metadata
    pub color: Option<String>,
    pub icon: Option<String>,

    // Dates
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub start_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,

    // Settings
    pub default_tags: Vec<String>,

    // Custom fields
    #[serde(default)]
    pub custom_fields: HashMap<String, serde_json::Value>,

    /// Learned estimation multiplier based on historical accuracy.
    ///
    /// A value of 1.3 means tasks in this project typically take 30% longer
    /// than initially estimated. This is used to adjust future estimates.
    #[serde(default)]
    pub estimation_multiplier: Option<f64>,
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: ProjectId::new(),
            name: name.into(),
            description: None,
            status: ProjectStatus::default(),
            parent_id: None,
            color: None,
            icon: None,
            created_at: now,
            updated_at: now,
            start_date: None,
            due_date: None,
            default_tags: Vec::new(),
            custom_fields: HashMap::new(),
            estimation_multiplier: None,
        }
    }

    #[must_use]
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    #[must_use]
    pub const fn with_parent(mut self, parent_id: ProjectId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.status == ProjectStatus::Active
    }

    /// Returns an adjusted estimate based on the project's historical accuracy.
    ///
    /// If this project has an `estimation_multiplier` of 1.3 (meaning tasks
    /// typically take 30% longer than estimated), a raw estimate of 60 minutes
    /// would return 78 minutes.
    ///
    /// # Examples
    ///
    /// ```
    /// use taskflow::domain::Project;
    ///
    /// let mut project = Project::new("Backend API");
    /// project.estimation_multiplier = Some(1.3);
    ///
    /// assert_eq!(project.suggested_estimate(60), 78);
    /// ```
    #[must_use]
    pub fn suggested_estimate(&self, raw_estimate: u32) -> u32 {
        self.estimation_multiplier.map_or(raw_estimate, |m| {
            (f64::from(raw_estimate) * m).round() as u32
        })
    }

    /// Builder method to set estimation multiplier.
    ///
    /// # Examples
    ///
    /// ```
    /// use taskflow::domain::Project;
    ///
    /// let project = Project::new("Backend API")
    ///     .with_estimation_multiplier(1.2);
    ///
    /// assert_eq!(project.suggested_estimate(100), 120);
    /// ```
    #[must_use]
    pub fn with_estimation_multiplier(mut self, multiplier: f64) -> Self {
        self.estimation_multiplier = Some(multiplier);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_new_creates_unique_id() {
        let project1 = Project::new("Project 1");
        let project2 = Project::new("Project 2");
        assert_ne!(project1.id, project2.id);
    }

    #[test]
    fn test_project_new_sets_defaults() {
        let project = Project::new("Test project");
        assert_eq!(project.name, "Test project");
        assert_eq!(project.status, ProjectStatus::Active);
        assert!(project.parent_id.is_none());
        assert!(project.color.is_none());
        assert!(project.description.is_none());
    }

    #[test]
    fn test_project_with_color() {
        let project = Project::new("Test").with_color("#ff0000");
        assert_eq!(project.color, Some("#ff0000".to_string()));
    }

    #[test]
    fn test_project_with_parent() {
        let parent = Project::new("Parent");
        let child = Project::new("Child").with_parent(parent.id);
        assert_eq!(child.parent_id, Some(parent.id));
    }

    #[test]
    fn test_project_is_active() {
        let active = Project::new("Active");
        assert!(active.is_active());

        let mut on_hold = Project::new("On Hold");
        on_hold.status = ProjectStatus::OnHold;
        assert!(!on_hold.is_active());

        let mut completed = Project::new("Completed");
        completed.status = ProjectStatus::Completed;
        assert!(!completed.is_active());

        let mut archived = Project::new("Archived");
        archived.status = ProjectStatus::Archived;
        assert!(!archived.is_active());
    }

    #[test]
    fn test_project_status_from_str_lossy() {
        assert_eq!(
            ProjectStatus::from_str_lossy("active"),
            ProjectStatus::Active
        );
        assert_eq!(
            ProjectStatus::from_str_lossy("on_hold"),
            ProjectStatus::OnHold
        );
        assert_eq!(
            ProjectStatus::from_str_lossy("completed"),
            ProjectStatus::Completed
        );
        assert_eq!(
            ProjectStatus::from_str_lossy("archived"),
            ProjectStatus::Archived
        );
        // Unknown defaults to Active
        assert_eq!(
            ProjectStatus::from_str_lossy("invalid"),
            ProjectStatus::Active
        );
        assert_eq!(ProjectStatus::from_str_lossy(""), ProjectStatus::Active);
    }

    #[test]
    fn test_project_suggested_estimate_no_multiplier() {
        let project = Project::new("Test");
        // Without multiplier, returns raw estimate unchanged
        assert_eq!(project.suggested_estimate(60), 60);
        assert_eq!(project.suggested_estimate(120), 120);
    }

    #[test]
    fn test_project_suggested_estimate_with_multiplier() {
        let project = Project::new("Test").with_estimation_multiplier(1.3);
        // 60 * 1.3 = 78
        assert_eq!(project.suggested_estimate(60), 78);
        // 100 * 1.3 = 130
        assert_eq!(project.suggested_estimate(100), 130);
    }

    #[test]
    fn test_project_suggested_estimate_under() {
        let project = Project::new("Test").with_estimation_multiplier(0.8);
        // 60 * 0.8 = 48
        assert_eq!(project.suggested_estimate(60), 48);
    }

    #[test]
    fn test_project_with_estimation_multiplier() {
        let project = Project::new("Test").with_estimation_multiplier(1.5);
        assert_eq!(project.estimation_multiplier, Some(1.5));
    }

    // ========================================================================
    // Hierarchical Structure Tests
    // ========================================================================

    #[test]
    fn test_project_hierarchy_three_levels() {
        let root = Project::new("Company");
        let dept = Project::new("Engineering").with_parent(root.id);
        let team = Project::new("Backend").with_parent(dept.id);

        assert_eq!(dept.parent_id, Some(root.id));
        assert_eq!(team.parent_id, Some(dept.id));
        assert!(root.parent_id.is_none());
    }

    #[test]
    fn test_project_hierarchy_siblings() {
        let parent = Project::new("Q1 Goals");
        let child1 = Project::new("Backend").with_parent(parent.id);
        let child2 = Project::new("Frontend").with_parent(parent.id);

        assert_eq!(child1.parent_id, child2.parent_id);
        assert_ne!(child1.id, child2.id);
    }

    #[test]
    fn test_project_can_have_no_parent() {
        let orphan = Project::new("Standalone Project");
        assert!(orphan.parent_id.is_none());
    }

    // ========================================================================
    // Color Validation Tests
    // ========================================================================

    #[test]
    fn test_project_color_hex_codes() {
        let valid_colors = [
            "#ff0000", "#00FF00", "#0000ff", "#3498db", "#e74c3c", "#F39C12",
        ];

        for color in valid_colors {
            let project = Project::new("Test").with_color(color);
            assert_eq!(project.color, Some(color.to_string()));
        }
    }

    #[test]
    fn test_project_color_named_colors() {
        let project = Project::new("Test").with_color("red");
        assert_eq!(project.color, Some("red".to_string()));

        let project2 = Project::new("Test").with_color("blue");
        assert_eq!(project2.color, Some("blue".to_string()));
    }

    #[test]
    fn test_project_color_accepts_any_string() {
        // Domain layer doesn't validate - accepts any string
        let project = Project::new("Test").with_color("not-a-valid-color");
        assert_eq!(project.color, Some("not-a-valid-color".to_string()));
    }

    #[test]
    fn test_project_color_empty_string() {
        let project = Project::new("Test").with_color("");
        assert_eq!(project.color, Some(String::new()));
    }

    // ========================================================================
    // Status Transition Tests
    // ========================================================================

    #[test]
    fn test_project_status_active_to_archived() {
        let mut project = Project::new("Test");
        assert_eq!(project.status, ProjectStatus::Active);

        project.status = ProjectStatus::Archived;
        assert_eq!(project.status, ProjectStatus::Archived);
        assert!(!project.is_active());
    }

    #[test]
    fn test_project_status_archived_to_active() {
        let mut project = Project::new("Test");
        project.status = ProjectStatus::Archived;

        // Can unarchive
        project.status = ProjectStatus::Active;
        assert!(project.is_active());
    }

    #[test]
    fn test_project_status_active_to_completed() {
        let mut project = Project::new("Test");
        project.status = ProjectStatus::Completed;
        assert_eq!(project.status, ProjectStatus::Completed);
        assert!(!project.is_active());
    }

    #[test]
    fn test_project_status_completed_to_archived() {
        let mut project = Project::new("Test");
        project.status = ProjectStatus::Completed;
        project.status = ProjectStatus::Archived;
        assert_eq!(project.status, ProjectStatus::Archived);
    }

    #[test]
    fn test_project_status_on_hold_transitions() {
        let mut project = Project::new("Test");
        project.status = ProjectStatus::OnHold;
        assert!(!project.is_active());

        // Can resume from on hold
        project.status = ProjectStatus::Active;
        assert!(project.is_active());
    }

    // ========================================================================
    // Custom Fields Tests
    // ========================================================================

    #[test]
    fn test_project_custom_fields_empty_by_default() {
        let project = Project::new("Test");
        assert!(project.custom_fields.is_empty());
    }

    #[test]
    fn test_project_custom_fields_can_store_strings() {
        let mut project = Project::new("Test");
        project
            .custom_fields
            .insert("client".to_string(), serde_json::json!("Acme Corp"));
        project
            .custom_fields
            .insert("budget".to_string(), serde_json::json!("$50,000"));

        assert_eq!(
            project.custom_fields.get("client"),
            Some(&serde_json::json!("Acme Corp"))
        );
        assert_eq!(
            project.custom_fields.get("budget"),
            Some(&serde_json::json!("$50,000"))
        );
    }

    #[test]
    fn test_project_custom_fields_can_store_numbers() {
        let mut project = Project::new("Test");
        project
            .custom_fields
            .insert("budget".to_string(), serde_json::json!(50000));
        project
            .custom_fields
            .insert("completion".to_string(), serde_json::json!(75.5));

        assert_eq!(
            project.custom_fields.get("budget"),
            Some(&serde_json::json!(50000))
        );
    }

    #[test]
    fn test_project_custom_fields_can_store_arrays() {
        let mut project = Project::new("Test");
        project.custom_fields.insert(
            "stakeholders".to_string(),
            serde_json::json!(["Alice", "Bob", "Carol"]),
        );

        assert_eq!(
            project.custom_fields.get("stakeholders"),
            Some(&serde_json::json!(["Alice", "Bob", "Carol"]))
        );
    }

    #[test]
    fn test_project_custom_fields_can_store_objects() {
        let mut project = Project::new("Test");
        project.custom_fields.insert(
            "metadata".to_string(),
            serde_json::json!({
                "repo": "github.com/user/repo",
                "ci": true
            }),
        );

        let metadata = project.custom_fields.get("metadata").unwrap();
        assert_eq!(metadata["repo"], "github.com/user/repo");
        assert_eq!(metadata["ci"], true);
    }

    // ========================================================================
    // Date Validation Tests
    // ========================================================================

    #[test]
    fn test_project_dates_can_be_set() {
        let mut project = Project::new("Test");
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let due = NaiveDate::from_ymd_opt(2025, 3, 31).unwrap();

        project.start_date = Some(start);
        project.due_date = Some(due);

        assert_eq!(project.start_date, Some(start));
        assert_eq!(project.due_date, Some(due));
    }

    #[test]
    fn test_project_start_date_before_due_date() {
        let mut project = Project::new("Test");
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let due = NaiveDate::from_ymd_opt(2025, 3, 31).unwrap();

        project.start_date = Some(start);
        project.due_date = Some(due);

        // Domain layer doesn't enforce order - that's app layer responsibility
        assert!(start < due);
    }

    #[test]
    fn test_project_dates_can_be_same_day() {
        let mut project = Project::new("Test");
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();

        project.start_date = Some(date);
        project.due_date = Some(date);

        assert_eq!(project.start_date, project.due_date);
    }

    // ========================================================================
    // Default Tags Tests
    // ========================================================================

    #[test]
    fn test_project_default_tags_empty_by_default() {
        let project = Project::new("Test");
        assert!(project.default_tags.is_empty());
    }

    #[test]
    fn test_project_default_tags_can_be_added() {
        let mut project = Project::new("Test");
        project.default_tags = vec!["urgent".to_string(), "backend".to_string()];

        assert_eq!(project.default_tags.len(), 2);
        assert!(project.default_tags.contains(&"urgent".to_string()));
        assert!(project.default_tags.contains(&"backend".to_string()));
    }

    // ========================================================================
    // Estimation Multiplier Edge Cases
    // ========================================================================

    #[test]
    fn test_project_estimation_multiplier_zero() {
        let project = Project::new("Test").with_estimation_multiplier(0.0);
        assert_eq!(project.suggested_estimate(100), 0);
    }

    #[test]
    fn test_project_estimation_multiplier_very_large() {
        let project = Project::new("Test").with_estimation_multiplier(10.0);
        assert_eq!(project.suggested_estimate(100), 1000);
    }

    #[test]
    fn test_project_estimation_multiplier_fractional() {
        let project = Project::new("Test").with_estimation_multiplier(0.5);
        assert_eq!(project.suggested_estimate(100), 50);
        assert_eq!(project.suggested_estimate(75), 38); // 37.5 rounds to 38
    }

    #[test]
    fn test_project_estimation_multiplier_rounding() {
        let project = Project::new("Test").with_estimation_multiplier(1.33);
        // 60 * 1.33 = 79.8, rounds to 80
        assert_eq!(project.suggested_estimate(60), 80);
    }

    // ========================================================================
    // Serialization Tests
    // ========================================================================

    #[test]
    fn test_project_serialization_roundtrip() {
        let original = Project::new("Test Project")
            .with_color("#ff0000")
            .with_estimation_multiplier(1.2);

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Project = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, original.name);
        assert_eq!(deserialized.color, original.color);
        assert_eq!(
            deserialized.estimation_multiplier,
            original.estimation_multiplier
        );
    }

    #[test]
    fn test_project_status_serialization() {
        let mut project = Project::new("Test");
        project.status = ProjectStatus::Archived;

        let json = serde_json::to_string(&project).unwrap();
        assert!(json.contains("\"archived\""));

        let deserialized: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status, ProjectStatus::Archived);
    }

    #[test]
    fn test_project_custom_fields_serialization() {
        let mut project = Project::new("Test");
        project
            .custom_fields
            .insert("key1".to_string(), serde_json::json!("value1"));

        let json = serde_json::to_string(&project).unwrap();
        let deserialized: Project = serde_json::from_str(&json).unwrap();

        assert_eq!(
            deserialized.custom_fields.get("key1"),
            Some(&serde_json::json!("value1"))
        );
    }
}
