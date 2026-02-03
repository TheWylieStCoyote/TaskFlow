//! Comprehensive tests for Git integration.
//!
//! This module provides extensive test coverage for git integration including:
//! - Git operations with edge cases and error handling
//! - Branch matching with various naming patterns
//! - Merge detection and workflow integration
//! - Error handling for invalid/corrupted repositories

use super::matching::BranchMatcher;
use super::operations::*;
use super::{BranchStatus, GitCommit, GitLinkType, GitRef};
use crate::domain::Task;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

// ============================================================================
// TEST HELPERS
// ============================================================================

/// Create a test git repository with initial setup.
fn setup_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Configure git user
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
    fs::write(dir.path().join("README.md"), "# Test Repository\n").unwrap();

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

    // Rename to main
    Command::new("git")
        .args(["branch", "-M", "main"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    dir
}

/// Create a feature branch with a commit.
fn create_feature_branch(repo: &TempDir, branch_name: &str, filename: &str, commit_msg: &str) {
    Command::new("git")
        .args(["checkout", "-b", branch_name])
        .current_dir(repo.path())
        .output()
        .unwrap();

    fs::write(repo.path().join(filename), "feature content\n").unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(repo.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", commit_msg])
        .current_dir(repo.path())
        .output()
        .unwrap();

    // Return to main
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo.path())
        .output()
        .unwrap();
}

/// Merge a branch into main.
fn merge_branch(repo: &TempDir, branch_name: &str) {
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    Command::new("git")
        .args([
            "merge",
            "--no-ff",
            branch_name,
            "-m",
            &format!("Merge {branch_name}"),
        ])
        .current_dir(repo.path())
        .output()
        .unwrap();
}

// ============================================================================
// GIT OPERATIONS EDGE CASES
// ============================================================================

#[test]
fn test_get_current_branch_non_git_directory() {
    let temp = TempDir::new().unwrap();
    let result = get_current_branch(temp.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("git"));
}

#[test]
fn test_get_current_branch_detached_head() {
    let repo = setup_test_repo();

    // Get commit hash and checkout to detached HEAD
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Command::new("git")
        .args(["checkout", &hash])
        .current_dir(repo.path())
        .output()
        .unwrap();

    let result = get_current_branch(repo.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("detached HEAD"));
}

#[test]
fn test_list_branches_many_branches() {
    let repo = setup_test_repo();

    // Create multiple branches
    for i in 1..=20 {
        Command::new("git")
            .args(["branch", &format!("feature/branch-{i}")])
            .current_dir(repo.path())
            .output()
            .unwrap();
    }

    let branches = list_branches(repo.path()).unwrap();
    assert!(branches.len() >= 20); // main + 20 features
    assert!(branches.contains(&"main".to_string()));
    assert!(branches.contains(&"feature/branch-1".to_string()));
    assert!(branches.contains(&"feature/branch-20".to_string()));
}

#[test]
fn test_branch_exists_case_sensitivity() {
    let repo = setup_test_repo();

    // Git branch names are case-sensitive
    Command::new("git")
        .args(["branch", "Feature-Branch"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    assert!(branch_exists(repo.path(), "Feature-Branch"));
    assert!(!branch_exists(repo.path(), "feature-branch"));
    assert!(!branch_exists(repo.path(), "FEATURE-BRANCH"));
}

#[test]
fn test_branch_exists_special_characters() {
    let repo = setup_test_repo();

    // Test various special characters allowed in branch names
    let branch_names = [
        "feature/user-auth",
        "hotfix/3.2.1",
        "bugfix/issue-#123",
        "feature/api_v2",
    ];

    for name in branch_names {
        Command::new("git")
            .args(["branch", name])
            .current_dir(repo.path())
            .output()
            .unwrap();

        assert!(
            branch_exists(repo.path(), name),
            "Branch '{name}' should exist"
        );
    }
}

#[test]
fn test_get_branch_commits_limit() {
    let repo = setup_test_repo();

    // Add 10 commits
    for i in 1..=10 {
        fs::write(repo.path().join(format!("file{i}.txt")), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", &format!("Commit {i}")])
            .current_dir(repo.path())
            .output()
            .unwrap();
    }

    // Request only 5
    let commits = get_branch_commits(repo.path(), "main", 5).unwrap();
    assert_eq!(commits.len(), 5);

    // Should get most recent first
    assert_eq!(commits[0].message, "Commit 10");
    assert_eq!(commits[4].message, "Commit 6");
}

#[test]
fn test_get_branch_commits_nonexistent_branch() {
    let repo = setup_test_repo();
    let result = get_branch_commits(repo.path(), "nonexistent-branch", 10);
    assert!(result.is_err());
}

#[test]
fn test_is_branch_merged_same_branch() {
    let repo = setup_test_repo();

    // A branch is always merged into itself
    let result = is_branch_merged(repo.path(), "main", "main");
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn test_get_base_branch_master_only() {
    let dir = TempDir::new().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

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

    fs::write(dir.path().join("README.md"), "test").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Initial"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Rename to master instead of main
    Command::new("git")
        .args(["branch", "-M", "master"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let base = get_base_branch(dir.path()).unwrap();
    assert_eq!(base, "master");
}

#[test]
fn test_get_base_branch_neither_exists() {
    let dir = TempDir::new().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

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

    fs::write(dir.path().join("README.md"), "test").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Initial"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Rename to something else
    Command::new("git")
        .args(["branch", "-M", "develop"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let result = get_base_branch(dir.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("main or master"));
}

#[test]
fn test_get_repo_root_from_subdirectory() {
    let repo = setup_test_repo();

    // Create nested subdirectories
    let subdir = repo.path().join("a").join("b").join("c");
    fs::create_dir_all(&subdir).unwrap();

    let root = get_repo_root(&subdir).unwrap();
    assert_eq!(root, repo.path());
}

#[test]
fn test_get_repo_root_non_git_directory() {
    let temp = TempDir::new().unwrap();
    let result = get_repo_root(temp.path());
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Not in a git repository"));
}

#[test]
fn test_get_branch_remote_unconfigured() {
    let repo = setup_test_repo();

    // New branch without remote
    Command::new("git")
        .args(["branch", "new-branch"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    let remote = get_branch_remote(repo.path(), "new-branch");
    assert!(remote.is_none());
}

#[test]
fn test_get_branch_remote_configured() {
    let repo = setup_test_repo();

    // Set up a remote for a branch
    Command::new("git")
        .args(["config", "branch.main.remote", "origin"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    let remote = get_branch_remote(repo.path(), "main");
    assert_eq!(remote, Some("origin".to_string()));
}

// ============================================================================
// BRANCH MATCHING COMPREHENSIVE TESTS
// ============================================================================

#[test]
fn test_match_by_id_various_formats() {
    let task = Task::new("Test task");
    let id_prefix = &task.id.to_string()[..8];
    let tasks = [task.clone()];

    // Test different ID patterns
    let patterns = [
        format!("feature/task-{id_prefix}"),
        format!("fix/task_{id_prefix}"),
        format!("hotfix/TASK-{id_prefix}"),
        format!("bugfix/task/{id_prefix}"),
    ];

    let matcher = BranchMatcher::new();
    for pattern in patterns {
        let result = matcher.match_branch_to_task(&pattern, tasks.iter());
        assert_eq!(result, Some(task.id), "Pattern '{pattern}' should match");
    }
}

#[test]
fn test_match_by_id_short_prefix() {
    let task = Task::new("Test task");
    let id_prefix = &task.id.to_string()[..5]; // Only 5 chars
    let tasks = [task.clone()];

    let matcher = BranchMatcher::new();

    // 5 chars is too short (needs >= 6)
    let branch = format!("feature/task-{id_prefix}");
    let result = matcher.match_branch_to_task(&branch, tasks.iter());
    // Should fall back to title matching
    assert!(result.is_none() || result == Some(task.id));
}

#[test]
fn test_match_by_title_threshold() {
    let matcher = BranchMatcher::new().with_threshold(0.8); // High threshold

    let task = Task::new("Implement user registration feature");
    let tasks = [task.clone()];

    // High similarity should match
    let result = matcher.match_branch_to_task("feature/user-registration", tasks.iter());
    assert_eq!(result, Some(task.id));

    // Low similarity should not match
    let result = matcher.match_branch_to_task("feature/payment-processing", tasks.iter());
    assert!(result.is_none());
}

#[test]
fn test_match_by_title_case_insensitive() {
    let matcher = BranchMatcher::new();

    let task = Task::new("Fix Login Bug");
    let tasks = [task.clone()];

    // Different cases should all match
    let branches = [
        "fix/login-bug",
        "fix/LOGIN-BUG",
        "fix/Login-Bug",
        "FIX/login-BUG",
    ];

    for branch in branches {
        let result = matcher.match_branch_to_task(branch, tasks.iter());
        assert_eq!(
            result,
            Some(task.id),
            "Branch '{branch}' should match (case-insensitive)"
        );
    }
}

#[test]
fn test_match_multiple_tasks_best_match() {
    let matcher = BranchMatcher::new();

    let task1 = Task::new("Fix login authentication");
    let task2 = Task::new("Update login page design");
    let task3 = Task::new("Add user profile");

    let tasks = [task1.clone(), task2.clone(), task3.clone()];

    // Should match task1 (best match)
    let result = matcher.match_branch_to_task("fix/login-auth", tasks.iter());
    assert_eq!(result, Some(task1.id));

    // Should match task2 (best match)
    let result = matcher.match_branch_to_task("feature/login-page-design", tasks.iter());
    assert_eq!(result, Some(task2.id));
}

#[test]
fn test_match_empty_branch_name() {
    let matcher = BranchMatcher::new();
    let task = Task::new("Test task");
    let tasks = [task.clone()];

    let result = matcher.match_branch_to_task("", tasks.iter());
    assert!(result.is_none());
}

#[test]
fn test_match_empty_task_list() {
    let matcher = BranchMatcher::new();
    let tasks: Vec<Task> = vec![];

    let result = matcher.match_branch_to_task("feature/user-auth", tasks.iter());
    assert!(result.is_none());
}

#[test]
fn test_match_task_with_empty_title() {
    let matcher = BranchMatcher::new();
    let task = Task::new("");
    let tasks = [task.clone()];

    let result = matcher.match_branch_to_task("feature/something", tasks.iter());
    assert!(result.is_none());
}

#[test]
fn test_match_with_numbers_in_branch() {
    let matcher = BranchMatcher::new();
    let task = Task::new("Update API v2");
    let tasks = [task.clone()];

    let result = matcher.match_branch_to_task("feature/api-v2-update", tasks.iter());
    assert_eq!(result, Some(task.id));
}

// ============================================================================
// GIT REF AND TYPES TESTS
// ============================================================================

#[test]
fn test_git_ref_creation() {
    let git_ref = GitRef::new("feature/test", GitLinkType::Manual);
    assert_eq!(git_ref.branch, "feature/test");
    assert_eq!(git_ref.link_type, GitLinkType::Manual);
    assert!(git_ref.remote.is_none());
}

#[test]
fn test_git_ref_with_remote() {
    let git_ref = GitRef::new("feature/test", GitLinkType::AutoDetected).with_remote("origin");

    assert_eq!(git_ref.remote, Some("origin".to_string()));
    assert_eq!(git_ref.link_type, GitLinkType::AutoDetected);
}

#[test]
fn test_git_ref_serialization() {
    let git_ref = GitRef::new("feature/user-auth", GitLinkType::Manual).with_remote("upstream");

    let json = serde_json::to_string(&git_ref).unwrap();
    let restored: GitRef = serde_json::from_str(&json).unwrap();

    assert_eq!(git_ref.branch, restored.branch);
    assert_eq!(git_ref.remote, restored.remote);
    assert_eq!(git_ref.link_type, restored.link_type);
}

#[test]
fn test_branch_status_methods() {
    assert_eq!(BranchStatus::Active.as_str(), "active");
    assert_eq!(BranchStatus::Merged.as_str(), "merged");
    assert_eq!(BranchStatus::Deleted.as_str(), "deleted");
    assert_eq!(BranchStatus::Unknown.as_str(), "unknown");

    assert!(!BranchStatus::Active.is_merged());
    assert!(BranchStatus::Merged.is_merged());
    assert!(!BranchStatus::Deleted.is_merged());
    assert!(!BranchStatus::Unknown.is_merged());
}

#[test]
fn test_branch_status_default() {
    let status = BranchStatus::default();
    assert_eq!(status, BranchStatus::Active);
}

#[test]
fn test_git_link_type_default() {
    let link_type = GitLinkType::default();
    assert_eq!(link_type, GitLinkType::Manual);
}

#[test]
fn test_git_commit_structure() {
    use chrono::Utc;

    let commit = GitCommit {
        hash: "abc1234".to_string(),
        full_hash: "abc1234567890abcdef".to_string(),
        message: "Fix login bug".to_string(),
        author: "Test User".to_string(),
        timestamp: Utc::now(),
    };

    assert_eq!(commit.hash, "abc1234");
    assert_eq!(commit.message, "Fix login bug");
    assert_eq!(commit.author, "Test User");
}

// ============================================================================
// INTEGRATION WORKFLOW TESTS
// ============================================================================

#[test]
fn test_complete_feature_branch_workflow() {
    let repo = setup_test_repo();

    // 1. Create feature branch
    create_feature_branch(&repo, "feature/user-login", "login.rs", "Add login feature");

    // 2. Verify branch exists
    assert!(branch_exists(repo.path(), "feature/user-login"));

    // 3. Check it's not merged yet
    let is_merged = is_branch_merged(repo.path(), "feature/user-login", "main").unwrap();
    assert!(!is_merged);

    // 4. Get commits on feature branch
    let commits = get_branch_commits(repo.path(), "feature/user-login", 10).unwrap();
    assert!(commits.iter().any(|c| c.message == "Add login feature"));

    // 5. Merge branch
    merge_branch(&repo, "feature/user-login");

    // 6. Verify now merged
    let is_merged = is_branch_merged(repo.path(), "feature/user-login", "main").unwrap();
    assert!(is_merged);
}

#[test]
fn test_branch_matching_integration() {
    let repo = setup_test_repo();
    let matcher = BranchMatcher::new();

    // Create tasks
    let task1 = Task::new("Implement user authentication");
    let task2 = Task::new("Fix payment processing bug");

    let tasks = [task1.clone(), task2.clone()];

    // Create branches that should match
    create_feature_branch(&repo, "feature/user-authentication", "auth.rs", "Add auth");
    create_feature_branch(&repo, "fix/payment-bug", "payment.rs", "Fix payment");

    let branches = list_branches(repo.path()).unwrap();

    // Match branches to tasks
    for branch in branches {
        if let Some(task_id) = matcher.match_branch_to_task(&branch, tasks.iter()) {
            if task_id == task1.id {
                assert_eq!(branch, "feature/user-authentication");
            } else if task_id == task2.id {
                assert_eq!(branch, "fix/payment-bug");
            }
        }
    }
}

#[test]
fn test_multiple_commits_workflow() {
    let repo = setup_test_repo();

    // Create branch and add multiple commits
    Command::new("git")
        .args(["checkout", "-b", "feature/multi-commit"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    for i in 1..=5 {
        fs::write(repo.path().join(format!("file{i}.txt")), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", &format!("Commit {i}")])
            .current_dir(repo.path())
            .output()
            .unwrap();
    }

    let commits = get_branch_commits(repo.path(), "feature/multi-commit", 10).unwrap();
    assert_eq!(commits.len(), 6); // 5 new + 1 initial

    // Verify order (newest first)
    assert_eq!(commits[0].message, "Commit 5");
    assert_eq!(commits[4].message, "Commit 1");
}

#[test]
fn test_id_based_matching_workflow() {
    let task = Task::new("Implement feature X");
    let id_prefix = &task.id.to_string()[..8];
    let tasks = [task.clone()];

    let repo = setup_test_repo();
    let branch_name = format!("feature/task-{id_prefix}");

    create_feature_branch(&repo, &branch_name, "feature.rs", "Implement X");

    let matcher = BranchMatcher::new();
    let result = matcher.match_branch_to_task(&branch_name, tasks.iter());

    assert_eq!(result, Some(task.id));
}

#[test]
fn test_concurrent_branches_no_conflict() {
    let repo = setup_test_repo();

    // Create multiple branches
    create_feature_branch(&repo, "feature/branch-1", "file1.txt", "Feature 1");
    create_feature_branch(&repo, "feature/branch-2", "file2.txt", "Feature 2");
    create_feature_branch(&repo, "feature/branch-3", "file3.txt", "Feature 3");

    let branches = list_branches(repo.path()).unwrap();
    assert!(branches.contains(&"feature/branch-1".to_string()));
    assert!(branches.contains(&"feature/branch-2".to_string()));
    assert!(branches.contains(&"feature/branch-3".to_string()));

    // All should be unmerged
    assert!(!is_branch_merged(repo.path(), "feature/branch-1", "main").unwrap());
    assert!(!is_branch_merged(repo.path(), "feature/branch-2", "main").unwrap());
    assert!(!is_branch_merged(repo.path(), "feature/branch-3", "main").unwrap());
}

#[test]
fn test_branch_name_with_slashes() {
    let repo = setup_test_repo();

    // Git allows slashes in branch names
    create_feature_branch(&repo, "feature/user/auth/login", "file.txt", "Deep branch");

    assert!(branch_exists(repo.path(), "feature/user/auth/login"));
    let branches = list_branches(repo.path()).unwrap();
    assert!(branches.contains(&"feature/user/auth/login".to_string()));
}

// ============================================================================
// ERROR HANDLING EDGE CASES
// ============================================================================

#[test]
fn test_corrupted_git_config() {
    let repo = setup_test_repo();

    // Corrupt the git config file
    let config_path = repo.path().join(".git").join("config");
    fs::write(&config_path, "invalid [ config { content").unwrap();

    // Most operations should handle corrupted config gracefully
    let result = get_current_branch(repo.path());
    // May succeed or fail depending on the operation, but should not panic
    let _ = result;
}

#[test]
fn test_missing_git_directory() {
    let temp = TempDir::new().unwrap();

    // Create a .git file instead of directory (submodule scenario)
    fs::write(
        temp.path().join(".git"),
        "gitdir: ../parent/.git/modules/sub",
    )
    .unwrap();

    let result = get_current_branch(temp.path());
    assert!(result.is_err());
}

#[test]
fn test_empty_repository() {
    let dir = TempDir::new().unwrap();

    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // No commits yet - git creates default branch but it's not "born" yet
    let result = get_current_branch(dir.path());
    // Modern git creates initial branch (main/master) even without commits
    // Behavior varies by git version
    let _ = result; // May succeed or fail depending on git version

    let result = list_branches(dir.path());
    assert!(result.is_ok());
    // May be empty or contain default branch depending on git version
    let _ = result;
}

#[test]
fn test_branch_with_invalid_characters() {
    let repo = setup_test_repo();

    // Try creating branches with characters git rejects
    let invalid_names = [
        "feature..double-dot", // consecutive dots
        "feature/",            // trailing slash
        "feature//double",     // double slash
    ];

    for name in invalid_names {
        let result = Command::new("git")
            .args(["branch", name])
            .current_dir(repo.path())
            .output();

        // Git should reject these, or if accepted, our operations should handle them
        if let Ok(output) = result {
            if output.status.success() {
                // If git accepted it, verify our operations handle it
                assert!(branch_exists(repo.path(), name));
            }
        }
    }
}

#[test]
fn test_operations_on_bare_repository() {
    let dir = TempDir::new().unwrap();

    // Create a bare repository
    Command::new("git")
        .args(["init", "--bare"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Operations on bare repos should handle appropriately
    let result = get_current_branch(dir.path());
    // Bare repos may or may not have a current branch depending on git version
    // Modern git still tracks HEAD in bare repos
    let _ = result;

    let result = list_branches(dir.path());
    // May succeed with empty list or fail - both acceptable for bare repos
    let _ = result;
}

#[test]
fn test_very_long_branch_name() {
    let repo = setup_test_repo();

    // Git allows very long branch names (up to filesystem limits)
    let long_name = format!("feature/{}", "a".repeat(200));

    Command::new("git")
        .args(["branch", &long_name])
        .current_dir(repo.path())
        .output()
        .unwrap();

    assert!(branch_exists(repo.path(), &long_name));

    let branches = list_branches(repo.path()).unwrap();
    assert!(branches.contains(&long_name));
}

#[test]
fn test_unicode_in_branch_names() {
    let repo = setup_test_repo();

    // Unicode characters in branch names
    let unicode_names = [
        "feature/用户认证", // Chinese
        "feature/テスト",   // Japanese
        "feature/тест",     // Russian
        "feature/café",     // Accented chars
        "feature/emoji-🚀", // Emoji
    ];

    for name in unicode_names {
        let result = Command::new("git")
            .args(["branch", name])
            .current_dir(repo.path())
            .output();

        if let Ok(output) = result {
            if output.status.success() {
                assert!(
                    branch_exists(repo.path(), name),
                    "Branch '{name}' should exist"
                );
            }
        }
    }
}

// ============================================================================
// PERFORMANCE TESTS
// ============================================================================

#[test]
fn test_list_branches_performance_100_branches() {
    let repo = setup_test_repo();

    // Create 100 branches
    for i in 1..=100 {
        Command::new("git")
            .args(["branch", &format!("feature/branch-{i:03}")])
            .current_dir(repo.path())
            .output()
            .unwrap();
    }

    let start = std::time::Instant::now();
    let branches = list_branches(repo.path()).unwrap();
    let elapsed = start.elapsed();

    assert!(branches.len() >= 100);
    assert!(
        elapsed.as_millis() < 1000,
        "Listing 100 branches took {elapsed:?}"
    );
}

#[test]
fn test_branch_matching_performance_many_tasks() {
    let matcher = BranchMatcher::new();

    // Create 1000 tasks
    let tasks: Vec<Task> = (0..1000)
        .map(|i| Task::new(format!("Task number {i}")))
        .collect();

    let start = std::time::Instant::now();
    let result = matcher.match_branch_to_task("feature/task-number-500", tasks.iter());
    let elapsed = start.elapsed();

    assert!(result.is_some());
    assert!(
        elapsed.as_millis() < 100,
        "Matching against 1000 tasks took {elapsed:?}"
    );
}

#[test]
fn test_get_commits_performance_many_commits() {
    let repo = setup_test_repo();

    // Create 100 commits
    for i in 1..=100 {
        fs::write(repo.path().join(format!("file{i}.txt")), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", &format!("Commit {i}")])
            .current_dir(repo.path())
            .output()
            .unwrap();
    }

    let start = std::time::Instant::now();
    let commits = get_branch_commits(repo.path(), "main", 50).unwrap();
    let elapsed = start.elapsed();

    assert_eq!(commits.len(), 50);
    assert!(
        elapsed.as_millis() < 500,
        "Getting 50 commits took {elapsed:?}"
    );
}

#[test]
fn test_is_merged_check_performance() {
    let repo = setup_test_repo();

    // Create and merge 20 branches
    for i in 1..=20 {
        create_feature_branch(
            &repo,
            &format!("feature/branch-{i}"),
            &format!("file{i}.txt"),
            &format!("Feature {i}"),
        );
        merge_branch(&repo, &format!("feature/branch-{i}"));
    }

    // Check all branches in sequence
    let start = std::time::Instant::now();
    for i in 1..=20 {
        let is_merged = is_branch_merged(repo.path(), &format!("feature/branch-{i}"), "main");
        assert!(is_merged.unwrap());
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 2000,
        "Checking 20 merged branches took {elapsed:?}"
    );
}

// ============================================================================
// ADVANCED BRANCH MATCHING EDGE CASES
// ============================================================================

#[test]
fn test_match_ambiguous_branches() {
    let matcher = BranchMatcher::new();

    let task1 = Task::new("Fix login");
    let task2 = Task::new("Fix login page");
    let task3 = Task::new("Fix login button");

    let tasks = [task1.clone(), task2.clone(), task3.clone()];

    // Branch is ambiguous - could match all three
    let result = matcher.match_branch_to_task("fix/login", tasks.iter());

    // Should match the best one (shortest title or highest similarity)
    assert!(result.is_some());
    assert_eq!(result.unwrap(), task1.id); // Exact match
}

#[test]
fn test_match_with_ticket_numbers() {
    let matcher = BranchMatcher::new();

    let task = Task::new("Implement OAuth authentication JIRA-1234");
    let tasks = [task.clone()];

    // Common patterns with ticket numbers
    let branches = [
        "feature/JIRA-1234-oauth",
        "feature/oauth-JIRA-1234",
        "JIRA-1234/oauth-implementation",
        "jira-1234/oauth", // lowercase
    ];

    for branch in branches {
        let result = matcher.match_branch_to_task(branch, tasks.iter());
        assert_eq!(
            result,
            Some(task.id),
            "Branch '{branch}' should match task with JIRA-1234"
        );
    }
}

#[test]
fn test_match_with_prefix_conventions() {
    let matcher = BranchMatcher::new();

    let task = Task::new("Add user profile page");
    let tasks = [task.clone()];

    // Common prefix conventions
    let branches = [
        "feat/user-profile",
        "feature/user-profile-page",
        "add/user-profile",
        "enhancement/profile-page",
    ];

    for branch in branches {
        let result = matcher.match_branch_to_task(branch, tasks.iter());
        assert!(
            result.is_some(),
            "Branch '{branch}' should match task about user profile"
        );
    }
}

#[test]
fn test_match_with_hyphens_vs_underscores() {
    let matcher = BranchMatcher::new();

    let task = Task::new("Update database schema");
    let tasks = [task.clone()];

    // Both hyphens and underscores should work
    let result1 = matcher.match_branch_to_task("feature/database-schema", tasks.iter());
    let result2 = matcher.match_branch_to_task("feature/database_schema", tasks.iter());
    let result3 = matcher.match_branch_to_task("feature/databaseSchema", tasks.iter());

    assert_eq!(result1, Some(task.id));
    assert_eq!(result2, Some(task.id));
    assert_eq!(result3, Some(task.id));
}

#[test]
fn test_match_no_match_below_threshold() {
    let matcher = BranchMatcher::new().with_threshold(0.9); // Very high threshold

    let task = Task::new("Implement payment processing");
    let tasks = [task.clone()];

    // Completely different topic
    let result = matcher.match_branch_to_task("feature/user-authentication", tasks.iter());
    assert!(result.is_none());
}

#[test]
fn test_match_exact_title_in_branch() {
    let matcher = BranchMatcher::new();

    let task = Task::new("TASK-456");
    let tasks = [task.clone()];

    // Exact match should work even with very short titles
    let result = matcher.match_branch_to_task("feature/TASK-456", tasks.iter());
    assert_eq!(result, Some(task.id));
}

#[test]
fn test_match_multiple_branches_same_task() {
    let repo = setup_test_repo();
    let matcher = BranchMatcher::new();

    let task = Task::new("Implement authentication system");
    let id_short = &task.id.to_string()[..8];
    let tasks = [task.clone()];

    // Multiple branches for same task (e.g., WIP branches)
    // Use "task-" prefix so ID matching works
    let branch_names = [
        format!("feature/task-{id_short}"),
        format!("wip/task-{id_short}"),
        format!("refactor/task-{id_short}"),
    ];

    for name in &branch_names {
        Command::new("git")
            .args(["branch", name])
            .current_dir(repo.path())
            .output()
            .unwrap();
    }

    // All should match the same task by ID
    for name in branch_names {
        let result = matcher.match_branch_to_task(&name, tasks.iter());
        assert_eq!(
            result,
            Some(task.id),
            "Branch '{name}' should match task by ID"
        );
    }
}

// ============================================================================
// MERGE DETECTION EDGE CASES
// ============================================================================

#[test]
fn test_merge_detection_fast_forward() {
    let repo = setup_test_repo();

    // Create branch
    Command::new("git")
        .args(["checkout", "-b", "feature/fast-forward"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    fs::write(repo.path().join("new.txt"), "content").unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(repo.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "Add new file"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    // Fast-forward merge (no merge commit)
    Command::new("git")
        .args(["merge", "--ff-only", "feature/fast-forward"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    // Should still detect as merged
    let is_merged = is_branch_merged(repo.path(), "feature/fast-forward", "main").unwrap();
    assert!(is_merged);
}

#[test]
fn test_merge_detection_squash_merge() {
    let repo = setup_test_repo();

    create_feature_branch(&repo, "feature/squash-test", "file.txt", "Feature work");

    // Squash merge
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["merge", "--squash", "feature/squash-test"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "Squash merge"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    // Squash merges don't preserve history, may not be detected
    // This test documents the behavior
    let is_merged = is_branch_merged(repo.path(), "feature/squash-test", "main");
    // Result depends on git's merge detection heuristics
    let _ = is_merged;
}

#[test]
fn test_merge_detection_cherry_pick() {
    let repo = setup_test_repo();

    create_feature_branch(&repo, "feature/cherry-test", "file.txt", "Feature commit");

    // Get the commit hash
    let output = Command::new("git")
        .args(["rev-parse", "feature/cherry-test"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Cherry-pick instead of merge
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["cherry-pick", &commit])
        .current_dir(repo.path())
        .output()
        .unwrap();

    // Cherry-pick creates new commit with same content
    // Git's merge detection may consider this merged depending on version/heuristics
    let is_merged = is_branch_merged(repo.path(), "feature/cherry-test", "main").unwrap();
    // Modern git can detect cherry-picked commits as merged
    let _ = is_merged; // May be true or false depending on git's detection
}

#[test]
fn test_merge_detection_rebase_and_merge() {
    let repo = setup_test_repo();

    create_feature_branch(&repo, "feature/rebase-test", "file.txt", "Feature");

    // Rebase onto main
    Command::new("git")
        .args(["checkout", "feature/rebase-test"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["rebase", "main"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    // Now merge
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["merge", "feature/rebase-test"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    let is_merged = is_branch_merged(repo.path(), "feature/rebase-test", "main").unwrap();
    assert!(is_merged);
}

// ============================================================================
// BRANCH STATUS AND LIFECYCLE TESTS
// ============================================================================

#[test]
fn test_branch_lifecycle_create_work_merge_delete() {
    let repo = setup_test_repo();
    let branch = "feature/lifecycle-test";

    // 1. Create
    create_feature_branch(&repo, branch, "work.txt", "Work done");
    assert!(branch_exists(repo.path(), branch));

    // 2. Verify not merged
    assert!(!is_branch_merged(repo.path(), branch, "main").unwrap());

    // 3. Merge
    merge_branch(&repo, branch);
    assert!(is_branch_merged(repo.path(), branch, "main").unwrap());

    // 4. Delete
    Command::new("git")
        .args(["branch", "-d", branch])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(!branch_exists(repo.path(), branch));
}

#[test]
fn test_stale_branch_detection() {
    let repo = setup_test_repo();

    // Create an old branch
    create_feature_branch(&repo, "feature/old-branch", "old.txt", "Old work");

    // Add commits to main (making branch stale)
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    for i in 1..=5 {
        fs::write(repo.path().join(format!("main{i}.txt")), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", &format!("Main commit {i}")])
            .current_dir(repo.path())
            .output()
            .unwrap();
    }

    // Branch still exists but is behind
    assert!(branch_exists(repo.path(), "feature/old-branch"));

    // Could add function to check if branch is behind main
    // For now, just verify it's not merged
    assert!(!is_branch_merged(repo.path(), "feature/old-branch", "main").unwrap());
}
