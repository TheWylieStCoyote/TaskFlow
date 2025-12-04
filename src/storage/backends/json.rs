use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::{Filter, Project, ProjectId, Tag, Task, TaskId, TimeEntry, TimeEntryId};
use crate::storage::{
    ExportData, ProjectRepository, StorageBackend, StorageError, StorageResult, TagRepository,
    TaskRepository, TimeEntryRepository,
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
    pub fn new(path: &Path) -> StorageResult<Self> {
        Ok(Self {
            path: path.to_path_buf(),
            data: ExportData::default(),
            dirty: false,
        })
    }

    fn load(&mut self) -> StorageResult<()> {
        if self.path.exists() {
            let content = fs::read_to_string(&self.path)
                .map_err(|e| StorageError::io(&self.path, e))?;
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
            fs::create_dir_all(parent)
                .map_err(|e| StorageError::io(parent, e))?;
        }

        let content = serde_json::to_string_pretty(&self.data)
            .map_err(|e| StorageError::serialization(e.to_string()))?;

        fs::write(&self.path, content)
            .map_err(|e| StorageError::io(&self.path, e))?;

        self.dirty = false;
        Ok(())
    }

    fn mark_dirty(&mut self) {
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
        let tasks = self.data.tasks.iter().filter(|task| {
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
                    .map(|d| d.to_lowercase().contains(&search_lower))
                    .unwrap_or(false);
                if !title_matches && !desc_matches {
                    return false;
                }
            }

            // Filter completed tasks
            if !filter.include_completed && task.status.is_complete() {
                return false;
            }

            true
        }).cloned().collect();

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
            return Err(StorageError::already_exists("Project", project.id.to_string()));
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
            return Err(StorageError::already_exists("TimeEntry", entry.id.0.to_string()));
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
        Ok(self.data.time_entries.iter().find(|e| e.is_running()).cloned())
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_and_get_task() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = JsonBackend::new(&path).unwrap();
        backend.initialize().unwrap();

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
}
