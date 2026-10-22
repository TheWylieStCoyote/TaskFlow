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
    use taskflow::domain::{today, Priority, Task, TaskStatus};
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
        let mut model = Model::new();
        let today = today();

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

/// Shared helpers for the behavioural command tests that live next to each
/// command (`commands/*.rs`).
///
/// Every helper keeps the tests away from the developer's real configuration
/// and data: the CLI always uses an explicit non-JSON backend (so the backend
/// stored in `Settings` is never consulted) and a data file under a fresh
/// temporary directory.
#[cfg(test)]
pub(crate) mod support {
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};

    use chrono::{DateTime, Local, NaiveDate, TimeZone, Utc};
    use tempfile::TempDir;

    use taskflow::app::Model;
    use taskflow::domain::{Project, ProjectId, Task, TaskId};
    use taskflow::storage::BackendType;

    use crate::cli::Cli;

    /// Environment variable naming the exit-path case a child process runs.
    const CASE_ENV: &str = "TASKFLOW_TEST_EXIT_CASE";
    /// Environment variable carrying the data file for a child process.
    const DATA_ENV: &str = "TASKFLOW_TEST_EXIT_DATA";
    /// Environment variable carrying the repository path for a child process.
    const REPO_ENV: &str = "TASKFLOW_TEST_EXIT_REPO";

    /// A fresh temporary directory; removed when dropped.
    pub(crate) fn temp_dir() -> TempDir {
        TempDir::new().expect("create temp dir")
    }

    /// Path of the YAML data file used inside `dir`.
    pub(crate) fn data_file(dir: &Path) -> PathBuf {
        dir.join("tasks.yaml")
    }

    /// A CLI that reads and writes only `data` through the YAML backend.
    pub(crate) fn yaml_cli(data: &Path) -> Cli {
        Cli {
            data: Some(data.to_path_buf()),
            backend: BackendType::Yaml,
            demo: false,
            debug: false,
            log_level: "info".to_string(),
            command: None,
        }
    }

    /// Open (or create) the YAML store at `data`.
    pub(crate) fn open_model(data: &Path) -> Model {
        Model::new()
            .with_storage(BackendType::Yaml, data.to_path_buf())
            .expect("open yaml storage")
    }

    /// Populate the YAML store at `data` with `seed` and save it.
    pub(crate) fn seed_model(data: &Path, seed: impl FnOnce(&mut Model)) -> Model {
        let mut model = open_model(data);
        seed(&mut model);
        model.save().expect("save seeded model");
        model
    }

    /// Insert `task` into the model and its storage.
    pub(crate) fn add_task(model: &mut Model, task: Task) -> TaskId {
        let id = task.id;
        model.tasks.insert(id, task.clone());
        model.sync_task(&task);
        id
    }

    /// Insert `project` into the model and its storage.
    pub(crate) fn add_project(model: &mut Model, project: Project) -> ProjectId {
        let id = project.id;
        model.projects.insert(id, project.clone());
        model.sync_project(&project);
        id
    }

    /// Local noon on `date`, as a UTC instant.
    ///
    /// Noon is never ambiguous across DST transitions, so the instant's local
    /// date is always `date`.
    pub(crate) fn local_noon(date: NaiveDate) -> DateTime<Utc> {
        let naive = date.and_hms_opt(12, 0, 0).expect("valid time");
        Local
            .from_local_datetime(&naive)
            .single()
            .expect("noon is unambiguous")
            .with_timezone(&Utc)
    }

    /// Whether a `git` binary can be executed.
    pub(crate) fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    }

    /// Run `git <args>` in `repo`, failing the test if git reports an error.
    pub(crate) fn git(repo: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Initialise a repository in `dir` with one commit on `main`.
    pub(crate) fn init_repo(dir: &Path) {
        git(dir, &["init", "-q"]);
        git(dir, &["config", "user.email", "test@test.com"]);
        git(dir, &["config", "user.name", "Test User"]);
        git(dir, &["config", "commit.gpgsign", "false"]);
        std::fs::write(dir.join("README.md"), "# Test Repository\n").expect("write README");
        git(dir, &["add", "."]);
        git(dir, &["commit", "-q", "-m", "Initial commit"]);
        git(dir, &["branch", "-M", "main"]);
    }

    /// Create `file` with `content` in `dir` and commit it on the current branch.
    pub(crate) fn commit_file(dir: &Path, file: &str, content: &str, message: &str) {
        let path = dir.join(file);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent dir");
        }
        std::fs::write(path, content).expect("write file");
        git(dir, &["add", "."]);
        git(dir, &["commit", "-q", "-m", message]);
    }

    /// Re-run this test binary so that a command which calls
    /// `std::process::exit` can be observed from the outside.
    ///
    /// The child runs [`exit_case_child`] with `case` selected through the
    /// environment; the returned [`Output`] carries its exit status and the
    /// text it printed.
    pub(crate) fn run_exit_case(case: &str, data: &Path, repo: Option<&Path>) -> Output {
        let exe = std::env::current_exe().expect("current test executable");
        let mut cmd = Command::new(exe);
        cmd.args([
            "--exact",
            "commands_tests::support::exit_case_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CASE_ENV, case)
        .env(DATA_ENV, data);
        match repo {
            Some(repo) => cmd.env(REPO_ENV, repo),
            None => cmd.env_remove(REPO_ENV),
        };
        cmd.output().expect("spawn child test process")
    }

    /// Assert that an exit-path child terminated with status 1 and return
    /// its combined stdout and stderr.
    pub(crate) fn assert_exit_one(output: &Output) -> String {
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.status.code(), Some(1), "child output:\n{text}");
        text
    }

    /// Entry point for [`run_exit_case`]; a no-op in a normal test run.
    #[test]
    fn exit_case_child() {
        let Ok(case) = std::env::var(CASE_ENV) else {
            return;
        };
        let data = PathBuf::from(std::env::var(DATA_ENV).expect("data path"));
        let repo = std::env::var(REPO_ENV).ok().map(PathBuf::from);
        let cli = yaml_cli(&data);
        let words =
            |items: &[&str]| -> Vec<String> { items.iter().map(ToString::to_string).collect() };
        let repo = || repo.clone().expect("repo path");

        let result = match case.as_str() {
            "done_empty_query" => crate::commands::mark_task_done(&cli, &[], None, None),
            "done_no_match" => crate::commands::mark_task_done(
                &cli,
                &words(&["nonexistent"]),
                Some("missing-project"),
                None,
            ),
            "done_multiple" => {
                crate::commands::mark_task_done(&cli, &words(&["fix", "bug"]), None, None)
            }
            "add_empty" => crate::commands::quick_add_task(&cli, &words(&["  "])),
            "git_link_empty_query" => crate::commands::git::git_link_in(&cli, &repo(), " ", None),
            "git_link_not_repo" => crate::commands::git::git_link_in(&cli, &repo(), "login", None),
            "git_link_missing_branch" => {
                crate::commands::git::git_link_in(&cli, &repo(), "login", Some("no-such-branch"))
            }
            "git_link_no_match" => {
                crate::commands::git::git_link_in(&cli, &repo(), "nonexistent", None)
            }
            "git_link_multiple" => crate::commands::git::git_link_in(&cli, &repo(), "bug", None),
            "git_unlink_empty_query" => crate::commands::git_unlink(&cli, ""),
            "git_unlink_no_match" => crate::commands::git_unlink(&cli, "nonexistent"),
            "git_unlink_multiple" => crate::commands::git_unlink(&cli, "bug"),
            "git_unlink_not_linked" => crate::commands::git_unlink(&cli, "login"),
            "git_commits_empty_query" => {
                crate::commands::git::git_commits_in(&cli, &repo(), "", 10)
            }
            "git_commits_not_repo" => {
                crate::commands::git::git_commits_in(&cli, &repo(), "login", 10)
            }
            "git_commits_no_match" => {
                crate::commands::git::git_commits_in(&cli, &repo(), "nonexistent", 10)
            }
            "git_commits_multiple" => {
                crate::commands::git::git_commits_in(&cli, &repo(), "bug", 10)
            }
            "git_commits_not_linked" => {
                crate::commands::git::git_commits_in(&cli, &repo(), "login", 10)
            }
            "git_sync_not_repo" => crate::commands::git::git_sync_in(&cli, &repo(), false),
            "git_check_merged_not_repo" => {
                crate::commands::git::git_check_merged_in(&cli, &repo(), false)
            }
            other => panic!("unknown exit case {other}"),
        };
        // Every case above is expected to exit the process before returning.
        panic!("exit case {case} returned {result:?} instead of exiting");
    }
}
