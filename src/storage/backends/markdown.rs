use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::domain::{
    Filter, PomodoroConfig, PomodoroSession, PomodoroStats, Project, ProjectId, Tag, Task, TaskId,
    TimeEntry, TimeEntryId, WorkLogEntry, WorkLogEntryId,
};
use crate::storage::{
    ExportData, ProjectRepository, StorageBackend, StorageError, StorageResult, TagRepository,
    TaskRepository, TimeEntryRepository, WorkLogRepository,
};

/// Markdown file-based storage backend
///
/// Stores tasks as individual markdown files with YAML frontmatter.
/// Great for version control, manual editing, and integration with
/// other markdown-based tools.
///
/// Directory structure:
/// ```text
/// data_dir/
///   tasks/
///     <uuid>.md
///   projects/
///     <uuid>.md
///   tags.yaml
///   time_entries.yaml
/// ```
/// Pomodoro state stored in YAML
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct PomodoroState {
    session: Option<PomodoroSession>,
    config: Option<PomodoroConfig>,
    stats: Option<PomodoroStats>,
}

pub struct MarkdownBackend {
    base_path: PathBuf,
    tasks_dir: PathBuf,
    projects_dir: PathBuf,
    // Cache for performance
    tasks_cache: HashMap<TaskId, Task>,
    projects_cache: HashMap<ProjectId, Project>,
    // Track file modification times for cache invalidation
    task_mtimes: HashMap<TaskId, SystemTime>,
    project_mtimes: HashMap<ProjectId, SystemTime>,
    tags: Vec<Tag>,
    time_entries: Vec<TimeEntry>,
    work_logs: Vec<WorkLogEntry>,
    pomodoro_state: PomodoroState,
    dirty: bool,
}

impl MarkdownBackend {
    /// Creates a new Markdown backend at the given path.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`] if the backend cannot be created.
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
            pomodoro_state: PomodoroState::default(),
            dirty: false,
        })
    }

    fn ensure_dirs(&self) -> StorageResult<()> {
        fs::create_dir_all(&self.tasks_dir).map_err(|e| StorageError::io(&self.tasks_dir, e))?;
        fs::create_dir_all(&self.projects_dir)
            .map_err(|e| StorageError::io(&self.projects_dir, e))?;
        Ok(())
    }

    fn load_tasks(&mut self) -> StorageResult<()> {
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
                if let Ok(task) = self.parse_task_file(&path) {
                    // Track file modification time
                    if let Ok(metadata) = fs::metadata(&path) {
                        if let Ok(mtime) = metadata.modified() {
                            self.task_mtimes.insert(task.id, mtime);
                        }
                    }
                    self.tasks_cache.insert(task.id, task);
                }
            }
        }

        Ok(())
    }

    fn load_projects(&mut self) -> StorageResult<()> {
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
                if let Ok(project) = self.parse_project_file(&path) {
                    // Track file modification time
                    if let Ok(metadata) = fs::metadata(&path) {
                        if let Ok(mtime) = metadata.modified() {
                            self.project_mtimes.insert(project.id, mtime);
                        }
                    }
                    self.projects_cache.insert(project.id, project);
                }
            }
        }

        Ok(())
    }

    fn load_tags(&mut self) -> StorageResult<()> {
        let tags_file = self.base_path.join("tags.yaml");
        if tags_file.exists() {
            let content =
                fs::read_to_string(&tags_file).map_err(|e| StorageError::io(&tags_file, e))?;
            self.tags = serde_yaml::from_str(&content).unwrap_or_default();
        }
        Ok(())
    }

    fn load_time_entries(&mut self) -> StorageResult<()> {
        let entries_file = self.base_path.join("time_entries.yaml");
        if entries_file.exists() {
            let content = fs::read_to_string(&entries_file)
                .map_err(|e| StorageError::io(&entries_file, e))?;
            self.time_entries = serde_yaml::from_str(&content).unwrap_or_default();
        }
        Ok(())
    }

    fn save_tags(&self) -> StorageResult<()> {
        let tags_file = self.base_path.join("tags.yaml");
        let content = serde_yaml::to_string(&self.tags)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        fs::write(&tags_file, content).map_err(|e| StorageError::io(&tags_file, e))?;
        Ok(())
    }

    fn save_time_entries(&self) -> StorageResult<()> {
        let entries_file = self.base_path.join("time_entries.yaml");
        let content = serde_yaml::to_string(&self.time_entries)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        fs::write(&entries_file, content).map_err(|e| StorageError::io(&entries_file, e))?;
        Ok(())
    }

    fn load_work_logs(&mut self) -> StorageResult<()> {
        let work_logs_file = self.base_path.join("work_logs.yaml");
        if work_logs_file.exists() {
            let content = fs::read_to_string(&work_logs_file)
                .map_err(|e| StorageError::io(&work_logs_file, e))?;
            self.work_logs = serde_yaml::from_str(&content).unwrap_or_default();
        }
        Ok(())
    }

    fn save_work_logs(&self) -> StorageResult<()> {
        let work_logs_file = self.base_path.join("work_logs.yaml");
        let content = serde_yaml::to_string(&self.work_logs)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        fs::write(&work_logs_file, content).map_err(|e| StorageError::io(&work_logs_file, e))?;
        Ok(())
    }

    fn load_pomodoro_state(&mut self) -> StorageResult<()> {
        let pomodoro_file = self.base_path.join("pomodoro.yaml");
        if pomodoro_file.exists() {
            let content = fs::read_to_string(&pomodoro_file)
                .map_err(|e| StorageError::io(&pomodoro_file, e))?;
            self.pomodoro_state = serde_yaml::from_str(&content).unwrap_or_default();
        }
        Ok(())
    }

    fn save_pomodoro_state(&self) -> StorageResult<()> {
        let pomodoro_file = self.base_path.join("pomodoro.yaml");
        let content = serde_yaml::to_string(&self.pomodoro_state)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        fs::write(&pomodoro_file, content).map_err(|e| StorageError::io(&pomodoro_file, e))?;
        Ok(())
    }

    fn parse_task_file(&self, path: &Path) -> StorageResult<Task> {
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

    fn parse_project_file(&self, path: &Path) -> StorageResult<Project> {
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

    #[allow(clippy::unused_self)]
    fn parse_frontmatter(&self, content: &str) -> StorageResult<(String, String)> {
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

    fn write_task_file(&mut self, task: &Task) -> StorageResult<()> {
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

        fs::write(&path, content).map_err(|e| StorageError::io(&path, e))?;

        // Update mtime cache after write
        if let Ok(mtime) = fs::metadata(&path).and_then(|m| m.modified()) {
            self.task_mtimes.insert(task.id, mtime);
        }

        Ok(())
    }

    fn write_project_file(&mut self, project: &Project) -> StorageResult<()> {
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

        fs::write(&path, content).map_err(|e| StorageError::io(&path, e))?;

        // Update mtime cache after write
        if let Ok(mtime) = fs::metadata(&path).and_then(|m| m.modified()) {
            self.project_mtimes.insert(project.id, mtime);
        }

        Ok(())
    }

    fn delete_task_file(&self, id: &TaskId) -> StorageResult<()> {
        let path = self.tasks_dir.join(format!("{}.md", id.0));
        if path.exists() {
            fs::remove_file(&path).map_err(|e| StorageError::io(&path, e))?;
        }
        Ok(())
    }

    fn delete_project_file(&self, id: &ProjectId) -> StorageResult<()> {
        let path = self.projects_dir.join(format!("{}.md", id.0));
        if path.exists() {
            fs::remove_file(&path).map_err(|e| StorageError::io(&path, e))?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    const fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Check if a task file was modified externally and reload if needed.
    /// Returns true if the cache was updated.
    fn check_task_modified(&mut self, id: &TaskId) -> bool {
        let path = self.tasks_dir.join(format!("{}.md", id.0));

        // Check current file mtime
        let current_mtime = match fs::metadata(&path).and_then(|m| m.modified()) {
            Ok(mtime) => mtime,
            Err(_) => return false, // File doesn't exist or can't read metadata
        };

        // Compare with cached mtime
        let cached_mtime = self.task_mtimes.get(id);
        let needs_reload = cached_mtime.is_none_or(|cached| *cached != current_mtime);

        if needs_reload {
            // Reload the file
            if let Ok(task) = self.parse_task_file(&path) {
                self.task_mtimes.insert(*id, current_mtime);
                self.tasks_cache.insert(*id, task);
                return true;
            }
        }

        false
    }

    /// Check if a project file was modified externally and reload if needed.
    /// Returns true if the cache was updated.
    fn check_project_modified(&mut self, id: &ProjectId) -> bool {
        let path = self.projects_dir.join(format!("{}.md", id.0));

        // Check current file mtime
        let current_mtime = match fs::metadata(&path).and_then(|m| m.modified()) {
            Ok(mtime) => mtime,
            Err(_) => return false, // File doesn't exist or can't read metadata
        };

        // Compare with cached mtime
        let cached_mtime = self.project_mtimes.get(id);
        let needs_reload = cached_mtime.is_none_or(|cached| *cached != current_mtime);

        if needs_reload {
            // Reload the file
            if let Ok(project) = self.parse_project_file(&path) {
                self.project_mtimes.insert(*id, current_mtime);
                self.projects_cache.insert(*id, project);
                return true;
            }
        }

        false
    }

    /// Check for any externally added or removed task files.
    /// Returns the number of changes detected.
    fn scan_for_task_changes(&mut self) -> usize {
        let mut changes = 0;

        if !self.tasks_dir.exists() {
            return 0;
        }

        // Collect current file IDs from disk
        let mut disk_ids: std::collections::HashSet<TaskId> = std::collections::HashSet::new();

        if let Ok(entries) = fs::read_dir(&self.tasks_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    // Extract ID from filename (uuid.md)
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(uuid) = uuid::Uuid::parse_str(stem) {
                            disk_ids.insert(TaskId(uuid));
                        }
                    }
                }
            }
        }

        // Check for new files (on disk but not in cache)
        for id in &disk_ids {
            if !self.tasks_cache.contains_key(id) {
                let path = self.tasks_dir.join(format!("{}.md", id.0));
                if let Ok(task) = self.parse_task_file(&path) {
                    if let Ok(mtime) = fs::metadata(&path).and_then(|m| m.modified()) {
                        self.task_mtimes.insert(*id, mtime);
                    }
                    self.tasks_cache.insert(*id, task);
                    changes += 1;
                }
            }
        }

        // Check for deleted files (in cache but not on disk)
        let cached_ids: Vec<TaskId> = self.tasks_cache.keys().cloned().collect();
        for id in cached_ids {
            if !disk_ids.contains(&id) {
                self.tasks_cache.remove(&id);
                self.task_mtimes.remove(&id);
                changes += 1;
            }
        }

        // Check for modified files
        for id in &disk_ids {
            if self.check_task_modified(id) {
                changes += 1;
            }
        }

        changes
    }

    /// Check for any externally added or removed project files.
    /// Returns the number of changes detected.
    fn scan_for_project_changes(&mut self) -> usize {
        let mut changes = 0;

        if !self.projects_dir.exists() {
            return 0;
        }

        // Collect current file IDs from disk
        let mut disk_ids: std::collections::HashSet<ProjectId> = std::collections::HashSet::new();

        if let Ok(entries) = fs::read_dir(&self.projects_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    // Extract ID from filename (uuid.md)
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(uuid) = uuid::Uuid::parse_str(stem) {
                            disk_ids.insert(ProjectId(uuid));
                        }
                    }
                }
            }
        }

        // Check for new files (on disk but not in cache)
        for id in &disk_ids {
            if !self.projects_cache.contains_key(id) {
                let path = self.projects_dir.join(format!("{}.md", id.0));
                if let Ok(project) = self.parse_project_file(&path) {
                    if let Ok(mtime) = fs::metadata(&path).and_then(|m| m.modified()) {
                        self.project_mtimes.insert(*id, mtime);
                    }
                    self.projects_cache.insert(*id, project);
                    changes += 1;
                }
            }
        }

        // Check for deleted files (in cache but not on disk)
        let cached_ids: Vec<ProjectId> = self.projects_cache.keys().cloned().collect();
        for id in cached_ids {
            if !disk_ids.contains(&id) {
                self.projects_cache.remove(&id);
                self.project_mtimes.remove(&id);
                changes += 1;
            }
        }

        // Check for modified files
        for id in &disk_ids {
            if self.check_project_modified(id) {
                changes += 1;
            }
        }

        changes
    }

    /// Refresh the cache by checking for external changes.
    /// Returns the total number of changes detected.
    pub fn refresh(&mut self) -> usize {
        let task_changes = self.scan_for_task_changes();
        let project_changes = self.scan_for_project_changes();
        task_changes + project_changes
    }
}

impl TaskRepository for MarkdownBackend {
    fn create_task(&mut self, task: &Task) -> StorageResult<()> {
        if self.tasks_cache.contains_key(&task.id) {
            return Err(StorageError::already_exists("Task", task.id.to_string()));
        }
        self.write_task_file(task)?;
        self.tasks_cache.insert(task.id, task.clone());
        Ok(())
    }

    fn get_task(&self, id: &TaskId) -> StorageResult<Option<Task>> {
        Ok(self.tasks_cache.get(id).cloned())
    }

    fn update_task(&mut self, task: &Task) -> StorageResult<()> {
        if !self.tasks_cache.contains_key(&task.id) {
            return Err(StorageError::not_found("Task", task.id.to_string()));
        }
        self.write_task_file(task)?;
        self.tasks_cache.insert(task.id, task.clone());
        Ok(())
    }

    fn delete_task(&mut self, id: &TaskId) -> StorageResult<()> {
        if !self.tasks_cache.contains_key(id) {
            return Err(StorageError::not_found("Task", id.to_string()));
        }
        self.delete_task_file(id)?;
        self.tasks_cache.remove(id);
        Ok(())
    }

    fn list_tasks(&self) -> StorageResult<Vec<Task>> {
        Ok(self.tasks_cache.values().cloned().collect())
    }

    fn list_tasks_filtered(&self, filter: &Filter) -> StorageResult<Vec<Task>> {
        let tasks = self
            .tasks_cache
            .values()
            .filter(|task| {
                if let Some(ref statuses) = filter.status {
                    if !statuses.contains(&task.status) {
                        return false;
                    }
                }
                if let Some(ref priorities) = filter.priority {
                    if !priorities.contains(&task.priority) {
                        return false;
                    }
                }
                if let Some(ref project_id) = filter.project_id {
                    if task.project_id.as_ref() != Some(project_id) {
                        return false;
                    }
                }
                if !filter.include_completed && task.status.is_complete() {
                    return false;
                }
                if let Some(ref search) = filter.search_text {
                    let search_lower = search.to_lowercase();
                    if !task.title.to_lowercase().contains(&search_lower) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();
        Ok(tasks)
    }

    fn get_tasks_by_project(&self, project_id: &ProjectId) -> StorageResult<Vec<Task>> {
        Ok(self
            .tasks_cache
            .values()
            .filter(|t| t.project_id.as_ref() == Some(project_id))
            .cloned()
            .collect())
    }

    fn get_tasks_by_tag(&self, tag: &str) -> StorageResult<Vec<Task>> {
        Ok(self
            .tasks_cache
            .values()
            .filter(|t| t.tags.contains(&tag.to_string()))
            .cloned()
            .collect())
    }
}

impl ProjectRepository for MarkdownBackend {
    fn create_project(&mut self, project: &Project) -> StorageResult<()> {
        if self.projects_cache.contains_key(&project.id) {
            return Err(StorageError::already_exists(
                "Project",
                project.id.to_string(),
            ));
        }
        self.write_project_file(project)?;
        self.projects_cache.insert(project.id, project.clone());
        Ok(())
    }

    fn get_project(&self, id: &ProjectId) -> StorageResult<Option<Project>> {
        Ok(self.projects_cache.get(id).cloned())
    }

    fn update_project(&mut self, project: &Project) -> StorageResult<()> {
        if !self.projects_cache.contains_key(&project.id) {
            return Err(StorageError::not_found("Project", project.id.to_string()));
        }
        self.write_project_file(project)?;
        self.projects_cache.insert(project.id, project.clone());
        Ok(())
    }

    fn delete_project(&mut self, id: &ProjectId) -> StorageResult<()> {
        if !self.projects_cache.contains_key(id) {
            return Err(StorageError::not_found("Project", id.to_string()));
        }
        self.delete_project_file(id)?;
        self.projects_cache.remove(id);
        Ok(())
    }

    fn list_projects(&self) -> StorageResult<Vec<Project>> {
        Ok(self.projects_cache.values().cloned().collect())
    }

    fn get_subprojects(&self, parent_id: &ProjectId) -> StorageResult<Vec<Project>> {
        Ok(self
            .projects_cache
            .values()
            .filter(|p| p.parent_id.as_ref() == Some(parent_id))
            .cloned()
            .collect())
    }
}

impl TagRepository for MarkdownBackend {
    fn save_tag(&mut self, tag: &Tag) -> StorageResult<()> {
        if let Some(existing) = self.tags.iter_mut().find(|t| t.name == tag.name) {
            *existing = tag.clone();
        } else {
            self.tags.push(tag.clone());
        }
        self.save_tags()?;
        Ok(())
    }

    fn get_tag(&self, name: &str) -> StorageResult<Option<Tag>> {
        Ok(self.tags.iter().find(|t| t.name == name).cloned())
    }

    fn delete_tag(&mut self, name: &str) -> StorageResult<()> {
        let len_before = self.tags.len();
        self.tags.retain(|t| t.name != name);
        if self.tags.len() == len_before {
            return Err(StorageError::not_found("Tag", name));
        }
        self.save_tags()?;
        Ok(())
    }

    fn list_tags(&self) -> StorageResult<Vec<Tag>> {
        Ok(self.tags.clone())
    }
}

impl TimeEntryRepository for MarkdownBackend {
    fn create_time_entry(&mut self, entry: &TimeEntry) -> StorageResult<()> {
        if self.time_entries.iter().any(|e| e.id == entry.id) {
            return Err(StorageError::already_exists(
                "TimeEntry",
                entry.id.0.to_string(),
            ));
        }
        self.time_entries.push(entry.clone());
        self.save_time_entries()?;
        Ok(())
    }

    fn get_time_entry(&self, id: &TimeEntryId) -> StorageResult<Option<TimeEntry>> {
        Ok(self.time_entries.iter().find(|e| &e.id == id).cloned())
    }

    fn update_time_entry(&mut self, entry: &TimeEntry) -> StorageResult<()> {
        if let Some(existing) = self.time_entries.iter_mut().find(|e| e.id == entry.id) {
            *existing = entry.clone();
            self.save_time_entries()?;
            Ok(())
        } else {
            Err(StorageError::not_found("TimeEntry", entry.id.0.to_string()))
        }
    }

    fn delete_time_entry(&mut self, id: &TimeEntryId) -> StorageResult<()> {
        let len_before = self.time_entries.len();
        self.time_entries.retain(|e| &e.id != id);
        if self.time_entries.len() == len_before {
            return Err(StorageError::not_found("TimeEntry", id.0.to_string()));
        }
        self.save_time_entries()?;
        Ok(())
    }

    fn get_entries_for_task(&self, task_id: &TaskId) -> StorageResult<Vec<TimeEntry>> {
        Ok(self
            .time_entries
            .iter()
            .filter(|e| &e.task_id == task_id)
            .cloned()
            .collect())
    }

    fn get_active_entry(&self) -> StorageResult<Option<TimeEntry>> {
        Ok(self.time_entries.iter().find(|e| e.is_running()).cloned())
    }
}

impl WorkLogRepository for MarkdownBackend {
    fn create_work_log(&mut self, entry: &WorkLogEntry) -> StorageResult<()> {
        if self.work_logs.iter().any(|e| e.id == entry.id) {
            return Err(StorageError::already_exists(
                "WorkLogEntry",
                entry.id.0.to_string(),
            ));
        }
        self.work_logs.push(entry.clone());
        self.save_work_logs()?;
        Ok(())
    }

    fn get_work_log(&self, id: &WorkLogEntryId) -> StorageResult<Option<WorkLogEntry>> {
        Ok(self.work_logs.iter().find(|e| &e.id == id).cloned())
    }

    fn update_work_log(&mut self, entry: &WorkLogEntry) -> StorageResult<()> {
        if let Some(existing) = self.work_logs.iter_mut().find(|e| e.id == entry.id) {
            *existing = entry.clone();
            self.save_work_logs()?;
            Ok(())
        } else {
            Err(StorageError::not_found(
                "WorkLogEntry",
                entry.id.0.to_string(),
            ))
        }
    }

    fn delete_work_log(&mut self, id: &WorkLogEntryId) -> StorageResult<()> {
        let len_before = self.work_logs.len();
        self.work_logs.retain(|e| &e.id != id);
        if self.work_logs.len() == len_before {
            return Err(StorageError::not_found("WorkLogEntry", id.0.to_string()));
        }
        self.save_work_logs()?;
        Ok(())
    }

    fn get_work_logs_for_task(&self, task_id: &TaskId) -> StorageResult<Vec<WorkLogEntry>> {
        let mut logs: Vec<_> = self
            .work_logs
            .iter()
            .filter(|e| &e.task_id == task_id)
            .cloned()
            .collect();
        // Sort by creation time, newest first
        logs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(logs)
    }

    fn list_work_logs(&self) -> StorageResult<Vec<WorkLogEntry>> {
        Ok(self.work_logs.clone())
    }
}

impl StorageBackend for MarkdownBackend {
    fn initialize(&mut self) -> StorageResult<()> {
        self.ensure_dirs()?;
        self.load_tasks()?;
        self.load_projects()?;
        self.load_tags()?;
        self.load_time_entries()?;
        self.load_work_logs()?;
        self.load_pomodoro_state()?;
        Ok(())
    }

    fn flush(&mut self) -> StorageResult<()> {
        // Files are written immediately, but we save auxiliary data
        self.save_tags()?;
        self.save_time_entries()?;
        self.save_work_logs()?;
        self.save_pomodoro_state()?;
        self.dirty = false;
        Ok(())
    }

    fn export_all(&self) -> StorageResult<ExportData> {
        Ok(ExportData {
            tasks: self.tasks_cache.values().cloned().collect(),
            projects: self.projects_cache.values().cloned().collect(),
            tags: self.tags.clone(),
            time_entries: self.time_entries.clone(),
            work_logs: self.work_logs.clone(),
            version: 1,
            pomodoro_session: self.pomodoro_state.session.clone(),
            pomodoro_config: self.pomodoro_state.config.clone(),
            pomodoro_stats: self.pomodoro_state.stats.clone(),
        })
    }

    #[allow(clippy::needless_collect)]
    fn import_all(&mut self, data: &ExportData) -> StorageResult<()> {
        // Clear existing data - collect needed to avoid borrow conflict
        for id in self.tasks_cache.keys().cloned().collect::<Vec<_>>() {
            self.delete_task_file(&id)?;
        }
        for id in self.projects_cache.keys().cloned().collect::<Vec<_>>() {
            self.delete_project_file(&id)?;
        }

        self.tasks_cache.clear();
        self.projects_cache.clear();
        self.tags.clear();
        self.time_entries.clear();
        self.work_logs.clear();

        // Import new data
        for project in &data.projects {
            self.create_project(project)?;
        }
        for task in &data.tasks {
            self.create_task(task)?;
        }
        for tag in &data.tags {
            self.save_tag(tag)?;
        }
        for entry in &data.time_entries {
            self.time_entries.push(entry.clone());
        }
        self.save_time_entries()?;

        // Import work logs
        for entry in &data.work_logs {
            self.work_logs.push(entry.clone());
        }
        self.save_work_logs()?;

        // Import Pomodoro state
        self.pomodoro_state = PomodoroState {
            session: data.pomodoro_session.clone(),
            config: data.pomodoro_config.clone(),
            stats: data.pomodoro_stats.clone(),
        };
        self.save_pomodoro_state()?;

        Ok(())
    }

    fn backend_type(&self) -> &'static str {
        "markdown"
    }

    fn set_pomodoro_session(&mut self, session: Option<&PomodoroSession>) -> StorageResult<()> {
        self.pomodoro_state.session = session.cloned();
        self.save_pomodoro_state()
    }

    fn set_pomodoro_config(&mut self, config: &PomodoroConfig) -> StorageResult<()> {
        self.pomodoro_state.config = Some(config.clone());
        self.save_pomodoro_state()
    }

    fn set_pomodoro_stats(&mut self, stats: &PomodoroStats) -> StorageResult<()> {
        self.pomodoro_state.stats = Some(stats.clone());
        self.save_pomodoro_state()
    }

    fn refresh(&mut self) -> usize {
        // Delegate to the existing refresh implementation
        let task_changes = self.scan_for_task_changes();
        let project_changes = self.scan_for_project_changes();
        task_changes + project_changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Priority;
    use tempfile::tempdir;

    fn create_test_backend() -> (tempfile::TempDir, MarkdownBackend) {
        let dir = tempdir().unwrap();
        let mut backend = MarkdownBackend::new(dir.path()).unwrap();
        backend.initialize().unwrap();
        (dir, backend)
    }

    #[test]
    fn test_markdown_ensure_dirs() {
        let dir = tempdir().unwrap();
        let mut backend = MarkdownBackend::new(dir.path()).unwrap();
        backend.initialize().unwrap();

        assert!(dir.path().join("tasks").exists());
        assert!(dir.path().join("projects").exists());
    }

    #[test]
    fn test_markdown_write_task_file() {
        let (dir, mut backend) = create_test_backend();

        let task = Task::new("Test task");
        backend.create_task(&task).unwrap();

        let file_path = dir.path().join("tasks").join(format!("{}.md", task.id.0));
        assert!(file_path.exists());

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.starts_with("---\n"));
        assert!(content.contains("title: Test task"));
    }

    #[test]
    fn test_markdown_parse_frontmatter() {
        let (_dir, backend) = create_test_backend();

        let content = "---\ntitle: Test\nstatus: todo\n---\n\nDescription here.";
        let (frontmatter, body) = backend.parse_frontmatter(content).unwrap();

        assert!(frontmatter.contains("title: Test"));
        assert!(body.contains("Description here"));
    }

    #[test]
    fn test_markdown_task_crud() {
        let (_dir, mut backend) = create_test_backend();

        // Create
        let task = Task::new("Test task");
        backend.create_task(&task).unwrap();

        // Read
        let retrieved = backend.get_task(&task.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test task");

        // Update
        let mut updated_task = task.clone();
        updated_task.title = "Updated task".to_string();
        backend.update_task(&updated_task).unwrap();

        let retrieved = backend.get_task(&task.id).unwrap().unwrap();
        assert_eq!(retrieved.title, "Updated task");

        // Delete
        backend.delete_task(&task.id).unwrap();
        assert!(backend.get_task(&task.id).unwrap().is_none());
    }

    #[test]
    fn test_markdown_project_crud() {
        let (_dir, mut backend) = create_test_backend();

        let project = Project::new("Test project");
        backend.create_project(&project).unwrap();

        let retrieved = backend.get_project(&project.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test project");

        backend.delete_project(&project.id).unwrap();
        assert!(backend.get_project(&project.id).unwrap().is_none());
    }

    #[test]
    fn test_markdown_tags_yaml() {
        let (dir, mut backend) = create_test_backend();

        let tag = Tag {
            name: "test-tag".to_string(),
            color: Some("#ff0000".to_string()),
            description: None,
        };

        backend.save_tag(&tag).unwrap();

        // Verify tags.yaml exists
        let tags_file = dir.path().join("tags.yaml");
        assert!(tags_file.exists());

        let content = fs::read_to_string(&tags_file).unwrap();
        assert!(content.contains("test-tag"));

        // Retrieve
        let retrieved = backend.get_tag("test-tag").unwrap();
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_markdown_time_entries_yaml() {
        let (dir, mut backend) = create_test_backend();

        let task = Task::new("Task");
        backend.create_task(&task).unwrap();

        let entry = TimeEntry::start(task.id.clone());
        backend.create_time_entry(&entry).unwrap();

        // Verify time_entries.yaml exists
        let entries_file = dir.path().join("time_entries.yaml");
        assert!(entries_file.exists());

        let content = fs::read_to_string(&entries_file).unwrap();
        assert!(content.contains(&task.id.0.to_string()));
    }

    #[test]
    fn test_markdown_description_in_body() {
        let (dir, mut backend) = create_test_backend();

        let mut task = Task::new("Task with description");
        task.description = Some("This is the description\nwith multiple lines.".to_string());
        backend.create_task(&task).unwrap();

        // Read file directly
        let file_path = dir.path().join("tasks").join(format!("{}.md", task.id.0));
        let content = fs::read_to_string(&file_path).unwrap();

        // Description should be in body, not frontmatter
        assert!(content.contains("This is the description"));
        // After the closing ---
        let parts: Vec<&str> = content.split("---").collect();
        assert!(parts.len() >= 3); // Start ---, frontmatter, closing ---
        assert!(parts[2].contains("This is the description"));
    }

    #[test]
    fn test_markdown_missing_frontmatter_error() {
        let (_dir, backend) = create_test_backend();

        let content = "Just some text without frontmatter";
        let result = backend.parse_frontmatter(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_markdown_persistence() {
        let dir = tempdir().unwrap();

        // Create and save
        {
            let mut backend = MarkdownBackend::new(dir.path()).unwrap();
            backend.initialize().unwrap();

            let task = Task::new("Persistent task");
            backend.create_task(&task).unwrap();
            backend.flush().unwrap();
        }

        // Load and verify
        {
            let mut backend = MarkdownBackend::new(dir.path()).unwrap();
            backend.initialize().unwrap();

            let tasks = backend.list_tasks().unwrap();
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].title, "Persistent task");
        }
    }

    #[test]
    fn test_markdown_export_import_roundtrip() {
        let (_dir, mut backend) = create_test_backend();

        // Create sample data
        let task = Task::new("Test task").with_priority(Priority::High);
        let project = Project::new("Test project");
        let tag = Tag {
            name: "important".to_string(),
            color: Some("#ff0000".to_string()),
            description: None,
        };

        backend.create_task(&task).unwrap();
        backend.create_project(&project).unwrap();
        backend.save_tag(&tag).unwrap();

        // Export
        let exported = backend.export_all().unwrap();

        // Create new backend and import
        let dir2 = tempdir().unwrap();
        let mut backend2 = MarkdownBackend::new(dir2.path()).unwrap();
        backend2.initialize().unwrap();
        backend2.import_all(&exported).unwrap();

        // Verify
        assert_eq!(backend2.list_tasks().unwrap().len(), 1);
        assert_eq!(backend2.list_projects().unwrap().len(), 1);
        assert_eq!(backend2.list_tags().unwrap().len(), 1);
    }

    #[test]
    fn test_markdown_backend_type() {
        let (_dir, backend) = create_test_backend();
        assert_eq!(backend.backend_type(), "markdown");
    }

    #[test]
    fn test_markdown_get_active_entry() {
        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("Task");
        backend.create_task(&task).unwrap();

        // No active entry initially
        assert!(backend.get_active_entry().unwrap().is_none());

        // Start an entry
        let entry = TimeEntry::start(task.id.clone());
        backend.create_time_entry(&entry).unwrap();

        // Now there's an active entry
        let active = backend.get_active_entry().unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, entry.id);
    }

    #[test]
    fn test_markdown_create_task_duplicate_id_fails() {
        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("Original");
        backend.create_task(&task).unwrap();

        let duplicate = Task {
            id: task.id.clone(),
            ..Task::new("Duplicate")
        };

        let result = backend.create_task(&duplicate);
        assert!(result.is_err());
    }

    #[test]
    fn test_markdown_update_task_not_found() {
        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("Non-existent");
        let result = backend.update_task(&task);
        assert!(result.is_err());
    }

    #[test]
    fn test_markdown_delete_task_not_found() {
        let (_dir, mut backend) = create_test_backend();

        let task_id = TaskId::new();
        let result = backend.delete_task(&task_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_markdown_cache_detects_external_modification() {
        let (dir, mut backend) = create_test_backend();

        // Create a task
        let task = Task::new("Original title");
        backend.create_task(&task).unwrap();

        // Verify it's in cache
        let cached = backend.get_task(&task.id).unwrap().unwrap();
        assert_eq!(cached.title, "Original title");

        // Externally modify the file (simulate text editor)
        let file_path = dir.path().join("tasks").join(format!("{}.md", task.id.0));
        let content = fs::read_to_string(&file_path).unwrap();
        let modified_content = content.replace("Original title", "Modified externally");

        // Wait a tiny bit to ensure mtime changes
        std::thread::sleep(std::time::Duration::from_millis(10));
        fs::write(&file_path, modified_content).unwrap();

        // Refresh should detect the change
        let changes = backend.refresh();
        assert!(changes > 0, "Should detect external modification");

        // Cache should now have the updated content
        let updated = backend.get_task(&task.id).unwrap().unwrap();
        assert_eq!(updated.title, "Modified externally");
    }

    #[test]
    fn test_markdown_cache_detects_external_file_addition() {
        let (dir, mut backend) = create_test_backend();

        // Start with no tasks
        assert_eq!(backend.list_tasks().unwrap().len(), 0);

        // Externally create a task file (simulate git pull or manual creation)
        // Use a real Task to get proper YAML serialization
        let new_task = Task::new("Externally created");
        let file_path = dir
            .path()
            .join("tasks")
            .join(format!("{}.md", new_task.id.0));

        // Write proper frontmatter using serde_yaml
        let mut task_for_yaml = new_task.clone();
        task_for_yaml.description = None;
        let frontmatter = serde_yaml::to_string(&task_for_yaml).unwrap();
        let content = format!("---\n{frontmatter}---\n");
        fs::write(&file_path, content).unwrap();

        // Refresh should detect the new file
        let changes = backend.refresh();
        assert!(changes > 0, "Should detect new file");

        // New task should be in cache
        let tasks = backend.list_tasks().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Externally created");
    }

    #[test]
    fn test_markdown_cache_detects_external_file_deletion() {
        let (dir, mut backend) = create_test_backend();

        // Create a task
        let task = Task::new("Will be deleted");
        backend.create_task(&task).unwrap();
        assert_eq!(backend.list_tasks().unwrap().len(), 1);

        // Externally delete the file (simulate git operation)
        let file_path = dir.path().join("tasks").join(format!("{}.md", task.id.0));
        fs::remove_file(&file_path).unwrap();

        // Refresh should detect the deletion
        let changes = backend.refresh();
        assert!(changes > 0, "Should detect file deletion");

        // Task should be removed from cache
        assert_eq!(backend.list_tasks().unwrap().len(), 0);
        assert!(backend.get_task(&task.id).unwrap().is_none());
    }

    #[test]
    fn test_markdown_cache_no_changes_detected() {
        let (_dir, mut backend) = create_test_backend();

        // Create a task
        let task = Task::new("Unchanged task");
        backend.create_task(&task).unwrap();

        // Refresh without any external changes
        let changes = backend.refresh();
        assert_eq!(changes, 0, "Should not detect changes when nothing changed");
    }

    #[test]
    fn test_markdown_project_cache_invalidation() {
        let (dir, mut backend) = create_test_backend();

        // Create a project
        let project = Project::new("Original project");
        backend.create_project(&project).unwrap();

        // Externally modify the file
        let file_path = dir
            .path()
            .join("projects")
            .join(format!("{}.md", project.id.0));
        let content = fs::read_to_string(&file_path).unwrap();
        let modified_content = content.replace("Original project", "Modified project");

        std::thread::sleep(std::time::Duration::from_millis(10));
        fs::write(&file_path, modified_content).unwrap();

        // Refresh should detect the change
        let changes = backend.refresh();
        assert!(changes > 0, "Should detect project modification");

        // Cache should have updated content
        let updated = backend.get_project(&project.id).unwrap().unwrap();
        assert_eq!(updated.name, "Modified project");
    }

    #[test]
    fn test_markdown_mtime_updated_on_write() {
        let (_dir, mut backend) = create_test_backend();

        // Create a task
        let task = Task::new("Test task");
        backend.create_task(&task).unwrap();

        // Verify mtime is tracked
        assert!(backend.task_mtimes.contains_key(&task.id));

        // Update the task
        let mut updated_task = task.clone();
        updated_task.title = "Updated task".to_string();
        backend.update_task(&updated_task).unwrap();

        // Mtime should still be tracked
        assert!(backend.task_mtimes.contains_key(&task.id));
    }

    #[test]
    fn test_markdown_refresh_via_trait() {
        use crate::storage::StorageBackend;

        let (dir, mut backend) = create_test_backend();

        // Create a task through normal API
        let task = Task::new("Original");
        backend.create_task(&task).unwrap();

        // Externally modify
        let file_path = dir.path().join("tasks").join(format!("{}.md", task.id.0));
        let content = fs::read_to_string(&file_path).unwrap();
        let modified = content.replace("Original", "Modified via trait");
        std::thread::sleep(std::time::Duration::from_millis(10));
        fs::write(&file_path, modified).unwrap();

        // Call refresh through the trait method
        let trait_backend: &mut dyn StorageBackend = &mut backend;
        let changes = trait_backend.refresh();
        assert!(changes > 0, "Trait refresh should detect changes");

        // Verify the change was picked up
        let updated = backend.get_task(&task.id).unwrap().unwrap();
        assert_eq!(updated.title, "Modified via trait");
    }
}
