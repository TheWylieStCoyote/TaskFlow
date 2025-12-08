//! Storage integration tests.

use crate::app::Model;
use crate::domain::Task;
use crate::storage::{backends::MarkdownBackend, BackendType, StorageBackend, TaskRepository};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_model_refresh_storage_without_backend() {
    let mut model = Model::new();
    // No storage backend attached
    assert!(model.storage.is_none());

    // refresh_storage should return 0 when no backend
    let changes = model.refresh_storage();
    assert_eq!(changes, 0);
}

#[test]
fn test_model_refresh_storage_with_markdown_backend() {
    let dir = tempdir().unwrap();

    // First, create a task directly in the markdown directory structure
    let mut backend = MarkdownBackend::new(dir.path()).unwrap();
    backend.initialize().unwrap();
    let task = Task::new("Original task");
    let task_id = task.id;
    backend.create_task(&task).unwrap();
    backend.flush().unwrap();
    drop(backend); // Close the backend

    // Create model using the proper API
    let mut model = Model::new()
        .with_storage(BackendType::Markdown, dir.path().to_path_buf())
        .unwrap();
    assert_eq!(model.tasks.len(), 1);
    assert_eq!(model.tasks.get(&task_id).unwrap().title, "Original task");

    // Externally modify the file
    let file_path = dir.path().join("tasks").join(format!("{}.md", task_id.0));
    let content = fs::read_to_string(&file_path).unwrap();
    let modified = content.replace("Original task", "Externally modified");
    std::thread::sleep(std::time::Duration::from_millis(10));
    fs::write(&file_path, modified).unwrap();

    // Refresh should detect the change
    let changes = model.refresh_storage();
    assert!(changes > 0, "Should detect external modification");

    // Model should have updated task
    assert_eq!(
        model.tasks.get(&task_id).unwrap().title,
        "Externally modified"
    );
}
