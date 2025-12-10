//! Tests for markdown backend.

use std::fs;

use tempfile::tempdir;

use crate::domain::{Priority, Project, Tag, Task, TaskId, TimeEntry};
use crate::storage::{
    ProjectRepository, StorageBackend, TagRepository, TaskRepository, TimeEntryRepository,
};

use super::MarkdownBackend;

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

    let entry = TimeEntry::start(task.id);
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
    let entry = TimeEntry::start(task.id);
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
        id: task.id,
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

// ==================== Habit Repository Tests ====================

#[test]
fn test_markdown_habit_crud() {
    use crate::domain::Habit;
    use crate::storage::HabitRepository;

    let (_dir, mut backend) = create_test_backend();

    // Create
    let habit = Habit::new("Exercise");
    backend.create_habit(&habit).unwrap();

    // Read
    let retrieved = backend.get_habit(&habit.id).unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "Exercise");

    // Update
    let mut updated_habit = habit.clone();
    updated_habit.name = "Morning Exercise".to_string();
    backend.update_habit(&updated_habit).unwrap();

    let retrieved = backend.get_habit(&habit.id).unwrap().unwrap();
    assert_eq!(retrieved.name, "Morning Exercise");

    // Delete
    backend.delete_habit(&habit.id).unwrap();
    assert!(backend.get_habit(&habit.id).unwrap().is_none());
}

#[test]
fn test_markdown_habit_list() {
    use crate::domain::Habit;
    use crate::storage::HabitRepository;

    let (_dir, mut backend) = create_test_backend();

    let habit1 = Habit::new("Exercise");
    let habit2 = Habit::new("Read");
    backend.create_habit(&habit1).unwrap();
    backend.create_habit(&habit2).unwrap();

    let habits = backend.list_habits().unwrap();
    assert_eq!(habits.len(), 2);
}

#[test]
fn test_markdown_habit_list_active() {
    use crate::domain::Habit;
    use crate::storage::HabitRepository;

    let (_dir, mut backend) = create_test_backend();

    let active = Habit::new("Active habit");
    let mut archived = Habit::new("Archived habit");
    archived.archived = true;

    backend.create_habit(&active).unwrap();
    backend.create_habit(&archived).unwrap();

    let active_habits = backend.list_active_habits().unwrap();
    assert_eq!(active_habits.len(), 1);
    assert_eq!(active_habits[0].name, "Active habit");
}

#[test]
fn test_markdown_habit_duplicate_id_fails() {
    use crate::domain::Habit;
    use crate::storage::HabitRepository;

    let (_dir, mut backend) = create_test_backend();

    let habit = Habit::new("Original");
    backend.create_habit(&habit).unwrap();

    let duplicate = Habit {
        id: habit.id,
        ..Habit::new("Duplicate")
    };

    let result = backend.create_habit(&duplicate);
    assert!(result.is_err());
}

#[test]
fn test_markdown_habit_update_not_found() {
    use crate::domain::Habit;
    use crate::storage::HabitRepository;

    let (_dir, mut backend) = create_test_backend();

    let habit = Habit::new("Non-existent");
    let result = backend.update_habit(&habit);
    assert!(result.is_err());
}

#[test]
fn test_markdown_habit_delete_not_found() {
    use crate::domain::HabitId;
    use crate::storage::HabitRepository;

    let (_dir, mut backend) = create_test_backend();

    let result = backend.delete_habit(&HabitId::new());
    assert!(result.is_err());
}

// ==================== Work Log Repository Tests ====================

#[test]
fn test_markdown_work_log_crud() {
    use crate::domain::WorkLogEntry;
    use crate::storage::WorkLogRepository;

    let (_dir, mut backend) = create_test_backend();

    let task = Task::new("Test task");
    backend.create_task(&task).unwrap();

    // Create
    let entry = WorkLogEntry::new(task.id, "Did some work");
    backend.create_work_log(&entry).unwrap();

    // Read
    let retrieved = backend.get_work_log(&entry.id).unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().summary(), "Did some work");

    // Update
    let mut updated_entry = entry.clone();
    updated_entry.content = "Did more work".to_string();
    backend.update_work_log(&updated_entry).unwrap();

    let retrieved = backend.get_work_log(&entry.id).unwrap().unwrap();
    assert_eq!(retrieved.summary(), "Did more work");

    // Delete
    backend.delete_work_log(&entry.id).unwrap();
    assert!(backend.get_work_log(&entry.id).unwrap().is_none());
}

#[test]
fn test_markdown_work_log_list() {
    use crate::domain::WorkLogEntry;
    use crate::storage::WorkLogRepository;

    let (_dir, mut backend) = create_test_backend();

    let task = Task::new("Test task");
    backend.create_task(&task).unwrap();

    let entry1 = WorkLogEntry::new(task.id, "Work 1");
    let entry2 = WorkLogEntry::new(task.id, "Work 2");
    backend.create_work_log(&entry1).unwrap();
    backend.create_work_log(&entry2).unwrap();

    let logs = backend.list_work_logs().unwrap();
    assert_eq!(logs.len(), 2);
}

#[test]
fn test_markdown_work_log_get_for_task() {
    use crate::domain::WorkLogEntry;
    use crate::storage::WorkLogRepository;

    let (_dir, mut backend) = create_test_backend();

    let task1 = Task::new("Task 1");
    let task2 = Task::new("Task 2");
    backend.create_task(&task1).unwrap();
    backend.create_task(&task2).unwrap();

    let entry1 = WorkLogEntry::new(task1.id, "Work on task 1");
    let entry2 = WorkLogEntry::new(task2.id, "Work on task 2");
    backend.create_work_log(&entry1).unwrap();
    backend.create_work_log(&entry2).unwrap();

    let logs = backend.get_work_logs_for_task(&task1.id).unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].summary(), "Work on task 1");
}

#[test]
fn test_markdown_work_log_duplicate_id_fails() {
    use crate::domain::WorkLogEntry;
    use crate::storage::WorkLogRepository;

    let (_dir, mut backend) = create_test_backend();

    let task = Task::new("Test task");
    backend.create_task(&task).unwrap();

    let entry = WorkLogEntry::new(task.id, "Original");
    backend.create_work_log(&entry).unwrap();

    let duplicate = WorkLogEntry {
        id: entry.id,
        ..WorkLogEntry::new(task.id, "Duplicate")
    };

    let result = backend.create_work_log(&duplicate);
    assert!(result.is_err());
}

#[test]
fn test_markdown_work_log_update_not_found() {
    use crate::domain::WorkLogEntry;
    use crate::storage::WorkLogRepository;

    let (_dir, mut backend) = create_test_backend();

    let task = Task::new("Test task");
    backend.create_task(&task).unwrap();

    let entry = WorkLogEntry::new(task.id, "Non-existent");
    let result = backend.update_work_log(&entry);
    assert!(result.is_err());
}

#[test]
fn test_markdown_work_log_delete_not_found() {
    use crate::domain::WorkLogEntryId;
    use crate::storage::WorkLogRepository;

    let (_dir, mut backend) = create_test_backend();

    let result = backend.delete_work_log(&WorkLogEntryId::new());
    assert!(result.is_err());
}
