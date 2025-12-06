use crate::domain::{Filter, Project, ProjectId, Tag, Task, TaskId, TimeEntry, TimeEntryId};

use super::error::StorageResult;

/// Repository trait for task operations.
pub trait TaskRepository {
    /// Creates a new task in storage.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the task cannot be persisted.
    fn create_task(&mut self, task: &Task) -> StorageResult<()>;

    /// Retrieves a task by ID.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn get_task(&self, id: &TaskId) -> StorageResult<Option<Task>>;

    /// Updates an existing task.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the task cannot be updated.
    fn update_task(&mut self, task: &Task) -> StorageResult<()>;

    /// Deletes a task by ID.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the task cannot be deleted.
    fn delete_task(&mut self, id: &TaskId) -> StorageResult<()>;

    /// Lists all tasks.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn list_tasks(&self) -> StorageResult<Vec<Task>>;

    /// Lists tasks matching the given filter.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn list_tasks_filtered(&self, filter: &Filter) -> StorageResult<Vec<Task>>;

    /// Gets all tasks belonging to a project.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn get_tasks_by_project(&self, project_id: &ProjectId) -> StorageResult<Vec<Task>>;

    /// Gets all tasks with a specific tag.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn get_tasks_by_tag(&self, tag: &str) -> StorageResult<Vec<Task>>;
}

/// Repository trait for project operations.
pub trait ProjectRepository {
    /// Creates a new project in storage.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the project cannot be persisted.
    fn create_project(&mut self, project: &Project) -> StorageResult<()>;

    /// Retrieves a project by ID.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn get_project(&self, id: &ProjectId) -> StorageResult<Option<Project>>;

    /// Updates an existing project.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the project cannot be updated.
    fn update_project(&mut self, project: &Project) -> StorageResult<()>;

    /// Deletes a project by ID.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the project cannot be deleted.
    fn delete_project(&mut self, id: &ProjectId) -> StorageResult<()>;

    /// Lists all projects.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn list_projects(&self) -> StorageResult<Vec<Project>>;

    /// Gets child projects of a parent.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn get_subprojects(&self, parent_id: &ProjectId) -> StorageResult<Vec<Project>>;
}

/// Repository trait for tag operations.
pub trait TagRepository {
    /// Creates or updates a tag.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the tag cannot be persisted.
    fn save_tag(&mut self, tag: &Tag) -> StorageResult<()>;

    /// Retrieves a tag by name.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn get_tag(&self, name: &str) -> StorageResult<Option<Tag>>;

    /// Deletes a tag by name.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the tag cannot be deleted.
    fn delete_tag(&mut self, name: &str) -> StorageResult<()>;

    /// Lists all tags.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn list_tags(&self) -> StorageResult<Vec<Tag>>;
}

/// Repository trait for time entry operations.
pub trait TimeEntryRepository {
    /// Creates a new time entry.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the entry cannot be persisted.
    fn create_time_entry(&mut self, entry: &TimeEntry) -> StorageResult<()>;

    /// Retrieves a time entry by ID.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn get_time_entry(&self, id: &TimeEntryId) -> StorageResult<Option<TimeEntry>>;

    /// Updates an existing time entry.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the entry cannot be updated.
    fn update_time_entry(&mut self, entry: &TimeEntry) -> StorageResult<()>;

    /// Deletes a time entry by ID.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the entry cannot be deleted.
    fn delete_time_entry(&mut self, id: &TimeEntryId) -> StorageResult<()>;

    /// Gets all time entries for a task.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn get_entries_for_task(&self, task_id: &TaskId) -> StorageResult<Vec<TimeEntry>>;

    /// Gets the currently running time entry (if any).
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
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

/// Unified storage backend trait combining all repositories.
pub trait StorageBackend:
    TaskRepository + ProjectRepository + TagRepository + TimeEntryRepository
{
    /// Initializes the storage backend (creates files/tables, etc.).
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if initialization fails.
    fn initialize(&mut self) -> StorageResult<()>;

    /// Flushes any pending changes to disk.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if flushing fails.
    fn flush(&mut self) -> StorageResult<()>;

    /// Exports all data for migration.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if export fails.
    fn export_all(&self) -> StorageResult<ExportData>;

    /// Imports data for migration.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if import fails.
    fn import_all(&mut self, data: &ExportData) -> StorageResult<()>;

    /// Returns the storage backend type name.
    fn backend_type(&self) -> &'static str;
}
