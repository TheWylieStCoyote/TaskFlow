//! Tests for SQLite backend.

use rusqlite::{params, Connection};
use tempfile::tempdir;

use crate::domain::{
    Filter, Habit, HabitId, Priority, Project, Tag, Task, TaskId, TaskStatus, TimeEntry,
    WorkLogEntry,
};
use crate::storage::{
    HabitRepository, ProjectRepository, StorageBackend, TagRepository, TaskRepository,
    TimeEntryRepository, WorkLogRepository,
};

use super::SqliteBackend;
use crate::domain::today;

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
            params!["11111111-2222-4333-8444-555555555555", "Test Task", r#"["tag1","tag2"]"#, "2024-01-01T00:00:00Z", "2024-01-01T00:00:00Z"],
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

// ==================== Habit Repository Tests ====================

#[test]
fn test_sqlite_habit_crud() {
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
fn test_sqlite_habit_list() {
    let (_dir, mut backend) = create_test_backend();

    let habit1 = Habit::new("Exercise");
    let habit2 = Habit::new("Read");
    backend.create_habit(&habit1).unwrap();
    backend.create_habit(&habit2).unwrap();

    let habits = backend.list_habits().unwrap();
    assert_eq!(habits.len(), 2);
}

#[test]
fn test_sqlite_habit_list_active() {
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
fn test_sqlite_habit_with_check_ins() {
    let (_dir, mut backend) = create_test_backend();

    let mut habit = Habit::new("Exercise");
    habit.check_in_today(true, Some("30 minutes".to_string()));

    backend.create_habit(&habit).unwrap();

    // Retrieve and verify check-in was stored
    let retrieved = backend.get_habit(&habit.id).unwrap().unwrap();
    assert_eq!(retrieved.check_ins.len(), 1);

    let today = today();
    let check_in = retrieved.check_ins.get(&today).unwrap();
    assert!(check_in.completed);
    assert_eq!(check_in.note, Some("30 minutes".to_string()));
}

#[test]
fn test_sqlite_habit_update_check_ins() {
    let (_dir, mut backend) = create_test_backend();

    let mut habit = Habit::new("Exercise");
    habit.check_in_today(true, None);
    backend.create_habit(&habit).unwrap();

    // Add more check-ins
    habit.check_in_today(false, Some("Skipped".to_string()));
    backend.update_habit(&habit).unwrap();

    // Verify update
    let retrieved = backend.get_habit(&habit.id).unwrap().unwrap();
    let today = today();
    let check_in = retrieved.check_ins.get(&today).unwrap();
    assert!(!check_in.completed);
}

#[test]
fn test_sqlite_habit_update_not_found() {
    let (_dir, mut backend) = create_test_backend();

    let habit = Habit::new("Non-existent");
    let result = backend.update_habit(&habit);
    assert!(result.is_err());
}

#[test]
fn test_sqlite_habit_delete_not_found() {
    let (_dir, mut backend) = create_test_backend();

    let result = backend.delete_habit(&HabitId::new());
    assert!(result.is_err());
}

#[test]
fn test_sqlite_habit_with_tags_and_color() {
    let (_dir, mut backend) = create_test_backend();

    let habit = Habit::new("Exercise")
        .with_color("#ff0000")
        .with_tags(vec!["health".into(), "fitness".into()]);

    backend.create_habit(&habit).unwrap();

    let retrieved = backend.get_habit(&habit.id).unwrap().unwrap();
    assert_eq!(retrieved.color, Some("#ff0000".to_string()));
    assert!(retrieved.tags.contains(&"health".to_string()));
    assert!(retrieved.tags.contains(&"fitness".to_string()));
}

// ==================== Work Log Repository Tests ====================

#[test]
fn test_sqlite_work_log_crud() {
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
fn test_sqlite_work_log_list() {
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
fn test_sqlite_work_log_get_for_task() {
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
fn test_sqlite_work_log_update_not_found() {
    let (_dir, mut backend) = create_test_backend();

    let task = Task::new("Test task");
    backend.create_task(&task).unwrap();

    let entry = WorkLogEntry::new(task.id, "Non-existent");
    let result = backend.update_work_log(&entry);
    assert!(result.is_err());
}

#[test]
fn test_sqlite_work_log_delete_not_found() {
    use crate::domain::WorkLogEntryId;

    let (_dir, mut backend) = create_test_backend();

    let result = backend.delete_work_log(&WorkLogEntryId::new());
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Durability regressions: columns that used to be dropped, pragmas, transactions
// ---------------------------------------------------------------------------

#[test]
fn test_sqlite_persists_snooze_and_recurrence_options() {
    use crate::domain::{Recurrence, RecurrenceOptions};
    use chrono::NaiveDate;

    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");

    let snooze = NaiveDate::from_ymd_opt(2030, 1, 15).unwrap();
    let end = NaiveDate::from_ymd_opt(2031, 6, 30).unwrap();
    let task = Task::new("Snoozed").with_recurrence_options(RecurrenceOptions {
        interval: 3,
        end_date: Some(end),
        max_occurrences: Some(7),
        occurrence_count: 2,
    });
    let mut task = task;
    task.recurrence = Some(Recurrence::Daily);
    task.snooze_until = Some(snooze);
    let id = task.id;

    {
        let mut backend = SqliteBackend::new(&path).unwrap();
        backend.initialize().unwrap();
        backend.create_task(&task).unwrap();
    }

    // Reopen from disk so nothing is served from memory.
    let mut backend = SqliteBackend::new(&path).unwrap();
    backend.initialize().unwrap();
    let loaded = backend.get_task(&id).unwrap().expect("task should exist");
    assert_eq!(loaded.snooze_until, Some(snooze));
    assert_eq!(loaded.recurrence_options.interval, 3);
    assert_eq!(loaded.recurrence_options.end_date, Some(end));
    assert_eq!(loaded.recurrence_options.max_occurrences, Some(7));
    assert_eq!(loaded.recurrence_options.occurrence_count, 2);

    // And updates round-trip too.
    let mut updated = loaded.clone();
    updated.snooze_until = None;
    updated.recurrence_options.interval = 1;
    backend.update_task(&updated).unwrap();
    let reloaded = backend.get_task(&id).unwrap().unwrap();
    assert_eq!(reloaded.snooze_until, None);
    assert_eq!(reloaded.recurrence_options.interval, 1);
}

#[test]
fn test_sqlite_migrates_old_schema_without_snooze_columns() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("old.db");

    // Simulate a database created before the columns existed.
    {
        let mut backend = SqliteBackend::new(&path).unwrap();
        backend.initialize().unwrap();
        let conn = backend.inner.conn().unwrap();
        conn.execute_batch(
            "ALTER TABLE tasks DROP COLUMN snooze_until; ALTER TABLE tasks DROP COLUMN recurrence_options;",
        )
        .unwrap();
        assert!(conn.prepare("SELECT snooze_until FROM tasks").is_err());
    }

    let mut backend = SqliteBackend::new(&path).unwrap();
    backend.initialize().unwrap();
    let conn = backend.inner.conn().unwrap();
    assert!(conn
        .prepare("SELECT snooze_until, recurrence_options FROM tasks")
        .is_ok());

    // A task with defaults still loads fine when the columns hold NULL.
    conn.execute(
        "INSERT INTO tasks (id, title, created_at, updated_at) VALUES (?1, 'legacy', ?2, ?2)",
        params![TaskId::new().0.to_string(), chrono::Utc::now().to_rfc3339()],
    )
    .unwrap();
    let tasks = backend.list_tasks().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].snooze_until, None);
    assert_eq!(
        tasks[0].recurrence_options,
        crate::domain::RecurrenceOptions::default()
    );
}

#[test]
fn test_sqlite_foreign_keys_cascade_on_task_delete() {
    let (_dir, mut backend) = create_test_backend();

    let mut task = Task::new("Parent");
    task.tags = vec!["alpha".to_string(), "beta".to_string()];
    let id = task.id;
    backend.create_task(&task).unwrap();
    backend
        .create_work_log(&WorkLogEntry::new(id, "did a thing"))
        .unwrap();

    let conn = backend.inner.conn().unwrap();
    let fk: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
        .unwrap();
    assert_eq!(fk, 1, "foreign_keys pragma must be enabled");
    let tag_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM task_tags", [], |r| r.get(0))
        .unwrap();
    assert_eq!(tag_rows, 2);

    backend.delete_task(&id).unwrap();

    let conn = backend.inner.conn().unwrap();
    let tag_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM task_tags", [], |r| r.get(0))
        .unwrap();
    let log_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM work_logs", [], |r| r.get(0))
        .unwrap();
    assert_eq!(tag_rows, 0, "task_tags should cascade");
    assert_eq!(log_rows, 0, "work_logs should cascade");
}

#[test]
fn test_sqlite_import_all_rolls_back_on_failure() {
    use crate::storage::ExportData;

    let (_dir, mut backend) = create_test_backend();
    backend.create_task(&Task::new("keep me")).unwrap();

    // A work log pointing at a task that is not part of the import violates the
    // foreign key and must abort the whole import.
    let mut data = ExportData::default();
    let orphan = WorkLogEntry::new(TaskId::new(), "orphan");
    data.work_logs.push(orphan);

    let result = backend.import_all(&data);
    assert!(result.is_err(), "import with orphan work log should fail");

    // The pre-existing data survived because the transaction rolled back.
    let tasks = backend.list_tasks().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "keep me");
}

// ============================================================================
// Goals
// ============================================================================

mod goal_fixtures {
    use chrono::{NaiveDate, TimeZone, Utc};

    use crate::domain::{
        Goal, GoalId, GoalStatus, KeyResult, KeyResultStatus, ProjectId, Quarter, TaskId,
    };

    pub(super) fn fixed_ts(day: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2025, 3, day, 9, 30, 15).unwrap()
    }

    /// A goal with every optional field populated.
    pub(super) fn full_goal() -> Goal {
        let mut goal = Goal::new("Ship v2")
            .with_description("Everything for the second release")
            .with_status(GoalStatus::OnHold)
            .with_dates(
                NaiveDate::from_ymd_opt(2025, 7, 1),
                NaiveDate::from_ymd_opt(2025, 9, 30),
            )
            .with_quarter(2025, Quarter::Q3)
            .with_manual_progress(42)
            .with_color("#ff8800")
            .with_icon("🚀");
        goal.created_at = fixed_ts(1);
        goal.updated_at = fixed_ts(2);
        goal
    }

    /// A key result with every optional field populated.
    pub(super) fn full_key_result(goal_id: GoalId) -> KeyResult {
        let mut kr = KeyResult::new(goal_id, "Reach 1000 users")
            .with_description("Signed-up accounts")
            .with_status(KeyResultStatus::AtRisk)
            .with_target(1000.0, Some("users"))
            .with_current_value(437.5)
            .with_manual_progress(55);
        kr.linked_project_ids = vec![ProjectId::new(), ProjectId::new()];
        kr.linked_task_ids = vec![TaskId::new()];
        kr.created_at = fixed_ts(3);
        kr.updated_at = fixed_ts(4);
        kr
    }
}

use chrono::NaiveDate;

use crate::domain::{
    Goal, GoalId, GoalStatus, KeyResult, KeyResultId, KeyResultStatus, PomodoroConfig,
    PomodoroSession, PomodoroStats, Quarter, SavedFilter, SortField, SortOrder, SortSpec,
};
use crate::storage::{ExportData, GoalRepository, KeyResultRepository, StorageError};
use goal_fixtures::{fixed_ts, full_goal, full_key_result};

fn raw_column<T: rusqlite::types::FromSql>(backend: &SqliteBackend, sql: &str, id: &str) -> T {
    backend
        .inner
        .conn()
        .unwrap()
        .query_row(sql, params![id], |r| r.get(0))
        .unwrap()
}

#[test]
fn test_sqlite_goal_roundtrip_all_fields() {
    let (_dir, mut backend) = create_test_backend();
    let goal = full_goal();

    backend.create_goal(&goal).unwrap();
    let retrieved = backend.get_goal(&goal.id).unwrap().unwrap();

    assert_eq!(retrieved.id, goal.id);
    assert_eq!(retrieved.name, "Ship v2");
    assert_eq!(
        retrieved.description.as_deref(),
        Some("Everything for the second release")
    );
    assert_eq!(retrieved.status, GoalStatus::OnHold);
    assert_eq!(retrieved.start_date, NaiveDate::from_ymd_opt(2025, 7, 1));
    assert_eq!(retrieved.due_date, NaiveDate::from_ymd_opt(2025, 9, 30));
    assert_eq!(retrieved.quarter, Some((2025, Quarter::Q3)));
    assert_eq!(retrieved.manual_progress, Some(42));
    assert_eq!(retrieved.color.as_deref(), Some("#ff8800"));
    assert_eq!(retrieved.icon.as_deref(), Some("🚀"));
    assert_eq!(retrieved.created_at, fixed_ts(1));
    assert_eq!(retrieved.updated_at, fixed_ts(2));
    assert_eq!(retrieved, goal);
}

#[test]
fn test_sqlite_goal_roundtrip_minimal_fields() {
    let (_dir, mut backend) = create_test_backend();
    let goal = Goal::new("Bare goal");

    backend.create_goal(&goal).unwrap();
    let retrieved = backend.get_goal(&goal.id).unwrap().unwrap();

    assert_eq!(retrieved.name, "Bare goal");
    assert_eq!(retrieved.status, GoalStatus::Active);
    assert_eq!(retrieved.description, None);
    assert_eq!(retrieved.start_date, None);
    assert_eq!(retrieved.due_date, None);
    assert_eq!(retrieved.quarter, None);
    assert_eq!(retrieved.manual_progress, None);
    assert_eq!(retrieved.color, None);
    assert_eq!(retrieved.icon, None);
}

#[test]
fn test_sqlite_get_goal_missing_returns_none() {
    let (_dir, backend) = create_test_backend();
    assert!(backend.get_goal(&GoalId::new()).unwrap().is_none());
}

#[test]
fn test_sqlite_create_goal_duplicate_id_fails() {
    let (_dir, mut backend) = create_test_backend();
    let goal = Goal::new("Once");
    backend.create_goal(&goal).unwrap();
    assert!(backend.create_goal(&goal).is_err());
    assert_eq!(backend.list_goals().unwrap().len(), 1);
}

#[test]
fn test_sqlite_update_goal_rewrites_every_column() {
    let (_dir, mut backend) = create_test_backend();
    let goal = full_goal();
    backend.create_goal(&goal).unwrap();

    let mut changed = goal.clone();
    changed.name = "Ship v3".to_string();
    changed.description = None;
    changed.status = GoalStatus::Completed;
    changed.start_date = None;
    changed.due_date = NaiveDate::from_ymd_opt(2026, 1, 15);
    changed.quarter = Some((2026, Quarter::Q1));
    changed.manual_progress = Some(100);
    changed.color = Some("#00ff00".to_string());
    changed.icon = None;
    changed.updated_at = fixed_ts(9);
    backend.update_goal(&changed).unwrap();

    let retrieved = backend.get_goal(&goal.id).unwrap().unwrap();
    assert_eq!(retrieved, changed);
    // created_at is immutable through update
    assert_eq!(retrieved.created_at, fixed_ts(1));
}

#[test]
fn test_sqlite_update_goal_clears_quarter() {
    let (_dir, mut backend) = create_test_backend();
    let goal = full_goal();
    backend.create_goal(&goal).unwrap();

    let mut changed = goal.clone();
    changed.quarter = None;
    backend.update_goal(&changed).unwrap();

    let retrieved = backend.get_goal(&goal.id).unwrap().unwrap();
    assert_eq!(retrieved.quarter, None);
    let raw: Option<String> = raw_column(
        &backend,
        "SELECT quarter FROM goals WHERE id = ?1",
        &goal.id.to_string(),
    );
    assert_eq!(raw, None);
}

#[test]
fn test_sqlite_update_goal_not_found() {
    let (_dir, mut backend) = create_test_backend();
    let goal = Goal::new("Ghost");
    let err = backend.update_goal(&goal).unwrap_err();
    assert!(
        matches!(&err, StorageError::NotFound { entity_type, id } if entity_type == "Goal" && *id == goal.id.to_string()),
        "unexpected error: {err:?}"
    );
}

#[test]
fn test_sqlite_delete_goal_not_found() {
    let (_dir, mut backend) = create_test_backend();
    let id = GoalId::new();
    let err = backend.delete_goal(&id).unwrap_err();
    assert!(
        matches!(&err, StorageError::NotFound { entity_type, .. } if entity_type == "Goal"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn test_sqlite_delete_goal_cascades_to_key_results() {
    let (_dir, mut backend) = create_test_backend();
    let keep = Goal::new("Keep");
    let drop = Goal::new("Drop");
    backend.create_goal(&keep).unwrap();
    backend.create_goal(&drop).unwrap();
    backend
        .create_key_result(&KeyResult::new(keep.id, "kr-keep"))
        .unwrap();
    backend
        .create_key_result(&KeyResult::new(drop.id, "kr-drop-1"))
        .unwrap();
    backend
        .create_key_result(&KeyResult::new(drop.id, "kr-drop-2"))
        .unwrap();

    backend.delete_goal(&drop.id).unwrap();

    assert!(backend.get_goal(&drop.id).unwrap().is_none());
    assert!(backend.get_goal(&keep.id).unwrap().is_some());
    let remaining = backend.list_key_results().unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].name, "kr-keep");
    assert!(backend
        .get_key_results_for_goal(&drop.id)
        .unwrap()
        .is_empty());
}

#[test]
fn test_sqlite_list_goals_sorted_by_name() {
    let (_dir, mut backend) = create_test_backend();
    for name in ["Charlie", "alpha", "Bravo"] {
        backend.create_goal(&Goal::new(name)).unwrap();
    }
    let names: Vec<String> = backend
        .list_goals()
        .unwrap()
        .into_iter()
        .map(|g| g.name)
        .collect();
    // SQLite's default BINARY collation sorts uppercase before lowercase.
    assert_eq!(names, vec!["Bravo", "Charlie", "alpha"]);
}

#[test]
fn test_sqlite_list_active_goals_filters_by_status() {
    let (_dir, mut backend) = create_test_backend();
    backend
        .create_goal(&Goal::new("Zeta active").with_status(GoalStatus::Active))
        .unwrap();
    backend
        .create_goal(&Goal::new("Alpha active").with_status(GoalStatus::Active))
        .unwrap();
    backend
        .create_goal(&Goal::new("On hold").with_status(GoalStatus::OnHold))
        .unwrap();
    backend
        .create_goal(&Goal::new("Completed").with_status(GoalStatus::Completed))
        .unwrap();
    backend
        .create_goal(&Goal::new("Archived").with_status(GoalStatus::Archived))
        .unwrap();

    let active: Vec<String> = backend
        .list_active_goals()
        .unwrap()
        .into_iter()
        .map(|g| g.name)
        .collect();
    assert_eq!(active, vec!["Alpha active", "Zeta active"]);
    assert_eq!(backend.list_goals().unwrap().len(), 5);
}

#[test]
fn test_sqlite_goal_status_stored_as_snake_case() {
    let (_dir, mut backend) = create_test_backend();
    let cases = [
        (GoalStatus::Active, "active"),
        (GoalStatus::OnHold, "on_hold"),
        (GoalStatus::Completed, "completed"),
        (GoalStatus::Archived, "archived"),
    ];
    for (status, expected) in cases {
        let goal = Goal::new(expected).with_status(status);
        backend.create_goal(&goal).unwrap();
        let raw: String = raw_column(
            &backend,
            "SELECT status FROM goals WHERE id = ?1",
            &goal.id.to_string(),
        );
        assert_eq!(raw, expected);
        assert_eq!(backend.get_goal(&goal.id).unwrap().unwrap().status, status);

        // update_goal uses the same mapping
        let mut flipped = goal.clone();
        flipped.status = GoalStatus::Archived;
        backend.update_goal(&flipped).unwrap();
        let raw: String = raw_column(
            &backend,
            "SELECT status FROM goals WHERE id = ?1",
            &goal.id.to_string(),
        );
        assert_eq!(raw, "archived");
    }
}

#[test]
fn test_sqlite_goal_from_row_is_lossy_for_enums_but_strict_for_values() {
    let (_dir, backend) = create_test_backend();
    let conn = backend.inner.conn().unwrap();
    let ts = "2025-03-01T09:30:15Z";

    // A bogus status and an out-of-range progress have sensible defaults, so
    // the row still loads.
    conn.execute(
        r"INSERT INTO goals (id, name, status, manual_progress, created_at, updated_at)
          VALUES (?1, 'Lossy', 'bogus', 250, ?2, '2025-03-02T09:30:15Z')",
        params![GoalId::new().to_string(), ts],
    )
    .unwrap();
    conn.execute(
        r"INSERT INTO goals (id, name, status, manual_progress, created_at, updated_at)
          VALUES (?1, 'Negative', 'active', -5, ?2, ?2)",
        params![GoalId::new().to_string(), ts],
    )
    .unwrap();

    // Each of these rows has one column that cannot be parsed. None of them may
    // load with an invented value (nil UUID, "now", None); each is skipped.
    let insert = |id: &str,
                  name: &str,
                  start: Option<&str>,
                  due: Option<&str>,
                  quarter: Option<&str>,
                  created: &str| {
        conn.execute(
            r"INSERT INTO goals (id, name, status, start_date, due_date, quarter, created_at, updated_at)
              VALUES (?1, ?2, 'active', ?3, ?4, ?5, ?6, ?7)",
            params![id, name, start, due, quarter, created, ts],
        )
        .unwrap();
    };
    let bad_created = GoalId::new();
    insert("not-a-uuid", "bad id", None, None, None, ts);
    insert(
        &bad_created.to_string(),
        "bad created_at",
        None,
        None,
        None,
        "garbage",
    );
    insert(
        &GoalId::new().to_string(),
        "bad start_date",
        Some("yesterday"),
        None,
        None,
        ts,
    );
    insert(
        &GoalId::new().to_string(),
        "bad due_date",
        None,
        Some("2025-13-45"),
        None,
        ts,
    );
    insert(
        &GoalId::new().to_string(),
        "bad quarter",
        None,
        None,
        Some("{not json"),
        ts,
    );
    // Blank optionals are NULL-equivalent, not corrupt.
    insert(
        &GoalId::new().to_string(),
        "blank optionals",
        Some(""),
        Some(""),
        Some(""),
        ts,
    );

    let goals = backend.list_goals().unwrap();
    let names: Vec<&str> = goals.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(names, ["Lossy", "Negative", "blank optionals"]);

    let lossy = goals.iter().find(|g| g.name == "Lossy").unwrap();
    assert_eq!(lossy.status, GoalStatus::Active);
    assert_eq!(lossy.manual_progress, Some(100));
    assert_eq!(lossy.created_at, fixed_ts(1));
    assert_eq!(lossy.updated_at, fixed_ts(2));

    let negative = goals.iter().find(|g| g.name == "Negative").unwrap();
    assert_eq!(negative.manual_progress, Some(0));

    let blank = goals.iter().find(|g| g.name == "blank optionals").unwrap();
    assert_eq!(blank.start_date, None);
    assert_eq!(blank.due_date, None);
    assert_eq!(blank.quarter, None);

    // Fetching the corrupt row directly is an error naming the column, not a
    // goal created "now".
    let err = backend.get_goal(&bad_created).unwrap_err();
    assert!(matches!(err, StorageError::Database(_)), "{err:?}");
    assert!(err.to_string().contains("column `created_at`"), "{err}");
    assert!(err.to_string().contains("garbage"), "{err}");
}

// ============================================================================
// Key results
// ============================================================================

#[test]
fn test_sqlite_key_result_roundtrip_all_fields() {
    let (_dir, mut backend) = create_test_backend();
    let goal = Goal::new("Parent");
    backend.create_goal(&goal).unwrap();
    let kr = full_key_result(goal.id);

    backend.create_key_result(&kr).unwrap();
    let retrieved = backend.get_key_result(&kr.id).unwrap().unwrap();

    assert_eq!(retrieved.id, kr.id);
    assert_eq!(retrieved.goal_id, goal.id);
    assert_eq!(retrieved.name, "Reach 1000 users");
    assert_eq!(retrieved.description.as_deref(), Some("Signed-up accounts"));
    assert_eq!(retrieved.status, KeyResultStatus::AtRisk);
    assert!((retrieved.target_value - 1000.0).abs() < f64::EPSILON);
    assert!((retrieved.current_value - 437.5).abs() < f64::EPSILON);
    assert_eq!(retrieved.unit.as_deref(), Some("users"));
    assert_eq!(retrieved.manual_progress, Some(55));
    assert_eq!(retrieved.linked_project_ids, kr.linked_project_ids);
    assert_eq!(retrieved.linked_task_ids, kr.linked_task_ids);
    assert_eq!(retrieved.created_at, fixed_ts(3));
    assert_eq!(retrieved.updated_at, fixed_ts(4));
    assert_eq!(retrieved, kr);

    // Linked ids are stored as JSON arrays of UUID strings.
    let raw_projects: String = raw_column(
        &backend,
        "SELECT linked_project_ids FROM key_results WHERE id = ?1",
        &kr.id.to_string(),
    );
    let parsed: Vec<String> = serde_json::from_str(&raw_projects).unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0], kr.linked_project_ids[0].0.to_string());
}

#[test]
fn test_sqlite_key_result_roundtrip_minimal_fields() {
    let (_dir, mut backend) = create_test_backend();
    let goal = Goal::new("Parent");
    backend.create_goal(&goal).unwrap();
    let kr = KeyResult::new(goal.id, "Bare");

    backend.create_key_result(&kr).unwrap();
    let retrieved = backend.get_key_result(&kr.id).unwrap().unwrap();

    assert_eq!(retrieved.status, KeyResultStatus::NotStarted);
    assert_eq!(retrieved.description, None);
    assert_eq!(retrieved.unit, None);
    assert_eq!(retrieved.manual_progress, None);
    assert!(retrieved.linked_project_ids.is_empty());
    assert!(retrieved.linked_task_ids.is_empty());
    assert!(retrieved.target_value.abs() < f64::EPSILON);
    assert!(retrieved.current_value.abs() < f64::EPSILON);
}

#[test]
fn test_sqlite_key_result_requires_existing_goal() {
    let (_dir, mut backend) = create_test_backend();
    let kr = KeyResult::new(GoalId::new(), "Orphan");
    assert!(
        backend.create_key_result(&kr).is_err(),
        "foreign key to goals must be enforced"
    );
    assert!(backend.list_key_results().unwrap().is_empty());
}

#[test]
fn test_sqlite_get_key_result_missing_returns_none() {
    let (_dir, backend) = create_test_backend();
    assert!(backend
        .get_key_result(&KeyResultId::new())
        .unwrap()
        .is_none());
}

#[test]
fn test_sqlite_update_key_result_rewrites_every_column() {
    let (_dir, mut backend) = create_test_backend();
    let goal_a = Goal::new("A");
    let goal_b = Goal::new("B");
    backend.create_goal(&goal_a).unwrap();
    backend.create_goal(&goal_b).unwrap();
    let kr = full_key_result(goal_a.id);
    backend.create_key_result(&kr).unwrap();

    let mut changed = kr.clone();
    changed.goal_id = goal_b.id;
    changed.name = "Renamed".to_string();
    changed.description = None;
    changed.status = KeyResultStatus::Completed;
    changed.target_value = 10.0;
    changed.current_value = 10.0;
    changed.unit = None;
    changed.manual_progress = None;
    changed.linked_project_ids.clear();
    changed.linked_task_ids.push(TaskId::new());
    changed.updated_at = fixed_ts(20);
    backend.update_key_result(&changed).unwrap();

    let retrieved = backend.get_key_result(&kr.id).unwrap().unwrap();
    assert_eq!(retrieved, changed);
    assert_eq!(retrieved.created_at, fixed_ts(3));
    assert_eq!(
        backend.get_key_results_for_goal(&goal_a.id).unwrap().len(),
        0
    );
    assert_eq!(
        backend.get_key_results_for_goal(&goal_b.id).unwrap().len(),
        1
    );
}

#[test]
fn test_sqlite_update_key_result_not_found() {
    let (_dir, mut backend) = create_test_backend();
    let goal = Goal::new("Parent");
    backend.create_goal(&goal).unwrap();
    let kr = KeyResult::new(goal.id, "Ghost");
    let err = backend.update_key_result(&kr).unwrap_err();
    assert!(
        matches!(&err, StorageError::NotFound { entity_type, id } if entity_type == "KeyResult" && *id == kr.id.to_string()),
        "unexpected error: {err:?}"
    );
}

#[test]
fn test_sqlite_delete_key_result() {
    let (_dir, mut backend) = create_test_backend();
    let goal = Goal::new("Parent");
    backend.create_goal(&goal).unwrap();
    let kr1 = KeyResult::new(goal.id, "one");
    let kr2 = KeyResult::new(goal.id, "two");
    backend.create_key_result(&kr1).unwrap();
    backend.create_key_result(&kr2).unwrap();

    backend.delete_key_result(&kr1.id).unwrap();

    assert!(backend.get_key_result(&kr1.id).unwrap().is_none());
    assert!(backend.get_key_result(&kr2.id).unwrap().is_some());
    // The parent goal is untouched.
    assert!(backend.get_goal(&goal.id).unwrap().is_some());

    let err = backend.delete_key_result(&kr1.id).unwrap_err();
    assert!(
        matches!(&err, StorageError::NotFound { entity_type, .. } if entity_type == "KeyResult"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn test_sqlite_get_key_results_for_goal_sorted_by_name() {
    let (_dir, mut backend) = create_test_backend();
    let goal_a = Goal::new("A");
    let goal_b = Goal::new("B");
    backend.create_goal(&goal_a).unwrap();
    backend.create_goal(&goal_b).unwrap();
    backend
        .create_key_result(&KeyResult::new(goal_a.id, "beta"))
        .unwrap();
    backend
        .create_key_result(&KeyResult::new(goal_a.id, "alpha"))
        .unwrap();
    backend
        .create_key_result(&KeyResult::new(goal_b.id, "gamma"))
        .unwrap();

    let for_a: Vec<String> = backend
        .get_key_results_for_goal(&goal_a.id)
        .unwrap()
        .into_iter()
        .map(|k| k.name)
        .collect();
    assert_eq!(for_a, vec!["alpha", "beta"]);

    let for_b = backend.get_key_results_for_goal(&goal_b.id).unwrap();
    assert_eq!(for_b.len(), 1);
    assert_eq!(for_b[0].goal_id, goal_b.id);

    assert!(backend
        .get_key_results_for_goal(&GoalId::new())
        .unwrap()
        .is_empty());
    assert_eq!(backend.list_key_results().unwrap().len(), 3);
}

#[test]
fn test_sqlite_key_result_status_stored_as_snake_case() {
    let (_dir, mut backend) = create_test_backend();
    let goal = Goal::new("Parent");
    backend.create_goal(&goal).unwrap();
    let cases = [
        (KeyResultStatus::NotStarted, "not_started"),
        (KeyResultStatus::InProgress, "in_progress"),
        (KeyResultStatus::AtRisk, "at_risk"),
        (KeyResultStatus::Completed, "completed"),
    ];
    for (status, expected) in cases {
        let kr = KeyResult::new(goal.id, expected).with_status(status);
        backend.create_key_result(&kr).unwrap();
        let raw: String = raw_column(
            &backend,
            "SELECT status FROM key_results WHERE id = ?1",
            &kr.id.to_string(),
        );
        assert_eq!(raw, expected);
        assert_eq!(
            backend.get_key_result(&kr.id).unwrap().unwrap().status,
            status
        );

        let mut flipped = kr.clone();
        flipped.status = KeyResultStatus::InProgress;
        backend.update_key_result(&flipped).unwrap();
        let raw: String = raw_column(
            &backend,
            "SELECT status FROM key_results WHERE id = ?1",
            &kr.id.to_string(),
        );
        assert_eq!(raw, "in_progress");
    }
}

#[test]
fn test_sqlite_key_result_from_row_is_lossy_for_enums_but_strict_for_values() {
    let (_dir, mut backend) = create_test_backend();
    let goal = Goal::new("Parent");
    backend.create_goal(&goal).unwrap();
    let conn = backend.inner.conn().unwrap();
    let ts = "2025-03-01T09:30:15Z";
    let goal_id = goal.id.to_string();

    let insert = |id: &str, name: &str, projects: &str, tasks: &str, updated: &str| {
        conn.execute(
            r"INSERT INTO key_results (id, goal_id, name, status, target_value, current_value,
                manual_progress, linked_project_ids, linked_task_ids, created_at, updated_at)
              VALUES (?1, ?2, ?3, 'unknown_status', 5.0, 2.5, 999, ?4, ?5, ?6, ?7)",
            params![id, goal_id, name, projects, tasks, ts, updated],
        )
        .unwrap();
    };
    // Unknown status and out-of-range progress are tolerated; everything else is valid.
    insert(&KeyResultId::new().to_string(), "Lossy", "[]", "[]", ts);
    // One unparsable column each: skipped rather than loaded with a nil UUID,
    // an empty list or an updated_at of "now".
    insert("nope", "bad id", "[]", "[]", ts);
    insert(
        &KeyResultId::new().to_string(),
        "bad project json",
        "not json at all",
        "[]",
        ts,
    );
    insert(
        &KeyResultId::new().to_string(),
        "bad task uuid",
        "[]",
        r#"["also-not-a-uuid"]"#,
        ts,
    );
    insert(
        &KeyResultId::new().to_string(),
        "bad updated_at",
        "[]",
        "[]",
        "bad",
    );
    conn.execute(
        r"INSERT INTO key_results (id, goal_id, name, manual_progress, created_at, updated_at)
          VALUES (?1, ?2, 'Negative', -1, ?3, ?3)",
        params![KeyResultId::new().to_string(), goal_id, ts],
    )
    .unwrap();

    let krs = backend.get_key_results_for_goal(&goal.id).unwrap();
    let names: Vec<&str> = krs.iter().map(|k| k.name.as_str()).collect();
    assert_eq!(names, ["Lossy", "Negative"]);
    assert_eq!(backend.list_key_results().unwrap().len(), 2);

    let lossy = krs.iter().find(|k| k.name == "Lossy").unwrap();
    assert_eq!(lossy.goal_id, goal.id);
    assert_eq!(lossy.status, KeyResultStatus::NotStarted);
    assert!((lossy.target_value - 5.0).abs() < f64::EPSILON);
    assert!((lossy.current_value - 2.5).abs() < f64::EPSILON);
    assert_eq!(lossy.manual_progress, Some(100));
    assert!(lossy.linked_project_ids.is_empty());
    assert!(lossy.linked_task_ids.is_empty());
    assert_eq!(lossy.created_at, fixed_ts(1));
    assert_eq!(lossy.updated_at, fixed_ts(1));

    let negative = krs.iter().find(|k| k.name == "Negative").unwrap();
    assert_eq!(negative.manual_progress, Some(0));
    assert_eq!(negative.status, KeyResultStatus::NotStarted);
}

#[test]
fn test_sqlite_export_import_includes_goals_and_key_results() {
    let (_dir, mut backend) = create_test_backend();
    let goal = full_goal();
    let kr = full_key_result(goal.id);
    backend.create_goal(&goal).unwrap();
    backend.create_key_result(&kr).unwrap();

    let exported = backend.export_all().unwrap();
    assert_eq!(exported.goals, vec![goal.clone()]);
    assert_eq!(exported.key_results, vec![kr.clone()]);

    let dir2 = tempdir().unwrap();
    let mut backend2 = SqliteBackend::new(&dir2.path().join("import.db")).unwrap();
    backend2.initialize().unwrap();
    backend2.import_all(&exported).unwrap();
    assert_eq!(backend2.get_goal(&goal.id).unwrap(), Some(goal));
    assert_eq!(backend2.get_key_result(&kr.id).unwrap(), Some(kr));
}

// ============================================================================
// Saved filters and Pomodoro state
// ============================================================================

#[test]
fn test_sqlite_saved_filters_roundtrip_through_import_export() {
    let (_dir, mut backend) = create_test_backend();
    assert!(backend.export_all().unwrap().saved_filters.is_empty());

    let filter = Filter {
        status: Some(vec![TaskStatus::Todo, TaskStatus::InProgress]),
        tags: Some(vec!["work".to_string()]),
        search_text: Some("login".to_string()),
        include_completed: true,
        ..Filter::default()
    };
    let with_icon = SavedFilter::new(
        "Work in flight",
        filter,
        SortSpec {
            field: SortField::Title,
            order: SortOrder::Ascending,
        },
    )
    .with_icon("💼");
    let without_icon = SavedFilter::new("Everything", Filter::default(), SortSpec::default());

    let data = ExportData {
        saved_filters: vec![with_icon.clone(), without_icon.clone()],
        ..ExportData::default()
    };
    backend.import_all(&data).unwrap();

    let mut saved = backend.export_all().unwrap().saved_filters;
    assert_eq!(saved.len(), 2);
    saved.sort_by(|a, b| a.name.cmp(&b.name));

    let everything = &saved[0];
    assert_eq!(everything.id, without_icon.id);
    assert_eq!(everything.name, "Everything");
    assert_eq!(everything.icon, None);
    assert!(everything.filter.status.is_none());
    assert!(matches!(everything.sort.field, SortField::CreatedAt));
    assert!(matches!(everything.sort.order, SortOrder::Descending));

    let work = &saved[1];
    assert_eq!(work.id, with_icon.id);
    assert_eq!(work.name, "Work in flight");
    assert_eq!(work.icon.as_deref(), Some("💼"));
    assert_eq!(
        work.filter.status,
        Some(vec![TaskStatus::Todo, TaskStatus::InProgress])
    );
    assert_eq!(work.filter.tags, Some(vec!["work".to_string()]));
    assert_eq!(work.filter.search_text.as_deref(), Some("login"));
    assert!(work.filter.include_completed);
    assert!(matches!(work.sort.field, SortField::Title));
    assert!(matches!(work.sort.order, SortOrder::Ascending));

    // Re-importing replaces rather than appends.
    backend.import_all(&data).unwrap();
    assert_eq!(backend.export_all().unwrap().saved_filters.len(), 2);
}

#[test]
fn test_sqlite_list_saved_filters_rejects_corrupt_json() {
    let (_dir, backend) = create_test_backend();
    let conn = backend.inner.conn().unwrap();
    conn.execute(
        "INSERT INTO saved_filters (id, name, filter_json, sort_json, icon) VALUES ('x', 'Broken', '{', '{}', NULL)",
        [],
    )
    .unwrap();

    let err = backend.export_all().unwrap_err();
    assert!(
        matches!(err, StorageError::Serialization { .. }),
        "unexpected error: {err:?}"
    );
}

#[test]
fn test_sqlite_pomodoro_state_roundtrip() {
    let (_dir, mut backend) = create_test_backend();

    let exported = backend.export_all().unwrap();
    assert!(exported.pomodoro_session.is_none());
    assert!(exported.pomodoro_config.is_none());
    assert!(exported.pomodoro_stats.is_none());

    let config = PomodoroConfig {
        work_duration_mins: 50,
        short_break_mins: 10,
        long_break_mins: 30,
        cycles_before_long_break: 2,
    };
    let task_id = TaskId::new();
    let session = PomodoroSession::new(task_id, &config, 3);
    let mut stats = PomodoroStats::new();
    stats.record_cycle(50);
    stats.record_cycle(50);

    backend.set_pomodoro_config(&config).unwrap();
    backend.set_pomodoro_session(Some(&session)).unwrap();
    backend.set_pomodoro_stats(&stats).unwrap();

    let exported = backend.export_all().unwrap();
    assert_eq!(exported.pomodoro_config, Some(config.clone()));
    let loaded_session = exported.pomodoro_session.unwrap();
    assert_eq!(loaded_session.task_id, task_id);
    assert_eq!(loaded_session.session_goal, 3);
    assert_eq!(loaded_session.remaining_secs, session.remaining_secs);
    let loaded_stats = exported.pomodoro_stats.unwrap();
    assert_eq!(loaded_stats.total_cycles, 2);
    assert_eq!(loaded_stats.total_work_mins, 100);

    // Overwriting replaces the single row per key.
    let config2 = PomodoroConfig::default();
    backend.set_pomodoro_config(&config2).unwrap();
    assert_eq!(backend.export_all().unwrap().pomodoro_config, Some(config2));

    // Clearing the session deletes its row and leaves the others alone.
    backend.set_pomodoro_session(None).unwrap();
    let exported = backend.export_all().unwrap();
    assert!(exported.pomodoro_session.is_none());
    assert!(exported.pomodoro_config.is_some());
    assert!(exported.pomodoro_stats.is_some());
    let rows: i64 = backend
        .inner
        .conn()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM pomodoro_state", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 2);
}

// ============================================================================
// Unparsable rows
// ============================================================================

mod unparsable_rows {
    use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

    use tracing_subscriber::fmt::MakeWriter;

    use super::super::rows::parse_rfc3339;
    use super::*;
    use crate::domain::TimeEntryId;

    const TS: &str = "2025-03-01T09:30:15Z";

    /// Every test here reaches the `warn!` in `collect_rows`. `tracing`
    /// caches a call site's interest the first time any thread hits it, using
    /// that thread's subscriber; a sibling test with no subscriber installed
    /// would cache "never" and silence the capturing test below. Running the
    /// module's tests one at a time removes the race.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn serial() -> MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Collects everything a `tracing` fmt subscriber writes.
    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<u8>>>);

    impl Capture {
        fn text(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    impl std::io::Write for Capture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for Capture {
        type Writer = Capture;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    #[test]
    fn parse_rfc3339_rejects_garbage_instead_of_inventing_now() {
        assert!(parse_rfc3339("garbage").is_err());
        assert!(parse_rfc3339("").is_err());
        assert!(parse_rfc3339("2025-03-01").is_err());
        assert_eq!(parse_rfc3339(TS).unwrap(), fixed_ts(1));
        assert_eq!(
            parse_rfc3339("2025-03-01T11:30:15+02:00").unwrap(),
            fixed_ts(1)
        );
    }

    #[test]
    fn list_tasks_skips_unparsable_rows_and_keeps_good_ones() {
        let _serial = serial();
        let (_dir, mut backend) = create_test_backend();
        let good = Task::new("good");
        backend.create_task(&good).unwrap();

        let conn = backend.inner.conn().unwrap();
        let insert = |id: &str,
                      title: &str,
                      tags: &str,
                      deps: &str,
                      created: &str,
                      done: Option<&str>| {
            conn.execute(
                    r"INSERT INTO tasks (id, title, tags, dependencies, created_at, updated_at, completed_at)
                      VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6)",
                    params![id, title, tags, deps, created, done],
                )
                .unwrap();
        };
        let bad_created = TaskId::new();
        insert(
            &bad_created.to_string(),
            "bad created_at",
            "[]",
            "[]",
            "garbage",
            None,
        );
        insert("not-a-uuid", "bad id", "[]", "[]", TS, None);
        insert(
            &TaskId::new().to_string(),
            "bad tags json",
            "{oops",
            "[]",
            TS,
            None,
        );
        insert(
            &TaskId::new().to_string(),
            "bad dependency uuid",
            "[]",
            r#"["nope"]"#,
            TS,
            None,
        );
        insert(
            &TaskId::new().to_string(),
            "bad completed_at",
            "[]",
            "[]",
            TS,
            Some("yesterday"),
        );
        insert(
            &TaskId::new().to_string(),
            "blank completed_at",
            "[]",
            "[]",
            TS,
            Some(""),
        );

        let tasks = backend.list_tasks().unwrap();
        let mut titles: Vec<&str> = tasks.iter().map(|t| t.title.as_str()).collect();
        titles.sort_unstable();
        assert_eq!(titles, ["blank completed_at", "good"]);

        let blank = tasks
            .iter()
            .find(|t| t.title == "blank completed_at")
            .unwrap();
        assert_eq!(blank.completed_at, None);
        assert_eq!(blank.created_at, fixed_ts(1));
        assert_eq!(blank.updated_at, fixed_ts(1));

        // Filtered listings go through the same path.
        assert_eq!(
            backend
                .list_tasks_filtered(&Filter::default())
                .unwrap()
                .len(),
            2
        );

        // A single-row lookup of the corrupt row is an error naming the column,
        // not a task created "now".
        let err = backend.get_task(&bad_created).unwrap_err();
        assert!(matches!(err, StorageError::Database(_)), "{err:?}");
        assert!(err.to_string().contains("column `created_at`"), "{err}");
        assert!(err.to_string().contains("garbage"), "{err}");
        assert!(backend.get_task(&good.id).unwrap().is_some());
    }

    #[test]
    fn time_entries_skip_unparsable_rows() {
        let _serial = serial();
        let (_dir, mut backend) = create_test_backend();
        let task = Task::new("tracked");
        backend.create_task(&task).unwrap();

        let conn = backend.inner.conn().unwrap();
        let insert = |id: &str, task_id: &str, started: &str, ended: Option<&str>| {
            conn.execute(
                "INSERT INTO time_entries (id, task_id, started_at, ended_at) VALUES (?1, ?2, ?3, ?4)",
                params![id, task_id, started, ended],
            )
            .unwrap();
        };
        let task_id = task.id.to_string();
        let finished = TimeEntryId::new();
        insert(
            &finished.to_string(),
            &task_id,
            TS,
            Some("2025-03-01T10:00:00Z"),
        );
        insert(&TimeEntryId::new().to_string(), &task_id, "noon", None);
        insert(&TimeEntryId::new().to_string(), &task_id, TS, Some("later"));
        insert(&TimeEntryId::new().to_string(), "not-a-task", TS, None);

        let entries = backend.get_entries_for_task(&task.id).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, finished);
        assert_eq!(entries[0].started_at, fixed_ts(1));
        assert_eq!(backend.list_all_time_entries().unwrap().len(), 1);

        // Only unparsable rows are still running, and a single-row lookup does
        // not turn one of them into a timer started "now".
        let err = backend.get_active_entry().unwrap_err();
        assert!(err.to_string().contains("column `"), "{err}");
    }

    #[test]
    fn habit_check_ins_skip_unparsable_rows() {
        let _serial = serial();
        let (_dir, mut backend) = create_test_backend();
        let habit = Habit::new("Stretch");
        backend.create_habit(&habit).unwrap();

        let conn = backend.inner.conn().unwrap();
        let insert = |date: &str, checked_at: &str| {
            conn.execute(
                "INSERT INTO habit_check_ins (habit_id, date, completed, checked_at) VALUES (?1, ?2, 1, ?3)",
                params![habit.id.to_string(), date, checked_at],
            )
            .unwrap();
        };
        insert("2025-03-01", TS);
        insert("someday", TS);
        insert("2025-03-02", "garbage");

        let loaded = backend.get_habit(&habit.id).unwrap().unwrap();
        assert_eq!(loaded.check_ins.len(), 1);
        let only = loaded
            .check_ins
            .get(&NaiveDate::from_ymd_opt(2025, 3, 1).unwrap())
            .unwrap();
        assert!(only.completed);
        assert_eq!(only.checked_at, fixed_ts(1));
        assert_eq!(backend.list_habits().unwrap()[0].check_ins.len(), 1);
    }

    #[test]
    fn skipped_row_is_reported_with_table_id_and_cause() {
        let _serial = serial();
        let (_dir, backend) = create_test_backend();
        let id = GoalId::new().to_string();
        backend
            .inner
            .conn()
            .unwrap()
            .execute(
                r"INSERT INTO goals (id, name, status, created_at, updated_at)
                  VALUES (?1, 'Broken', 'active', 'garbage', ?2)",
                params![id, TS],
            )
            .unwrap();

        let capture = Capture::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(capture.clone())
            .with_ansi(false)
            .finish();
        let goals = tracing::subscriber::with_default(subscriber, || {
            // Sibling tests hit the same `warn!` call site with no subscriber
            // installed, which can leave its cached interest at "never" if the
            // call site registers while this dispatcher is being added.
            // Recompute so the event reaches our capturing subscriber.
            tracing::callsite::rebuild_interest_cache();
            backend.list_goals().unwrap()
        });
        assert!(goals.is_empty());

        let log = capture.text();
        assert!(log.contains("WARN"), "{log}");
        assert!(log.contains("Skipping unparsable SQLite row"), "{log}");
        assert!(log.contains("table=\"goals\""), "{log}");
        assert!(log.contains(&format!("row_id={id} ")), "{log}");
        assert!(log.contains("column `created_at`"), "{log}");
        assert!(log.contains("\"garbage\""), "{log}");
        assert_eq!(log.matches("Skipping unparsable").count(), 1, "{log}");
    }
}
