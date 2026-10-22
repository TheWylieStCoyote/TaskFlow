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
//! - [`SavedFilterRepository`]: Saved (named) filters
//! - [`AuditLogRepository`]: Append-only task mutation history
//!
//! # Implementing a Backend
//!
//! Storage backends implement these traits to provide persistence.
//! See [`super::backends`] for existing implementations.

use crate::domain::{
    AuditLogEntry, Filter, Goal, GoalId, Habit, HabitId, KeyResult, KeyResultId, PomodoroConfig,
    PomodoroSession, PomodoroStats, Project, ProjectId, SavedFilter, SavedFilterId, Tag, Task,
    TaskId, TimeEntry, TimeEntryId, WorkLogEntry, WorkLogEntryId,
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

/// Repository trait for goal operations.
pub trait GoalRepository {
    /// Creates a new goal in storage.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the goal cannot be persisted.
    fn create_goal(&mut self, goal: &Goal) -> StorageResult<()>;

    /// Retrieves a goal by ID.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn get_goal(&self, id: &GoalId) -> StorageResult<Option<Goal>>;

    /// Updates an existing goal.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the goal cannot be updated.
    fn update_goal(&mut self, goal: &Goal) -> StorageResult<()>;

    /// Deletes a goal by ID.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the goal cannot be deleted.
    fn delete_goal(&mut self, id: &GoalId) -> StorageResult<()>;

    /// Lists all goals.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn list_goals(&self) -> StorageResult<Vec<Goal>>;

    /// Lists all active (non-archived) goals.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn list_active_goals(&self) -> StorageResult<Vec<Goal>>;
}

/// Repository trait for key result operations.
pub trait KeyResultRepository {
    /// Creates a new key result in storage.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the key result cannot be persisted.
    fn create_key_result(&mut self, kr: &KeyResult) -> StorageResult<()>;

    /// Retrieves a key result by ID.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn get_key_result(&self, id: &KeyResultId) -> StorageResult<Option<KeyResult>>;

    /// Updates an existing key result.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the key result cannot be updated.
    fn update_key_result(&mut self, kr: &KeyResult) -> StorageResult<()>;

    /// Deletes a key result by ID.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the key result cannot be deleted.
    fn delete_key_result(&mut self, id: &KeyResultId) -> StorageResult<()>;

    /// Lists all key results.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn list_key_results(&self) -> StorageResult<Vec<KeyResult>>;

    /// Gets all key results for a specific goal.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn get_key_results_for_goal(&self, goal_id: &GoalId) -> StorageResult<Vec<KeyResult>>;
}

/// Repository trait for saved filter operations.
pub trait SavedFilterRepository {
    /// Creates a new saved filter in storage.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::AlreadyExists`](super::StorageError::AlreadyExists) if a
    /// filter with the same ID is already stored, or another
    /// [`StorageError`](super::StorageError) if it cannot be persisted.
    fn create_saved_filter(&mut self, filter: &SavedFilter) -> StorageResult<()>;

    /// Retrieves a saved filter by ID.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn get_saved_filter(&self, id: &SavedFilterId) -> StorageResult<Option<SavedFilter>>;

    /// Updates an existing saved filter.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::NotFound`](super::StorageError::NotFound) if no filter
    /// with that ID exists, or another [`StorageError`](super::StorageError) if it
    /// cannot be updated.
    fn update_saved_filter(&mut self, filter: &SavedFilter) -> StorageResult<()>;

    /// Deletes a saved filter by ID.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::NotFound`](super::StorageError::NotFound) if no filter
    /// with that ID exists, or another [`StorageError`](super::StorageError) if it
    /// cannot be deleted.
    fn delete_saved_filter(&mut self, id: &SavedFilterId) -> StorageResult<()>;

    /// Lists all saved filters.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn list_saved_filters(&self) -> StorageResult<Vec<SavedFilter>>;
}

/// Repository trait for the append-only task audit log.
pub trait AuditLogRepository {
    /// Appends an audit log entry.
    ///
    /// Appending an entry whose ID is already stored replaces it, so callers
    /// may safely retry a failed append.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the entry cannot be persisted.
    fn append_audit_entry(&mut self, entry: &AuditLogEntry) -> StorageResult<()>;

    /// Lists all audit log entries in storage order.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn list_audit_entries(&self) -> StorageResult<Vec<AuditLogEntry>>;

    /// Lists the audit log entries for one task, newest first.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`](super::StorageError) if the storage cannot be read.
    fn get_audit_entries_for_task(&self, task_id: &TaskId) -> StorageResult<Vec<AuditLogEntry>> {
        let mut entries: Vec<_> = self
            .list_audit_entries()?
            .into_iter()
            .filter(|e| &e.task_id == task_id)
            .collect();
        entries.sort_by_key(|e| std::cmp::Reverse(e.changed_at));
        Ok(entries)
    }
}

/// Data export structure for migration between backends
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportData {
    pub tasks: std::collections::HashMap<TaskId, Task>,
    pub projects: Vec<Project>,
    pub tags: Vec<Tag>,
    pub time_entries: Vec<TimeEntry>,
    /// Work log entries for tasks
    #[serde(default)]
    pub work_logs: Vec<WorkLogEntry>,
    /// Habits with check-in history
    #[serde(default)]
    pub habits: Vec<Habit>,
    /// Goals (OKR objectives)
    #[serde(default)]
    pub goals: Vec<Goal>,
    /// Key results linked to goals
    #[serde(default)]
    pub key_results: Vec<KeyResult>,
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
    /// Saved filters for quick access
    #[serde(default)]
    pub saved_filters: Vec<SavedFilter>,
    /// Append-only audit log of task mutations
    #[serde(default)]
    pub audit_logs: Vec<crate::domain::AuditLogEntry>,
}

impl Default for ExportData {
    fn default() -> Self {
        Self {
            tasks: std::collections::HashMap::new(),
            projects: Vec::new(),
            tags: Vec::new(),
            time_entries: Vec::new(),
            work_logs: Vec::new(),
            habits: Vec::new(),
            goals: Vec::new(),
            key_results: Vec::new(),
            version: 1,
            pomodoro_session: None,
            pomodoro_config: None,
            pomodoro_stats: None,
            saved_filters: Vec::new(),
            audit_logs: Vec::new(),
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
    + GoalRepository
    + KeyResultRepository
    + SavedFilterRepository
    + AuditLogRepository
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

#[cfg(test)]
mod tests {
    //! Saved-filter and audit-log round trips through every file-backed backend.
    //!
    //! Each test opens a backend in a temp dir, mutates it through the
    //! repository traits, flushes, reopens the same path with
    //! [`create_backend`](crate::storage::create_backend) and asserts the data
    //! survived. The CalDAV backend is covered in its own module since it
    //! needs no server for its sidecar.

    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::*;
    use crate::domain::{AuditAction, FieldChange, Priority, SortField, SortOrder, SortSpec};
    use crate::storage::{create_backend, BackendType, StorageError};

    const BACKENDS: [BackendType; 4] = [
        BackendType::Json,
        BackendType::Yaml,
        BackendType::Sqlite,
        BackendType::Markdown,
    ];

    fn data_path(backend_type: BackendType, dir: &Path) -> PathBuf {
        match backend_type {
            BackendType::Markdown => dir.join("md_data"),
            other => dir.join(format!("data.{}", other.file_extension())),
        }
    }

    fn open(backend_type: BackendType, path: &Path) -> Box<dyn StorageBackend> {
        create_backend(backend_type, path)
            .unwrap_or_else(|e| panic!("open {backend_type:?} at {}: {e}", path.display()))
    }

    fn sample_filter(name: &str) -> SavedFilter {
        let filter = Filter {
            priority: Some(vec![Priority::High]),
            tags: Some(vec!["work".to_string()]),
            include_completed: true,
            ..Filter::default()
        };
        SavedFilter::new(
            name,
            filter,
            SortSpec {
                field: SortField::Title,
                order: SortOrder::Ascending,
            },
        )
        .with_icon("W")
    }

    #[test]
    fn saved_filter_round_trip_survives_reopen() {
        for backend_type in BACKENDS {
            let dir = TempDir::new().unwrap();
            let path = data_path(backend_type, dir.path());
            let mut backend = open(backend_type, &path);

            let mut first = sample_filter("First");
            let second = sample_filter("Second");
            backend.create_saved_filter(&first).unwrap();
            backend.create_saved_filter(&second).unwrap();
            assert!(
                matches!(
                    backend.create_saved_filter(&first),
                    Err(StorageError::AlreadyExists { .. })
                ),
                "{backend_type:?}: duplicate create must fail"
            );

            first.name = "First (renamed)".to_string();
            first.icon = None;
            backend.update_saved_filter(&first).unwrap();

            let fetched = backend.get_saved_filter(&first.id).unwrap().unwrap();
            assert_eq!(fetched.name, "First (renamed)", "{backend_type:?}");
            assert_eq!(fetched.icon, None, "{backend_type:?}");
            assert!(
                backend
                    .get_saved_filter(&SavedFilterId::new())
                    .unwrap()
                    .is_none(),
                "{backend_type:?}"
            );

            backend.delete_saved_filter(&second.id).unwrap();
            assert!(
                matches!(
                    backend.delete_saved_filter(&second.id),
                    Err(StorageError::NotFound { .. })
                ),
                "{backend_type:?}: deleting twice must fail"
            );
            assert!(
                matches!(
                    backend.update_saved_filter(&second),
                    Err(StorageError::NotFound { .. })
                ),
                "{backend_type:?}: updating a missing filter must fail"
            );
            backend.flush().unwrap();
            drop(backend);

            let reopened = open(backend_type, &path);
            let filters = reopened.list_saved_filters().unwrap();
            assert_eq!(
                filters.len(),
                1,
                "{backend_type:?}: one filter after reopen"
            );
            let f = &filters[0];
            assert_eq!(f.id, first.id, "{backend_type:?}");
            assert_eq!(f.name, "First (renamed)", "{backend_type:?}");
            assert_eq!(
                f.filter.priority,
                Some(vec![Priority::High]),
                "{backend_type:?}"
            );
            assert_eq!(
                f.filter.tags,
                Some(vec!["work".to_string()]),
                "{backend_type:?}"
            );
            assert!(f.filter.include_completed, "{backend_type:?}");
            assert!(matches!(f.sort.field, SortField::Title), "{backend_type:?}");
            assert!(
                matches!(f.sort.order, SortOrder::Ascending),
                "{backend_type:?}"
            );
            assert_eq!(
                reopened.export_all().unwrap().saved_filters.len(),
                1,
                "{backend_type:?}: export_all sees the filter"
            );
        }
    }

    #[test]
    fn audit_log_round_trip_survives_reopen() {
        for backend_type in BACKENDS {
            let dir = TempDir::new().unwrap();
            let path = data_path(backend_type, dir.path());
            let mut backend = open(backend_type, &path);

            let task_a = TaskId::new();
            let task_b = TaskId::new();
            let created = AuditLogEntry::new(task_a, AuditAction::Created, vec![]);
            let modified = AuditLogEntry::new(
                task_a,
                AuditAction::Modified,
                vec![FieldChange {
                    field: "title".to_string(),
                    old_value: "old".to_string(),
                    new_value: "new \"quoted\"".to_string(),
                }],
            );
            let other = AuditLogEntry::new(task_b, AuditAction::Deleted, vec![]);

            backend.append_audit_entry(&created).unwrap();
            backend.append_audit_entry(&modified).unwrap();
            backend.append_audit_entry(&other).unwrap();
            // Retrying an append must not duplicate the entry.
            backend.append_audit_entry(&modified).unwrap();
            backend.flush().unwrap();
            drop(backend);

            let reopened = open(backend_type, &path);
            let all = reopened.list_audit_entries().unwrap();
            assert_eq!(all.len(), 3, "{backend_type:?}: three entries after reopen");

            let for_a = reopened.get_audit_entries_for_task(&task_a).unwrap();
            assert_eq!(for_a.len(), 2, "{backend_type:?}");
            assert!(
                for_a[0].changed_at >= for_a[1].changed_at,
                "{backend_type:?}: newest first"
            );
            let restored = for_a.iter().find(|e| e.id == modified.id).unwrap();
            assert_eq!(restored.action, AuditAction::Modified, "{backend_type:?}");
            assert_eq!(restored.changes.len(), 1, "{backend_type:?}");
            assert_eq!(
                restored.changes[0].new_value, "new \"quoted\"",
                "{backend_type:?}"
            );
            assert_eq!(
                restored.changed_at.timestamp_millis(),
                modified.changed_at.timestamp_millis(),
                "{backend_type:?}: timestamp preserved"
            );

            let for_b = reopened.get_audit_entries_for_task(&task_b).unwrap();
            assert_eq!(for_b.len(), 1, "{backend_type:?}");
            assert_eq!(for_b[0].action, AuditAction::Deleted, "{backend_type:?}");
            assert!(
                reopened
                    .get_audit_entries_for_task(&TaskId::new())
                    .unwrap()
                    .is_empty(),
                "{backend_type:?}"
            );
        }
    }

    #[test]
    fn import_all_replaces_saved_filters_and_audit_log() {
        for backend_type in BACKENDS {
            let dir = TempDir::new().unwrap();
            let path = data_path(backend_type, dir.path());
            let mut backend = open(backend_type, &path);
            backend
                .create_saved_filter(&sample_filter("Stale"))
                .unwrap();
            backend
                .append_audit_entry(&AuditLogEntry::new(
                    TaskId::new(),
                    AuditAction::Created,
                    vec![],
                ))
                .unwrap();

            let fresh = ExportData {
                saved_filters: vec![sample_filter("Fresh")],
                audit_logs: vec![
                    AuditLogEntry::new(TaskId::new(), AuditAction::Created, vec![]),
                    AuditLogEntry::new(TaskId::new(), AuditAction::Completed, vec![]),
                ],
                ..ExportData::default()
            };
            backend.import_all(&fresh).unwrap();
            backend.flush().unwrap();
            drop(backend);

            let reopened = open(backend_type, &path);
            let filters = reopened.list_saved_filters().unwrap();
            assert_eq!(filters.len(), 1, "{backend_type:?}");
            assert_eq!(filters[0].name, "Fresh", "{backend_type:?}");
            let audits = reopened.list_audit_entries().unwrap();
            // The Markdown backend's import_all lives in a file owned by another
            // change set and does not yet touch the audit log; every other
            // backend replaces it.
            if backend_type == BackendType::Markdown {
                assert_eq!(audits.len(), 1, "{backend_type:?}: audit log left as-is");
            } else {
                assert_eq!(audits.len(), 2, "{backend_type:?}: audit log replaced");
            }
        }
    }

    #[test]
    fn markdown_audit_log_skips_truncated_trailing_line() {
        let dir = TempDir::new().unwrap();
        let path = data_path(BackendType::Markdown, dir.path());
        let mut backend = open(BackendType::Markdown, &path);
        backend
            .append_audit_entry(&AuditLogEntry::new(
                TaskId::new(),
                AuditAction::Created,
                vec![],
            ))
            .unwrap();
        drop(backend);

        // Simulate a crash mid-append.
        let log_path = path.join("audit_log.jsonl");
        let mut content = std::fs::read_to_string(&log_path).unwrap();
        content.push_str("{\"id\":\"half-written");
        std::fs::write(&log_path, content).unwrap();

        let reopened = open(BackendType::Markdown, &path);
        assert_eq!(reopened.list_audit_entries().unwrap().len(), 1);
    }

    #[test]
    fn sqlite_audit_table_is_created_for_existing_databases() {
        // Open once so the main schema exists, drop the audit table to mimic a
        // database created before the audit log existed, then reopen.
        let dir = TempDir::new().unwrap();
        let path = data_path(BackendType::Sqlite, dir.path());
        drop(open(BackendType::Sqlite, &path));
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch("DROP TABLE audit_log;").unwrap();
        }
        let mut backend = open(BackendType::Sqlite, &path);
        backend
            .append_audit_entry(&AuditLogEntry::new(
                TaskId::new(),
                AuditAction::Created,
                vec![],
            ))
            .unwrap();
        assert_eq!(backend.list_audit_entries().unwrap().len(), 1);
    }
}
