use crate::domain::{Filter, Project, ProjectId, Tag, Task, TaskId, TimeEntry, TimeEntryId};

use super::error::StorageResult;

/// Repository trait for task operations
pub trait TaskRepository {
    /// Create a new task
    fn create_task(&mut self, task: &Task) -> StorageResult<()>;

    /// Get task by ID
    fn get_task(&self, id: &TaskId) -> StorageResult<Option<Task>>;

    /// Update an existing task
    fn update_task(&mut self, task: &Task) -> StorageResult<()>;

    /// Delete a task by ID
    fn delete_task(&mut self, id: &TaskId) -> StorageResult<()>;

    /// List all tasks
    fn list_tasks(&self) -> StorageResult<Vec<Task>>;

    /// List tasks with optional filtering
    fn list_tasks_filtered(&self, filter: &Filter) -> StorageResult<Vec<Task>>;

    /// Get tasks by project
    fn get_tasks_by_project(&self, project_id: &ProjectId) -> StorageResult<Vec<Task>>;

    /// Get tasks by tag
    fn get_tasks_by_tag(&self, tag: &str) -> StorageResult<Vec<Task>>;
}

/// Repository trait for project operations
pub trait ProjectRepository {
    /// Create a new project
    fn create_project(&mut self, project: &Project) -> StorageResult<()>;

    /// Get project by ID
    fn get_project(&self, id: &ProjectId) -> StorageResult<Option<Project>>;

    /// Update an existing project
    fn update_project(&mut self, project: &Project) -> StorageResult<()>;

    /// Delete a project by ID
    fn delete_project(&mut self, id: &ProjectId) -> StorageResult<()>;

    /// List all projects
    fn list_projects(&self) -> StorageResult<Vec<Project>>;

    /// Get child projects of a parent
    fn get_subprojects(&self, parent_id: &ProjectId) -> StorageResult<Vec<Project>>;
}

/// Repository trait for tag operations
pub trait TagRepository {
    /// Create or update a tag
    fn save_tag(&mut self, tag: &Tag) -> StorageResult<()>;

    /// Get tag by name
    fn get_tag(&self, name: &str) -> StorageResult<Option<Tag>>;

    /// Delete a tag by name
    fn delete_tag(&mut self, name: &str) -> StorageResult<()>;

    /// List all tags
    fn list_tags(&self) -> StorageResult<Vec<Tag>>;
}

/// Repository trait for time entry operations
pub trait TimeEntryRepository {
    /// Create a new time entry
    fn create_time_entry(&mut self, entry: &TimeEntry) -> StorageResult<()>;

    /// Get time entry by ID
    fn get_time_entry(&self, id: &TimeEntryId) -> StorageResult<Option<TimeEntry>>;

    /// Update an existing time entry
    fn update_time_entry(&mut self, entry: &TimeEntry) -> StorageResult<()>;

    /// Delete a time entry by ID
    fn delete_time_entry(&mut self, id: &TimeEntryId) -> StorageResult<()>;

    /// Get all time entries for a task
    fn get_entries_for_task(&self, task_id: &TaskId) -> StorageResult<Vec<TimeEntry>>;

    /// Get the currently running time entry (if any)
    fn get_active_entry(&self) -> StorageResult<Option<TimeEntry>>;
}

/// Data export structure for migration between backends
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportData {
    pub tasks: Vec<Task>,
    pub projects: Vec<Project>,
    pub tags: Vec<Tag>,
    pub time_entries: Vec<TimeEntry>,
    pub version: u32,
}

impl Default for ExportData {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            projects: Vec::new(),
            tags: Vec::new(),
            time_entries: Vec::new(),
            version: 1,
        }
    }
}

/// Unified storage backend trait combining all repositories
pub trait StorageBackend:
    TaskRepository + ProjectRepository + TagRepository + TimeEntryRepository
{
    /// Initialize the storage backend (create files/tables, etc.)
    fn initialize(&mut self) -> StorageResult<()>;

    /// Flush any pending changes to disk
    fn flush(&mut self) -> StorageResult<()>;

    /// Export all data (for migration)
    fn export_all(&self) -> StorageResult<ExportData>;

    /// Import data (for migration)
    fn import_all(&mut self, data: &ExportData) -> StorageResult<()>;

    /// Get the storage backend type name
    fn backend_type(&self) -> &'static str;
}
