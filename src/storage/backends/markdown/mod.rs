//! Markdown file-based storage backend.
//!
//! Stores tasks as individual markdown files with YAML frontmatter.
//! Great for version control, manual editing, and integration with
//! other markdown-based tools.
//!
//! Directory structure:
//! ```text
//! data_dir/
//!   tasks/
//!     <uuid>.md
//!   projects/
//!     <uuid>.md
//!   tags.yaml
//!   time_entries.yaml
//! ```

mod cache;
mod file_io;
mod habit_repo;
mod project_repo;
mod storage;
mod tag_repo;
mod task_repo;
mod time_entry_repo;
mod work_log_repo;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::domain::{
    Habit, PomodoroConfig, PomodoroSession, PomodoroStats, Project, ProjectId, SavedFilter, Tag,
    Task, TaskId, TimeEntry, WorkLogEntry,
};
use crate::storage::StorageResult;

/// Pomodoro state stored in YAML.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct PomodoroState {
    pub(crate) session: Option<PomodoroSession>,
    pub(crate) config: Option<PomodoroConfig>,
    pub(crate) stats: Option<PomodoroStats>,
}

/// Markdown file-based storage backend.
///
/// Stores tasks and projects as individual markdown files with YAML frontmatter.
/// Auxiliary data (tags, time entries, work logs) is stored in YAML files.
pub struct MarkdownBackend {
    pub(crate) base_path: PathBuf,
    pub(crate) tasks_dir: PathBuf,
    pub(crate) projects_dir: PathBuf,
    // Cache for performance
    pub(crate) tasks_cache: HashMap<TaskId, Task>,
    pub(crate) projects_cache: HashMap<ProjectId, Project>,
    // Track file modification times for cache invalidation
    pub(crate) task_mtimes: HashMap<TaskId, SystemTime>,
    pub(crate) project_mtimes: HashMap<ProjectId, SystemTime>,
    pub(crate) tags: Vec<Tag>,
    pub(crate) time_entries: Vec<TimeEntry>,
    pub(crate) work_logs: Vec<WorkLogEntry>,
    pub(crate) habits: Vec<Habit>,
    pub(crate) saved_filters: Vec<SavedFilter>,
    pub(crate) pomodoro_state: PomodoroState,
    pub(crate) dirty: bool,
}

impl MarkdownBackend {
    /// Creates a new Markdown backend at the given path.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::storage::StorageError`] if the backend cannot be created.
    pub fn new(path: &Path) -> StorageResult<Self> {
        Ok(Self {
            base_path: path.to_path_buf(),
            tasks_dir: path.join("tasks"),
            projects_dir: path.join("projects"),
            tasks_cache: HashMap::new(),
            projects_cache: HashMap::new(),
            task_mtimes: HashMap::new(),
            project_mtimes: HashMap::new(),
            tags: Vec::new(),
            time_entries: Vec::new(),
            work_logs: Vec::new(),
            habits: Vec::new(),
            saved_filters: Vec::new(),
            pomodoro_state: PomodoroState::default(),
            dirty: false,
        })
    }

    #[allow(dead_code)]
    pub(crate) const fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Refresh the cache by checking for external changes.
    /// Returns the total number of changes detected.
    pub fn refresh(&mut self) -> usize {
        let task_changes = self.scan_for_task_changes();
        let project_changes = self.scan_for_project_changes();
        task_changes + project_changes
    }
}
