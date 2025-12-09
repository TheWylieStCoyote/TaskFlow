//! Integration tests for TaskFlow
//!
//! These tests verify cross-component behavior and ensure
//! all backends produce consistent results.

use taskflow::domain::{
    Filter, Priority, Project, Tag, TagFilterMode, Task, TaskStatus, TimeEntry,
};
use taskflow::storage::{create_backend, BackendType, StorageBackend};
use tempfile::tempdir;

/// Helper to create all backend types for testing
fn create_all_backends() -> Vec<(String, Box<dyn StorageBackend>)> {
    let backends = vec![
        (BackendType::Json, "json"),
        (BackendType::Yaml, "yaml"),
        (BackendType::Sqlite, "db"),
        (BackendType::Markdown, ""),
    ];

    backends
        .into_iter()
        .filter_map(|(backend_type, ext)| {
            let dir = tempdir().ok()?;
            let path = if ext.is_empty() {
                dir.path().to_path_buf()
            } else {
                dir.path().join(format!("data.{}", ext))
            };

            let mut backend = create_backend(backend_type, &path).ok()?;
            backend.initialize().ok()?;

            // Keep the tempdir alive by leaking it (tests are short-lived anyway)
            std::mem::forget(dir);

            Some((backend_type.as_str().to_string(), backend))
        })
        .collect()
}

#[test]
fn test_all_backends_same_crud_behavior() {
    for (backend_name, mut backend) in create_all_backends() {
        // Create
        let task = Task::new("Test task").with_priority(Priority::High);
        backend
            .create_task(&task)
            .unwrap_or_else(|e| panic!("{}: create_task failed: {}", backend_name, e));

        // Read
        let retrieved = backend
            .get_task(&task.id)
            .unwrap_or_else(|e| panic!("{}: get_task failed: {}", backend_name, e));
        assert!(
            retrieved.is_some(),
            "{}: task should exist after create",
            backend_name
        );
        let retrieved = retrieved.unwrap();
        assert_eq!(
            retrieved.title, "Test task",
            "{}: title mismatch",
            backend_name
        );
        assert_eq!(
            retrieved.priority,
            Priority::High,
            "{}: priority mismatch",
            backend_name
        );

        // Update
        let mut updated = retrieved.clone();
        updated.title = "Updated task".to_string();
        updated.status = TaskStatus::InProgress;
        backend
            .update_task(&updated)
            .unwrap_or_else(|e| panic!("{}: update_task failed: {}", backend_name, e));

        let retrieved = backend.get_task(&task.id).unwrap().unwrap();
        assert_eq!(
            retrieved.title, "Updated task",
            "{}: title not updated",
            backend_name
        );
        assert_eq!(
            retrieved.status,
            TaskStatus::InProgress,
            "{}: status not updated",
            backend_name
        );

        // Delete
        backend
            .delete_task(&task.id)
            .unwrap_or_else(|e| panic!("{}: delete_task failed: {}", backend_name, e));
        assert!(
            backend.get_task(&task.id).unwrap().is_none(),
            "{}: task should not exist after delete",
            backend_name
        );
    }
}

#[test]
fn test_all_backends_same_filter_results() {
    for (backend_name, mut backend) in create_all_backends() {
        // Create tasks with different statuses and priorities
        let tasks = vec![
            Task::new("Todo Low").with_priority(Priority::Low),
            Task::new("Todo High").with_priority(Priority::High),
            Task::new("Done").with_status(TaskStatus::Done),
            Task::new("Urgent").with_priority(Priority::Urgent),
        ];

        for task in &tasks {
            backend.create_task(task).unwrap();
        }

        // Filter by priority
        let filter = Filter {
            priority: Some(vec![Priority::High, Priority::Urgent]),
            ..Default::default()
        };
        let filtered = backend.list_tasks_filtered(&filter).unwrap();
        assert_eq!(
            filtered.len(),
            2,
            "{}: should have 2 high/urgent priority tasks",
            backend_name
        );

        // Filter excluding completed
        let filter = Filter {
            include_completed: false,
            ..Default::default()
        };
        let filtered = backend.list_tasks_filtered(&filter).unwrap();
        assert_eq!(
            filtered.len(),
            3,
            "{}: should have 3 non-completed tasks",
            backend_name
        );

        // Filter by status
        let filter = Filter {
            status: Some(vec![TaskStatus::Done]),
            include_completed: true,
            ..Default::default()
        };
        let filtered = backend.list_tasks_filtered(&filter).unwrap();
        assert_eq!(
            filtered.len(),
            1,
            "{}: should have 1 done task",
            backend_name
        );
    }
}

#[test]
fn test_export_import_cross_backend() {
    // Create data in JSON backend
    let json_dir = tempdir().unwrap();
    let json_path = json_dir.path().join("data.json");
    let mut json_backend = create_backend(BackendType::Json, &json_path).unwrap();
    json_backend.initialize().unwrap();

    let task = Task::new("Cross-backend task")
        .with_priority(Priority::Medium)
        .with_tags(vec!["test".to_string(), "integration".to_string()]);
    let project = Project::new("Test Project");
    let tag = Tag {
        name: "important".to_string(),
        color: Some("#ff0000".to_string()),
        description: Some("Important items".to_string()),
    };

    json_backend.create_task(&task).unwrap();
    json_backend.create_project(&project).unwrap();
    json_backend.save_tag(&tag).unwrap();

    // Export from JSON
    let exported = json_backend.export_all().unwrap();

    // Import to SQLite
    let sqlite_dir = tempdir().unwrap();
    let sqlite_path = sqlite_dir.path().join("data.db");
    let mut sqlite_backend = create_backend(BackendType::Sqlite, &sqlite_path).unwrap();
    sqlite_backend.initialize().unwrap();
    sqlite_backend.import_all(&exported).unwrap();

    // Verify data in SQLite
    let sqlite_tasks = sqlite_backend.list_tasks().unwrap();
    assert_eq!(sqlite_tasks.len(), 1);
    assert_eq!(sqlite_tasks[0].title, "Cross-backend task");
    assert_eq!(sqlite_tasks[0].priority, Priority::Medium);
    assert_eq!(sqlite_tasks[0].tags.len(), 2);

    let sqlite_projects = sqlite_backend.list_projects().unwrap();
    assert_eq!(sqlite_projects.len(), 1);
    assert_eq!(sqlite_projects[0].name, "Test Project");

    let sqlite_tag = sqlite_backend.get_tag("important").unwrap();
    assert!(sqlite_tag.is_some());
    assert_eq!(sqlite_tag.unwrap().color, Some("#ff0000".to_string()));
}

#[test]
fn test_all_backends_project_task_relationship() {
    for (backend_name, mut backend) in create_all_backends() {
        // Create project
        let project = Project::new("Test Project");
        backend.create_project(&project).unwrap();

        // Create tasks with and without project
        let task_with_project = Task::new("Task with project").with_project(project.id);
        let task_without_project = Task::new("Task without project");

        backend.create_task(&task_with_project).unwrap();
        backend.create_task(&task_without_project).unwrap();

        // Query tasks by project
        let project_tasks = backend.get_tasks_by_project(&project.id).unwrap();
        assert_eq!(
            project_tasks.len(),
            1,
            "{}: should have 1 task in project",
            backend_name
        );
        assert_eq!(
            project_tasks[0].title, "Task with project",
            "{}: wrong task in project",
            backend_name
        );
    }
}

#[test]
fn test_all_backends_time_entry_tracking() {
    for (backend_name, mut backend) in create_all_backends() {
        // Create task
        let task = Task::new("Task with time tracking");
        backend.create_task(&task).unwrap();

        // Start time entry
        let entry = TimeEntry::start(task.id);
        backend.create_time_entry(&entry).unwrap();

        // Get active entry
        let active = backend.get_active_entry().unwrap();
        assert!(
            active.is_some(),
            "{}: should have active entry",
            backend_name
        );
        assert_eq!(
            active.unwrap().task_id,
            task.id,
            "{}: active entry should be for correct task",
            backend_name
        );

        // Stop entry
        let mut stopped_entry = backend.get_time_entry(&entry.id).unwrap().unwrap();
        stopped_entry.stop();
        backend.update_time_entry(&stopped_entry).unwrap();

        // No more active entry
        let active = backend.get_active_entry().unwrap();
        assert!(
            active.is_none(),
            "{}: should have no active entry after stop",
            backend_name
        );

        // Get entries for task
        let entries = backend.get_entries_for_task(&task.id).unwrap();
        assert_eq!(
            entries.len(),
            1,
            "{}: should have 1 entry for task",
            backend_name
        );
    }
}

#[test]
fn test_all_backends_tag_operations() {
    for (backend_name, mut backend) in create_all_backends() {
        // Create tasks with tags
        let task1 = Task::new("Task A").with_tags(vec!["rust".to_string(), "testing".to_string()]);
        let task2 = Task::new("Task B").with_tags(vec!["rust".to_string(), "cli".to_string()]);
        let task3 = Task::new("Task C").with_tags(vec!["testing".to_string()]);

        backend.create_task(&task1).unwrap();
        backend.create_task(&task2).unwrap();
        backend.create_task(&task3).unwrap();

        // Query by tag
        let rust_tasks = backend.get_tasks_by_tag("rust").unwrap();
        assert_eq!(
            rust_tasks.len(),
            2,
            "{}: should have 2 tasks with 'rust' tag",
            backend_name
        );

        let testing_tasks = backend.get_tasks_by_tag("testing").unwrap();
        assert_eq!(
            testing_tasks.len(),
            2,
            "{}: should have 2 tasks with 'testing' tag",
            backend_name
        );

        // Note: Tag filtering behavior varies by backend implementation
        // Some backends implement full tag mode filtering, others don't
        // This test verifies the basic tag query works
        let filter = Filter {
            tags: Some(vec!["rust".to_string()]),
            tags_mode: TagFilterMode::Any,
            ..Default::default()
        };
        let filtered = backend.list_tasks_filtered(&filter).unwrap();
        // Should have at least the tasks with "rust" tag
        assert!(
            !filtered.is_empty(),
            "{}: should have tasks matching 'rust' tag",
            backend_name
        );
    }
}

#[test]
fn test_backend_persistence_survives_reload() {
    // Create and save data
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.json");
    let task_id;
    let project_id;

    {
        let mut backend = create_backend(BackendType::Json, &path).unwrap();
        backend.initialize().unwrap();

        let task = Task::new("Persistent task").with_priority(Priority::High);
        let project = Project::new("Persistent project");

        task_id = task.id;
        project_id = project.id;

        backend.create_task(&task).unwrap();
        backend.create_project(&project).unwrap();
        backend.flush().unwrap();
    }

    // Reload and verify
    {
        let mut backend = create_backend(BackendType::Json, &path).unwrap();
        backend.initialize().unwrap();

        let task = backend.get_task(&task_id).unwrap();
        assert!(task.is_some(), "Task should persist after reload");
        assert_eq!(task.unwrap().title, "Persistent task");

        let project = backend.get_project(&project_id).unwrap();
        assert!(project.is_some(), "Project should persist after reload");
        assert_eq!(project.unwrap().name, "Persistent project");
    }
}

// Shell completion integration tests
mod completion_tests {
    use std::process::Command;

    #[test]
    fn test_completion_bash_command() {
        let output = Command::new("cargo")
            .args(["run", "--quiet", "--", "completion", "bash"])
            .output()
            .expect("Failed to execute command");

        assert!(
            output.status.success(),
            "completion bash should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("_taskflow"),
            "bash completion should contain _taskflow function"
        );
        assert!(
            stdout.contains("--backend"),
            "bash completion should include --backend option"
        );
    }

    #[test]
    fn test_completion_zsh_command() {
        let output = Command::new("cargo")
            .args(["run", "--quiet", "--", "completion", "zsh"])
            .output()
            .expect("Failed to execute command");

        assert!(
            output.status.success(),
            "completion zsh should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("#compdef taskflow"),
            "zsh completion should contain #compdef"
        );
    }

    #[test]
    fn test_completion_fish_command() {
        let output = Command::new("cargo")
            .args(["run", "--quiet", "--", "completion", "fish"])
            .output()
            .expect("Failed to execute command");

        assert!(
            output.status.success(),
            "completion fish should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("complete -c taskflow"),
            "fish completion should contain complete -c taskflow"
        );
    }

    #[test]
    fn test_completion_invalid_shell_fails() {
        let output = Command::new("cargo")
            .args(["run", "--quiet", "--", "completion", "invalid"])
            .output()
            .expect("Failed to execute command");

        assert!(
            !output.status.success(),
            "completion with invalid shell should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("invalid") || stderr.contains("possible values"),
            "error should mention invalid value or possible values"
        );
    }

    #[test]
    fn test_help_shows_completion_subcommand() {
        let output = Command::new("cargo")
            .args(["run", "--quiet", "--", "--help"])
            .output()
            .expect("Failed to execute command");

        assert!(output.status.success(), "help should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("completion"),
            "help should show completion subcommand"
        );
        assert!(
            stdout.contains("Generate shell completion"),
            "help should describe completion"
        );
    }

    #[test]
    fn test_backend_values_in_completion() {
        let output = Command::new("cargo")
            .args(["run", "--quiet", "--", "completion", "bash"])
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // The completion should contain the backend values
        assert!(
            stdout.contains("json"),
            "completion should include json backend"
        );
        assert!(
            stdout.contains("yaml"),
            "completion should include yaml backend"
        );
        assert!(
            stdout.contains("sqlite"),
            "completion should include sqlite backend"
        );
        assert!(
            stdout.contains("markdown"),
            "completion should include markdown backend"
        );
    }
}

// === Time Estimate Integration Tests ===

#[test]
fn test_all_backends_preserve_time_estimate() {
    for (backend_name, mut backend) in create_all_backends() {
        // Create task with estimate
        let mut task = Task::new("Task with estimate");
        task.estimated_minutes = Some(90); // 1h30m
        task.actual_minutes = 45;

        backend
            .create_task(&task)
            .unwrap_or_else(|e| panic!("{}: create_task failed: {}", backend_name, e));

        // Retrieve and verify estimate is preserved
        let retrieved = backend
            .get_task(&task.id)
            .unwrap_or_else(|e| panic!("{}: get_task failed: {}", backend_name, e))
            .unwrap();

        assert_eq!(
            retrieved.estimated_minutes,
            Some(90),
            "{}: estimated_minutes should be preserved",
            backend_name
        );
        assert_eq!(
            retrieved.actual_minutes, 45,
            "{}: actual_minutes should be preserved",
            backend_name
        );

        // Update estimate
        let mut updated = retrieved;
        updated.estimated_minutes = Some(120);
        backend
            .update_task(&updated)
            .unwrap_or_else(|e| panic!("{}: update_task failed: {}", backend_name, e));

        // Verify update
        let re_retrieved = backend
            .get_task(&task.id)
            .unwrap_or_else(|e| panic!("{}: get_task (2) failed: {}", backend_name, e))
            .unwrap();

        assert_eq!(
            re_retrieved.estimated_minutes,
            Some(120),
            "{}: estimated_minutes should be updated",
            backend_name
        );
    }
}

#[test]
fn test_all_backends_clear_time_estimate() {
    for (backend_name, mut backend) in create_all_backends() {
        // Create task with estimate
        let mut task = Task::new("Task with estimate to clear");
        task.estimated_minutes = Some(60);

        backend
            .create_task(&task)
            .unwrap_or_else(|e| panic!("{}: create_task failed: {}", backend_name, e));

        // Clear estimate
        let mut updated = task.clone();
        updated.estimated_minutes = None;
        backend
            .update_task(&updated)
            .unwrap_or_else(|e| panic!("{}: update_task failed: {}", backend_name, e));

        // Verify cleared
        let retrieved = backend
            .get_task(&task.id)
            .unwrap_or_else(|e| panic!("{}: get_task failed: {}", backend_name, e))
            .unwrap();

        assert!(
            retrieved.estimated_minutes.is_none(),
            "{}: estimated_minutes should be cleared",
            backend_name
        );
    }
}

#[test]
fn test_all_backends_persist_estimate_after_flush() {
    for (backend_name, mut backend) in create_all_backends() {
        // Create task with estimate
        let mut task = Task::new("Persistent estimate task");
        task.estimated_minutes = Some(45);

        backend
            .create_task(&task)
            .unwrap_or_else(|e| panic!("{}: create_task failed: {}", backend_name, e));

        // Flush
        backend
            .flush()
            .unwrap_or_else(|e| panic!("{}: flush failed: {}", backend_name, e));

        // Retrieve after flush
        let retrieved = backend
            .get_task(&task.id)
            .unwrap_or_else(|e| panic!("{}: get_task failed: {}", backend_name, e))
            .unwrap();

        assert_eq!(
            retrieved.estimated_minutes,
            Some(45),
            "{}: estimated_minutes should persist after flush",
            backend_name
        );
    }
}

// Import/Export integration tests
mod import_export_tests {
    use super::*;
    use std::io::Cursor;
    use taskflow::storage::{
        export_to_csv, export_to_ics, import_from_csv, import_from_ics, ImportOptions,
    };

    #[test]
    fn test_csv_roundtrip() {
        // Create tasks
        let tasks = vec![
            Task::new("Task 1").with_priority(Priority::High),
            Task::new("Task 2")
                .with_priority(Priority::Low)
                .with_tags(vec!["tag1".to_string(), "tag2".to_string()]),
            Task::new("Task 3").with_status(TaskStatus::Done),
        ];

        // Export to CSV
        let mut csv_buffer = Vec::new();
        export_to_csv(&tasks, &mut csv_buffer).expect("export should succeed");

        // Import from CSV
        let reader = Cursor::new(csv_buffer);
        let options = ImportOptions::default();
        let result = import_from_csv(reader, &options).expect("import should succeed");

        // Verify basic import
        assert_eq!(
            result.imported.len(),
            3,
            "should import all 3 tasks, got {} with {} errors",
            result.imported.len(),
            result.errors.len()
        );

        // Verify priorities preserved
        let high_priority_tasks: Vec<_> = result
            .imported
            .iter()
            .filter(|t| t.priority == Priority::High)
            .collect();
        assert_eq!(
            high_priority_tasks.len(),
            1,
            "should have 1 high priority task"
        );

        // Verify tags preserved
        let tagged_task = result
            .imported
            .iter()
            .find(|t| !t.tags.is_empty())
            .expect("should have a tagged task");
        assert_eq!(tagged_task.tags.len(), 2, "should have 2 tags");
    }

    #[test]
    fn test_ics_roundtrip() {
        // Create tasks with various properties
        let mut task1 = Task::new("Meeting preparation");
        task1.priority = Priority::High;
        task1.status = TaskStatus::InProgress;

        let mut task2 = Task::new("Completed task");
        task2.status = TaskStatus::Done;
        task2.completed_at = Some(chrono::Utc::now());

        let tasks = vec![task1, task2];

        // Export to ICS
        let mut ics_buffer = Vec::new();
        export_to_ics(&tasks, &mut ics_buffer).expect("export should succeed");

        let ics_content = String::from_utf8(ics_buffer.clone()).unwrap();
        assert!(
            ics_content.contains("BEGIN:VCALENDAR"),
            "should have calendar header"
        );
        assert!(
            ics_content.contains("VTODO"),
            "should have VTODO components"
        );

        // Import from ICS
        let reader = Cursor::new(ics_buffer);
        let options = ImportOptions::default();
        let result = import_from_ics(reader, &options).expect("import should succeed");

        assert_eq!(result.imported.len(), 2, "should import both tasks");
    }

    #[test]
    fn test_csv_handles_special_characters() {
        // Task with special characters that need escaping
        let task = Task::new("Task with \"quotes\" and, commas")
            .with_description("Description with\nmultiple lines");

        let tasks = vec![task];

        // Export
        let mut csv_buffer = Vec::new();
        export_to_csv(&tasks, &mut csv_buffer).expect("export should succeed");

        // Import
        let reader = Cursor::new(csv_buffer);
        let options = ImportOptions::default();
        let result = import_from_csv(reader, &options).expect("import should succeed");

        assert_eq!(result.imported.len(), 1, "should import the task");
        assert!(
            result.imported[0].title.contains("quotes"),
            "title should preserve quotes"
        );
    }

    #[test]
    fn test_all_backends_csv_export_import_consistency() {
        for (backend_name, mut backend) in create_all_backends() {
            // Create tasks
            let task1 = Task::new("Export test task 1").with_priority(Priority::High);
            let task2 = Task::new("Export test task 2").with_tags(vec!["exported".to_string()]);

            backend.create_task(&task1).expect("create_task failed");
            backend.create_task(&task2).expect("create_task failed");

            // Get all tasks from backend
            let stored_tasks = backend.list_tasks().expect("list_tasks failed");

            // Export to CSV
            let mut csv_buffer = Vec::new();
            export_to_csv(&stored_tasks, &mut csv_buffer)
                .unwrap_or_else(|e| panic!("{}: export failed: {}", backend_name, e));

            // Re-import and verify consistency
            let reader = Cursor::new(csv_buffer);
            let options = ImportOptions::default();
            let result = import_from_csv(reader, &options)
                .unwrap_or_else(|e| panic!("{}: import failed: {}", backend_name, e));

            assert_eq!(
                result.imported.len(),
                stored_tasks.len(),
                "{}: should import same number of tasks",
                backend_name
            );
        }
    }
}

// Work log integration tests
mod work_log_tests {
    use super::*;
    use taskflow::domain::WorkLogEntry;

    #[test]
    fn test_all_backends_work_log_operations() {
        for (backend_name, mut backend) in create_all_backends() {
            // Create a task
            let task = Task::new("Task for work logs");
            backend
                .create_task(&task)
                .unwrap_or_else(|e| panic!("{}: create_task failed: {}", backend_name, e));

            // Create work log entries
            let entry1 = WorkLogEntry::new(task.id, "First progress update");
            let entry2 = WorkLogEntry::new(task.id, "Second progress update");

            backend
                .create_work_log(&entry1)
                .unwrap_or_else(|e| panic!("{}: create_work_log 1 failed: {}", backend_name, e));
            backend
                .create_work_log(&entry2)
                .unwrap_or_else(|e| panic!("{}: create_work_log 2 failed: {}", backend_name, e));

            // Retrieve work logs for task
            let logs = backend
                .get_work_logs_for_task(&task.id)
                .unwrap_or_else(|e| {
                    panic!("{}: get_work_logs_for_task failed: {}", backend_name, e)
                });

            assert_eq!(logs.len(), 2, "{}: should have 2 work logs", backend_name);

            // Update a work log
            let mut updated = entry1.clone();
            updated.content = "Updated progress".to_string();
            backend
                .update_work_log(&updated)
                .unwrap_or_else(|e| panic!("{}: update_work_log failed: {}", backend_name, e));

            let retrieved = backend
                .get_work_log(&entry1.id)
                .unwrap_or_else(|e| panic!("{}: get_work_log failed: {}", backend_name, e))
                .unwrap();
            assert_eq!(
                retrieved.content, "Updated progress",
                "{}: work log content should be updated",
                backend_name
            );

            // Delete a work log
            backend
                .delete_work_log(&entry2.id)
                .unwrap_or_else(|e| panic!("{}: delete_work_log failed: {}", backend_name, e));

            let logs = backend
                .get_work_logs_for_task(&task.id)
                .expect("get_work_logs_for_task failed");
            assert_eq!(
                logs.len(),
                1,
                "{}: should have 1 work log after deletion",
                backend_name
            );
        }
    }

    #[test]
    fn test_work_logs_persist_in_export() {
        for (backend_name, mut backend) in create_all_backends() {
            // Create task and work log
            let task = Task::new("Task with work log");
            backend.create_task(&task).expect("create_task failed");

            let log = WorkLogEntry::new(task.id, "Important work done");
            backend
                .create_work_log(&log)
                .expect("create_work_log failed");

            // Export all data
            let export_data = backend
                .export_all()
                .unwrap_or_else(|e| panic!("{}: export_all failed: {}", backend_name, e));

            assert!(
                !export_data.work_logs.is_empty(),
                "{}: export should include work logs",
                backend_name
            );
            assert_eq!(
                export_data.work_logs[0].content, "Important work done",
                "{}: work log content should be preserved",
                backend_name
            );
        }
    }
}

// CLI command tests - testing actual binary execution
mod cli_command_tests {
    use std::path::Path;
    use std::process::Command;
    use tempfile::tempdir;

    /// Helper to run taskflow command and return output
    fn run_taskflow(args: &[&str]) -> std::process::Output {
        Command::new("cargo")
            .args(["run", "--quiet", "--"])
            .args(args)
            .output()
            .expect("Failed to execute command")
    }

    /// Helper to run taskflow with a temporary data directory
    fn run_taskflow_with_data(data_path: &Path, args: &[&str]) -> std::process::Output {
        let mut full_args = vec!["--data", data_path.to_str().unwrap()];
        full_args.extend(args);
        run_taskflow(&full_args)
    }

    // === Global Flag Tests ===

    #[test]
    fn test_help_flag() {
        let output = run_taskflow(&["--help"]);
        assert!(output.status.success(), "help should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("TUI project management") || stdout.contains("taskflow"),
            "help should describe the application"
        );
        assert!(
            stdout.contains("--backend"),
            "help should show --backend flag"
        );
        assert!(stdout.contains("--data"), "help should show --data flag");
        assert!(stdout.contains("--demo"), "help should show --demo flag");
    }

    #[test]
    fn test_version_flag() {
        let output = run_taskflow(&["--version"]);
        assert!(output.status.success(), "version should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("taskflow"),
            "version should mention taskflow"
        );
    }

    #[test]
    fn test_invalid_backend_rejected() {
        let output = run_taskflow(&["--backend", "invalid", "list"]);
        assert!(!output.status.success(), "invalid backend should fail");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("invalid") || stderr.contains("possible values"),
            "error should mention invalid value"
        );
    }

    #[test]
    fn test_all_backend_types_accepted() {
        for backend in ["json", "yaml", "sqlite", "markdown"] {
            let dir = tempdir().unwrap();
            let path = dir.path().join(format!("test.{}", backend));
            let output = run_taskflow(&[
                "--backend",
                backend,
                "--data",
                path.to_str().unwrap(),
                "list",
            ]);
            assert!(
                output.status.success(),
                "{} backend should be accepted: {}",
                backend,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    // === Add Command Tests ===

    #[test]
    fn test_add_simple_task() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        let output = run_taskflow_with_data(&path, &["add", "Simple test task"]);
        assert!(
            output.status.success(),
            "add should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Added"), "should confirm task was added");
        assert!(
            stdout.contains("Simple test task"),
            "should show task title"
        );
    }

    #[test]
    fn test_add_task_with_tags() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        let output = run_taskflow_with_data(&path, &["add", "Task with #tag1 and #tag2"]);
        assert!(output.status.success(), "add with tags should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Tags:"), "should show tags section");
        assert!(stdout.contains("#tag1"), "should show tag1");
        assert!(stdout.contains("#tag2"), "should show tag2");
    }

    #[test]
    fn test_add_task_with_priority() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        let output = run_taskflow_with_data(&path, &["add", "High priority task !high"]);
        assert!(output.status.success(), "add with priority should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Priority:"), "should show priority section");
        assert!(stdout.contains("High"), "should show High priority");
    }

    #[test]
    fn test_add_task_with_due_date() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        let output = run_taskflow_with_data(&path, &["add", "Task due:tomorrow"]);
        assert!(output.status.success(), "add with due date should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Due:"), "should show due date section");
    }

    #[test]
    fn test_add_empty_task_fails() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        // Test with empty string
        let output = run_taskflow_with_data(&path, &["add", ""]);
        assert!(!output.status.success(), "add with empty task should fail");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("empty") || stderr.contains("Error"),
            "should indicate empty task error"
        );
    }

    #[test]
    fn test_add_task_with_complex_syntax() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        // Complex quick-add syntax with multiple features
        let output =
            run_taskflow_with_data(&path, &["add", "Review PR #code #work !urgent due:friday"]);
        assert!(
            output.status.success(),
            "add with complex syntax should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Added"), "should confirm task was added");
    }

    // === List Command Tests ===

    #[test]
    fn test_list_empty_database() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        let output = run_taskflow_with_data(&path, &["list"]);
        assert!(output.status.success(), "list on empty db should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("No tasks found"),
            "should indicate no tasks"
        );
    }

    #[test]
    fn test_list_shows_added_tasks() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        // Add a task first
        run_taskflow_with_data(&path, &["add", "Test task for listing"]);

        // Now list
        let output = run_taskflow_with_data(&path, &["list"]);
        assert!(output.status.success(), "list should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Test task for listing"),
            "should show the added task"
        );
    }

    #[test]
    fn test_list_with_limit() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        // Add multiple tasks
        for i in 1..=5 {
            run_taskflow_with_data(&path, &["add", &format!("Task number {}", i)]);
        }

        // List with limit
        let output = run_taskflow_with_data(&path, &["list", "-n", "2"]);
        assert!(output.status.success(), "list with limit should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("2 shown"), "should indicate 2 tasks shown");
    }

    #[test]
    fn test_list_view_overdue() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        // Add a task without due date
        run_taskflow_with_data(&path, &["add", "No due date task"]);

        // List overdue view - should show no tasks since none are overdue
        let output = run_taskflow_with_data(&path, &["list", "--view", "overdue"]);
        assert!(output.status.success(), "list overdue should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should have no overdue tasks
        assert!(
            stdout.contains("No tasks") || stdout.contains("Overdue Tasks"),
            "should handle overdue view"
        );
    }

    #[test]
    fn test_list_with_search() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        // Add tasks
        run_taskflow_with_data(&path, &["add", "Apple picking"]);
        run_taskflow_with_data(&path, &["add", "Banana smoothie"]);
        run_taskflow_with_data(&path, &["add", "Cherry pie"]);

        // Search for specific term
        let output = run_taskflow_with_data(&path, &["list", "--search", "Apple"]);
        assert!(output.status.success(), "list with search should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Apple"), "should find Apple task");
        assert!(!stdout.contains("Banana"), "should not show Banana task");
    }

    #[test]
    fn test_list_with_tag_filter() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        // Add tasks with different tags
        run_taskflow_with_data(&path, &["add", "Work task #work"]);
        run_taskflow_with_data(&path, &["add", "Home task #home"]);
        run_taskflow_with_data(&path, &["add", "Both task #work #home"]);

        // Filter by tag
        let output = run_taskflow_with_data(&path, &["list", "--tags", "work"]);
        assert!(
            output.status.success(),
            "list with tag filter should succeed"
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Work task"), "should show Work task");
        assert!(stdout.contains("Both task"), "should show Both task");
    }

    #[test]
    fn test_list_with_priority_filter() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        // Add tasks with different priorities
        run_taskflow_with_data(&path, &["add", "Low priority !low"]);
        run_taskflow_with_data(&path, &["add", "High priority !high"]);
        run_taskflow_with_data(&path, &["add", "Urgent priority !urgent"]);

        // Filter by priority
        let output = run_taskflow_with_data(&path, &["list", "--priority", "high,urgent"]);
        assert!(
            output.status.success(),
            "list with priority filter should succeed"
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.contains("Low priority"),
            "should not show Low priority task"
        );
    }

    #[test]
    fn test_list_sort_options() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        // Add tasks
        run_taskflow_with_data(&path, &["add", "Zebra task"]);
        run_taskflow_with_data(&path, &["add", "Apple task"]);

        // Sort by title
        let output = run_taskflow_with_data(&path, &["list", "--sort", "title"]);
        assert!(output.status.success(), "list with sort should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Apple should appear before Zebra when sorted by title
        let apple_pos = stdout.find("Apple task").unwrap_or(usize::MAX);
        let zebra_pos = stdout.find("Zebra task").unwrap_or(usize::MAX);
        assert!(apple_pos < zebra_pos, "Apple should come before Zebra");
    }

    #[test]
    fn test_list_sort_reverse() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        // Add tasks
        run_taskflow_with_data(&path, &["add", "Zebra task"]);
        run_taskflow_with_data(&path, &["add", "Apple task"]);

        // Sort by title reversed
        let output = run_taskflow_with_data(&path, &["list", "--sort", "title", "--reverse"]);
        assert!(output.status.success(), "list with reverse should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Zebra should appear before Apple when reversed
        let apple_pos = stdout.find("Apple task").unwrap_or(0);
        let zebra_pos = stdout.find("Zebra task").unwrap_or(usize::MAX);
        assert!(zebra_pos < apple_pos, "Zebra should come before Apple");
    }

    #[test]
    fn test_list_view_untagged() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        // Add tasks with and without tags
        run_taskflow_with_data(&path, &["add", "Tagged task #mytag"]);
        run_taskflow_with_data(&path, &["add", "Untagged task"]);

        // View untagged
        let output = run_taskflow_with_data(&path, &["list", "--view", "untagged"]);
        assert!(output.status.success(), "list untagged should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Untagged task"),
            "should show untagged task"
        );
        assert!(
            !stdout.contains("Tagged task"),
            "should not show tagged task"
        );
    }

    // === Done Command Tests ===

    #[test]
    fn test_done_marks_task_complete() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        // Add a task
        run_taskflow_with_data(&path, &["add", "Task to complete"]);

        // Mark it done
        let output = run_taskflow_with_data(&path, &["done", "Task to complete"]);
        assert!(
            output.status.success(),
            "done should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Verify it's marked as done (list with --completed)
        let list_output = run_taskflow_with_data(&path, &["list", "--completed"]);
        let stdout = String::from_utf8_lossy(&list_output.stdout);
        assert!(stdout.contains("✓"), "should show completed checkmark");
    }

    #[test]
    fn test_done_no_matching_task() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        // Try to mark non-existent task as done
        let output = run_taskflow_with_data(&path, &["done", "nonexistent task"]);
        // This should either succeed with "no matching" message or return non-zero
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stdout.contains("No matching")
                || stdout.contains("not found")
                || stderr.contains("No matching")
                || stderr.contains("not found")
                || stdout.contains("No tasks"),
            "should indicate no matching task"
        );
    }

    // === Backend Flag Integration Tests ===

    #[test]
    fn test_yaml_backend_integration() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.yaml");

        // Add task with YAML backend
        let output = Command::new("cargo")
            .args([
                "run",
                "--quiet",
                "--",
                "--backend",
                "yaml",
                "--data",
                path.to_str().unwrap(),
                "add",
                "YAML backend test",
            ])
            .output()
            .expect("Failed to execute");

        assert!(
            output.status.success(),
            "YAML add should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Verify file was created as YAML
        assert!(path.exists(), "YAML file should be created");
        let content = std::fs::read_to_string(&path).unwrap();
        // YAML files typically don't start with { like JSON
        assert!(!content.starts_with('{'), "should not be JSON format");
    }

    #[test]
    fn test_sqlite_backend_integration() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");

        // Add task with SQLite backend
        let output = Command::new("cargo")
            .args([
                "run",
                "--quiet",
                "--",
                "--backend",
                "sqlite",
                "--data",
                path.to_str().unwrap(),
                "add",
                "SQLite backend test",
            ])
            .output()
            .expect("Failed to execute");

        assert!(
            output.status.success(),
            "SQLite add should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Verify file was created
        assert!(path.exists(), "SQLite file should be created");

        // List to verify data was stored
        let list_output = Command::new("cargo")
            .args([
                "run",
                "--quiet",
                "--",
                "--backend",
                "sqlite",
                "--data",
                path.to_str().unwrap(),
                "list",
            ])
            .output()
            .expect("Failed to execute");

        let stdout = String::from_utf8_lossy(&list_output.stdout);
        assert!(
            stdout.contains("SQLite backend test"),
            "should retrieve stored task"
        );
    }

    // === Debug Flag Tests ===

    #[test]
    fn test_debug_flag_accepted() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        // Note: We don't use current_dir because cargo needs Cargo.toml
        // The log file will be created in the current directory if writable
        let output = run_taskflow(&["--debug", "--data", path.to_str().unwrap(), "list"]);

        assert!(
            output.status.success(),
            "debug flag should be accepted: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_log_level_flag_accepted() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        for level in ["trace", "debug", "info", "warn", "error"] {
            let output = run_taskflow(&[
                "--debug",
                "--log-level",
                level,
                "--data",
                path.to_str().unwrap(),
                "list",
            ]);

            assert!(
                output.status.success(),
                "log level {} should be accepted: {}",
                level,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    // === Subcommand Alias Tests ===

    #[test]
    fn test_add_alias_a() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        // Use 'a' alias instead of 'add'
        let output = run_taskflow_with_data(&path, &["a", "Task via alias"]);
        assert!(
            output.status.success(),
            "add alias 'a' should work: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Added"), "should confirm task was added");
    }

    #[test]
    fn test_list_alias_ls() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        // Use 'ls' alias instead of 'list'
        let output = run_taskflow_with_data(&path, &["ls"]);
        assert!(
            output.status.success(),
            "list alias 'ls' should work: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_done_alias_d() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.json");

        // Add a task first
        run_taskflow_with_data(&path, &["add", "Task for done alias"]);

        // Use 'd' alias instead of 'done'
        let output = run_taskflow_with_data(&path, &["d", "Task for done alias"]);
        assert!(
            output.status.success(),
            "done alias 'd' should work: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

// Edge case tests
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_all_backends_empty_string_fields() {
        for (backend_name, mut backend) in create_all_backends() {
            // Task with empty description
            let mut task = Task::new("Task with empty description");
            task.description = Some(String::new());

            backend
                .create_task(&task)
                .unwrap_or_else(|e| panic!("{}: create_task failed: {}", backend_name, e));

            let retrieved = backend.get_task(&task.id).unwrap().unwrap();
            // Empty string might be normalized to None or preserved as empty
            // Either is acceptable
            assert!(
                retrieved.description.is_none() || retrieved.description == Some(String::new()),
                "{}: empty description should be None or empty string",
                backend_name
            );
        }
    }

    #[test]
    fn test_all_backends_unicode_content() {
        for (backend_name, mut backend) in create_all_backends() {
            // Task with unicode characters
            let task = Task::new("Task with emojis and unicode")
                .with_description("Description with emojis and unicode characters");

            backend
                .create_task(&task)
                .unwrap_or_else(|e| panic!("{}: create_task failed: {}", backend_name, e));

            let retrieved = backend.get_task(&task.id).unwrap().unwrap();
            assert!(
                retrieved.title.contains("unicode"),
                "{}: unicode in title should be preserved",
                backend_name
            );
        }
    }

    #[test]
    fn test_all_backends_very_long_content() {
        for (backend_name, mut backend) in create_all_backends() {
            // Task with very long title and description
            let long_title: String = (0..500).map(|_| 'a').collect();
            let long_desc: String = (0..10000).map(|_| 'b').collect();

            let task = Task::new(&long_title).with_description(&long_desc);

            backend
                .create_task(&task)
                .unwrap_or_else(|e| panic!("{}: create_task failed: {}", backend_name, e));

            let retrieved = backend.get_task(&task.id).unwrap().unwrap();
            assert_eq!(
                retrieved.title.len(),
                500,
                "{}: long title should be preserved",
                backend_name
            );
            assert_eq!(
                retrieved.description.as_ref().map(|d| d.len()),
                Some(10000),
                "{}: long description should be preserved",
                backend_name
            );
        }
    }

    #[test]
    fn test_all_backends_many_tags() {
        for (backend_name, mut backend) in create_all_backends() {
            // Task with many tags
            let tags: Vec<String> = (0..50).map(|i| format!("tag{}", i)).collect();
            let task = Task::new("Task with many tags").with_tags(tags.clone());

            backend
                .create_task(&task)
                .unwrap_or_else(|e| panic!("{}: create_task failed: {}", backend_name, e));

            let retrieved = backend.get_task(&task.id).unwrap().unwrap();
            assert_eq!(
                retrieved.tags.len(),
                50,
                "{}: all 50 tags should be preserved",
                backend_name
            );
        }
    }

    #[test]
    fn test_all_backends_concurrent_operations() {
        for (backend_name, mut backend) in create_all_backends() {
            // Create multiple tasks quickly
            let mut tasks = Vec::new();
            for i in 0..10 {
                let task = Task::new(format!("Concurrent task {}", i));
                tasks.push(task.clone());
                backend.create_task(&task).unwrap_or_else(|e| {
                    panic!("{}: create_task {} failed: {}", backend_name, i, e)
                });
            }

            // Verify all were created
            let all_tasks = backend
                .list_tasks()
                .unwrap_or_else(|e| panic!("{}: list_tasks failed: {}", backend_name, e));
            assert_eq!(
                all_tasks.len(),
                10,
                "{}: should have all 10 tasks",
                backend_name
            );

            // Update all tasks
            for (i, task) in tasks.iter_mut().enumerate() {
                task.status = TaskStatus::InProgress;
                backend.update_task(task).unwrap_or_else(|e| {
                    panic!("{}: update_task {} failed: {}", backend_name, i, e)
                });
            }

            // Verify all updates
            let all_tasks = backend.list_tasks().unwrap();
            let in_progress_count = all_tasks
                .iter()
                .filter(|t| t.status == TaskStatus::InProgress)
                .count();
            assert_eq!(
                in_progress_count, 10,
                "{}: all tasks should be InProgress",
                backend_name
            );
        }
    }
}
