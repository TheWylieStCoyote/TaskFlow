use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::{Filter, Project, ProjectId, Tag, Task, TaskId, TimeEntry, TimeEntryId};
use crate::storage::{
    ExportData, ProjectRepository, StorageBackend, StorageError, StorageResult, TagRepository,
    TaskRepository, TimeEntryRepository,
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
pub struct MarkdownBackend {
    base_path: PathBuf,
    tasks_dir: PathBuf,
    projects_dir: PathBuf,
    // Cache for performance
    tasks_cache: HashMap<TaskId, Task>,
    projects_cache: HashMap<ProjectId, Project>,
    tags: Vec<Tag>,
    time_entries: Vec<TimeEntry>,
    dirty: bool,
}

impl MarkdownBackend {
    pub fn new(path: &Path) -> StorageResult<Self> {
        Ok(Self {
            base_path: path.to_path_buf(),
            tasks_dir: path.join("tasks"),
            projects_dir: path.join("projects"),
            tasks_cache: HashMap::new(),
            projects_cache: HashMap::new(),
            tags: Vec::new(),
            time_entries: Vec::new(),
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

        if !self.tasks_dir.exists() {
            return Ok(());
        }

        for entry in
            fs::read_dir(&self.tasks_dir).map_err(|e| StorageError::io(&self.tasks_dir, e))?
        {
            let entry = entry.map_err(|e| StorageError::io(&self.tasks_dir, e))?;
            let path = entry.path();

            if path.extension().map(|e| e == "md").unwrap_or(false) {
                if let Ok(task) = self.parse_task_file(&path) {
                    self.tasks_cache.insert(task.id.clone(), task);
                }
            }
        }

        Ok(())
    }

    fn load_projects(&mut self) -> StorageResult<()> {
        self.projects_cache.clear();

        if !self.projects_dir.exists() {
            return Ok(());
        }

        for entry in
            fs::read_dir(&self.projects_dir).map_err(|e| StorageError::io(&self.projects_dir, e))?
        {
            let entry = entry.map_err(|e| StorageError::io(&self.projects_dir, e))?;
            let path = entry.path();

            if path.extension().map(|e| e == "md").unwrap_or(false) {
                if let Ok(project) = self.parse_project_file(&path) {
                    self.projects_cache.insert(project.id.clone(), project);
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

    fn write_task_file(&self, task: &Task) -> StorageResult<()> {
        let path = self.tasks_dir.join(format!("{}.md", task.id.0));

        // Create frontmatter-friendly version (without description in frontmatter)
        let mut task_for_yaml = task.clone();
        task_for_yaml.description = None;

        let frontmatter = serde_yaml::to_string(&task_for_yaml)
            .map_err(|e| StorageError::serialization(e.to_string()))?;

        let mut content = format!("---\n{}---\n", frontmatter);

        // Add description as body
        if let Some(ref desc) = task.description {
            content.push('\n');
            content.push_str(desc);
            content.push('\n');
        }

        fs::write(&path, content).map_err(|e| StorageError::io(&path, e))?;

        Ok(())
    }

    fn write_project_file(&self, project: &Project) -> StorageResult<()> {
        let path = self.projects_dir.join(format!("{}.md", project.id.0));

        let mut project_for_yaml = project.clone();
        project_for_yaml.description = None;

        let frontmatter = serde_yaml::to_string(&project_for_yaml)
            .map_err(|e| StorageError::serialization(e.to_string()))?;

        let mut content = format!("---\n{}---\n", frontmatter);

        if let Some(ref desc) = project.description {
            content.push('\n');
            content.push_str(desc);
            content.push('\n');
        }

        fs::write(&path, content).map_err(|e| StorageError::io(&path, e))?;

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
    fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

impl TaskRepository for MarkdownBackend {
    fn create_task(&mut self, task: &Task) -> StorageResult<()> {
        if self.tasks_cache.contains_key(&task.id) {
            return Err(StorageError::already_exists("Task", task.id.to_string()));
        }
        self.write_task_file(task)?;
        self.tasks_cache.insert(task.id.clone(), task.clone());
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
        self.tasks_cache.insert(task.id.clone(), task.clone());
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
        self.projects_cache
            .insert(project.id.clone(), project.clone());
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
        self.projects_cache
            .insert(project.id.clone(), project.clone());
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

impl StorageBackend for MarkdownBackend {
    fn initialize(&mut self) -> StorageResult<()> {
        self.ensure_dirs()?;
        self.load_tasks()?;
        self.load_projects()?;
        self.load_tags()?;
        self.load_time_entries()?;
        Ok(())
    }

    fn flush(&mut self) -> StorageResult<()> {
        // Files are written immediately, but we save auxiliary data
        self.save_tags()?;
        self.save_time_entries()?;
        self.dirty = false;
        Ok(())
    }

    fn export_all(&self) -> StorageResult<ExportData> {
        Ok(ExportData {
            tasks: self.tasks_cache.values().cloned().collect(),
            projects: self.projects_cache.values().cloned().collect(),
            tags: self.tags.clone(),
            time_entries: self.time_entries.clone(),
            version: 1,
        })
    }

    fn import_all(&mut self, data: &ExportData) -> StorageResult<()> {
        // Clear existing data
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

        Ok(())
    }

    fn backend_type(&self) -> &'static str {
        "markdown"
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
}
