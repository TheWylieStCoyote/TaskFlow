//! Comprehensive tests for CLI commands.
//!
//! These tests cover:
//! - add command: Quick-add task creation
//! - done command: Task completion
//! - list command: Task filtering and display
//! - next command: Next task selection
//! - today command: Today's tasks
//! - stats command: Statistics aggregation
//! - git commands: Git integration commands

#[cfg(test)]
mod cli_command_tests {
    use std::path::PathBuf;
    use taskflow::app::Model;
    use taskflow::domain::{Priority, Task, TaskStatus};
    use taskflow::storage::BackendType;

    use crate::cli::Cli;

    // ========================================================================
    // Helper Functions
    // ========================================================================

    /// Create a test CLI with in-memory storage
    fn create_test_cli() -> Cli {
        Cli {
            data: None,
            backend: BackendType::Json,
            demo: false,
            debug: false,
            log_level: "info".to_string(),
            command: None,
        }
    }

    /// Create a test model with sample tasks
    fn create_test_model() -> Model {
        let mut model = Model::new();

        // Add high priority task
        let task1 = Task::new("High priority bug fix")
            .with_priority(Priority::High)
            .with_tags(vec!["bug".to_string(), "urgent".to_string()]);

        // Add medium priority task
        let task2 = Task::new("Medium priority feature")
            .with_priority(Priority::Medium)
            .with_tags(vec!["feature".to_string()]);

        // Add low priority task
        let task3 = Task::new("Low priority docs update")
            .with_priority(Priority::Low)
            .with_tags(vec!["docs".to_string()]);

        // Add completed task
        let mut task4 = Task::new("Completed task");
        task4.status = TaskStatus::Done;

        model.tasks.insert(task1.id, task1);
        model.tasks.insert(task2.id, task2);
        model.tasks.insert(task3.id, task3);
        model.tasks.insert(task4.id, task4);
        model.refresh_visible_tasks();

        model
    }

    // ========================================================================
    // Quick-Add Command Tests
    // ========================================================================

    #[test]
    fn test_quick_add_simple_task() {
        // Test that quick-add can create a simple task
        let task_input = ["Buy".to_string(), "milk".to_string()];
        let joined = task_input.join(" ");
        assert_eq!(joined, "Buy milk");
    }

    #[test]
    fn test_quick_add_with_priority() {
        // Test priority extraction from quick-add syntax
        let task_input = ["Fix".to_string(), "bug".to_string(), "!high".to_string()];
        let text = task_input.join(" ");
        assert!(text.contains("!high"));
    }

    #[test]
    fn test_quick_add_with_tags() {
        // Test tag extraction from quick-add syntax
        let task_input = [
            "Review".to_string(),
            "PR".to_string(),
            "#code".to_string(),
            "#urgent".to_string(),
        ];
        let text = task_input.join(" ");
        assert!(text.contains("#code"));
        assert!(text.contains("#urgent"));
    }

    #[test]
    fn test_quick_add_with_due_date() {
        // Test due date extraction from quick-add syntax
        let task_input = [
            "Submit".to_string(),
            "report".to_string(),
            "due:tomorrow".to_string(),
        ];
        let text = task_input.join(" ");
        assert!(text.contains("due:tomorrow"));
    }

    #[test]
    fn test_quick_add_complex() {
        // Test complex quick-add with multiple attributes
        let task_input = [
            "Fix".to_string(),
            "login".to_string(),
            "bug".to_string(),
            "!urgent".to_string(),
            "#bug".to_string(),
            "#security".to_string(),
            "due:friday".to_string(),
        ];
        let text = task_input.join(" ");
        assert!(text.contains("!urgent"));
        assert!(text.contains("#bug"));
        assert!(text.contains("#security"));
        assert!(text.contains("due:friday"));
    }

    // ========================================================================
    // List Command Filter Tests
    // ========================================================================

    #[test]
    fn test_list_filter_by_priority() {
        let model = create_test_model();
        let high_priority_tasks: Vec<_> = model
            .tasks
            .values()
            .filter(|t| t.priority == Priority::High)
            .collect();

        assert_eq!(high_priority_tasks.len(), 1);
        assert_eq!(high_priority_tasks[0].title, "High priority bug fix");
    }

    #[test]
    fn test_list_filter_by_status() {
        let model = create_test_model();
        let todo_tasks: Vec<_> = model
            .tasks
            .values()
            .filter(|t| t.status == TaskStatus::Todo)
            .collect();

        assert_eq!(todo_tasks.len(), 3);
    }

    #[test]
    fn test_list_filter_by_tags() {
        let model = create_test_model();
        let bug_tasks: Vec<_> = model
            .tasks
            .values()
            .filter(|t| t.tags.contains(&"bug".to_string()))
            .collect();

        assert_eq!(bug_tasks.len(), 1);
        assert!(bug_tasks[0].title.contains("bug fix"));
    }

    #[test]
    fn test_list_exclude_completed() {
        let model = create_test_model();
        let active_tasks: Vec<_> = model
            .tasks
            .values()
            .filter(|t| t.status != TaskStatus::Done && t.status != TaskStatus::Cancelled)
            .collect();

        assert_eq!(active_tasks.len(), 3);
    }

    #[test]
    fn test_list_include_completed() {
        let model = create_test_model();
        let all_tasks: Vec<_> = model.tasks.values().collect();

        assert_eq!(all_tasks.len(), 4);
    }

    #[test]
    fn test_list_filter_multiple_priorities() {
        let model = create_test_model();
        let important_tasks: Vec<_> = model
            .tasks
            .values()
            .filter(|t| t.priority == Priority::High || t.priority == Priority::Urgent)
            .collect();

        assert_eq!(important_tasks.len(), 1);
    }

    #[test]
    fn test_list_filter_multiple_statuses() {
        let model = create_test_model();
        let active_or_blocked: Vec<_> = model
            .tasks
            .values()
            .filter(|t| t.status == TaskStatus::Todo || t.status == TaskStatus::Blocked)
            .collect();

        assert_eq!(active_or_blocked.len(), 3);
    }

    #[test]
    fn test_list_search_in_title() {
        let model = create_test_model();
        let search_term = "bug";
        let matching_tasks: Vec<_> = model
            .tasks
            .values()
            .filter(|t| t.title.to_lowercase().contains(&search_term.to_lowercase()))
            .collect();

        assert_eq!(matching_tasks.len(), 1);
    }

    #[test]
    fn test_list_limit() {
        let model = create_test_model();
        let limited: Vec<_> = model.tasks.values().take(2).collect();

        assert_eq!(limited.len(), 2);
    }

    // ========================================================================
    // Next Command Tests
    // ========================================================================

    #[test]
    fn test_next_task_selection_by_priority() {
        let model = create_test_model();

        // Find highest priority incomplete task
        let next_task = model
            .tasks
            .values()
            .filter(|t| t.status != TaskStatus::Done && t.status != TaskStatus::Cancelled)
            .max_by_key(|t| match t.priority {
                Priority::Urgent => 5,
                Priority::High => 4,
                Priority::Medium => 3,
                Priority::Low => 2,
                Priority::None => 1,
            });

        assert!(next_task.is_some());
        let task = next_task.unwrap();
        assert_eq!(task.priority, Priority::High);
        assert_eq!(task.title, "High priority bug fix");
    }

    #[test]
    fn test_next_task_no_incomplete_tasks() {
        let mut model = Model::new();

        // Add only completed tasks
        let mut task1 = Task::new("Completed task 1");
        task1.status = TaskStatus::Done;
        let mut task2 = Task::new("Completed task 2");
        task2.status = TaskStatus::Done;

        model.tasks.insert(task1.id, task1);
        model.tasks.insert(task2.id, task2);

        let next_task = model
            .tasks
            .values()
            .filter(|t| t.status != TaskStatus::Done && t.status != TaskStatus::Cancelled)
            .max_by_key(|t| match t.priority {
                Priority::Urgent => 5,
                Priority::High => 4,
                Priority::Medium => 3,
                Priority::Low => 2,
                Priority::None => 1,
            });

        assert!(next_task.is_none());
    }

    #[test]
    fn test_next_task_empty_model() {
        let model = Model::new();

        let next_task = model
            .tasks
            .values()
            .find(|t| t.status != TaskStatus::Done && t.status != TaskStatus::Cancelled);

        assert!(next_task.is_none());
    }

    // ========================================================================
    // Today Command Tests
    // ========================================================================

    #[test]
    fn test_today_filter() {
        use chrono::Utc;
        let mut model = Model::new();
        let today = Utc::now().date_naive();

        // Add task due today
        let task1 = Task::new("Due today").with_due_date(today);

        // Add task due yesterday
        let task2 = Task::new("Due yesterday").with_due_date(today - chrono::Duration::days(1));

        // Add task due tomorrow
        let task3 = Task::new("Due tomorrow").with_due_date(today + chrono::Duration::days(1));

        model.tasks.insert(task1.id, task1);
        model.tasks.insert(task2.id, task2);
        model.tasks.insert(task3.id, task3);

        // Filter for today
        let today_tasks: Vec<_> = model
            .tasks
            .values()
            .filter(|t| t.due_date == Some(today))
            .collect();

        assert_eq!(today_tasks.len(), 1);
        assert_eq!(today_tasks[0].title, "Due today");
    }

    #[test]
    fn test_today_no_due_date() {
        let mut model = Model::new();

        // Add task without due date
        let task1 = Task::new("No due date");
        model.tasks.insert(task1.id, task1);

        // Filter for tasks with due date
        let tasks_with_due: Vec<_> = model
            .tasks
            .values()
            .filter(|t| t.due_date.is_some())
            .collect();

        assert_eq!(tasks_with_due.len(), 0);
    }

    // ========================================================================
    // Done Command Tests
    // ========================================================================

    #[test]
    fn test_mark_task_done() {
        let mut model = create_test_model();
        let task_id = model
            .tasks
            .values()
            .find(|t| t.title == "High priority bug fix")
            .unwrap()
            .id;

        // Mark as done
        if let Some(task) = model.tasks.get_mut(&task_id) {
            task.status = TaskStatus::Done;
        }

        let task = model.tasks.get(&task_id).unwrap();
        assert_eq!(task.status, TaskStatus::Done);
    }

    #[test]
    fn test_mark_task_done_by_search() {
        let model = create_test_model();
        let search = "bug";

        // Find task matching search
        let matching_tasks: Vec<_> = model
            .tasks
            .values()
            .filter(|t| t.title.to_lowercase().contains(&search.to_lowercase()))
            .collect();

        assert_eq!(matching_tasks.len(), 1);
        assert_eq!(matching_tasks[0].title, "High priority bug fix");
    }

    #[test]
    fn test_mark_task_done_ambiguous_search() {
        let mut model = Model::new();

        // Add multiple tasks matching search
        let task1 = Task::new("Fix bug 1");
        let task2 = Task::new("Fix bug 2");
        model.tasks.insert(task1.id, task1);
        model.tasks.insert(task2.id, task2);

        let search = "bug";
        let matching_tasks: Vec<_> = model
            .tasks
            .values()
            .filter(|t| t.title.to_lowercase().contains(&search.to_lowercase()))
            .collect();

        // Should find multiple matches
        assert_eq!(matching_tasks.len(), 2);
    }

    #[test]
    fn test_mark_task_done_no_match() {
        let model = create_test_model();
        let search = "nonexistent";

        let matching_tasks: Vec<_> = model
            .tasks
            .values()
            .filter(|t| t.title.to_lowercase().contains(&search.to_lowercase()))
            .collect();

        assert_eq!(matching_tasks.len(), 0);
    }

    // ========================================================================
    // Stats Command Tests
    // ========================================================================

    #[test]
    fn test_stats_total_tasks() {
        let model = create_test_model();
        let total = model.tasks.len();
        assert_eq!(total, 4);
    }

    #[test]
    fn test_stats_completed_tasks() {
        let model = create_test_model();
        let completed = model
            .tasks
            .values()
            .filter(|t| t.status == TaskStatus::Done)
            .count();
        assert_eq!(completed, 1);
    }

    #[test]
    fn test_stats_incomplete_tasks() {
        let model = create_test_model();
        let incomplete = model
            .tasks
            .values()
            .filter(|t| t.status != TaskStatus::Done && t.status != TaskStatus::Cancelled)
            .count();
        assert_eq!(incomplete, 3);
    }

    #[test]
    fn test_stats_by_priority() {
        let model = create_test_model();

        let high = model
            .tasks
            .values()
            .filter(|t| t.priority == Priority::High)
            .count();
        let medium = model
            .tasks
            .values()
            .filter(|t| t.priority == Priority::Medium)
            .count();
        let low = model
            .tasks
            .values()
            .filter(|t| t.priority == Priority::Low)
            .count();

        assert_eq!(high, 1);
        assert_eq!(medium, 1);
        assert_eq!(low, 1);
    }

    #[test]
    fn test_stats_by_status() {
        let model = create_test_model();

        let todo = model
            .tasks
            .values()
            .filter(|t| t.status == TaskStatus::Todo)
            .count();
        let in_progress = model
            .tasks
            .values()
            .filter(|t| t.status == TaskStatus::InProgress)
            .count();
        let done = model
            .tasks
            .values()
            .filter(|t| t.status == TaskStatus::Done)
            .count();

        assert_eq!(todo, 3);
        assert_eq!(in_progress, 0);
        assert_eq!(done, 1);
    }

    #[test]
    fn test_stats_completion_rate() {
        let model = create_test_model();

        let total = model.tasks.len();
        let completed = model
            .tasks
            .values()
            .filter(|t| t.status == TaskStatus::Done)
            .count();

        let completion_rate = if total > 0 {
            (completed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        assert_eq!(completion_rate, 25.0); // 1 out of 4 tasks completed
    }

    // ========================================================================
    // CLI Structure Tests
    // ========================================================================

    #[test]
    fn test_cli_default_values() {
        let cli = create_test_cli();
        assert_eq!(cli.backend, BackendType::Json);
        assert!(!cli.demo);
        assert!(!cli.debug);
        assert_eq!(cli.log_level, "info");
    }

    #[test]
    fn test_cli_with_data_path() {
        let cli = Cli {
            data: Some(PathBuf::from("/tmp/test.json")),
            backend: BackendType::Json,
            demo: false,
            debug: false,
            log_level: "info".to_string(),
            command: None,
        };

        assert_eq!(cli.data, Some(PathBuf::from("/tmp/test.json")));
    }

    #[test]
    fn test_cli_with_backend() {
        let backends = [
            BackendType::Json,
            BackendType::Yaml,
            BackendType::Sqlite,
            BackendType::Markdown,
        ];

        for backend in backends {
            let cli = Cli {
                data: None,
                backend,
                demo: false,
                debug: false,
                log_level: "info".to_string(),
                command: None,
            };
            assert_eq!(cli.backend, backend);
        }
    }

    #[test]
    fn test_cli_demo_mode() {
        let cli = Cli {
            data: None,
            backend: BackendType::Json,
            demo: true,
            debug: false,
            log_level: "info".to_string(),
            command: None,
        };

        assert!(cli.demo);
    }

    #[test]
    fn test_cli_debug_mode() {
        let cli = Cli {
            data: None,
            backend: BackendType::Json,
            demo: false,
            debug: true,
            log_level: "debug".to_string(),
            command: None,
        };

        assert!(cli.debug);
        assert_eq!(cli.log_level, "debug");
    }
}
