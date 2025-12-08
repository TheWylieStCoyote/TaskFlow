//! Tests for YAML backend.

use super::*;
use crate::domain::{Priority, Project, Tag, Task, TaskStatus, TimeEntry};
use crate::storage::{
    ProjectRepository, StorageBackend, TagRepository, TaskRepository, TimeEntryRepository,
};
use tempfile::tempdir;

fn create_test_backend() -> (tempfile::TempDir, YamlBackend) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.yaml");
    let mut backend = YamlBackend::new(&path).unwrap();
    backend.initialize().unwrap();
    (dir, backend)
}

#[test]
fn test_yaml_task_crud() {
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
fn test_yaml_persistence() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.yaml");

    // Create and save
    {
        let mut backend = YamlBackend::new(&path).unwrap();
        backend.initialize().unwrap();

        let task = Task::new("Persistent task");
        backend.create_task(&task).unwrap();
        backend.flush().unwrap();
    }

    // Load and verify
    {
        let mut backend = YamlBackend::new(&path).unwrap();
        backend.initialize().unwrap();

        let tasks = backend.list_tasks().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Persistent task");
    }
}

#[test]
fn test_yaml_human_readable() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.yaml");

    let mut backend = YamlBackend::new(&path).unwrap();
    backend.initialize().unwrap();

    let task = Task::new("Human readable task")
        .with_priority(Priority::High)
        .with_status(TaskStatus::InProgress);
    backend.create_task(&task).unwrap();
    backend.flush().unwrap();

    // Read file content directly and verify it's YAML
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("title: Human readable task"));
    assert!(content.contains("priority: high"));
    assert!(content.contains("status: inprogress") || content.contains("status: in_progress"));
}

#[test]
fn test_yaml_project_crud() {
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
fn test_yaml_tag_crud() {
    let (_dir, mut backend) = create_test_backend();

    let tag = Tag {
        name: "test-tag".to_string(),
        color: Some("#ff0000".to_string()),
        description: None,
    };

    backend.save_tag(&tag).unwrap();

    let retrieved = backend.get_tag("test-tag").unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().color, Some("#ff0000".to_string()));

    backend.delete_tag("test-tag").unwrap();
    assert!(backend.get_tag("test-tag").unwrap().is_none());
}

#[test]
fn test_yaml_time_entry_crud() {
    let (_dir, mut backend) = create_test_backend();

    let task = Task::new("Task for time entry");
    backend.create_task(&task).unwrap();

    let entry = TimeEntry::start(task.id.clone());
    backend.create_time_entry(&entry).unwrap();

    let retrieved = backend.get_time_entry(&entry.id).unwrap();
    assert!(retrieved.is_some());
    assert!(retrieved.unwrap().is_running());

    backend.delete_time_entry(&entry.id).unwrap();
    assert!(backend.get_time_entry(&entry.id).unwrap().is_none());
}

#[test]
fn test_yaml_export_import_roundtrip() {
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
    let path2 = dir2.path().join("import.yaml");
    let mut backend2 = YamlBackend::new(&path2).unwrap();
    backend2.initialize().unwrap();
    backend2.import_all(&exported).unwrap();

    // Verify
    assert_eq!(backend2.list_tasks().unwrap().len(), 1);
    assert_eq!(backend2.list_projects().unwrap().len(), 1);
    assert_eq!(backend2.list_tags().unwrap().len(), 1);
}

#[test]
fn test_yaml_backend_type() {
    let (_dir, backend) = create_test_backend();
    assert_eq!(backend.backend_type(), "yaml");
}

#[test]
fn test_yaml_create_task_duplicate_id_fails() {
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
fn test_yaml_get_active_entry() {
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
