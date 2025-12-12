//! Git CLI wrapper operations.
//!
//! This module provides functions that wrap git CLI commands
//! for common operations like getting the current branch,
//! listing commits, and checking merge status.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

use super::GitCommit;

/// Get the current branch name.
///
/// # Errors
///
/// Returns an error if:
/// - Not in a git repository
/// - Git command fails
/// - In detached HEAD state (no branch name)
pub fn get_current_branch(repo_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args([
            "-C",
            &repo_path.to_string_lossy(),
            "branch",
            "--show-current",
        ])
        .output()
        .context("Failed to execute git branch")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git branch failed: {}", stderr.trim());
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        anyhow::bail!("Not on a branch (detached HEAD state)");
    }

    Ok(branch)
}

/// Get the remote for a branch (defaults to None if not configured).
#[must_use]
pub fn get_branch_remote(repo_path: &Path, branch: &str) -> Option<String> {
    let output = Command::new("git")
        .args([
            "-C",
            &repo_path.to_string_lossy(),
            "config",
            &format!("branch.{branch}.remote"),
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let remote = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if remote.is_empty() {
        None
    } else {
        Some(remote)
    }
}

/// List all local branches.
///
/// # Errors
///
/// Returns an error if not in a git repository or git command fails.
pub fn list_branches(repo_path: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args([
            "-C",
            &repo_path.to_string_lossy(),
            "branch",
            "--format=%(refname:short)",
        ])
        .output()
        .context("Failed to execute git branch")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git branch failed: {}", stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// Get commits for a branch (limited to `limit` commits).
///
/// Returns commits in reverse chronological order (newest first).
///
/// # Errors
///
/// Returns an error if the branch doesn't exist or git command fails.
pub fn get_branch_commits(repo_path: &Path, branch: &str, limit: usize) -> Result<Vec<GitCommit>> {
    let output = Command::new("git")
        .args([
            "-C",
            &repo_path.to_string_lossy(),
            "log",
            branch,
            &format!("-n{limit}"),
            "--format=%h|%H|%s|%an|%aI",
        ])
        .output()
        .context("Failed to execute git log")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git log failed: {}", stderr.trim());
    }

    let mut commits = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let parts: Vec<&str> = line.splitn(5, '|').collect();
        if parts.len() >= 5 {
            let timestamp = DateTime::parse_from_rfc3339(parts[4])
                .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));

            commits.push(GitCommit {
                hash: parts[0].to_string(),
                full_hash: parts[1].to_string(),
                message: parts[2].to_string(),
                author: parts[3].to_string(),
                timestamp,
            });
        }
    }

    Ok(commits)
}

/// Check if a branch is merged into the base branch.
///
/// # Errors
///
/// Returns an error if git command fails.
pub fn is_branch_merged(repo_path: &Path, branch: &str, base: &str) -> Result<bool> {
    let output = Command::new("git")
        .args([
            "-C",
            &repo_path.to_string_lossy(),
            "branch",
            "--merged",
            base,
        ])
        .output()
        .context("Failed to execute git branch --merged")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git branch --merged failed: {}", stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.trim().trim_start_matches("* ") == branch))
}

/// Get the default base branch (main or master).
///
/// Tries "main" first, then falls back to "master".
///
/// # Errors
///
/// Returns an error if neither main nor master branch exists.
pub fn get_base_branch(repo_path: &Path) -> Result<String> {
    for branch in ["main", "master"] {
        let output = Command::new("git")
            .args([
                "-C",
                &repo_path.to_string_lossy(),
                "rev-parse",
                "--verify",
                branch,
            ])
            .output()
            .context("Failed to execute git rev-parse")?;

        if output.status.success() {
            return Ok(branch.to_string());
        }
    }

    anyhow::bail!("Could not find main or master branch")
}

/// Check if a branch exists (locally).
#[must_use]
pub fn branch_exists(repo_path: &Path, branch: &str) -> bool {
    Command::new("git")
        .args([
            "-C",
            &repo_path.to_string_lossy(),
            "rev-parse",
            "--verify",
            branch,
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if the given path is inside a git repository.
#[must_use]
pub fn is_git_repo(path: &Path) -> bool {
    Command::new("git")
        .args(["-C", &path.to_string_lossy(), "rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the root directory of the git repository.
///
/// # Errors
///
/// Returns an error if not in a git repository.
pub fn get_repo_root(path: &Path) -> Result<std::path::PathBuf> {
    let output = Command::new("git")
        .args([
            "-C",
            &path.to_string_lossy(),
            "rev-parse",
            "--show-toplevel",
        ])
        .output()
        .context("Failed to execute git rev-parse")?;

    if !output.status.success() {
        anyhow::bail!("Not in a git repository");
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(std::path::PathBuf::from(root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_test_repo() -> TempDir {
        let dir = TempDir::new().unwrap();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // Configure git user (required for commits)
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // Create initial commit
        std::fs::write(dir.path().join("README.md"), "# Test\n").unwrap();

        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // Rename default branch to main
        Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        dir
    }

    #[test]
    fn test_is_git_repo() {
        let repo = setup_test_repo();
        assert!(is_git_repo(repo.path()));

        let non_repo = TempDir::new().unwrap();
        assert!(!is_git_repo(non_repo.path()));
    }

    #[test]
    fn test_get_current_branch() {
        let repo = setup_test_repo();
        let branch = get_current_branch(repo.path()).unwrap();
        assert_eq!(branch, "main");
    }

    #[test]
    fn test_get_base_branch() {
        let repo = setup_test_repo();
        let base = get_base_branch(repo.path()).unwrap();
        assert_eq!(base, "main");
    }

    #[test]
    fn test_branch_exists() {
        let repo = setup_test_repo();
        assert!(branch_exists(repo.path(), "main"));
        assert!(!branch_exists(repo.path(), "nonexistent-branch"));
    }

    #[test]
    fn test_list_branches() {
        let repo = setup_test_repo();

        // Create a feature branch
        Command::new("git")
            .args(["checkout", "-b", "feature/test"])
            .current_dir(repo.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(repo.path())
            .output()
            .unwrap();

        let branches = list_branches(repo.path()).unwrap();
        assert!(branches.contains(&"main".to_string()));
        assert!(branches.contains(&"feature/test".to_string()));
    }

    #[test]
    fn test_get_branch_commits() {
        let repo = setup_test_repo();

        // Add another commit
        std::fs::write(repo.path().join("file.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Add file"])
            .current_dir(repo.path())
            .output()
            .unwrap();

        let commits = get_branch_commits(repo.path(), "main", 10).unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].message, "Add file");
        assert_eq!(commits[1].message, "Initial commit");
    }

    #[test]
    fn test_is_branch_merged() {
        let repo = setup_test_repo();

        // Create and merge a feature branch
        Command::new("git")
            .args(["checkout", "-b", "feature/merged"])
            .current_dir(repo.path())
            .output()
            .unwrap();

        std::fs::write(repo.path().join("feature.txt"), "feature").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Feature commit"])
            .current_dir(repo.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(repo.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["merge", "feature/merged"])
            .current_dir(repo.path())
            .output()
            .unwrap();

        assert!(is_branch_merged(repo.path(), "feature/merged", "main").unwrap());

        // Create unmerged branch
        Command::new("git")
            .args(["checkout", "-b", "feature/unmerged"])
            .current_dir(repo.path())
            .output()
            .unwrap();

        std::fs::write(repo.path().join("unmerged.txt"), "unmerged").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Unmerged commit"])
            .current_dir(repo.path())
            .output()
            .unwrap();

        assert!(!is_branch_merged(repo.path(), "feature/unmerged", "main").unwrap());
    }

    #[test]
    fn test_get_repo_root() {
        let repo = setup_test_repo();
        let subdir = repo.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        let root = get_repo_root(&subdir).unwrap();
        assert_eq!(root, repo.path());
    }
}
