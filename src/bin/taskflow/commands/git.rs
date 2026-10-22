//! Git integration CLI commands.

use std::env;
use std::path::Path;

use chrono::Utc;
use tracing::{debug, info, warn};

use taskflow::domain::git::{
    matching::BranchMatcher,
    operations::{
        branch_exists, get_base_branch, get_branch_commits, get_branch_remote, get_current_branch,
        is_branch_merged, is_git_repo,
    },
    BranchStatus, GitLinkType, GitRef,
};
use taskflow::domain::TaskStatus;

use crate::cli::Cli;
use crate::load_model_for_cli;

/// Link a task to a git branch.
///
/// If no branch is specified, links to the current branch.
pub fn git_link(cli: &Cli, task_query: &str, branch: Option<&str>) -> anyhow::Result<()> {
    // Determine repository path
    let repo_path = env::current_dir()?;
    git_link_in(cli, &repo_path, task_query, branch)
}

/// [`git_link`] against the repository at `repo_path`.
pub(crate) fn git_link_in(
    cli: &Cli,
    repo_path: &Path,
    task_query: &str,
    branch: Option<&str>,
) -> anyhow::Result<()> {
    let query = task_query.to_lowercase();
    if query.trim().is_empty() {
        eprintln!("Error: Task query cannot be empty");
        eprintln!("Usage: taskflow git link <task-query> [--branch <branch>]");
        std::process::exit(1);
    }

    if !is_git_repo(repo_path) {
        eprintln!("Error: Not in a git repository");
        std::process::exit(1);
    }

    // Determine branch to link
    let branch_name = match branch {
        Some(b) => {
            if !branch_exists(repo_path, b) {
                eprintln!("Error: Branch '{}' does not exist", b);
                std::process::exit(1);
            }
            b.to_string()
        }
        None => get_current_branch(repo_path)?,
    };

    // Load model
    let mut model = load_model_for_cli(cli)?;

    // Find matching task
    let matches: Vec<_> = model
        .tasks
        .values()
        .filter(|t| t.title.to_lowercase().contains(&query))
        .collect();

    let task_id = match matches.len() {
        0 => {
            eprintln!("No tasks found matching: \"{}\"", query);
            std::process::exit(1);
        }
        1 => matches[0].id,
        n => {
            eprintln!("Multiple tasks match '{}' ({} found):", query, n);
            for (i, task) in matches.iter().enumerate() {
                println!("  {}. {}", i + 1, task.title);
            }
            eprintln!("\nUse a more specific query to match a single task.");
            std::process::exit(1);
        }
    };

    let task_title = matches[0].title.clone();

    // Create git ref
    let git_ref = GitRef {
        branch: branch_name.clone(),
        remote: get_branch_remote(repo_path, &branch_name),
        linked_at: Utc::now(),
        link_type: GitLinkType::Manual,
    };

    // Update task
    if let Some(task) = model.tasks.get_mut(&task_id) {
        task.git_ref = Some(git_ref);
        task.updated_at = Utc::now();
    }

    model.sync_task_by_id(&task_id);
    if let Err(e) = model.save() {
        warn!(error = %e, "Could not save after linking task to branch");
        eprintln!("Warning: Could not save: {e}");
    }

    println!("✓ Linked '{}' to branch '{}'", task_title, branch_name);
    Ok(())
}

/// Unlink a task from its git branch.
pub fn git_unlink(cli: &Cli, task_query: &str) -> anyhow::Result<()> {
    let query = task_query.to_lowercase();
    if query.trim().is_empty() {
        eprintln!("Error: Task query cannot be empty");
        eprintln!("Usage: taskflow git unlink <task-query>");
        std::process::exit(1);
    }

    let mut model = load_model_for_cli(cli)?;

    // Find matching task
    let matches: Vec<_> = model
        .tasks
        .values()
        .filter(|t| t.title.to_lowercase().contains(&query))
        .collect();

    let task_id = match matches.len() {
        0 => {
            eprintln!("No tasks found matching: \"{}\"", query);
            std::process::exit(1);
        }
        1 => matches[0].id,
        n => {
            eprintln!("Multiple tasks match '{}' ({} found):", query, n);
            for (i, task) in matches.iter().enumerate() {
                let linked = task
                    .git_ref
                    .as_ref()
                    .map(|r| format!(" → {}", r.branch))
                    .unwrap_or_default();
                println!("  {}. {}{}", i + 1, task.title, linked);
            }
            eprintln!("\nUse a more specific query to match a single task.");
            std::process::exit(1);
        }
    };

    let task_title = matches[0].title.clone();
    let had_link = matches[0].git_ref.is_some();

    if !had_link {
        eprintln!("Task '{}' is not linked to any branch", task_title);
        std::process::exit(1);
    }

    // Remove git ref
    if let Some(task) = model.tasks.get_mut(&task_id) {
        task.git_ref = None;
        task.updated_at = Utc::now();
    }

    model.sync_task_by_id(&task_id);
    if let Err(e) = model.save() {
        warn!(error = %e, "Could not save after unlinking task from branch");
        eprintln!("Warning: Could not save: {e}");
    }

    println!("✓ Unlinked '{}' from git branch", task_title);
    Ok(())
}

/// Show commit history for a linked task.
pub fn git_commits(cli: &Cli, task_query: &str, limit: usize) -> anyhow::Result<()> {
    let repo_path = env::current_dir()?;
    git_commits_in(cli, &repo_path, task_query, limit)
}

/// [`git_commits`] against the repository at `repo_path`.
pub(crate) fn git_commits_in(
    cli: &Cli,
    repo_path: &Path,
    task_query: &str,
    limit: usize,
) -> anyhow::Result<()> {
    let query = task_query.to_lowercase();
    if query.trim().is_empty() {
        eprintln!("Error: Task query cannot be empty");
        eprintln!("Usage: taskflow git commits <task-query> [--limit <n>]");
        std::process::exit(1);
    }

    if !is_git_repo(repo_path) {
        eprintln!("Error: Not in a git repository");
        std::process::exit(1);
    }

    let model = load_model_for_cli(cli)?;

    // Find matching task
    let matches: Vec<_> = model
        .tasks
        .values()
        .filter(|t| t.title.to_lowercase().contains(&query))
        .collect();

    let task = match matches.len() {
        0 => {
            eprintln!("No tasks found matching: \"{}\"", query);
            std::process::exit(1);
        }
        1 => matches[0],
        n => {
            eprintln!("Multiple tasks match '{}' ({} found):", query, n);
            for (i, task) in matches.iter().enumerate() {
                println!("  {}. {}", i + 1, task.title);
            }
            eprintln!("\nUse a more specific query to match a single task.");
            std::process::exit(1);
        }
    };

    let git_ref = match &task.git_ref {
        Some(r) => r,
        None => {
            eprintln!("Task '{}' is not linked to any branch", task.title);
            eprintln!("Use 'taskflow git link' to link it to a branch first.");
            std::process::exit(1);
        }
    };

    // Get commits
    let commits = get_branch_commits(repo_path, &git_ref.branch, limit)?;

    if commits.is_empty() {
        println!("No commits found on branch '{}'", git_ref.branch);
        return Ok(());
    }

    println!("Commits for '{}' (branch: {})", task.title, git_ref.branch);
    println!("{}", "─".repeat(60));

    for commit in &commits {
        let date = commit.timestamp.format("%Y-%m-%d %H:%M");
        println!("{} {} {}", commit.hash, date, commit.message);
        println!("  by {}", commit.author);
    }

    Ok(())
}

/// Show status of all git-linked tasks.
pub fn git_status(cli: &Cli) -> anyhow::Result<()> {
    let repo_path = env::current_dir()?;
    git_status_in(cli, &repo_path)
}

/// [`git_status`] against the repository at `repo_path`.
pub(crate) fn git_status_in(cli: &Cli, repo_path: &Path) -> anyhow::Result<()> {
    let in_git_repo = is_git_repo(repo_path);

    let model = load_model_for_cli(cli)?;

    // Find all tasks with git links
    let linked_tasks: Vec<_> = model
        .tasks
        .values()
        .filter(|t| t.git_ref.is_some())
        .collect();

    if linked_tasks.is_empty() {
        println!("No tasks are linked to git branches.");
        println!("Use 'taskflow git link <task>' to link a task to a branch.");
        return Ok(());
    }

    // Get base branch for merge detection
    let base_branch = if in_git_repo {
        get_base_branch(repo_path).ok()
    } else {
        None
    };

    println!("Git-Linked Tasks ({} total)", linked_tasks.len());
    println!("{}", "─".repeat(70));

    for task in &linked_tasks {
        let git_ref = task.git_ref.as_ref().unwrap();
        let status_icon = match task.status {
            TaskStatus::Done => "✓",
            TaskStatus::InProgress => "▶",
            TaskStatus::Blocked => "⊘",
            TaskStatus::Cancelled => "✗",
            TaskStatus::Todo => "○",
        };

        // Determine branch status
        let branch_status = linked_branch_status(
            repo_path,
            in_git_repo,
            base_branch.as_deref(),
            &git_ref.branch,
        );

        let branch_status_str = match branch_status {
            BranchStatus::Active => "(active)",
            BranchStatus::Merged => "(merged)",
            BranchStatus::Deleted => "(deleted)",
            BranchStatus::Unknown => "",
        };

        let link_type = match git_ref.link_type {
            GitLinkType::Manual => "",
            GitLinkType::AutoDetected => " [auto]",
        };

        println!(
            "{} {} → {}{} {}",
            status_icon, task.title, git_ref.branch, link_type, branch_status_str
        );
    }

    Ok(())
}

/// Classify a linked branch for `git status` output.
///
/// Outside a repository, or when no base branch could be determined, the
/// status is [`BranchStatus::Unknown`].
pub(crate) fn linked_branch_status(
    repo_path: &Path,
    in_git_repo: bool,
    base_branch: Option<&str>,
    branch: &str,
) -> BranchStatus {
    if !in_git_repo {
        return BranchStatus::Unknown;
    }
    if !branch_exists(repo_path, branch) {
        return BranchStatus::Deleted;
    }
    match base_branch {
        Some(base) => {
            if is_branch_merged(repo_path, branch, base).unwrap_or(false) {
                BranchStatus::Merged
            } else {
                BranchStatus::Active
            }
        }
        None => BranchStatus::Unknown,
    }
}

/// Auto-detect and link tasks to branches based on naming conventions.
pub fn git_sync(cli: &Cli, dry_run: bool) -> anyhow::Result<()> {
    let repo_path = env::current_dir()?;
    git_sync_in(cli, &repo_path, dry_run)
}

/// [`git_sync`] against the repository at `repo_path`.
pub(crate) fn git_sync_in(cli: &Cli, repo_path: &Path, dry_run: bool) -> anyhow::Result<()> {
    if !is_git_repo(repo_path) {
        eprintln!("Error: Not in a git repository");
        std::process::exit(1);
    }

    let mut model = load_model_for_cli(cli)?;
    let matcher = BranchMatcher::new();

    // Get current branch
    let current_branch = get_current_branch(repo_path)?;
    info!(branch = %current_branch, "Checking current branch for task matches");

    // Find unlinked, incomplete tasks
    let unlinked_tasks: Vec<_> = model
        .tasks
        .values()
        .filter(|t| t.git_ref.is_none() && !t.status.is_complete())
        .cloned()
        .collect();

    if unlinked_tasks.is_empty() {
        println!("No unlinked tasks to sync.");
        return Ok(());
    }

    // Try to match current branch to a task
    if let Some(task_id) = matcher.match_branch_to_task(&current_branch, unlinked_tasks.iter()) {
        let task = model.tasks.get(&task_id).unwrap();
        debug!(task_id = %task_id, task_title = %task.title, "Found matching task for current branch");

        if dry_run {
            println!(
                "Would link: '{}' → {} (auto-detected)",
                task.title, current_branch
            );
        } else {
            let git_ref = GitRef {
                branch: current_branch.clone(),
                remote: get_branch_remote(repo_path, &current_branch),
                linked_at: Utc::now(),
                link_type: GitLinkType::AutoDetected,
            };

            let task_title = task.title.clone();

            if let Some(task) = model.tasks.get_mut(&task_id) {
                task.git_ref = Some(git_ref);
                task.updated_at = Utc::now();
            }

            model.sync_task_by_id(&task_id);
            if let Err(e) = model.save() {
                warn!(error = %e, "Could not save after syncing git links");
                eprintln!("Warning: Could not save: {e}");
            }

            println!(
                "✓ Linked '{}' → {} (auto-detected)",
                task_title, current_branch
            );
        }
    } else {
        println!(
            "No matching task found for current branch '{}'",
            current_branch
        );
        println!("\nTips for auto-detection:");
        println!("  - Include task ID in branch name: feature/task-<id-prefix>");
        println!("  - Use task title words in branch name: fix/login-authentication");
    }

    Ok(())
}

/// Check for merged branches and auto-complete linked tasks.
pub fn git_check_merged(cli: &Cli, dry_run: bool) -> anyhow::Result<()> {
    let repo_path = env::current_dir()?;
    git_check_merged_in(cli, &repo_path, dry_run)
}

/// [`git_check_merged`] against the repository at `repo_path`.
pub(crate) fn git_check_merged_in(
    cli: &Cli,
    repo_path: &Path,
    dry_run: bool,
) -> anyhow::Result<()> {
    if !is_git_repo(repo_path) {
        eprintln!("Error: Not in a git repository");
        std::process::exit(1);
    }

    let mut model = load_model_for_cli(cli)?;

    // Get base branch
    let base_branch = get_base_branch(repo_path)?;
    info!(base = %base_branch, "Checking for merged branches");

    // Find incomplete tasks with git links
    let linked_tasks: Vec<_> = model
        .tasks
        .values()
        .filter(|t| t.git_ref.is_some() && !t.status.is_complete())
        .cloned()
        .collect();

    if linked_tasks.is_empty() {
        println!("No linked incomplete tasks to check.");
        return Ok(());
    }

    let mut completed_count = 0;

    for task in &linked_tasks {
        let git_ref = task.git_ref.as_ref().unwrap();

        // Check if branch is merged
        let is_merged = is_branch_merged(repo_path, &git_ref.branch, &base_branch)?;

        if is_merged {
            if dry_run {
                println!(
                    "Would complete: '{}' (branch '{}' merged into {})",
                    task.title, git_ref.branch, base_branch
                );
            } else {
                if let Some(t) = model.tasks.get_mut(&task.id) {
                    t.status = TaskStatus::Done;
                    t.completed_at = Some(Utc::now());
                    t.updated_at = Utc::now();
                }
                model.sync_task_by_id(&task.id);
                println!(
                    "✓ Completed '{}' (branch '{}' merged)",
                    task.title, git_ref.branch
                );
                completed_count += 1;
            }
        }
    }

    if !dry_run && completed_count > 0 {
        if let Err(e) = model.save() {
            warn!(error = %e, "Could not save after auto-completing merged tasks");
            eprintln!("Warning: Could not save: {e}");
        }
    }

    if completed_count == 0 && !dry_run {
        println!("No merged branches found for linked tasks.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use chrono::Utc;
    use tempfile::TempDir;

    use taskflow::app::Model;
    use taskflow::domain::git::{BranchStatus, GitLinkType, GitRef};
    use taskflow::domain::{Task, TaskStatus};

    use super::{
        git_check_merged_in, git_commits_in, git_link_in, git_status_in, git_sync_in, git_unlink,
        linked_branch_status,
    };
    use crate::commands_tests::support::{
        add_task, assert_exit_one, commit_file, data_file, git, git_available, init_repo,
        open_model, run_exit_case, seed_model, temp_dir, yaml_cli,
    };

    /// A temp directory holding both a git repository (`repo/`) and the
    /// YAML data file, or `None` when git cannot run on this machine.
    struct Fixture {
        _dir: TempDir,
        repo: std::path::PathBuf,
        data: std::path::PathBuf,
    }

    fn fixture() -> Option<Fixture> {
        if !git_available() {
            eprintln!("git binary not available; skipping");
            return None;
        }
        let dir = temp_dir();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_repo(&repo);
        let data = data_file(dir.path());
        Some(Fixture {
            _dir: dir,
            repo,
            data,
        })
    }

    fn manual_ref(branch: &str) -> GitRef {
        GitRef {
            branch: branch.to_string(),
            remote: None,
            linked_at: Utc::now(),
            link_type: GitLinkType::Manual,
        }
    }

    fn task_named<'a>(model: &'a Model, title: &str) -> &'a Task {
        model
            .tasks
            .values()
            .find(|t| t.title == title)
            .unwrap_or_else(|| panic!("task {title} exists"))
    }

    /// Create `branch` from the current HEAD with one commit, then return to `main`.
    fn feature_branch(repo: &Path, branch: &str, merge: bool) {
        git(repo, &["checkout", "-q", "-b", branch]);
        commit_file(
            repo,
            &format!("{}.txt", branch.replace('/', "_")),
            "work\n",
            &format!("Work on {branch}"),
        );
        git(repo, &["checkout", "-q", "main"]);
        if merge {
            git(repo, &["merge", "-q", "--ff-only", branch]);
        }
    }

    // ------------------------------------------------------------------
    // link / unlink
    // ------------------------------------------------------------------

    #[test]
    fn link_records_current_branch_on_the_single_match() {
        let Some(f) = fixture() else { return };
        seed_model(&f.data, |model| {
            add_task(model, Task::new("Implement login"));
            add_task(model, Task::new("Unrelated"));
        });

        git_link_in(&yaml_cli(&f.data), &f.repo, "LOGIN", None).expect("link succeeds");

        let model = open_model(&f.data);
        let git_ref = task_named(&model, "Implement login")
            .git_ref
            .as_ref()
            .expect("link saved");
        assert_eq!(git_ref.branch, "main");
        assert_eq!(git_ref.link_type, GitLinkType::Manual);
        assert_eq!(git_ref.remote, None);
        assert!(task_named(&model, "Unrelated").git_ref.is_none());
    }

    #[test]
    fn link_accepts_an_explicit_existing_branch() {
        let Some(f) = fixture() else { return };
        git(&f.repo, &["branch", "feature/login"]);
        seed_model(&f.data, |model| {
            add_task(model, Task::new("Implement login"));
        });

        git_link_in(&yaml_cli(&f.data), &f.repo, "login", Some("feature/login"))
            .expect("link succeeds");

        let model = open_model(&f.data);
        let git_ref = task_named(&model, "Implement login")
            .git_ref
            .as_ref()
            .unwrap();
        assert_eq!(git_ref.branch, "feature/login");
    }

    #[test]
    fn link_exit_paths() {
        let Some(f) = fixture() else { return };
        seed_model(&f.data, |model| {
            add_task(model, Task::new("Implement login"));
            add_task(model, Task::new("Fix bug A"));
            add_task(model, Task::new("Fix bug B"));
        });
        let not_a_repo = f._dir.path().join("plain");
        std::fs::create_dir_all(&not_a_repo).unwrap();

        let text = assert_exit_one(&run_exit_case(
            "git_link_empty_query",
            &f.data,
            Some(&f.repo),
        ));
        assert!(text.contains("Error: Task query cannot be empty"));

        let text = assert_exit_one(&run_exit_case(
            "git_link_not_repo",
            &f.data,
            Some(&not_a_repo),
        ));
        assert!(text.contains("Error: Not in a git repository"));

        let text = assert_exit_one(&run_exit_case(
            "git_link_missing_branch",
            &f.data,
            Some(&f.repo),
        ));
        assert!(text.contains("Error: Branch 'no-such-branch' does not exist"));

        let text = assert_exit_one(&run_exit_case("git_link_no_match", &f.data, Some(&f.repo)));
        assert!(text.contains("No tasks found matching: \"nonexistent\""));

        let text = assert_exit_one(&run_exit_case("git_link_multiple", &f.data, Some(&f.repo)));
        assert!(text.contains("Multiple tasks match 'bug' (2 found):"));
        assert!(text.contains("Use a more specific query to match a single task."));

        let model = open_model(&f.data);
        assert!(model.tasks.values().all(|t| t.git_ref.is_none()));
    }

    #[test]
    fn unlink_removes_the_link() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        seed_model(&data, |model| {
            add_task(
                model,
                Task::new("Implement login").with_git_ref(manual_ref("feature/login")),
            );
        });

        git_unlink(&yaml_cli(&data), "login").expect("unlink succeeds");

        let model = open_model(&data);
        assert!(task_named(&model, "Implement login").git_ref.is_none());
    }

    #[test]
    fn unlink_exit_paths() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        seed_model(&data, |model| {
            add_task(model, Task::new("Implement login"));
            add_task(
                model,
                Task::new("Fix bug A").with_git_ref(manual_ref("feature/a")),
            );
            add_task(model, Task::new("Fix bug B"));
        });

        let text = assert_exit_one(&run_exit_case("git_unlink_empty_query", &data, None));
        assert!(text.contains("Usage: taskflow git unlink <task-query>"));

        let text = assert_exit_one(&run_exit_case("git_unlink_no_match", &data, None));
        assert!(text.contains("No tasks found matching: \"nonexistent\""));

        let text = assert_exit_one(&run_exit_case("git_unlink_multiple", &data, None));
        assert!(text.contains("Multiple tasks match 'bug' (2 found):"));
        assert!(text.contains("Fix bug A → feature/a"));

        let text = assert_exit_one(&run_exit_case("git_unlink_not_linked", &data, None));
        assert!(text.contains("Task 'Implement login' is not linked to any branch"));

        let model = open_model(&data);
        assert!(task_named(&model, "Fix bug A").git_ref.is_some());
    }

    // ------------------------------------------------------------------
    // commits
    // ------------------------------------------------------------------

    #[test]
    fn commits_lists_history_of_the_linked_branch() {
        let Some(f) = fixture() else { return };
        commit_file(&f.repo, "second.txt", "2\n", "Second commit");
        seed_model(&f.data, |model| {
            add_task(
                model,
                Task::new("Implement login").with_git_ref(manual_ref("main")),
            );
        });

        git_commits_in(&yaml_cli(&f.data), &f.repo, "login", 10).expect("commits succeed");
        git_commits_in(&yaml_cli(&f.data), &f.repo, "login", 1).expect("limit honoured");
    }

    #[test]
    fn commits_fails_when_branch_is_gone() {
        let Some(f) = fixture() else { return };
        seed_model(&f.data, |model| {
            add_task(
                model,
                Task::new("Implement login").with_git_ref(manual_ref("deleted/branch")),
            );
        });

        let err = git_commits_in(&yaml_cli(&f.data), &f.repo, "login", 10).unwrap_err();
        assert!(err.to_string().contains("git log failed"), "{err}");
    }

    #[test]
    fn commits_exit_paths() {
        let Some(f) = fixture() else { return };
        seed_model(&f.data, |model| {
            add_task(model, Task::new("Implement login"));
            add_task(model, Task::new("Fix bug A"));
            add_task(model, Task::new("Fix bug B"));
        });
        let not_a_repo = f._dir.path().join("plain");
        std::fs::create_dir_all(&not_a_repo).unwrap();

        let text = assert_exit_one(&run_exit_case(
            "git_commits_empty_query",
            &f.data,
            Some(&f.repo),
        ));
        assert!(text.contains("Usage: taskflow git commits <task-query> [--limit <n>]"));

        let text = assert_exit_one(&run_exit_case(
            "git_commits_not_repo",
            &f.data,
            Some(&not_a_repo),
        ));
        assert!(text.contains("Error: Not in a git repository"));

        let text = assert_exit_one(&run_exit_case(
            "git_commits_no_match",
            &f.data,
            Some(&f.repo),
        ));
        assert!(text.contains("No tasks found matching: \"nonexistent\""));

        let text = assert_exit_one(&run_exit_case(
            "git_commits_multiple",
            &f.data,
            Some(&f.repo),
        ));
        assert!(text.contains("Multiple tasks match 'bug' (2 found):"));

        let text = assert_exit_one(&run_exit_case(
            "git_commits_not_linked",
            &f.data,
            Some(&f.repo),
        ));
        assert!(text.contains("Task 'Implement login' is not linked to any branch"));
        assert!(text.contains("Use 'taskflow git link' to link it to a branch first."));
    }

    // ------------------------------------------------------------------
    // status
    // ------------------------------------------------------------------

    #[test]
    fn linked_branch_status_classifies_branches() {
        let Some(f) = fixture() else { return };
        feature_branch(&f.repo, "feature/merged", true);
        feature_branch(&f.repo, "feature/open", false);

        assert_eq!(
            linked_branch_status(&f.repo, false, Some("main"), "main"),
            BranchStatus::Unknown,
            "outside a repository nothing can be determined"
        );
        assert_eq!(
            linked_branch_status(&f.repo, true, Some("main"), "feature/gone"),
            BranchStatus::Deleted
        );
        assert_eq!(
            linked_branch_status(&f.repo, true, Some("main"), "feature/merged"),
            BranchStatus::Merged
        );
        assert_eq!(
            linked_branch_status(&f.repo, true, Some("main"), "feature/open"),
            BranchStatus::Active
        );
        assert_eq!(
            linked_branch_status(&f.repo, true, None, "feature/open"),
            BranchStatus::Unknown,
            "without a base branch merge state is unknown"
        );
    }

    #[test]
    fn status_runs_with_and_without_links_and_repository() {
        let Some(f) = fixture() else { return };
        feature_branch(&f.repo, "feature/merged", true);

        seed_model(&f.data, |model| {
            add_task(model, Task::new("Unlinked"));
        });
        git_status_in(&yaml_cli(&f.data), &f.repo).expect("no links");

        seed_model(&f.data, |model| {
            add_task(
                model,
                Task::new("Merged work")
                    .with_status(TaskStatus::Done)
                    .with_git_ref(manual_ref("feature/merged")),
            );
            let mut auto = manual_ref("main");
            auto.link_type = GitLinkType::AutoDetected;
            add_task(
                model,
                Task::new("Active work")
                    .with_status(TaskStatus::InProgress)
                    .with_git_ref(auto),
            );
            add_task(
                model,
                Task::new("Lost work")
                    .with_status(TaskStatus::Blocked)
                    .with_git_ref(manual_ref("feature/deleted")),
            );
            add_task(
                model,
                Task::new("Dropped work")
                    .with_status(TaskStatus::Cancelled)
                    .with_git_ref(manual_ref("feature/deleted")),
            );
        });
        git_status_in(&yaml_cli(&f.data), &f.repo).expect("with links");

        let not_a_repo = f._dir.path().join("plain");
        std::fs::create_dir_all(&not_a_repo).unwrap();
        git_status_in(&yaml_cli(&f.data), &not_a_repo).expect("outside a repository");
    }

    // ------------------------------------------------------------------
    // sync
    // ------------------------------------------------------------------

    #[test]
    fn sync_links_task_matching_the_current_branch_title() {
        let Some(f) = fixture() else { return };
        git(
            &f.repo,
            &["checkout", "-q", "-b", "feature/fix-login-authentication"],
        );
        seed_model(&f.data, |model| {
            add_task(model, Task::new("Fix login authentication"));
            add_task(model, Task::new("Write release notes"));
        });
        let cli = yaml_cli(&f.data);

        git_sync_in(&cli, &f.repo, true).expect("dry run succeeds");
        let model = open_model(&f.data);
        assert!(
            task_named(&model, "Fix login authentication")
                .git_ref
                .is_none(),
            "dry run must not save"
        );

        git_sync_in(&cli, &f.repo, false).expect("sync succeeds");
        let model = open_model(&f.data);
        let git_ref = task_named(&model, "Fix login authentication")
            .git_ref
            .as_ref()
            .expect("auto link saved");
        assert_eq!(git_ref.branch, "feature/fix-login-authentication");
        assert_eq!(git_ref.link_type, GitLinkType::AutoDetected);
        assert!(task_named(&model, "Write release notes").git_ref.is_none());
    }

    #[test]
    fn sync_links_task_by_id_prefix_in_branch_name() {
        let Some(f) = fixture() else { return };
        let task = Task::new("Totally unrelated title");
        let prefix: String = task.id.to_string().chars().take(8).collect();
        git(
            &f.repo,
            &["checkout", "-q", "-b", &format!("task-{prefix}")],
        );
        seed_model(&f.data, |model| {
            add_task(model, task);
        });

        git_sync_in(&yaml_cli(&f.data), &f.repo, false).expect("sync succeeds");

        let model = open_model(&f.data);
        let git_ref = task_named(&model, "Totally unrelated title")
            .git_ref
            .as_ref()
            .unwrap();
        assert_eq!(git_ref.branch, format!("task-{prefix}"));
    }

    #[test]
    fn sync_reports_when_nothing_matches_or_nothing_is_unlinked() {
        let Some(f) = fixture() else { return };
        git(&f.repo, &["checkout", "-q", "-b", "chore/zzz-qqq"]);
        seed_model(&f.data, |model| {
            add_task(model, Task::new("Write docs"));
        });
        git_sync_in(&yaml_cli(&f.data), &f.repo, false).expect("no match is not an error");
        let model = open_model(&f.data);
        assert!(task_named(&model, "Write docs").git_ref.is_none());

        seed_model(&f.data, |model| {
            add_task(
                model,
                Task::new("Linked").with_git_ref(manual_ref("chore/zzz-qqq")),
            );
            add_task(model, Task::new("Done").with_status(TaskStatus::Done));
            let id = task_named(model, "Write docs").id;
            model.tasks.remove(&id);
            model.delete_task_from_storage(&id);
        });
        git_sync_in(&yaml_cli(&f.data), &f.repo, false).expect("nothing to sync");
    }

    #[test]
    fn sync_and_check_merged_exit_outside_a_repository() {
        if !git_available() {
            return;
        }
        let dir = temp_dir();
        let data = data_file(dir.path());
        seed_model(&data, |_| {});

        let text = assert_exit_one(&run_exit_case("git_sync_not_repo", &data, Some(dir.path())));
        assert!(text.contains("Error: Not in a git repository"));

        let text = assert_exit_one(&run_exit_case(
            "git_check_merged_not_repo",
            &data,
            Some(dir.path()),
        ));
        assert!(text.contains("Error: Not in a git repository"));
    }

    // ------------------------------------------------------------------
    // check-merged
    // ------------------------------------------------------------------

    #[test]
    fn check_merged_completes_tasks_whose_branch_was_merged() {
        let Some(f) = fixture() else { return };
        feature_branch(&f.repo, "feature/merged", true);
        feature_branch(&f.repo, "feature/open", false);
        seed_model(&f.data, |model| {
            add_task(
                model,
                Task::new("Merged work").with_git_ref(manual_ref("feature/merged")),
            );
            add_task(
                model,
                Task::new("Open work").with_git_ref(manual_ref("feature/open")),
            );
            add_task(
                model,
                Task::new("Already done")
                    .with_status(TaskStatus::Done)
                    .with_git_ref(manual_ref("feature/merged")),
            );
        });
        let cli = yaml_cli(&f.data);

        git_check_merged_in(&cli, &f.repo, true).expect("dry run succeeds");
        let model = open_model(&f.data);
        assert_eq!(task_named(&model, "Merged work").status, TaskStatus::Todo);

        git_check_merged_in(&cli, &f.repo, false).expect("check succeeds");
        let model = open_model(&f.data);
        let merged = task_named(&model, "Merged work");
        assert_eq!(merged.status, TaskStatus::Done);
        assert!(merged.completed_at.is_some());
        assert_eq!(task_named(&model, "Open work").status, TaskStatus::Todo);

        // Nothing left to complete: both remaining branches are reported as unmerged.
        git_check_merged_in(&cli, &f.repo, false).expect("second run succeeds");
    }

    #[test]
    fn check_merged_with_no_linked_tasks() {
        let Some(f) = fixture() else { return };
        seed_model(&f.data, |model| {
            add_task(model, Task::new("Unlinked"));
        });
        git_check_merged_in(&yaml_cli(&f.data), &f.repo, false).expect("nothing to check");
    }

    #[test]
    fn check_merged_fails_when_a_linked_branch_is_missing() {
        let Some(f) = fixture() else { return };
        seed_model(&f.data, |model| {
            add_task(
                model,
                Task::new("Lost work").with_git_ref(manual_ref("feature/deleted")),
            );
        });
        // `git branch --merged main` succeeds even when the linked branch is
        // gone; the task is simply not completed.
        git_check_merged_in(&yaml_cli(&f.data), &f.repo, false).expect("missing branch tolerated");
        let model = open_model(&f.data);
        assert_eq!(task_named(&model, "Lost work").status, TaskStatus::Todo);
    }
}
