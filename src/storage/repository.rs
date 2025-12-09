//! Storage repository traits.
//!
//! This module defines the repository pattern interfaces for data persistence.
//! Each entity type has a corresponding repository trait that storage backends
//! implement to provide CRUD operations.
//!
//! # Repository Traits
//!
//! - [`TaskRepository`]: Task CRUD and filtered queries
//! - [`ProjectRepository`]: Project management
//! - [`TagRepository`]: Tag retrieval
//! - [`TimeEntryRepository`]: Time tracking entries
//! - [`WorkLogRepository`]: Work log entries
//! - [`HabitRepository`]: Habit tracking
//! - [`PomodoroRepository`]: Pomodoro session tracking
//!
//! # Implementing a Backend
//!
//! Storage backends implement these traits to provide persistence.
//! See [`super::backends`] for existing implementations.

use crate::domain::{
    Filter, Habit, HabitId, PomodoroConfig, PomodoroSession, PomodoroStats, Project, ProjectId,
    Tag, Task, TaskId, TimeEntry, TimeEntryId, WorkLogEntry, WorkLogEntryId,
};

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

/// Repository trait for work log entry operations.
pub trait WorkLogRepository {
    /// Creates a new work log entry.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the entry cannot be persisted.
    fn create_work_log(&mut self, entry: &WorkLogEntry) -> StorageResult<()>;

    /// Retrieves a work log entry by ID.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn get_work_log(&self, id: &WorkLogEntryId) -> StorageResult<Option<WorkLogEntry>>;

    /// Updates an existing work log entry.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the entry cannot be updated.
    fn update_work_log(&mut self, entry: &WorkLogEntry) -> StorageResult<()>;

    /// Deletes a work log entry by ID.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the entry cannot be deleted.
    fn delete_work_log(&mut self, id: &WorkLogEntryId) -> StorageResult<()>;

    /// Gets all work log entries for a task, ordered by creation time (newest first).
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn get_work_logs_for_task(&self, task_id: &TaskId) -> StorageResult<Vec<WorkLogEntry>>;

    /// Lists all work log entries.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn list_work_logs(&self) -> StorageResult<Vec<WorkLogEntry>>;
}

/// Repository trait for habit operations.
pub trait HabitRepository {
    /// Creates a new habit in storage.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the habit cannot be persisted.
    fn create_habit(&mut self, habit: &Habit) -> StorageResult<()>;

    /// Retrieves a habit by ID.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn get_habit(&self, id: &HabitId) -> StorageResult<Option<Habit>>;

    /// Updates an existing habit.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the habit cannot be updated.
    fn update_habit(&mut self, habit: &Habit) -> StorageResult<()>;

    /// Deletes a habit by ID.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the habit cannot be deleted.
    fn delete_habit(&mut self, id: &HabitId) -> StorageResult<()>;

    /// Lists all habits.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn list_habits(&self) -> StorageResult<Vec<Habit>>;

    /// Lists all active (non-archived) habits.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn list_active_habits(&self) -> StorageResult<Vec<Habit>>;
}

/// Data export structure for migration between backends
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportData {
    pub tasks: Vec<Task>,
    pub projects: Vec<Project>,
    pub tags: Vec<Tag>,
    pub time_entries: Vec<TimeEntry>,
    /// Work log entries for tasks
    #[serde(default)]
    pub work_logs: Vec<WorkLogEntry>,
    /// Habits with check-in history
    #[serde(default)]
    pub habits: Vec<Habit>,
    pub version: u32,
    /// Active Pomodoro session (if any)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pomodoro_session: Option<PomodoroSession>,
    /// Pomodoro configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pomodoro_config: Option<PomodoroConfig>,
    /// Pomodoro statistics
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pomodoro_stats: Option<PomodoroStats>,
}

impl Default for ExportData {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            projects: Vec::new(),
            tags: Vec::new(),
            time_entries: Vec::new(),
            work_logs: Vec::new(),
            habits: Vec::new(),
            version: 1,
            pomodoro_session: None,
            pomodoro_config: None,
            pomodoro_stats: None,
        }
    }
}

/// Unified storage backend trait combining all repositories.
pub trait StorageBackend:
    TaskRepository
    + ProjectRepository
    + TagRepository
    + TimeEntryRepository
    + WorkLogRepository
    + HabitRepository
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

    /// Sets the active Pomodoro session.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the session cannot be saved.
    fn set_pomodoro_session(&mut self, session: Option<&PomodoroSession>) -> StorageResult<()>;

    /// Sets the Pomodoro configuration.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the config cannot be saved.
    fn set_pomodoro_config(&mut self, config: &PomodoroConfig) -> StorageResult<()>;

    /// Sets the Pomodoro statistics.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the stats cannot be saved.
    fn set_pomodoro_stats(&mut self, stats: &PomodoroStats) -> StorageResult<()>;

    /// Refreshes the storage backend by detecting external changes.
    ///
    /// This method scans for files that have been modified, added, or deleted
    /// externally (e.g., by a text editor or git operations) and updates the
    /// internal cache accordingly.
    ///
    /// Returns the number of changes detected and applied.
    ///
    /// The default implementation is a no-op that returns 0, suitable for
    /// backends that don't support external modification detection (like SQLite).
    fn refresh(&mut self) -> usize {
        0
    }
}
