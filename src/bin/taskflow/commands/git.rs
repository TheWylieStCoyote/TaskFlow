//! Git integration CLI commands.

use std::env;

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
    let query = task_query.to_lowercase();
    if query.trim().is_empty() {
        eprintln!("Error: Task query cannot be empty");
        eprintln!("Usage: taskflow git link <task-query> [--branch <branch>]");
        std::process::exit(1);
    }

    // Determine repository path
    let repo_path = env::current_dir()?;
    if !is_git_repo(&repo_path) {
        eprintln!("Error: Not in a git repository");
        std::process::exit(1);
    }

    // Determine branch to link
    let branch_name = match branch {
        Some(b) => {
            if !branch_exists(&repo_path, b) {
                eprintln!("Error: Branch '{}' does not exist", b);
                std::process::exit(1);
            }
            b.to_string()
        }
        None => get_current_branch(&repo_path)?,
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
        remote: get_branch_remote(&repo_path, &branch_name),
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
    let query = task_query.to_lowercase();
    if query.trim().is_empty() {
        eprintln!("Error: Task query cannot be empty");
        eprintln!("Usage: taskflow git commits <task-query> [--limit <n>]");
        std::process::exit(1);
    }

    let repo_path = env::current_dir()?;
    if !is_git_repo(&repo_path) {
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
    let commits = get_branch_commits(&repo_path, &git_ref.branch, limit)?;

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
    let in_git_repo = is_git_repo(&repo_path);

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
        get_base_branch(&repo_path).ok()
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
        let branch_status = if in_git_repo {
            if !branch_exists(&repo_path, &git_ref.branch) {
                BranchStatus::Deleted
            } else if let Some(ref base) = base_branch {
                if is_branch_merged(&repo_path, &git_ref.branch, base).unwrap_or(false) {
                    BranchStatus::Merged
                } else {
                    BranchStatus::Active
                }
            } else {
                BranchStatus::Unknown
            }
        } else {
            BranchStatus::Unknown
        };

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

/// Auto-detect and link tasks to branches based on naming conventions.
pub fn git_sync(cli: &Cli, dry_run: bool) -> anyhow::Result<()> {
    let repo_path = env::current_dir()?;
    if !is_git_repo(&repo_path) {
        eprintln!("Error: Not in a git repository");
        std::process::exit(1);
    }

    let mut model = load_model_for_cli(cli)?;
    let matcher = BranchMatcher::new();

    // Get current branch
    let current_branch = get_current_branch(&repo_path)?;
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
                remote: get_branch_remote(&repo_path, &current_branch),
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
    if !is_git_repo(&repo_path) {
        eprintln!("Error: Not in a git repository");
        std::process::exit(1);
    }

    let mut model = load_model_for_cli(cli)?;

    // Get base branch
    let base_branch = get_base_branch(&repo_path)?;
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
        let is_merged = is_branch_merged(&repo_path, &git_ref.branch, &base_branch)?;

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
