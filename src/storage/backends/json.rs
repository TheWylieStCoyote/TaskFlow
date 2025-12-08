use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::{
    Filter, PomodoroConfig, PomodoroSession, PomodoroStats, Project, ProjectId, Tag, Task, TaskId,
    TimeEntry, TimeEntryId, WorkLogEntry, WorkLogEntryId,
};
use crate::storage::{
    ExportData, ProjectRepository, StorageBackend, StorageError, StorageResult, TagRepository,
    TaskRepository, TimeEntryRepository, WorkLogRepository,
};

/// JSON file-based storage backend
///
/// Stores all data in a single JSON file for simplicity.
/// Good for small to medium datasets and easy backup/version control.
pub struct JsonBackend {
    path: PathBuf,
    data: ExportData,
    dirty: bool,
}

impl JsonBackend {
    /// Creates a new JSON backend at the given path.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`] if the backend cannot be created.
    pub fn new(path: &Path) -> StorageResult<Self> {
        Ok(Self {
            path: path.to_path_buf(),
            data: ExportData::default(),
            dirty: false,
        })
    }

    fn load(&mut self) -> StorageResult<()> {
        if self.path.exists() {
            let content =
                fs::read_to_string(&self.path).map_err(|e| StorageError::io(&self.path, e))?;
            self.data = serde_json::from_str(&content)?;
        }
        self.dirty = false;
        Ok(())
    }

    fn save(&mut self) -> StorageResult<()> {
        if !self.dirty {
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| StorageError::io(parent, e))?;
        }

        let content = serde_json::to_string_pretty(&self.data)
            .map_err(|e| StorageError::serialization(e.to_string()))?;

        fs::write(&self.path, content).map_err(|e| StorageError::io(&self.path, e))?;

        self.dirty = false;
        Ok(())
    }

    const fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

impl TaskRepository for JsonBackend {
    fn create_task(&mut self, task: &Task) -> StorageResult<()> {
        if self.data.tasks.iter().any(|t| t.id == task.id) {
            return Err(StorageError::already_exists("Task", task.id.to_string()));
        }
        self.data.tasks.push(task.clone());
        self.mark_dirty();
        Ok(())
    }

    fn get_task(&self, id: &TaskId) -> StorageResult<Option<Task>> {
        Ok(self.data.tasks.iter().find(|t| &t.id == id).cloned())
    }

    fn update_task(&mut self, task: &Task) -> StorageResult<()> {
        if let Some(existing) = self.data.tasks.iter_mut().find(|t| t.id == task.id) {
            *existing = task.clone();
            self.mark_dirty();
            Ok(())
        } else {
            Err(StorageError::not_found("Task", task.id.to_string()))
        }
    }

    fn delete_task(&mut self, id: &TaskId) -> StorageResult<()> {
        let len_before = self.data.tasks.len();
        self.data.tasks.retain(|t| &t.id != id);
        if self.data.tasks.len() == len_before {
            return Err(StorageError::not_found("Task", id.to_string()));
        }
        self.mark_dirty();
        Ok(())
    }

    fn list_tasks(&self) -> StorageResult<Vec<Task>> {
        Ok(self.data.tasks.clone())
    }

    fn list_tasks_filtered(&self, filter: &Filter) -> StorageResult<Vec<Task>> {
        let tasks = self
            .data
            .tasks
            .iter()
            .filter(|task| {
                // Filter by status
                if let Some(ref statuses) = filter.status {
                    if !statuses.contains(&task.status) {
                        return false;
                    }
                }

                // Filter by priority
                if let Some(ref priorities) = filter.priority {
                    if !priorities.contains(&task.priority) {
                        return false;
                    }
                }

                // Filter by project
                if let Some(ref project_id) = filter.project_id {
                    if task.project_id.as_ref() != Some(project_id) {
                        return false;
                    }
                }

                // Filter by tags
                if let Some(ref tags) = filter.tags {
                    let has_tags = match filter.tags_mode {
                        crate::domain::TagFilterMode::Any => {
                            tags.iter().any(|t| task.tags.contains(t))
                        }
                        crate::domain::TagFilterMode::All => {
                            tags.iter().all(|t| task.tags.contains(t))
                        }
                    };
                    if !has_tags && !tags.is_empty() {
                        return false;
                    }
                }

                // Filter by due date
                if let Some(due_before) = filter.due_before {
                    if let Some(due) = task.due_date {
                        if due >= due_before {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                if let Some(due_after) = filter.due_after {
                    if let Some(due) = task.due_date {
                        if due <= due_after {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                // Filter by search text
                if let Some(ref search) = filter.search_text {
                    let search_lower = search.to_lowercase();
                    let title_matches = task.title.to_lowercase().contains(&search_lower);
                    let desc_matches = task
                        .description
                        .as_ref()
                        .is_some_and(|d| d.to_lowercase().contains(&search_lower));
                    if !title_matches && !desc_matches {
                        return false;
                    }
                }

                // Filter completed tasks
                if !filter.include_completed && task.status.is_complete() {
                    return false;
                }

                true
            })
            .cloned()
            .collect();

        Ok(tasks)
    }

    fn get_tasks_by_project(&self, project_id: &ProjectId) -> StorageResult<Vec<Task>> {
        Ok(self
            .data
            .tasks
            .iter()
            .filter(|t| t.project_id.as_ref() == Some(project_id))
            .cloned()
            .collect())
    }

    fn get_tasks_by_tag(&self, tag: &str) -> StorageResult<Vec<Task>> {
        Ok(self
            .data
            .tasks
            .iter()
            .filter(|t| t.tags.contains(&tag.to_string()))
            .cloned()
            .collect())
    }
}

impl ProjectRepository for JsonBackend {
    fn create_project(&mut self, project: &Project) -> StorageResult<()> {
        if self.data.projects.iter().any(|p| p.id == project.id) {
            return Err(StorageError::already_exists(
                "Project",
                project.id.to_string(),
            ));
        }
        self.data.projects.push(project.clone());
        self.mark_dirty();
        Ok(())
    }

    fn get_project(&self, id: &ProjectId) -> StorageResult<Option<Project>> {
        Ok(self.data.projects.iter().find(|p| &p.id == id).cloned())
    }

    fn update_project(&mut self, project: &Project) -> StorageResult<()> {
        if let Some(existing) = self.data.projects.iter_mut().find(|p| p.id == project.id) {
            *existing = project.clone();
            self.mark_dirty();
            Ok(())
        } else {
            Err(StorageError::not_found("Project", project.id.to_string()))
        }
    }

    fn delete_project(&mut self, id: &ProjectId) -> StorageResult<()> {
        let len_before = self.data.projects.len();
        self.data.projects.retain(|p| &p.id != id);
        if self.data.projects.len() == len_before {
            return Err(StorageError::not_found("Project", id.to_string()));
        }
        self.mark_dirty();
        Ok(())
    }

    fn list_projects(&self) -> StorageResult<Vec<Project>> {
        Ok(self.data.projects.clone())
    }

    fn get_subprojects(&self, parent_id: &ProjectId) -> StorageResult<Vec<Project>> {
        Ok(self
            .data
            .projects
            .iter()
            .filter(|p| p.parent_id.as_ref() == Some(parent_id))
            .cloned()
            .collect())
    }
}

impl TagRepository for JsonBackend {
    fn save_tag(&mut self, tag: &Tag) -> StorageResult<()> {
        if let Some(existing) = self.data.tags.iter_mut().find(|t| t.name == tag.name) {
            *existing = tag.clone();
        } else {
            self.data.tags.push(tag.clone());
        }
        self.mark_dirty();
        Ok(())
    }

    fn get_tag(&self, name: &str) -> StorageResult<Option<Tag>> {
        Ok(self.data.tags.iter().find(|t| t.name == name).cloned())
    }

    fn delete_tag(&mut self, name: &str) -> StorageResult<()> {
        let len_before = self.data.tags.len();
        self.data.tags.retain(|t| t.name != name);
        if self.data.tags.len() == len_before {
            return Err(StorageError::not_found("Tag", name));
        }
        self.mark_dirty();
        Ok(())
    }

    fn list_tags(&self) -> StorageResult<Vec<Tag>> {
        Ok(self.data.tags.clone())
    }
}

impl TimeEntryRepository for JsonBackend {
    fn create_time_entry(&mut self, entry: &TimeEntry) -> StorageResult<()> {
        if self.data.time_entries.iter().any(|e| e.id == entry.id) {
            return Err(StorageError::already_exists(
                "TimeEntry",
                entry.id.0.to_string(),
            ));
        }
        self.data.time_entries.push(entry.clone());
        self.mark_dirty();
        Ok(())
    }

    fn get_time_entry(&self, id: &TimeEntryId) -> StorageResult<Option<TimeEntry>> {
        Ok(self.data.time_entries.iter().find(|e| &e.id == id).cloned())
    }

    fn update_time_entry(&mut self, entry: &TimeEntry) -> StorageResult<()> {
        if let Some(existing) = self.data.time_entries.iter_mut().find(|e| e.id == entry.id) {
            *existing = entry.clone();
            self.mark_dirty();
            Ok(())
        } else {
            Err(StorageError::not_found("TimeEntry", entry.id.0.to_string()))
        }
    }

    fn delete_time_entry(&mut self, id: &TimeEntryId) -> StorageResult<()> {
        let len_before = self.data.time_entries.len();
        self.data.time_entries.retain(|e| &e.id != id);
        if self.data.time_entries.len() == len_before {
            return Err(StorageError::not_found("TimeEntry", id.0.to_string()));
        }
        self.mark_dirty();
        Ok(())
    }

    fn get_entries_for_task(&self, task_id: &TaskId) -> StorageResult<Vec<TimeEntry>> {
        Ok(self
            .data
            .time_entries
            .iter()
            .filter(|e| &e.task_id == task_id)
            .cloned()
            .collect())
    }

    fn get_active_entry(&self) -> StorageResult<Option<TimeEntry>> {
        Ok(self
            .data
            .time_entries
            .iter()
            .find(|e| e.is_running())
            .cloned())
    }
}

impl WorkLogRepository for JsonBackend {
    fn create_work_log(&mut self, entry: &WorkLogEntry) -> StorageResult<()> {
        if self.data.work_logs.iter().any(|e| e.id == entry.id) {
            return Err(StorageError::already_exists(
                "WorkLogEntry",
                entry.id.0.to_string(),
            ));
        }
        self.data.work_logs.push(entry.clone());
        self.mark_dirty();
        Ok(())
    }

    fn get_work_log(&self, id: &WorkLogEntryId) -> StorageResult<Option<WorkLogEntry>> {
        Ok(self.data.work_logs.iter().find(|e| &e.id == id).cloned())
    }

    fn update_work_log(&mut self, entry: &WorkLogEntry) -> StorageResult<()> {
        if let Some(existing) = self.data.work_logs.iter_mut().find(|e| e.id == entry.id) {
            *existing = entry.clone();
            self.mark_dirty();
            Ok(())
        } else {
            Err(StorageError::not_found(
                "WorkLogEntry",
                entry.id.0.to_string(),
            ))
        }
    }

    fn delete_work_log(&mut self, id: &WorkLogEntryId) -> StorageResult<()> {
        let len_before = self.data.work_logs.len();
        self.data.work_logs.retain(|e| &e.id != id);
        if self.data.work_logs.len() == len_before {
            return Err(StorageError::not_found("WorkLogEntry", id.0.to_string()));
        }
        self.mark_dirty();
        Ok(())
    }

    fn get_work_logs_for_task(&self, task_id: &TaskId) -> StorageResult<Vec<WorkLogEntry>> {
        let mut logs: Vec<_> = self
            .data
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
        Ok(self.data.work_logs.clone())
    }
}

impl StorageBackend for JsonBackend {
    fn initialize(&mut self) -> StorageResult<()> {
        self.load()
    }

    fn flush(&mut self) -> StorageResult<()> {
        self.save()
    }

    fn export_all(&self) -> StorageResult<ExportData> {
        Ok(self.data.clone())
    }

    fn import_all(&mut self, data: &ExportData) -> StorageResult<()> {
        self.data = data.clone();
        self.mark_dirty();
        self.save()
    }

    fn backend_type(&self) -> &'static str {
        "json"
    }

    fn set_pomodoro_session(&mut self, session: Option<&PomodoroSession>) -> StorageResult<()> {
        self.data.pomodoro_session = session.cloned();
        self.mark_dirty();
        Ok(())
    }

    fn set_pomodoro_config(&mut self, config: &PomodoroConfig) -> StorageResult<()> {
        self.data.pomodoro_config = Some(config.clone());
        self.mark_dirty();
        Ok(())
    }

    fn set_pomodoro_stats(&mut self, stats: &PomodoroStats) -> StorageResult<()> {
        self.data.pomodoro_stats = Some(stats.clone());
        self.mark_dirty();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Priority, TagFilterMode, TaskStatus};
    use tempfile::tempdir;

    fn create_test_backend() -> (tempfile::TempDir, JsonBackend) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = JsonBackend::new(&path).unwrap();
        backend.initialize().unwrap();
        (dir, backend)
    }

    #[test]
    fn test_create_and_get_task() {
        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("Test task");
        backend.create_task(&task).unwrap();

        let retrieved = backend.get_task(&task.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test task");
    }

    #[test]
    fn test_persistence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        // Create and save
        {
            let mut backend = JsonBackend::new(&path).unwrap();
            backend.initialize().unwrap();
            let task = Task::new("Persistent task");
            backend.create_task(&task).unwrap();
            backend.flush().unwrap();
        }

        // Load and verify
        {
            let mut backend = JsonBackend::new(&path).unwrap();
            backend.initialize().unwrap();
            let tasks = backend.list_tasks().unwrap();
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].title, "Persistent task");
        }
    }

    #[test]
    fn test_create_task_duplicate_id_fails() {
        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("Original");
        backend.create_task(&task).unwrap();

        // Try to create another task with the same ID
        let mut duplicate = Task::new("Duplicate");
        duplicate.id = task.id.clone();

        let result = backend.create_task(&duplicate);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_task_not_found() {
        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("Non-existent");
        let result = backend.update_task(&task);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_task_not_found() {
        let (_dir, mut backend) = create_test_backend();

        let task_id = TaskId::new();
        let result = backend.delete_task(&task_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_tasks_empty() {
        let (_dir, backend) = create_test_backend();

        let tasks = backend.list_tasks().unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_list_tasks_filtered_by_status() {
        let (_dir, mut backend) = create_test_backend();

        let task1 = Task::new("Todo task");
        let task2 = Task::new("Done task").with_status(TaskStatus::Done);
        backend.create_task(&task1).unwrap();
        backend.create_task(&task2).unwrap();

        let filter = Filter {
            status: Some(vec![TaskStatus::Todo]),
            include_completed: true,
            ..Filter::default()
        };

        let tasks = backend.list_tasks_filtered(&filter).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Todo task");
    }

    #[test]
    fn test_list_tasks_filtered_by_priority() {
        let (_dir, mut backend) = create_test_backend();

        let task1 = Task::new("High priority").with_priority(Priority::High);
        let task2 = Task::new("Low priority").with_priority(Priority::Low);
        backend.create_task(&task1).unwrap();
        backend.create_task(&task2).unwrap();

        let filter = Filter {
            priority: Some(vec![Priority::High]),
            ..Filter::default()
        };

        let tasks = backend.list_tasks_filtered(&filter).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "High priority");
    }

    #[test]
    fn test_list_tasks_filtered_by_tags_any() {
        let (_dir, mut backend) = create_test_backend();

        let task1 = Task::new("Task with rust").with_tags(vec!["rust".to_string()]);
        let task2 = Task::new("Task with python").with_tags(vec!["python".to_string()]);
        let task3 =
            Task::new("Task with both").with_tags(vec!["rust".to_string(), "python".to_string()]);
        backend.create_task(&task1).unwrap();
        backend.create_task(&task2).unwrap();
        backend.create_task(&task3).unwrap();

        let filter = Filter {
            tags: Some(vec!["rust".to_string()]),
            tags_mode: TagFilterMode::Any,
            ..Filter::default()
        };

        let tasks = backend.list_tasks_filtered(&filter).unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_list_tasks_filtered_by_tags_all() {
        let (_dir, mut backend) = create_test_backend();

        let task1 = Task::new("Task with rust").with_tags(vec!["rust".to_string()]);
        let task2 = Task::new("Task with both")
            .with_tags(vec!["rust".to_string(), "important".to_string()]);
        backend.create_task(&task1).unwrap();
        backend.create_task(&task2).unwrap();

        let filter = Filter {
            tags: Some(vec!["rust".to_string(), "important".to_string()]),
            tags_mode: TagFilterMode::All,
            ..Filter::default()
        };

        let tasks = backend.list_tasks_filtered(&filter).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Task with both");
    }

    #[test]
    fn test_get_tasks_by_project() {
        let (_dir, mut backend) = create_test_backend();

        let project = Project::new("Test project");
        backend.create_project(&project).unwrap();

        let task1 = Task::new("In project").with_project(project.id.clone());
        let task2 = Task::new("Not in project");
        backend.create_task(&task1).unwrap();
        backend.create_task(&task2).unwrap();

        let tasks = backend.get_tasks_by_project(&project.id).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "In project");
    }

    #[test]
    fn test_get_tasks_by_tag() {
        let (_dir, mut backend) = create_test_backend();

        let task1 = Task::new("Tagged").with_tags(vec!["important".to_string()]);
        let task2 = Task::new("Not tagged");
        backend.create_task(&task1).unwrap();
        backend.create_task(&task2).unwrap();

        let tasks = backend.get_tasks_by_tag("important").unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Tagged");
    }

    #[test]
    fn test_project_crud() {
        let (_dir, mut backend) = create_test_backend();

        // Create
        let project = Project::new("Test project");
        backend.create_project(&project).unwrap();

        // Read
        let retrieved = backend.get_project(&project.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test project");

        // Update
        let mut updated = project.clone();
        updated.name = "Updated project".to_string();
        backend.update_project(&updated).unwrap();

        let retrieved = backend.get_project(&project.id).unwrap().unwrap();
        assert_eq!(retrieved.name, "Updated project");

        // Delete
        backend.delete_project(&project.id).unwrap();
        let retrieved = backend.get_project(&project.id).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_tag_crud() {
        let (_dir, mut backend) = create_test_backend();

        // Create (save_tag is upsert)
        let tag = Tag::new("rust");
        backend.save_tag(&tag).unwrap();

        // Read
        let retrieved = backend.get_tag("rust").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "rust");

        // Update (upsert with same name)
        let mut updated = tag.clone();
        updated.color = Some("#ff0000".to_string());
        backend.save_tag(&updated).unwrap();

        let retrieved = backend.get_tag("rust").unwrap().unwrap();
        assert_eq!(retrieved.color, Some("#ff0000".to_string()));

        // Delete
        backend.delete_tag("rust").unwrap();
        let retrieved = backend.get_tag("rust").unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_time_entry_crud() {
        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("Test task");
        backend.create_task(&task).unwrap();

        // Create
        let entry = TimeEntry::start(task.id.clone());
        backend.create_time_entry(&entry).unwrap();

        // Read
        let retrieved = backend.get_time_entry(&entry.id).unwrap();
        assert!(retrieved.is_some());

        // Update
        let mut updated = entry.clone();
        updated.stop();
        backend.update_time_entry(&updated).unwrap();

        let retrieved = backend.get_time_entry(&entry.id).unwrap().unwrap();
        assert!(retrieved.ended_at.is_some());

        // Delete
        backend.delete_time_entry(&entry.id).unwrap();
        let retrieved = backend.get_time_entry(&entry.id).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_get_active_entry() {
        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("Test task");
        backend.create_task(&task).unwrap();

        // No active entry initially
        let active = backend.get_active_entry().unwrap();
        assert!(active.is_none());

        // Create running entry
        let entry = TimeEntry::start(task.id.clone());
        backend.create_time_entry(&entry).unwrap();

        let active = backend.get_active_entry().unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, entry.id);

        // Stop entry
        let mut stopped = entry.clone();
        stopped.stop();
        backend.update_time_entry(&stopped).unwrap();

        let active = backend.get_active_entry().unwrap();
        assert!(active.is_none());
    }

    #[test]
    fn test_get_entries_for_task() {
        let (_dir, mut backend) = create_test_backend();

        let task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");
        backend.create_task(&task1).unwrap();
        backend.create_task(&task2).unwrap();

        let mut entry1 = TimeEntry::start(task1.id.clone());
        entry1.stop();
        let mut entry2 = TimeEntry::start(task1.id.clone());
        entry2.stop();
        let mut entry3 = TimeEntry::start(task2.id.clone());
        entry3.stop();

        backend.create_time_entry(&entry1).unwrap();
        backend.create_time_entry(&entry2).unwrap();
        backend.create_time_entry(&entry3).unwrap();

        let entries = backend.get_entries_for_task(&task1.id).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_export_import_roundtrip() {
        let (_dir, mut backend) = create_test_backend();

        // Create some data
        let task = Task::new("Test task");
        let project = Project::new("Test project");
        let tag = Tag::new("test");
        backend.create_task(&task).unwrap();
        backend.create_project(&project).unwrap();
        backend.save_tag(&tag).unwrap();

        // Export
        let exported = backend.export_all().unwrap();
        assert_eq!(exported.tasks.len(), 1);
        assert_eq!(exported.projects.len(), 1);
        assert_eq!(exported.tags.len(), 1);

        // Create new backend and import
        let dir2 = tempdir().unwrap();
        let path2 = dir2.path().join("test2.json");
        let mut backend2 = JsonBackend::new(&path2).unwrap();
        backend2.initialize().unwrap();

        backend2.import_all(&exported).unwrap();

        assert_eq!(backend2.list_tasks().unwrap().len(), 1);
        assert_eq!(backend2.list_projects().unwrap().len(), 1);
        assert_eq!(backend2.list_tags().unwrap().len(), 1);
    }

    #[test]
    fn test_flush_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("new_file.json");

        assert!(!path.exists());

        let mut backend = JsonBackend::new(&path).unwrap();
        backend.initialize().unwrap();
        let task = Task::new("Test");
        backend.create_task(&task).unwrap();
        backend.flush().unwrap();

        assert!(path.exists());
    }

    #[test]
    fn test_initialize_loads_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        // Create file manually with some data
        let data = ExportData {
            tasks: vec![Task::new("Pre-existing task")],
            ..Default::default()
        };
        std::fs::write(&path, serde_json::to_string(&data).unwrap()).unwrap();

        // Initialize should load existing data
        let mut backend = JsonBackend::new(&path).unwrap();
        backend.initialize().unwrap();

        let tasks = backend.list_tasks().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Pre-existing task");
    }

    #[test]
    fn test_pomodoro_state_persistence() {
        use crate::domain::{PomodoroConfig, PomodoroSession, PomodoroStats};

        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        // Create backend and add pomodoro state
        {
            let mut backend = JsonBackend::new(&path).unwrap();
            backend.initialize().unwrap();

            // Create a task for the Pomodoro session
            let task = Task::new("Work task");
            backend.create_task(&task).unwrap();

            // Set Pomodoro state
            let config = PomodoroConfig::default().with_work_duration(30);
            let session = PomodoroSession::new(task.id.clone(), &config, 4);
            let mut stats = PomodoroStats::new();
            stats.record_cycle(25);

            backend.set_pomodoro_config(&config).unwrap();
            backend.set_pomodoro_session(Some(&session)).unwrap();
            backend.set_pomodoro_stats(&stats).unwrap();
            backend.flush().unwrap();
        }

        // Reload and verify
        {
            let mut backend = JsonBackend::new(&path).unwrap();
            backend.initialize().unwrap();

            let exported = backend.export_all().unwrap();

            assert!(exported.pomodoro_session.is_some());
            assert!(exported.pomodoro_config.is_some());
            assert!(exported.pomodoro_stats.is_some());

            let config = exported.pomodoro_config.unwrap();
            assert_eq!(config.work_duration_mins, 30);

            let stats = exported.pomodoro_stats.unwrap();
            assert_eq!(stats.total_cycles, 1);
        }
    }

    #[test]
    fn test_pomodoro_session_clear() {
        use crate::domain::{PomodoroConfig, PomodoroSession};

        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("Work task");
        backend.create_task(&task).unwrap();

        let config = PomodoroConfig::default();
        let session = PomodoroSession::new(task.id.clone(), &config, 4);

        // Set session
        backend.set_pomodoro_session(Some(&session)).unwrap();
        let exported = backend.export_all().unwrap();
        assert!(exported.pomodoro_session.is_some());

        // Clear session
        backend.set_pomodoro_session(None).unwrap();
        let exported = backend.export_all().unwrap();
        assert!(exported.pomodoro_session.is_none());
    }

    // Edge case tests for error handling

    #[test]
    fn test_corrupted_json_file_handling() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("corrupted.json");

        // Write invalid JSON
        fs::write(&path, "{ not valid json ]").unwrap();

        // Try to initialize - should fail with deserialization error
        let mut backend = JsonBackend::new(&path).unwrap();
        let result = backend.initialize();
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_json_file_handling() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.json");

        // Write empty file
        fs::write(&path, "").unwrap();

        // Try to initialize - should fail with deserialization error
        let mut backend = JsonBackend::new(&path).unwrap();
        let result = backend.initialize();
        assert!(result.is_err());
    }

    #[test]
    fn test_partial_json_file_handling() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("partial.json");

        // Write partial JSON (missing closing brace)
        fs::write(&path, r#"{"tasks": {"abc": {"id": "abc""#).unwrap();

        // Try to initialize - should fail
        let mut backend = JsonBackend::new(&path).unwrap();
        let result = backend.initialize();
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_json_wrong_schema() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wrong_schema.json");

        // Write valid JSON but wrong schema
        fs::write(&path, r#"{"wrong": "schema", "tasks": "not_a_map"}"#).unwrap();

        // Try to initialize - should fail with deserialization error
        let mut backend = JsonBackend::new(&path).unwrap();
        let result = backend.initialize();
        assert!(result.is_err());
    }

    #[test]
    fn test_project_not_found_error() {
        let (_dir, backend) = create_test_backend();
        let result = backend.get_project(&ProjectId::new());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_time_entry_not_found_error() {
        let (_dir, backend) = create_test_backend();
        let result = backend.get_time_entry(&TimeEntryId::new());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_update_project_not_found() {
        let (_dir, mut backend) = create_test_backend();
        let project = Project::new("Non-existent");
        let result = backend.update_project(&project);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_project_not_found() {
        let (_dir, mut backend) = create_test_backend();
        let result = backend.delete_project(&ProjectId::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_update_time_entry_not_found() {
        let (_dir, mut backend) = create_test_backend();
        let entry = TimeEntry::start(TaskId::new());
        let result = backend.update_time_entry(&entry);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_time_entry_not_found() {
        let (_dir, mut backend) = create_test_backend();
        let result = backend.delete_time_entry(&TimeEntryId::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_concurrent_task_operations() {
        let (_dir, mut backend) = create_test_backend();

        // Create many tasks rapidly
        let mut task_ids = Vec::new();
        for i in 0..100 {
            let task = Task::new(format!("Task {i}"));
            task_ids.push(task.id.clone());
            backend.create_task(&task).unwrap();
        }

        // Verify all tasks exist
        let tasks = backend.list_tasks().unwrap();
        assert_eq!(tasks.len(), 100);

        // Delete all tasks
        for id in &task_ids {
            backend.delete_task(id).unwrap();
        }

        // Verify all tasks deleted
        let tasks = backend.list_tasks().unwrap();
        assert_eq!(tasks.len(), 0);
    }

    #[test]
    fn test_special_characters_in_task_title() {
        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("Task with \"quotes\" and \\ backslash and emoji 🎉");
        backend.create_task(&task).unwrap();

        let retrieved = backend.get_task(&task.id).unwrap().unwrap();
        assert_eq!(retrieved.title, task.title);
    }

    #[test]
    fn test_unicode_in_project_name() {
        let (_dir, mut backend) = create_test_backend();

        let project = Project::new("项目 プロジェクト مشروع");
        backend.create_project(&project).unwrap();

        let retrieved = backend.get_project(&project.id).unwrap().unwrap();
        assert_eq!(retrieved.name, project.name);
    }

    #[test]
    fn test_very_long_task_title() {
        let (_dir, mut backend) = create_test_backend();

        let long_title = "A".repeat(10000);
        let task = Task::new(long_title.clone());
        backend.create_task(&task).unwrap();

        let retrieved = backend.get_task(&task.id).unwrap().unwrap();
        assert_eq!(retrieved.title, long_title);
    }
}
