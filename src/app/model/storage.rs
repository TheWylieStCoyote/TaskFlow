//! Storage-related methods for the Model.

use std::path::PathBuf;

use chrono::Utc;
use tracing::warn;

use crate::domain::{
    Goal, GoalId, Habit, HabitId, KeyResult, KeyResultId, Project, ProjectId, Task, TaskId,
    TimeEntry, WorkLogEntry, WorkLogEntryId,
};
use crate::storage::{
    self, BackendType, GoalRepository, HabitRepository, KeyResultRepository, ProjectRepository,
    WorkLogRepository,
};

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
            self.tasks.insert(task.id, task);
        }

        // Load projects from storage
        let projects = ProjectRepository::list_projects(backend.as_mut())?;
        for project in projects {
            self.projects.insert(project.id, project);
        }

        // Load time entries from storage
        let export_data = backend.export_all()?;
        for entry in export_data.time_entries {
            // Track active entry if still running
            if entry.is_running() {
                self.active_time_entry = Some(entry.id);
            }
            self.time_entries.insert(entry.id, entry);
        }

        // Load work log entries from storage
        for work_log in export_data.work_logs {
            self.work_logs.insert(work_log.id, work_log);
        }

        // Load habits from storage
        for habit in export_data.habits {
            self.habits.insert(habit.id, habit);
        }
        self.refresh_visible_habits();

        // Load goals and key results from storage
        for goal in export_data.goals {
            self.goals.insert(goal.id, goal);
        }
        for kr in export_data.key_results {
            self.key_results.insert(kr.id, kr);
        }
        self.refresh_visible_goals();

        // Load Pomodoro state
        if let Some(mut session) = export_data.pomodoro_session {
            // Recalculate remaining time based on elapsed time since last save
            let config = export_data
                .pomodoro_config
                .as_ref()
                .unwrap_or(&self.pomodoro.config);
            session.recalculate_remaining_time(config);

            // Validate that the task still exists
            if self.tasks.contains_key(&session.task_id) {
                self.pomodoro.session = Some(session);
            }
            // If task doesn't exist, discard the session
        }
        if let Some(config) = export_data.pomodoro_config {
            self.pomodoro.config = config;
        }
        if let Some(stats) = export_data.pomodoro_stats {
            self.pomodoro.stats = stats;
        }

        // Load saved filters
        for filter in export_data.saved_filters {
            self.saved_filters.insert(filter.id.clone(), filter);
        }

        self.storage.backend = Some(backend);
        self.storage.data_path = Some(path);
        self.refresh_visible_tasks();

        Ok(self)
    }

    /// Saves current state to storage.
    ///
    /// Flushes any pending changes to the configured storage backend.
    /// Clears the dirty flag on success.
    /// Does nothing in sample data mode.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage backend fails to flush data.
    pub fn save(&mut self) -> anyhow::Result<()> {
        // Don't save in sample data mode
        if self.storage.sample_data_mode {
            return Ok(());
        }
        if let Some(ref mut backend) = self.storage.backend {
            // Sync Pomodoro state before flushing
            backend.set_pomodoro_session(self.pomodoro.session.as_ref())?;
            backend.set_pomodoro_config(&self.pomodoro.config)?;
            backend.set_pomodoro_stats(&self.pomodoro.stats)?;

            backend.flush()?;
            self.storage.dirty = false;
        }
        Ok(())
    }

    /// Syncs a task change to storage.
    ///
    /// Creates or updates the task in the storage backend.
    /// Sets the dirty flag to indicate unsaved changes.
    /// Does nothing in sample data mode.
    pub fn sync_task(&mut self, task: &Task) {
        if self.storage.sample_data_mode {
            return;
        }
        if let Some(ref mut backend) = self.storage.backend {
            // Try update first, if not found, create
            if let Err(e) = backend.update_task(task) {
                if let Err(e2) = backend.create_task(task) {
                    warn!(
                        "Failed to sync task {}: update={}, create={}",
                        task.id, e, e2
                    );
                }
            }
            self.storage.dirty = true;
        }
    }

    /// Syncs a task by ID to storage.
    ///
    /// Looks up the task in the model and syncs it to the storage backend.
    /// This avoids the need to clone the task when you have a mutable borrow.
    /// Does nothing in sample data mode.
    pub fn sync_task_by_id(&mut self, task_id: &TaskId) {
        if self.storage.sample_data_mode {
            return;
        }
        if let (Some(ref mut backend), Some(task)) =
            (&mut self.storage.backend, self.tasks.get(task_id))
        {
            // Try update first, if not found, create
            if let Err(e) = backend.update_task(task) {
                if let Err(e2) = backend.create_task(task) {
                    warn!(
                        "Failed to sync task {}: update={}, create={}",
                        task_id, e, e2
                    );
                }
            }
            self.storage.dirty = true;
        }
    }

    /// Deletes a task from storage.
    ///
    /// Removes the task from the storage backend.
    /// Does nothing in sample data mode.
    pub fn delete_task_from_storage(&mut self, id: &TaskId) {
        if self.storage.sample_data_mode {
            return;
        }
        if let Some(ref mut backend) = self.storage.backend {
            if let Err(e) = backend.delete_task(id) {
                warn!("Failed to delete task {} from storage: {}", id, e);
            }
            self.storage.dirty = true;
        }
    }

    /// Syncs a project to storage.
    ///
    /// Creates or updates the project in the storage backend.
    /// Does nothing in sample data mode.
    pub fn sync_project(&mut self, project: &Project) {
        if self.storage.sample_data_mode {
            return;
        }
        if let Some(ref mut backend) = self.storage.backend {
            // Try update first, if not found, create
            if let Err(e) = backend.update_project(project) {
                if let Err(e2) = backend.create_project(project) {
                    warn!(
                        "Failed to sync project {}: update={}, create={}",
                        project.id, e, e2
                    );
                }
            }
            self.storage.dirty = true;
        }
    }

    /// Syncs a project by ID to storage.
    ///
    /// Looks up the project in the model and syncs it to the storage backend.
    /// This avoids the need to clone the project when you have a mutable borrow.
    /// Does nothing in sample data mode.
    pub fn sync_project_by_id(&mut self, project_id: &ProjectId) {
        if self.storage.sample_data_mode {
            return;
        }
        if let (Some(ref mut backend), Some(project)) =
            (&mut self.storage.backend, self.projects.get(project_id))
        {
            // Try update first, if not found, create
            if let Err(e) = backend.update_project(project) {
                if let Err(e2) = backend.create_project(project) {
                    warn!(
                        "Failed to sync project {}: update={}, create={}",
                        project_id, e, e2
                    );
                }
            }
            self.storage.dirty = true;
        }
    }

    /// Syncs a habit to storage.
    ///
    /// Creates or updates the habit in the storage backend.
    /// Does nothing in sample data mode.
    pub fn sync_habit(&mut self, habit: &Habit) {
        if self.storage.sample_data_mode {
            return;
        }
        if let Some(ref mut backend) = self.storage.backend {
            // Try update first, if not found, create
            if let Err(e) = HabitRepository::update_habit(backend.as_mut(), habit) {
                if let Err(e2) = HabitRepository::create_habit(backend.as_mut(), habit) {
                    warn!(
                        "Failed to sync habit {}: update={}, create={}",
                        habit.id, e, e2
                    );
                }
            }
            self.storage.dirty = true;
        }
    }

    /// Syncs a habit by ID to storage.
    ///
    /// Looks up the habit in the model and syncs it to the storage backend.
    /// Does nothing in sample data mode.
    pub fn sync_habit_by_id(&mut self, habit_id: &HabitId) {
        if self.storage.sample_data_mode {
            return;
        }
        if let (Some(ref mut backend), Some(habit)) =
            (&mut self.storage.backend, self.habits.get(habit_id))
        {
            // Try update first, if not found, create
            if let Err(e) = HabitRepository::update_habit(backend.as_mut(), habit) {
                if let Err(e2) = HabitRepository::create_habit(backend.as_mut(), habit) {
                    warn!(
                        "Failed to sync habit {}: update={}, create={}",
                        habit_id, e, e2
                    );
                }
            }
            self.storage.dirty = true;
        }
    }

    /// Deletes a habit from storage.
    ///
    /// Removes the habit from the storage backend.
    /// Does nothing in sample data mode.
    pub fn delete_habit_from_storage(&mut self, id: &HabitId) {
        if self.storage.sample_data_mode {
            return;
        }
        if let Some(ref mut backend) = self.storage.backend {
            if let Err(e) = HabitRepository::delete_habit(backend.as_mut(), id) {
                warn!("Failed to delete habit {} from storage: {}", id, e);
            }
            self.storage.dirty = true;
        }
    }

    /// Syncs a goal to storage.
    ///
    /// Creates or updates the goal in the storage backend.
    /// Does nothing in sample data mode.
    pub fn sync_goal(&mut self, goal: &Goal) {
        if self.storage.sample_data_mode {
            return;
        }
        if let Some(ref mut backend) = self.storage.backend {
            // Try update first, if not found, create
            if let Err(e) = GoalRepository::update_goal(backend.as_mut(), goal) {
                if let Err(e2) = GoalRepository::create_goal(backend.as_mut(), goal) {
                    warn!(
                        "Failed to sync goal {}: update={}, create={}",
                        goal.id, e, e2
                    );
                }
            }
            self.storage.dirty = true;
        }
    }

    /// Syncs a goal by ID to storage.
    ///
    /// Looks up the goal in the model and syncs it to the storage backend.
    /// Does nothing in sample data mode.
    pub fn sync_goal_by_id(&mut self, goal_id: &GoalId) {
        if self.storage.sample_data_mode {
            return;
        }
        if let (Some(ref mut backend), Some(goal)) =
            (&mut self.storage.backend, self.goals.get(goal_id))
        {
            // Try update first, if not found, create
            if let Err(e) = GoalRepository::update_goal(backend.as_mut(), goal) {
                if let Err(e2) = GoalRepository::create_goal(backend.as_mut(), goal) {
                    warn!(
                        "Failed to sync goal {}: update={}, create={}",
                        goal_id, e, e2
                    );
                }
            }
            self.storage.dirty = true;
        }
    }

    /// Deletes a goal from storage.
    ///
    /// Removes the goal from the storage backend.
    /// Does nothing in sample data mode.
    pub fn delete_goal_from_storage(&mut self, id: &GoalId) {
        if self.storage.sample_data_mode {
            return;
        }
        if let Some(ref mut backend) = self.storage.backend {
            if let Err(e) = GoalRepository::delete_goal(backend.as_mut(), id) {
                warn!("Failed to delete goal {} from storage: {}", id, e);
            }
            self.storage.dirty = true;
        }
    }

    /// Syncs a key result to storage.
    ///
    /// Creates or updates the key result in the storage backend.
    /// Does nothing in sample data mode.
    pub fn sync_key_result(&mut self, kr: &KeyResult) {
        if self.storage.sample_data_mode {
            return;
        }
        if let Some(ref mut backend) = self.storage.backend {
            // Try update first, if not found, create
            if let Err(e) = KeyResultRepository::update_key_result(backend.as_mut(), kr) {
                if let Err(e2) = KeyResultRepository::create_key_result(backend.as_mut(), kr) {
                    warn!(
                        "Failed to sync key result {}: update={}, create={}",
                        kr.id, e, e2
                    );
                }
            }
            self.storage.dirty = true;
        }
    }

    /// Syncs a key result by ID to storage.
    ///
    /// Looks up the key result in the model and syncs it to the storage backend.
    /// Does nothing in sample data mode.
    pub fn sync_key_result_by_id(&mut self, kr_id: &KeyResultId) {
        if self.storage.sample_data_mode {
            return;
        }
        if let (Some(ref mut backend), Some(kr)) =
            (&mut self.storage.backend, self.key_results.get(kr_id))
        {
            // Try update first, if not found, create
            if let Err(e) = KeyResultRepository::update_key_result(backend.as_mut(), kr) {
                if let Err(e2) = KeyResultRepository::create_key_result(backend.as_mut(), kr) {
                    warn!(
                        "Failed to sync key result {}: update={}, create={}",
                        kr_id, e, e2
                    );
                }
            }
            self.storage.dirty = true;
        }
    }

    /// Deletes a key result from storage.
    ///
    /// Removes the key result from the storage backend.
    /// Does nothing in sample data mode.
    pub fn delete_key_result_from_storage(&mut self, id: &KeyResultId) {
        if self.storage.sample_data_mode {
            return;
        }
        if let Some(ref mut backend) = self.storage.backend {
            if let Err(e) = KeyResultRepository::delete_key_result(backend.as_mut(), id) {
                warn!("Failed to delete key result {} from storage: {}", id, e);
            }
            self.storage.dirty = true;
        }
    }

    /// Syncs a time entry to storage.
    ///
    /// Creates or updates the time entry in the storage backend.
    pub fn sync_time_entry(&mut self, entry: &TimeEntry) {
        if self.storage.sample_data_mode {
            return;
        }
        if let Some(ref mut backend) = self.storage.backend {
            // Try update first, if not found, create
            if let Err(e) = backend.update_time_entry(entry) {
                if let Err(e2) = backend.create_time_entry(entry) {
                    warn!(
                        "Failed to sync time entry {}: update={}, create={}",
                        entry.id, e, e2
                    );
                }
            }
            self.storage.dirty = true;
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
        let changes = if let Some(ref mut backend) = self.storage.backend {
            let changes = backend.refresh();
            if changes > 0 {
                // Reload all data from storage
                if let Ok(tasks) = backend.list_tasks() {
                    self.tasks.clear();
                    for task in tasks {
                        self.tasks.insert(task.id, task);
                    }
                }
                if let Ok(projects) = ProjectRepository::list_projects(backend.as_mut()) {
                    self.projects.clear();
                    for project in projects {
                        self.projects.insert(project.id, project);
                    }
                }
                if let Ok(habits) = HabitRepository::list_habits(backend.as_mut()) {
                    self.habits.clear();
                    for habit in habits {
                        self.habits.insert(habit.id, habit);
                    }
                }
                if let Ok(goals) = GoalRepository::list_goals(backend.as_mut()) {
                    self.goals.clear();
                    for goal in goals {
                        self.goals.insert(goal.id, goal);
                    }
                }
                if let Ok(key_results) = KeyResultRepository::list_key_results(backend.as_mut()) {
                    self.key_results.clear();
                    for kr in key_results {
                        self.key_results.insert(kr.id, kr);
                    }
                }
            }
            changes
        } else {
            0
        };

        // Refresh visible lists outside the backend borrow
        if changes > 0 {
            self.refresh_visible_habits();
            self.refresh_visible_goals();
            self.refresh_visible_tasks();
        }

        changes
    }

    /// Returns true if a storage backend is configured.
    #[must_use]
    pub fn has_storage(&self) -> bool {
        self.storage.backend.is_some()
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

    /// Syncs a work log entry to storage.
    ///
    /// Creates or updates the work log entry in the storage backend.
    pub fn sync_work_log(&mut self, entry: &WorkLogEntry) {
        if self.storage.sample_data_mode {
            return;
        }
        if let Some(ref mut backend) = self.storage.backend {
            // Try update first, if not found, create
            if let Err(e) = WorkLogRepository::update_work_log(backend.as_mut(), entry) {
                if let Err(e2) = WorkLogRepository::create_work_log(backend.as_mut(), entry) {
                    warn!(
                        "Failed to sync work log {}: update={}, create={}",
                        entry.id, e, e2
                    );
                }
            }
            self.storage.dirty = true;
        }
    }

    /// Deletes a work log entry from storage.
    ///
    /// Removes the work log entry from the storage backend.
    pub fn delete_work_log_from_storage(&mut self, id: &WorkLogEntryId) {
        if self.storage.sample_data_mode {
            return;
        }
        if let Some(ref mut backend) = self.storage.backend {
            if let Err(e) = WorkLogRepository::delete_work_log(backend.as_mut(), id) {
                warn!("Failed to delete work log {} from storage: {}", id, e);
            }
            self.storage.dirty = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // has_storage tests
    // ========================================================================

    #[test]
    fn test_has_storage_default() {
        let model = Model::new();
        assert!(!model.has_storage());
    }

    // ========================================================================
    // modify_task_with_undo tests
    // ========================================================================

    #[test]
    fn test_modify_task_with_undo_changes_task() {
        let mut model = Model::new();

        let task = Task::new("Original title");
        let task_id = task.id;
        model.tasks.insert(task.id, task);

        let result = model.modify_task_with_undo(&task_id, |t| {
            t.title = "Modified title".to_string();
        });

        assert!(result);
        assert_eq!(model.tasks.get(&task_id).unwrap().title, "Modified title");
    }

    #[test]
    fn test_modify_task_with_undo_updates_timestamp() {
        let mut model = Model::new();

        let task = Task::new("Test");
        let task_id = task.id;
        let original_updated_at = task.updated_at;
        model.tasks.insert(task.id, task);

        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));

        model.modify_task_with_undo(&task_id, |t| {
            t.title = "Modified".to_string();
        });

        let modified_task = model.tasks.get(&task_id).unwrap();
        assert!(modified_task.updated_at > original_updated_at);
    }

    #[test]
    fn test_modify_task_with_undo_pushes_undo_action() {
        let mut model = Model::new();

        let task = Task::new("Test");
        let task_id = task.id;
        model.tasks.insert(task.id, task);

        assert!(model.undo_stack.is_empty());

        model.modify_task_with_undo(&task_id, |t| {
            t.title = "Modified".to_string();
        });

        assert_eq!(model.undo_stack.len(), 1);
        match model.undo_stack.peek() {
            Some(UndoAction::TaskModified { before, after }) => {
                assert_eq!(before.title, "Test");
                assert_eq!(after.title, "Modified");
            }
            _ => panic!("Expected TaskModified undo action"),
        }
    }

    #[test]
    fn test_modify_task_with_undo_nonexistent_returns_false() {
        let mut model = Model::new();
        let random_id = TaskId::new();

        let result = model.modify_task_with_undo(&random_id, |t| {
            t.title = "Modified".to_string();
        });

        assert!(!result);
        assert!(model.undo_stack.is_empty());
    }

    // ========================================================================
    // modify_project_with_undo tests
    // ========================================================================

    #[test]
    fn test_modify_project_with_undo_changes_project() {
        let mut model = Model::new();

        let project = Project::new("Original name");
        let project_id = project.id;
        model.projects.insert(project.id, project);

        let result = model.modify_project_with_undo(&project_id, |p| {
            p.name = "Modified name".to_string();
        });

        assert!(result);
        assert_eq!(
            model.projects.get(&project_id).unwrap().name,
            "Modified name"
        );
    }

    #[test]
    fn test_modify_project_with_undo_updates_timestamp() {
        let mut model = Model::new();

        let project = Project::new("Test");
        let project_id = project.id;
        let original_updated_at = project.updated_at;
        model.projects.insert(project.id, project);

        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));

        model.modify_project_with_undo(&project_id, |p| {
            p.name = "Modified".to_string();
        });

        let modified_project = model.projects.get(&project_id).unwrap();
        assert!(modified_project.updated_at > original_updated_at);
    }

    #[test]
    fn test_modify_project_with_undo_pushes_undo_action() {
        let mut model = Model::new();

        let project = Project::new("Test");
        let project_id = project.id;
        model.projects.insert(project.id, project);

        assert!(model.undo_stack.is_empty());

        model.modify_project_with_undo(&project_id, |p| {
            p.name = "Modified".to_string();
        });

        assert_eq!(model.undo_stack.len(), 1);
        match model.undo_stack.peek() {
            Some(UndoAction::ProjectModified { before, after }) => {
                assert_eq!(before.name, "Test");
                assert_eq!(after.name, "Modified");
            }
            _ => panic!("Expected ProjectModified undo action"),
        }
    }

    #[test]
    fn test_modify_project_with_undo_nonexistent_returns_false() {
        let mut model = Model::new();
        let random_id = ProjectId::new();

        let result = model.modify_project_with_undo(&random_id, |p| {
            p.name = "Modified".to_string();
        });

        assert!(!result);
        assert!(model.undo_stack.is_empty());
    }

    // ========================================================================
    // sample_data_mode tests
    // ========================================================================

    #[test]
    fn test_sample_data_mode_prevents_persistence() {
        let model = Model::new().with_sample_data();
        assert!(model.storage.sample_data_mode);
        assert!(model.storage.backend.is_none());
    }

    #[test]
    fn test_sample_data_mode_default_is_false() {
        let model = Model::new();
        assert!(!model.storage.sample_data_mode);
    }

    #[test]
    fn test_with_sample_data_clears_existing_backend() {
        // Even if we somehow had a backend set, with_sample_data should clear it
        let mut model = Model::new();
        model.storage.sample_data_mode = false;
        // Simulate having data
        model.tasks.insert(TaskId::new(), Task::new("Test"));

        // Calling with_sample_data should clear backend and set sample_data_mode
        let model = model.with_sample_data();
        assert!(model.storage.sample_data_mode);
        assert!(model.storage.backend.is_none());
    }
}
