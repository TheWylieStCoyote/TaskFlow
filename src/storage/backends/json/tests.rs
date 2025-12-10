//! Tests for JSON backend.

use tempfile::tempdir;

use crate::domain::{
    Priority, Project, ProjectId, Tag, TagFilterMode, Task, TaskId, TaskStatus, TimeEntry,
    TimeEntryId,
};
use crate::storage::{
    ExportData, ProjectRepository, StorageBackend, TagRepository, TaskRepository,
    TimeEntryRepository,
};

use super::JsonBackend;

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
    duplicate.id = task.id;

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

    let filter = crate::domain::Filter {
        status: Some(vec![TaskStatus::Todo]),
        include_completed: true,
        ..Default::default()
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

    let filter = crate::domain::Filter {
        priority: Some(vec![Priority::High]),
        ..Default::default()
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

    let filter = crate::domain::Filter {
        tags: Some(vec!["rust".to_string()]),
        tags_mode: TagFilterMode::Any,
        ..Default::default()
    };

    let tasks = backend.list_tasks_filtered(&filter).unwrap();
    assert_eq!(tasks.len(), 2);
}

#[test]
fn test_list_tasks_filtered_by_tags_all() {
    let (_dir, mut backend) = create_test_backend();

    let task1 = Task::new("Task with rust").with_tags(vec!["rust".to_string()]);
    let task2 =
        Task::new("Task with both").with_tags(vec!["rust".to_string(), "important".to_string()]);
    backend.create_task(&task1).unwrap();
    backend.create_task(&task2).unwrap();

    let filter = crate::domain::Filter {
        tags: Some(vec!["rust".to_string(), "important".to_string()]),
        tags_mode: TagFilterMode::All,
        ..Default::default()
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

    let task1 = Task::new("In project").with_project(project.id);
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
    let entry = TimeEntry::start(task.id);
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
    let entry = TimeEntry::start(task.id);
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

    let mut entry1 = TimeEntry::start(task1.id);
    entry1.stop();
    let mut entry2 = TimeEntry::start(task1.id);
    entry2.stop();
    let mut entry3 = TimeEntry::start(task2.id);
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
        let session = PomodoroSession::new(task.id, &config, 4);
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
    let session = PomodoroSession::new(task.id, &config, 4);

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
        task_ids.push(task.id);
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

// ==================== Habit Repository Tests ====================

#[test]
fn test_habit_crud() {
    use crate::domain::Habit;
    use crate::storage::HabitRepository;

    let (_dir, mut backend) = create_test_backend();

    // Create
    let habit = Habit::new("Exercise daily");
    backend.create_habit(&habit).unwrap();

    // Read
    let retrieved = backend.get_habit(&habit.id).unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "Exercise daily");

    // Update
    let mut updated = habit.clone();
    updated.name = "Exercise 30 min".to_string();
    backend.update_habit(&updated).unwrap();

    let retrieved = backend.get_habit(&habit.id).unwrap().unwrap();
    assert_eq!(retrieved.name, "Exercise 30 min");

    // Delete
    backend.delete_habit(&habit.id).unwrap();
    let retrieved = backend.get_habit(&habit.id).unwrap();
    assert!(retrieved.is_none());
}

#[test]
fn test_habit_list() {
    use crate::domain::Habit;
    use crate::storage::HabitRepository;

    let (_dir, mut backend) = create_test_backend();

    let habit1 = Habit::new("Exercise");
    let habit2 = Habit::new("Meditate");
    backend.create_habit(&habit1).unwrap();
    backend.create_habit(&habit2).unwrap();

    let habits = backend.list_habits().unwrap();
    assert_eq!(habits.len(), 2);
}

#[test]
fn test_habit_list_active() {
    use crate::domain::Habit;
    use crate::storage::HabitRepository;

    let (_dir, mut backend) = create_test_backend();

    let habit1 = Habit::new("Active habit");
    let mut habit2 = Habit::new("Archived habit");
    habit2.archived = true;

    backend.create_habit(&habit1).unwrap();
    backend.create_habit(&habit2).unwrap();

    let active = backend.list_active_habits().unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].name, "Active habit");
}

#[test]
fn test_habit_duplicate_id_fails() {
    use crate::domain::Habit;
    use crate::storage::HabitRepository;

    let (_dir, mut backend) = create_test_backend();

    let habit = Habit::new("Original");
    backend.create_habit(&habit).unwrap();

    let mut duplicate = Habit::new("Duplicate");
    duplicate.id = habit.id;

    let result = backend.create_habit(&duplicate);
    assert!(result.is_err());
}

#[test]
fn test_habit_update_not_found() {
    use crate::domain::Habit;
    use crate::storage::HabitRepository;

    let (_dir, mut backend) = create_test_backend();

    let habit = Habit::new("Non-existent");
    let result = backend.update_habit(&habit);
    assert!(result.is_err());
}

#[test]
fn test_habit_delete_not_found() {
    use crate::domain::HabitId;
    use crate::storage::HabitRepository;

    let (_dir, mut backend) = create_test_backend();

    let result = backend.delete_habit(&HabitId::new());
    assert!(result.is_err());
}

// ==================== Work Log Repository Tests ====================

#[test]
fn test_work_log_crud() {
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
    assert_eq!(retrieved.unwrap().content, "Did some work");

    // Update
    let mut updated = entry.clone();
    updated.content = "Updated work description".to_string();
    backend.update_work_log(&updated).unwrap();

    let retrieved = backend.get_work_log(&entry.id).unwrap().unwrap();
    assert_eq!(retrieved.content, "Updated work description");

    // Delete
    backend.delete_work_log(&entry.id).unwrap();
    let retrieved = backend.get_work_log(&entry.id).unwrap();
    assert!(retrieved.is_none());
}

#[test]
fn test_work_log_list() {
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
fn test_work_log_get_for_task() {
    use crate::domain::WorkLogEntry;
    use crate::storage::WorkLogRepository;

    let (_dir, mut backend) = create_test_backend();

    let task1 = Task::new("Task 1");
    let task2 = Task::new("Task 2");
    backend.create_task(&task1).unwrap();
    backend.create_task(&task2).unwrap();

    let entry1 = WorkLogEntry::new(task1.id, "Work for task 1");
    let entry2 = WorkLogEntry::new(task1.id, "More work for task 1");
    let entry3 = WorkLogEntry::new(task2.id, "Work for task 2");
    backend.create_work_log(&entry1).unwrap();
    backend.create_work_log(&entry2).unwrap();
    backend.create_work_log(&entry3).unwrap();

    let logs = backend.get_work_logs_for_task(&task1.id).unwrap();
    assert_eq!(logs.len(), 2);
}

#[test]
fn test_work_log_duplicate_id_fails() {
    use crate::domain::WorkLogEntry;
    use crate::storage::WorkLogRepository;

    let (_dir, mut backend) = create_test_backend();

    let task = Task::new("Test task");
    backend.create_task(&task).unwrap();

    let entry = WorkLogEntry::new(task.id, "Original");
    backend.create_work_log(&entry).unwrap();

    let mut duplicate = WorkLogEntry::new(task.id, "Duplicate");
    duplicate.id = entry.id;

    let result = backend.create_work_log(&duplicate);
    assert!(result.is_err());
}

#[test]
fn test_work_log_update_not_found() {
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
fn test_work_log_delete_not_found() {
    use crate::domain::WorkLogEntryId;
    use crate::storage::WorkLogRepository;

    let (_dir, mut backend) = create_test_backend();

    let result = backend.delete_work_log(&WorkLogEntryId::new());
    assert!(result.is_err());
}
