//! File I/O operations for markdown backend.
//!
//! Handles reading, writing, and parsing of markdown files with YAML frontmatter.
//!
//! Auxiliary collections (tags, time entries, goals, ...) live in one YAML file
//! each. Loading them is strict: a file that exists but does not parse makes
//! `initialize()` fail with [`StorageError::CorruptFile`] instead of being
//! read as empty, because the next `flush()` would otherwise write that empty
//! collection over the only copy of the data.

use crate::domain::{Project, ProjectId, Task, TaskId};
use crate::storage::atomic_write;
use crate::storage::{StorageError, StorageResult};
use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;
use tracing::warn;

use super::MarkdownBackend;

/// Load one auxiliary YAML file.
///
/// A missing or blank file is `T::default()`. A file with content that does
/// not deserialize is a [`StorageError::CorruptFile`] naming the path and the
/// parse error; the file itself is not touched.
fn load_yaml_file<T: DeserializeOwned + Default>(path: &Path) -> StorageResult<T> {
    if !path.exists() {
        return Ok(T::default());
    }
    let content = fs::read_to_string(path).map_err(|e| StorageError::io(path, e))?;
    if content.trim().is_empty() {
        return Ok(T::default());
    }
    serde_yaml::from_str(&content).map_err(|e| StorageError::corrupt_file(path, e.to_string()))
}

impl MarkdownBackend {
    /// Ensure the required directories exist.
    pub(crate) fn ensure_dirs(&self) -> StorageResult<()> {
        fs::create_dir_all(&self.tasks_dir).map_err(|e| StorageError::io(&self.tasks_dir, e))?;
        fs::create_dir_all(&self.projects_dir)
            .map_err(|e| StorageError::io(&self.projects_dir, e))?;
        Ok(())
    }

    /// Load all tasks from the tasks directory.
    pub(crate) fn load_tasks(&mut self) -> StorageResult<()> {
        self.tasks_cache.clear();
        self.task_mtimes.clear();

        if !self.tasks_dir.exists() {
            return Ok(());
        }

        for entry in
            fs::read_dir(&self.tasks_dir).map_err(|e| StorageError::io(&self.tasks_dir, e))?
        {
            let entry = entry.map_err(|e| StorageError::io(&self.tasks_dir, e))?;
            let path = entry.path();

            if path.extension().is_some_and(|e| e == "md") {
                match self.parse_task_file(&path) {
                    Ok(task) => {
                        // Track file modification time
                        if let Ok(metadata) = fs::metadata(&path) {
                            if let Ok(mtime) = metadata.modified() {
                                self.task_mtimes.insert(task.id, mtime);
                            }
                        }
                        self.tasks_cache.insert(task.id, task);
                    }
                    Err(e) => warn!(
                        path = %path.display(),
                        error = %e,
                        "Skipping unparsable task file"
                    ),
                }
            }
        }

        Ok(())
    }

    /// Load all projects from the projects directory.
    pub(crate) fn load_projects(&mut self) -> StorageResult<()> {
        self.projects_cache.clear();
        self.project_mtimes.clear();

        if !self.projects_dir.exists() {
            return Ok(());
        }

        for entry in
            fs::read_dir(&self.projects_dir).map_err(|e| StorageError::io(&self.projects_dir, e))?
        {
            let entry = entry.map_err(|e| StorageError::io(&self.projects_dir, e))?;
            let path = entry.path();

            if path.extension().is_some_and(|e| e == "md") {
                match self.parse_project_file(&path) {
                    Ok(project) => {
                        // Track file modification time
                        if let Ok(metadata) = fs::metadata(&path) {
                            if let Ok(mtime) = metadata.modified() {
                                self.project_mtimes.insert(project.id, mtime);
                            }
                        }
                        self.projects_cache.insert(project.id, project);
                    }
                    Err(e) => warn!(
                        path = %path.display(),
                        error = %e,
                        "Skipping unparsable project file"
                    ),
                }
            }
        }

        Ok(())
    }

    /// Load tags from tags.yaml.
    pub(crate) fn load_tags(&mut self) -> StorageResult<()> {
        self.tags = load_yaml_file(&self.base_path.join("tags.yaml"))?;
        Ok(())
    }

    /// Load time entries from time_entries.yaml.
    pub(crate) fn load_time_entries(&mut self) -> StorageResult<()> {
        self.time_entries = load_yaml_file(&self.base_path.join("time_entries.yaml"))?;
        Ok(())
    }

    /// Save tags to tags.yaml.
    pub(crate) fn save_tags(&self) -> StorageResult<()> {
        let tags_file = self.base_path.join("tags.yaml");
        let content = serde_yaml::to_string(&self.tags)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        atomic_write(&tags_file, content)?;
        Ok(())
    }

    /// Save time entries to time_entries.yaml.
    pub(crate) fn save_time_entries(&self) -> StorageResult<()> {
        let entries_file = self.base_path.join("time_entries.yaml");
        let content = serde_yaml::to_string(&self.time_entries)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        atomic_write(&entries_file, content)?;
        Ok(())
    }

    /// Load work logs from work_logs.yaml.
    pub(crate) fn load_work_logs(&mut self) -> StorageResult<()> {
        self.work_logs = load_yaml_file(&self.base_path.join("work_logs.yaml"))?;
        Ok(())
    }

    /// Save work logs to work_logs.yaml.
    pub(crate) fn save_work_logs(&self) -> StorageResult<()> {
        let work_logs_file = self.base_path.join("work_logs.yaml");
        let content = serde_yaml::to_string(&self.work_logs)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        atomic_write(&work_logs_file, content)?;
        Ok(())
    }

    /// Load habits from habits.yaml.
    pub(crate) fn load_habits(&mut self) -> StorageResult<()> {
        self.habits = load_yaml_file(&self.base_path.join("habits.yaml"))?;
        Ok(())
    }

    /// Save habits to habits.yaml.
    pub(crate) fn save_habits(&self) -> StorageResult<()> {
        let habits_file = self.base_path.join("habits.yaml");
        let content = serde_yaml::to_string(&self.habits)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        atomic_write(&habits_file, content)?;
        Ok(())
    }

    /// Load goals from goals.yaml.
    pub(crate) fn load_goals(&mut self) -> StorageResult<()> {
        self.goals = load_yaml_file(&self.base_path.join("goals.yaml"))?;
        Ok(())
    }

    /// Save goals to goals.yaml.
    pub(crate) fn save_goals(&self) -> StorageResult<()> {
        let goals_file = self.base_path.join("goals.yaml");
        let content = serde_yaml::to_string(&self.goals)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        atomic_write(&goals_file, content)?;
        Ok(())
    }

    /// Load key results from key_results.yaml.
    pub(crate) fn load_key_results(&mut self) -> StorageResult<()> {
        self.key_results = load_yaml_file(&self.base_path.join("key_results.yaml"))?;
        Ok(())
    }

    /// Save key results to key_results.yaml.
    pub(crate) fn save_key_results(&self) -> StorageResult<()> {
        let key_results_file = self.base_path.join("key_results.yaml");
        let content = serde_yaml::to_string(&self.key_results)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        atomic_write(&key_results_file, content)?;
        Ok(())
    }

    /// Load pomodoro state from pomodoro.yaml.
    pub(crate) fn load_pomodoro_state(&mut self) -> StorageResult<()> {
        self.pomodoro_state = load_yaml_file(&self.base_path.join("pomodoro.yaml"))?;
        Ok(())
    }

    /// Save pomodoro state to pomodoro.yaml.
    pub(crate) fn save_pomodoro_state(&self) -> StorageResult<()> {
        let pomodoro_file = self.base_path.join("pomodoro.yaml");
        let content = serde_yaml::to_string(&self.pomodoro_state)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        atomic_write(&pomodoro_file, content)?;
        Ok(())
    }

    /// Load saved filters from saved_filters.yaml.
    pub(crate) fn load_saved_filters(&mut self) -> StorageResult<()> {
        self.saved_filters = load_yaml_file(&self.base_path.join("saved_filters.yaml"))?;
        Ok(())
    }

    /// Save saved filters to saved_filters.yaml.
    pub(crate) fn save_saved_filters(&self) -> StorageResult<()> {
        let filters_file = self.base_path.join("saved_filters.yaml");
        let content = serde_yaml::to_string(&self.saved_filters)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        atomic_write(&filters_file, content)?;
        Ok(())
    }

    /// Parse a task from a markdown file with YAML frontmatter.
    pub(crate) fn parse_task_file(&self, path: &Path) -> StorageResult<Task> {
        let content = fs::read_to_string(path).map_err(|e| StorageError::io(path, e))?;

        // Parse frontmatter and body
        let (frontmatter, body) = self.parse_frontmatter(&content)?;

        // Deserialize task from frontmatter
        let mut task: Task = serde_yaml::from_str(&frontmatter)
            .map_err(|e| StorageError::deserialization(e.to_string()))?;

        // Set description from body if present
        let body = body.trim();
        if !body.is_empty() {
            task.description = Some(body.to_string());
        }

        Ok(task)
    }

    /// Parse a project from a markdown file with YAML frontmatter.
    pub(crate) fn parse_project_file(&self, path: &Path) -> StorageResult<Project> {
        let content = fs::read_to_string(path).map_err(|e| StorageError::io(path, e))?;

        let (frontmatter, body) = self.parse_frontmatter(&content)?;

        let mut project: Project = serde_yaml::from_str(&frontmatter)
            .map_err(|e| StorageError::deserialization(e.to_string()))?;

        let body = body.trim();
        if !body.is_empty() {
            project.description = Some(body.to_string());
        }

        Ok(project)
    }

    /// Parse YAML frontmatter from markdown content.
    ///
    /// Returns (frontmatter, body) tuple.
    #[allow(clippy::unused_self)]
    pub(crate) fn parse_frontmatter(&self, content: &str) -> StorageResult<(String, String)> {
        let content = content.trim();

        if !content.starts_with("---") {
            return Err(StorageError::deserialization(
                "Missing frontmatter delimiter".to_string(),
            ));
        }

        let rest = &content[3..];
        if let Some(end_pos) = rest.find("\n---") {
            let frontmatter = rest[..end_pos].trim().to_string();
            let body = rest[end_pos + 4..].to_string();
            Ok((frontmatter, body))
        } else {
            Err(StorageError::deserialization(
                "Missing closing frontmatter delimiter".to_string(),
            ))
        }
    }

    /// Write a task to a markdown file.
    pub(crate) fn write_task_file(&mut self, task: &Task) -> StorageResult<()> {
        let path = self.tasks_dir.join(format!("{}.md", task.id.0));

        // Create frontmatter-friendly version (without description in frontmatter)
        let mut task_for_yaml = task.clone();
        task_for_yaml.description = None;

        let frontmatter = serde_yaml::to_string(&task_for_yaml)
            .map_err(|e| StorageError::serialization(e.to_string()))?;

        let mut content = format!("---\n{frontmatter}---\n");

        // Add description as body
        if let Some(ref desc) = task.description {
            content.push('\n');
            content.push_str(desc);
            content.push('\n');
        }

        atomic_write(&path, content)?;

        // Update mtime cache after write
        if let Ok(mtime) = fs::metadata(&path).and_then(|m| m.modified()) {
            self.task_mtimes.insert(task.id, mtime);
        }

        Ok(())
    }

    /// Write a project to a markdown file.
    pub(crate) fn write_project_file(&mut self, project: &Project) -> StorageResult<()> {
        let id = project.id.0;
        let path = self.projects_dir.join(format!("{id}.md"));

        let mut project_for_yaml = project.clone();
        project_for_yaml.description = None;

        let frontmatter = serde_yaml::to_string(&project_for_yaml)
            .map_err(|e| StorageError::serialization(e.to_string()))?;

        let mut content = format!("---\n{frontmatter}---\n");

        if let Some(ref desc) = project.description {
            content.push('\n');
            content.push_str(desc);
            content.push('\n');
        }

        atomic_write(&path, content)?;

        // Update mtime cache after write
        if let Ok(mtime) = fs::metadata(&path).and_then(|m| m.modified()) {
            self.project_mtimes.insert(project.id, mtime);
        }

        Ok(())
    }

    /// Delete a task markdown file.
    pub(crate) fn delete_task_file(&self, id: &TaskId) -> StorageResult<()> {
        let path = self.tasks_dir.join(format!("{}.md", id.0));
        if path.exists() {
            fs::remove_file(&path).map_err(|e| StorageError::io(&path, e))?;
        }
        Ok(())
    }

    /// Delete a project markdown file.
    pub(crate) fn delete_project_file(&self, id: &ProjectId) -> StorageResult<()> {
        let path = self.projects_dir.join(format!("{}.md", id.0));
        if path.exists() {
            fs::remove_file(&path).map_err(|e| StorageError::io(&path, e))?;
        }
        Ok(())
    }
}
