//! Tests for SQLite backend.

use rusqlite::{params, Connection};
use tempfile::tempdir;

use crate::domain::{Filter, Priority, Project, Tag, Task, TaskId, TaskStatus, TimeEntry};
use crate::storage::{
    ProjectRepository, StorageBackend, TagRepository, TaskRepository, TimeEntryRepository,
};

use super::SqliteBackend;

fn create_test_backend() -> (tempfile::TempDir, SqliteBackend) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    let mut backend = SqliteBackend::new(&path).unwrap();
    backend.initialize().unwrap();
    (dir, backend)
}

#[test]
fn test_sqlite_initialize_creates_tables() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    let mut backend = SqliteBackend::new(&path).unwrap();
    backend.initialize().unwrap();

    let conn = backend.inner.conn().unwrap();

    // Check tables exist
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    assert!(tables.contains(&"tasks".to_string()));
    assert!(tables.contains(&"projects".to_string()));
    assert!(tables.contains(&"tags".to_string()));
    assert!(tables.contains(&"time_entries".to_string()));
}

#[test]
fn test_sqlite_task_crud() {
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
fn test_sqlite_uuid_roundtrip() {
    let (_dir, mut backend) = create_test_backend();

    let task = Task::new("UUID test");
    let original_id = task.id;
    backend.create_task(&task).unwrap();

    let retrieved = backend.get_task(&original_id).unwrap().unwrap();
    assert_eq!(retrieved.id, original_id);
}

#[test]
fn test_sqlite_datetime_roundtrip() {
    let (_dir, mut backend) = create_test_backend();

    let task = Task::new("DateTime test");
    let original_created = task.created_at;
    backend.create_task(&task).unwrap();

    let retrieved = backend.get_task(&task.id).unwrap().unwrap();
    // Compare timestamps at second precision (RFC3339 may lose subseconds)
    assert_eq!(
        retrieved.created_at.timestamp(),
        original_created.timestamp()
    );
}

#[test]
fn test_sqlite_enum_roundtrip() {
    let (_dir, mut backend) = create_test_backend();

    let task = Task::new("Enum test")
        .with_priority(Priority::Urgent)
        .with_status(TaskStatus::InProgress);
    backend.create_task(&task).unwrap();

    let retrieved = backend.get_task(&task.id).unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::Urgent);
    assert_eq!(retrieved.status, TaskStatus::InProgress);
}

#[test]
fn test_sqlite_json_fields_roundtrip() {
    let (_dir, mut backend) = create_test_backend();

    let task = Task::new("JSON test").with_tags(vec!["tag1".to_string(), "tag2".to_string()]);
    backend.create_task(&task).unwrap();

    let retrieved = backend.get_task(&task.id).unwrap().unwrap();
    assert_eq!(retrieved.tags.len(), 2);
    assert!(retrieved.tags.contains(&"tag1".to_string()));
    assert!(retrieved.tags.contains(&"tag2".to_string()));
}

#[test]
fn test_sqlite_null_optional_fields() {
    let (_dir, mut backend) = create_test_backend();

    let task = Task::new("Null fields test");
    // task has None for: description, project_id, parent_task_id, due_date, etc.
    backend.create_task(&task).unwrap();

    let retrieved = backend.get_task(&task.id).unwrap().unwrap();
    assert!(retrieved.description.is_none());
    assert!(retrieved.project_id.is_none());
    assert!(retrieved.due_date.is_none());
    assert!(retrieved.completed_at.is_none());
}

#[test]
fn test_sqlite_project_crud() {
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
fn test_sqlite_tag_crud() {
    let (_dir, mut backend) = create_test_backend();

    let tag = Tag {
        name: "test-tag".to_string(),
        color: Some("#ff0000".to_string()),
        description: Some("A test tag".to_string()),
    };

    backend.save_tag(&tag).unwrap();

    let retrieved = backend.get_tag("test-tag").unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.color, Some("#ff0000".to_string()));
    assert_eq!(retrieved.description, Some("A test tag".to_string()));

    backend.delete_tag("test-tag").unwrap();
    assert!(backend.get_tag("test-tag").unwrap().is_none());
}

#[test]
fn test_sqlite_time_entry_crud() {
    let (_dir, mut backend) = create_test_backend();

    let task = Task::new("Task for time entry");
    backend.create_task(&task).unwrap();

    let entry = TimeEntry::start(task.id);
    backend.create_time_entry(&entry).unwrap();

    let retrieved = backend.get_time_entry(&entry.id).unwrap();
    assert!(retrieved.is_some());
    assert!(retrieved.unwrap().is_running());

    backend.delete_time_entry(&entry.id).unwrap();
    assert!(backend.get_time_entry(&entry.id).unwrap().is_none());
}

#[test]
fn test_sqlite_get_tasks_by_project() {
    let (_dir, mut backend) = create_test_backend();

    let project = Project::new("Test project");
    backend.create_project(&project).unwrap();

    let task1 = Task::new("Task 1").with_project(project.id);
    let task2 = Task::new("Task 2").with_project(project.id);
    let task3 = Task::new("Task 3"); // No project

    backend.create_task(&task1).unwrap();
    backend.create_task(&task2).unwrap();
    backend.create_task(&task3).unwrap();

    let project_tasks = backend.get_tasks_by_project(&project.id).unwrap();
    assert_eq!(project_tasks.len(), 2);
}

#[test]
fn test_sqlite_get_active_entry() {
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
fn test_sqlite_export_import_roundtrip() {
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
    let path2 = dir2.path().join("import.db");
    let mut backend2 = SqliteBackend::new(&path2).unwrap();
    backend2.initialize().unwrap();
    backend2.import_all(&exported).unwrap();

    // Verify
    assert_eq!(backend2.list_tasks().unwrap().len(), 1);
    assert_eq!(backend2.list_projects().unwrap().len(), 1);
    assert_eq!(backend2.list_tags().unwrap().len(), 1);
}

#[test]
fn test_sqlite_backend_type() {
    let (_dir, backend) = create_test_backend();
    assert_eq!(backend.backend_type(), "sqlite");
}

#[test]
fn test_sqlite_update_task_not_found() {
    let (_dir, mut backend) = create_test_backend();

    let task = Task::new("Non-existent");
    let result = backend.update_task(&task);
    assert!(result.is_err());
}

#[test]
fn test_sqlite_delete_task_not_found() {
    let (_dir, mut backend) = create_test_backend();

    let task_id = TaskId::new();
    let result = backend.delete_task(&task_id);
    assert!(result.is_err());
}

#[test]
fn test_sqlite_subprojects() {
    let (_dir, mut backend) = create_test_backend();

    let parent = Project::new("Parent");
    backend.create_project(&parent).unwrap();

    let child1 = Project::new("Child 1").with_parent(parent.id);
    let child2 = Project::new("Child 2").with_parent(parent.id);

    backend.create_project(&child1).unwrap();
    backend.create_project(&child2).unwrap();

    let subprojects = backend.get_subprojects(&parent.id).unwrap();
    assert_eq!(subprojects.len(), 2);
}

#[test]
fn test_sqlite_get_tasks_by_tag_with_junction_table() {
    let (_dir, mut backend) = create_test_backend();

    // Create tasks with various tags
    let task1 = Task::new("Task 1").with_tags(vec!["work".to_string(), "urgent".to_string()]);
    let task2 = Task::new("Task 2").with_tags(vec!["work".to_string(), "low".to_string()]);
    let task3 = Task::new("Task 3").with_tags(vec!["personal".to_string()]);

    backend.create_task(&task1).unwrap();
    backend.create_task(&task2).unwrap();
    backend.create_task(&task3).unwrap();

    // Query by single tag
    let work_tasks = backend.get_tasks_by_tag("work").unwrap();
    assert_eq!(work_tasks.len(), 2);

    let personal_tasks = backend.get_tasks_by_tag("personal").unwrap();
    assert_eq!(personal_tasks.len(), 1);
    assert_eq!(personal_tasks[0].title, "Task 3");

    // Query non-existent tag
    let empty = backend.get_tasks_by_tag("nonexistent").unwrap();
    assert!(empty.is_empty());
}

#[test]
fn test_sqlite_tag_query_special_characters() {
    let (_dir, mut backend) = create_test_backend();

    // Create tasks with tags containing special characters
    // These would break the old LIKE-based pattern matching
    let task1 = Task::new("Task 1").with_tags(vec![
        "tag\"with\"quotes".to_string(),
        "tag,with,commas".to_string(),
    ]);
    let task2 =
        Task::new("Task 2").with_tags(vec!["tag[with]brackets".to_string(), "100%".to_string()]);
    let task3 = Task::new("Task 3").with_tags(vec![
        "tag'with'apostrophes".to_string(),
        "tag\\with\\backslash".to_string(),
    ]);

    backend.create_task(&task1).unwrap();
    backend.create_task(&task2).unwrap();
    backend.create_task(&task3).unwrap();

    // Query by tags with special characters - should work correctly with junction table
    let quotes = backend.get_tasks_by_tag("tag\"with\"quotes").unwrap();
    assert_eq!(quotes.len(), 1);
    assert_eq!(quotes[0].title, "Task 1");

    let commas = backend.get_tasks_by_tag("tag,with,commas").unwrap();
    assert_eq!(commas.len(), 1);

    let brackets = backend.get_tasks_by_tag("tag[with]brackets").unwrap();
    assert_eq!(brackets.len(), 1);

    let percent = backend.get_tasks_by_tag("100%").unwrap();
    assert_eq!(percent.len(), 1);

    let apostrophes = backend.get_tasks_by_tag("tag'with'apostrophes").unwrap();
    assert_eq!(apostrophes.len(), 1);
}

#[test]
fn test_sqlite_filtered_tags_any_mode() {
    let (_dir, mut backend) = create_test_backend();

    let task1 = Task::new("Task 1").with_tags(vec!["a".to_string(), "b".to_string()]);
    let task2 = Task::new("Task 2").with_tags(vec!["b".to_string(), "c".to_string()]);
    let task3 = Task::new("Task 3").with_tags(vec!["c".to_string(), "d".to_string()]);
    let task4 = Task::new("Task 4").with_tags(vec!["e".to_string()]);

    backend.create_task(&task1).unwrap();
    backend.create_task(&task2).unwrap();
    backend.create_task(&task3).unwrap();
    backend.create_task(&task4).unwrap();

    // Filter by tags in ANY mode (task has at least one of the tags)
    let filter = Filter {
        tags: Some(vec!["a".to_string(), "c".to_string()]),
        tags_mode: crate::domain::TagFilterMode::Any,
        include_completed: true,
        ..Default::default()
    };

    let filtered = backend.list_tasks_filtered(&filter).unwrap();
    // Should match task1 (has a), task2 (has c), task3 (has c)
    assert_eq!(filtered.len(), 3);
}

#[test]
fn test_sqlite_filtered_tags_all_mode() {
    let (_dir, mut backend) = create_test_backend();

    let task1 =
        Task::new("Task 1").with_tags(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    let task2 = Task::new("Task 2").with_tags(vec!["a".to_string(), "b".to_string()]);
    let task3 = Task::new("Task 3").with_tags(vec!["a".to_string()]);
    let task4 = Task::new("Task 4").with_tags(vec!["b".to_string(), "c".to_string()]);

    backend.create_task(&task1).unwrap();
    backend.create_task(&task2).unwrap();
    backend.create_task(&task3).unwrap();
    backend.create_task(&task4).unwrap();

    // Filter by tags in ALL mode (task has ALL of the tags)
    let filter = Filter {
        tags: Some(vec!["a".to_string(), "b".to_string()]),
        tags_mode: crate::domain::TagFilterMode::All,
        include_completed: true,
        ..Default::default()
    };

    let filtered = backend.list_tasks_filtered(&filter).unwrap();
    // Should match task1 (has a,b,c) and task2 (has a,b)
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_sqlite_tag_update_syncs_junction_table() {
    let (_dir, mut backend) = create_test_backend();

    // Create task with initial tags
    let mut task = Task::new("Test").with_tags(vec!["initial".to_string()]);
    backend.create_task(&task).unwrap();

    // Verify initial tag query works
    let initial_tasks = backend.get_tasks_by_tag("initial").unwrap();
    assert_eq!(initial_tasks.len(), 1);

    // Update task with new tags
    task.tags = vec!["updated".to_string(), "new".to_string()];
    backend.update_task(&task).unwrap();

    // Old tag should no longer match
    let old_tag_tasks = backend.get_tasks_by_tag("initial").unwrap();
    assert!(old_tag_tasks.is_empty());

    // New tags should match
    let updated_tasks = backend.get_tasks_by_tag("updated").unwrap();
    assert_eq!(updated_tasks.len(), 1);

    let new_tasks = backend.get_tasks_by_tag("new").unwrap();
    assert_eq!(new_tasks.len(), 1);
}

#[test]
fn test_sqlite_task_tags_table_created() {
    let (_dir, backend) = create_test_backend();
    let conn = backend.inner.conn().unwrap();

    // Check task_tags table exists
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    assert!(tables.contains(&"task_tags".to_string()));
}

#[test]
fn test_sqlite_tag_migration_from_json() {
    // This test simulates opening an existing database with JSON tags
    // and verifying migration populates the junction table
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");

    // First, create database without migration (simulate old data)
    {
        let conn = Connection::open(&path).unwrap();
        // Create old schema without task_tags table
        conn.execute_batch(
            r"
            CREATE TABLE tasks (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL DEFAULT 'todo',
                priority TEXT NOT NULL DEFAULT 'none',
                project_id TEXT,
                parent_task_id TEXT,
                tags TEXT NOT NULL DEFAULT '[]',
                dependencies TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                due_date TEXT,
                scheduled_date TEXT,
                completed_at TEXT,
                recurrence TEXT,
                estimated_minutes INTEGER,
                actual_minutes INTEGER NOT NULL DEFAULT 0,
                sort_order INTEGER,
                next_task_id TEXT,
                custom_fields TEXT NOT NULL DEFAULT '{}'
            );
            ",
        )
        .unwrap();

        // Insert task with JSON tags
        conn.execute(
            "INSERT INTO tasks (id, title, tags, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params!["test-id", "Test Task", r#"["tag1","tag2"]"#, "2024-01-01T00:00:00Z", "2024-01-01T00:00:00Z"],
        )
        .unwrap();
    }

    // Now open with our backend (should trigger migration)
    let mut backend = SqliteBackend::new(&path).unwrap();
    backend.initialize().unwrap();

    // Verify migration created the junction table and populated it
    let tag1_tasks = backend.get_tasks_by_tag("tag1").unwrap();
    assert_eq!(tag1_tasks.len(), 1);
    assert_eq!(tag1_tasks[0].title, "Test Task");

    let tag2_tasks = backend.get_tasks_by_tag("tag2").unwrap();
    assert_eq!(tag2_tasks.len(), 1);
}
