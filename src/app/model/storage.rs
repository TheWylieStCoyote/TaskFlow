//! Storage-related methods for the Model.

use std::path::PathBuf;

use chrono::Utc;

use crate::domain::{Project, ProjectId, Task, TaskId, TimeEntry};
use crate::storage::{self, BackendType, ProjectRepository};

use super::{Model, UndoAction};

impl Model {
    /// Configures storage and loads existing data.
    ///
    /// Initializes a storage backend and loads any existing tasks and projects.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend fails to initialize or load data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use taskflow::app::Model;
    /// use taskflow::storage::BackendType;
    /// use std::path::PathBuf;
    ///
    /// let model = Model::new()
    ///     .with_storage(BackendType::Json, PathBuf::from("tasks.json"))
    ///     .expect("Failed to load storage");
    /// ```
    pub fn with_storage(
        mut self,
        backend_type: BackendType,
        path: PathBuf,
    ) -> anyhow::Result<Self> {
        let mut backend = storage::create_backend(backend_type, &path)?;

        // Load tasks from storage
        let tasks = backend.list_tasks()?;
        for task in tasks {
            self.tasks.insert(task.id.clone(), task);
        }

        // Load projects from storage
        let projects = ProjectRepository::list_projects(backend.as_mut())?;
        for project in projects {
            self.projects.insert(project.id.clone(), project);
        }

        // Load time entries from storage
        let export_data = backend.export_all()?;
        for entry in export_data.time_entries {
            // Track active entry if still running
            if entry.is_running() {
                self.active_time_entry = Some(entry.id.clone());
            }
            self.time_entries.insert(entry.id.clone(), entry);
        }

        // Load Pomodoro state
        if let Some(mut session) = export_data.pomodoro_session {
            // Recalculate remaining time based on elapsed time since last save
            let config = export_data
                .pomodoro_config
                .as_ref()
                .unwrap_or(&self.pomodoro_config);
            session.recalculate_remaining_time(config);

            // Validate that the task still exists
            if self.tasks.contains_key(&session.task_id) {
                self.pomodoro_session = Some(session);
            }
            // If task doesn't exist, discard the session
        }
        if let Some(config) = export_data.pomodoro_config {
            self.pomodoro_config = config;
        }
        if let Some(stats) = export_data.pomodoro_stats {
            self.pomodoro_stats = stats;
        }

        self.storage = Some(backend);
        self.data_path = Some(path);
        self.refresh_visible_tasks();

        Ok(self)
    }

    /// Saves current state to storage.
    ///
    /// Flushes any pending changes to the configured storage backend.
    /// Clears the dirty flag on success.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage backend fails to flush data.
    pub fn save(&mut self) -> anyhow::Result<()> {
        if let Some(ref mut backend) = self.storage {
            // Sync Pomodoro state before flushing
            backend.set_pomodoro_session(self.pomodoro_session.as_ref())?;
            backend.set_pomodoro_config(&self.pomodoro_config)?;
            backend.set_pomodoro_stats(&self.pomodoro_stats)?;

            backend.flush()?;
            self.dirty = false;
        }
        Ok(())
    }

    /// Syncs a task change to storage.
    ///
    /// Creates or updates the task in the storage backend.
    /// Sets the dirty flag to indicate unsaved changes.
    pub fn sync_task(&mut self, task: &Task) {
        if let Some(ref mut backend) = self.storage {
            // Try update first, if not found, create
            if backend.update_task(task).is_err() {
                let _ = backend.create_task(task);
            }
            self.dirty = true;
        }
    }

    /// Deletes a task from storage.
    ///
    /// Removes the task from the storage backend.
    pub fn delete_task_from_storage(&mut self, id: &TaskId) {
        if let Some(ref mut backend) = self.storage {
            let _ = backend.delete_task(id);
            self.dirty = true;
        }
    }

    /// Syncs a project to storage.
    ///
    /// Creates or updates the project in the storage backend.
    pub fn sync_project(&mut self, project: &Project) {
        if let Some(ref mut backend) = self.storage {
            // Try update first, if not found, create
            if backend.update_project(project).is_err() {
                let _ = backend.create_project(project);
            }
            self.dirty = true;
        }
    }

    /// Syncs a time entry to storage.
    ///
    /// Creates or updates the time entry in the storage backend.
    pub fn sync_time_entry(&mut self, entry: &TimeEntry) {
        if let Some(ref mut backend) = self.storage {
            // Try update first, if not found, create
            if backend.update_time_entry(entry).is_err() {
                let _ = backend.create_time_entry(entry);
            }
            self.dirty = true;
        }
    }

    /// Refreshes storage to detect external file changes.
    ///
    /// Scans for files that have been modified, added, or deleted externally
    /// and reloads the data from storage. Returns the number of changes detected.
    ///
    /// This is primarily useful for the markdown backend when files are edited
    /// externally (e.g., by a text editor or git operations).
    pub fn refresh_storage(&mut self) -> usize {
        if let Some(ref mut backend) = self.storage {
            let changes = backend.refresh();
            if changes > 0 {
                // Reload all data from storage
                if let Ok(tasks) = backend.list_tasks() {
                    self.tasks.clear();
                    for task in tasks {
                        self.tasks.insert(task.id.clone(), task);
                    }
                }
                if let Ok(projects) = ProjectRepository::list_projects(backend.as_mut()) {
                    self.projects.clear();
                    for project in projects {
                        self.projects.insert(project.id.clone(), project);
                    }
                }
                self.refresh_visible_tasks();
            }
            changes
        } else {
            0
        }
    }

    /// Returns true if a storage backend is configured.
    #[must_use]
    pub fn has_storage(&self) -> bool {
        self.storage.is_some()
    }

    /// Modifies a task with undo support.
    ///
    /// This helper method centralizes the common pattern of:
    /// 1. Cloning the task for the "before" state
    /// 2. Applying modifications via the provided closure
    /// 3. Setting the updated_at timestamp
    /// 4. Syncing to storage
    /// 5. Pushing the undo action
    ///
    /// Returns true if the task was found and modified, false otherwise.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The ID of the task to modify
    /// * `modifier` - A closure that takes a mutable reference to the task and modifies it
    pub fn modify_task_with_undo<F>(&mut self, task_id: &TaskId, modifier: F) -> bool
    where
        F: FnOnce(&mut Task),
    {
        if let Some(task) = self.tasks.get_mut(task_id) {
            let before = task.clone();
            modifier(task);
            task.updated_at = Utc::now();
            let after = task.clone();
            self.sync_task(&after);
            self.undo_stack.push(UndoAction::TaskModified {
                before: Box::new(before),
                after: Box::new(after),
            });
            true
        } else {
            false
        }
    }

    /// Modifies a project with undo support.
    ///
    /// Similar to `modify_task_with_undo`, this centralizes the project modification pattern.
    ///
    /// Returns true if the project was found and modified, false otherwise.
    pub fn modify_project_with_undo<F>(&mut self, project_id: &ProjectId, modifier: F) -> bool
    where
        F: FnOnce(&mut Project),
    {
        if let Some(project) = self.projects.get_mut(project_id) {
            let before = project.clone();
            modifier(project);
            project.updated_at = Utc::now();
            let after = project.clone();
            self.sync_project(&after);
            self.undo_stack.push(UndoAction::ProjectModified {
                before: Box::new(before),
                after: Box::new(after),
            });
            true
        } else {
            false
        }
    }
}
