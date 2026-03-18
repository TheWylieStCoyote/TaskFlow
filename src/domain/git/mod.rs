//! Git integration types and operations.
//!
//! This module provides functionality for linking tasks to git branches,
//! tracking commit history, and auto-completing tasks when branches are merged.
//!
//! # Features
//!
//! - **Branch Linking**: Associate tasks with git branches (manual or auto-detected)
//! - **Commit History**: View commits on a task's linked branch
//! - **Merge Detection**: Auto-complete tasks when their branch is merged
//! - **Branch Matching**: Auto-detect task-branch links from naming conventions
//!
//! # Examples
//!
//! ```
//! use taskflow::domain::git::{GitRef, GitLinkType};
//! use chrono::Utc;
//!
//! // Create a git reference for a task
//! let git_ref = GitRef {
//!     branch: "feature/login-fix".to_string(),
//!     remote: Some("origin".to_string()),
//!     linked_at: Utc::now(),
//!     link_type: GitLinkType::Manual,
//! };
//! ```

pub mod matching;
pub mod operations;
pub mod scan;

pub use scan::{scan_git_todos, scan_git_todos_with_patterns, GitTodoItem, DEFAULT_PATTERNS};

#[cfg(test)]
mod comprehensive_tests;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Git metadata associated with a task.
///
/// When a task is linked to a git branch, this struct stores
/// the branch name, remote, and linking metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitRef {
    /// Branch name (e.g., "feature/TASK-123" or "fix-login-bug")
    pub branch: String,

    /// Remote name (optional, e.g., "origin")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,

    /// When the link was created
    pub linked_at: DateTime<Utc>,

    /// How the link was established
    pub link_type: GitLinkType,
}

impl GitRef {
    /// Create a new GitRef with the given branch name.
    #[must_use]
    pub fn new(branch: impl Into<String>, link_type: GitLinkType) -> Self {
        Self {
            branch: branch.into(),
            remote: None,
            linked_at: Utc::now(),
            link_type,
        }
    }

    /// Set the remote for this git reference.
    #[must_use]
    pub fn with_remote(mut self, remote: impl Into<String>) -> Self {
        self.remote = Some(remote.into());
        self
    }
}

/// How the task-branch link was created.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GitLinkType {
    /// User explicitly linked via CLI or TUI
    #[default]
    Manual,
    /// Auto-detected from branch naming convention
    AutoDetected,
}

/// Git commit information for task history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitCommit {
    /// Short hash (7 characters)
    pub hash: String,
    /// Full SHA-1 hash
    pub full_hash: String,
    /// First line of commit message
    pub message: String,
    /// Author name
    pub author: String,
    /// Commit timestamp
    pub timestamp: DateTime<Utc>,
}

/// Branch status relative to the base branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BranchStatus {
    /// Branch exists and is not merged
    #[default]
    Active,
    /// Branch has been merged into base (main/master)
    Merged,
    /// Branch no longer exists (deleted after merge)
    Deleted,
    /// Cannot determine status (not in a git repo, etc.)
    Unknown,
}

impl BranchStatus {
    /// Returns a display string for the status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Merged => "merged",
            Self::Deleted => "deleted",
            Self::Unknown => "unknown",
        }
    }

    /// Returns true if the branch is merged.
    #[must_use]
    pub const fn is_merged(&self) -> bool {
        matches!(self, Self::Merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_ref_new() {
        let git_ref = GitRef::new("feature/test", GitLinkType::Manual);
        assert_eq!(git_ref.branch, "feature/test");
        assert_eq!(git_ref.link_type, GitLinkType::Manual);
        assert!(git_ref.remote.is_none());
    }

    #[test]
    fn test_git_ref_with_remote() {
        let git_ref = GitRef::new("feature/test", GitLinkType::AutoDetected).with_remote("origin");
        assert_eq!(git_ref.remote, Some("origin".to_string()));
    }

    #[test]
    fn test_branch_status_as_str() {
        assert_eq!(BranchStatus::Active.as_str(), "active");
        assert_eq!(BranchStatus::Merged.as_str(), "merged");
        assert_eq!(BranchStatus::Deleted.as_str(), "deleted");
        assert_eq!(BranchStatus::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_branch_status_is_merged() {
        assert!(!BranchStatus::Active.is_merged());
        assert!(BranchStatus::Merged.is_merged());
        assert!(!BranchStatus::Deleted.is_merged());
    }

    #[test]
    fn test_git_ref_serialization() {
        let git_ref = GitRef::new("feature/test", GitLinkType::Manual).with_remote("origin");
        let json = serde_json::to_string(&git_ref).unwrap();
        let restored: GitRef = serde_json::from_str(&json).unwrap();
        assert_eq!(git_ref, restored);
    }
}
