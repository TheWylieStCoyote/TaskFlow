use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for projects
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(pub Uuid);

impl ProjectId {
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

/// Project status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    #[default]
    Active,
    OnHold,
    Completed,
    Archived,
}

impl ProjectStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectStatus::Active => "active",
            ProjectStatus::OnHold => "on_hold",
            ProjectStatus::Completed => "completed",
            ProjectStatus::Archived => "archived",
        }
    }
}

/// Project entity for grouping related tasks
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
        }
    }

    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn with_parent(mut self, parent_id: ProjectId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn is_active(&self) -> bool {
        self.status == ProjectStatus::Active
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
        let child = Project::new("Child").with_parent(parent.id.clone());
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
}
