//! Git sync types and status structures.

use std::path::PathBuf;

use thiserror::Error;

/// Result type for sync operations.
pub type SyncResult<T> = Result<T, SyncError>;

/// Git sync-specific errors.
#[derive(Error, Debug)]
pub enum SyncError {
    /// Repository not found or not initialized.
    #[error("Not a git repository: {0}")]
    NotARepository(PathBuf),

    /// No remote configured.
    #[error("No remote configured with name '{0}'")]
    NoRemote(String),

    /// Remote operation failed.
    #[error("Remote operation failed: {0}")]
    RemoteError(String),

    /// Authentication failed.
    #[error("Authentication failed: {0}")]
    AuthError(String),

    /// Merge conflicts detected.
    #[error("Merge conflicts in {count} file(s)")]
    Conflicts { count: usize, files: Vec<PathBuf> },

    /// Working directory is dirty.
    #[error("Working directory has uncommitted changes")]
    DirtyWorkDir,

    /// Underlying git2 error.
    #[error("Git error: {0}")]
    Git(String),

    /// Network error.
    #[error("Network error: {0}")]
    Network(String),

    /// Reference not found.
    #[error("Reference not found: {0}")]
    RefNotFound(String),
}

impl From<git2::Error> for SyncError {
    fn from(err: git2::Error) -> Self {
        // Map specific git2 error classes to our error types
        match err.class() {
            git2::ErrorClass::Net | git2::ErrorClass::Http => {
                Self::Network(err.message().to_string())
            }
            git2::ErrorClass::Ssh => Self::AuthError(err.message().to_string()),
            git2::ErrorClass::Reference => Self::RefNotFound(err.message().to_string()),
            _ => Self::Git(err.message().to_string()),
        }
    }
}

impl From<std::io::Error> for SyncError {
    fn from(err: std::io::Error) -> Self {
        Self::Git(format!("IO error: {err}"))
    }
}

/// Status of the git repository.
#[derive(Debug, Clone, Default)]
pub struct GitStatus {
    /// Whether the path is a valid git repository.
    pub is_repo: bool,
    /// Whether a remote is configured.
    pub has_remote: bool,
    /// Name of the current branch.
    pub branch: Option<String>,
    /// Number of commits ahead of remote.
    pub ahead: usize,
    /// Number of commits behind remote.
    pub behind: usize,
    /// Number of staged files.
    pub staged: usize,
    /// Number of modified (unstaged) files.
    pub modified: usize,
    /// Number of untracked files.
    pub untracked: usize,
    /// Files with merge conflicts.
    pub conflicts: Vec<PathBuf>,
}

impl GitStatus {
    /// Returns true if the repository is clean (no uncommitted changes).
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.staged == 0 && self.modified == 0 && self.conflicts.is_empty()
    }

    /// Returns true if the repository is synced with remote.
    #[must_use]
    pub fn is_synced(&self) -> bool {
        self.is_clean() && self.ahead == 0 && self.behind == 0
    }

    /// Returns true if there are merge conflicts.
    #[must_use]
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Returns a short status string for display in the status bar.
    #[must_use]
    pub fn short_status(&self) -> String {
        if !self.is_repo {
            return String::new();
        }

        if !self.conflicts.is_empty() {
            return format!("! {} conflict(s)", self.conflicts.len());
        }

        let mut parts = Vec::new();

        if self.ahead > 0 {
            parts.push(format!("^{}", self.ahead));
        }
        if self.behind > 0 {
            parts.push(format!("v{}", self.behind));
        }
        if self.staged > 0 {
            parts.push(format!("+{}", self.staged));
        }
        if self.modified > 0 {
            parts.push(format!("~{}", self.modified));
        }

        if parts.is_empty() {
            "= synced".to_string()
        } else {
            parts.join(" ")
        }
    }
}

/// Result of a pull operation.
#[derive(Debug, Clone)]
pub enum PullResult {
    /// Already up to date.
    UpToDate,
    /// Fast-forward merge succeeded.
    FastForward {
        /// Number of commits pulled.
        commits: usize,
    },
    /// Merge was performed.
    Merged {
        /// Number of commits pulled.
        commits: usize,
    },
    /// Merge conflicts detected.
    Conflicts {
        /// Files with conflicts.
        files: Vec<PathBuf>,
    },
}

impl PullResult {
    /// Returns a human-readable description of the result.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::UpToDate => "Already up to date".to_string(),
            Self::FastForward { commits } => {
                format!("Fast-forward: {commits} commit(s) pulled")
            }
            Self::Merged { commits } => format!("Merged: {commits} commit(s) pulled"),
            Self::Conflicts { files } => {
                format!("Conflicts in {} file(s)", files.len())
            }
        }
    }
}

/// Result of a push operation.
#[derive(Debug, Clone)]
pub enum PushResult {
    /// Push succeeded.
    Success {
        /// Number of commits pushed.
        commits: usize,
    },
    /// Nothing to push.
    NothingToPush,
    /// Push was rejected (needs pull first).
    Rejected,
}

impl PushResult {
    /// Returns a human-readable description of the result.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Success { commits } => format!("{commits} commit(s) pushed"),
            Self::NothingToPush => "Nothing to push".to_string(),
            Self::Rejected => "Push rejected - pull first".to_string(),
        }
    }
}

/// How to resolve a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Keep our version (local changes).
    Ours,
    /// Accept their version (remote changes).
    Theirs,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_status_is_clean() {
        let status = GitStatus::default();
        assert!(status.is_clean());

        let dirty = GitStatus {
            modified: 1,
            ..Default::default()
        };
        assert!(!dirty.is_clean());
    }

    #[test]
    fn test_git_status_is_synced() {
        let synced = GitStatus {
            is_repo: true,
            ..Default::default()
        };
        assert!(synced.is_synced());

        let not_synced = GitStatus {
            is_repo: true,
            ahead: 2,
            ..Default::default()
        };
        assert!(!not_synced.is_synced());
    }

    #[test]
    fn test_git_status_short_status() {
        let status = GitStatus {
            is_repo: true,
            ahead: 2,
            behind: 1,
            staged: 3,
            modified: 4,
            ..Default::default()
        };
        let short = status.short_status();
        assert!(short.contains("^2"));
        assert!(short.contains("v1"));
        assert!(short.contains("+3"));
        assert!(short.contains("~4"));
    }

    #[test]
    fn test_git_status_short_status_synced() {
        let status = GitStatus {
            is_repo: true,
            ..Default::default()
        };
        assert_eq!(status.short_status(), "= synced");
    }

    #[test]
    fn test_git_status_short_status_conflicts() {
        let status = GitStatus {
            is_repo: true,
            conflicts: vec![PathBuf::from("file.md")],
            ..Default::default()
        };
        assert!(status.short_status().contains("conflict"));
    }

    #[test]
    fn test_pull_result_message() {
        assert_eq!(PullResult::UpToDate.message(), "Already up to date");
        assert!(PullResult::FastForward { commits: 3 }
            .message()
            .contains('3'));
        assert!(PullResult::Conflicts {
            files: vec![PathBuf::from("a.md")]
        }
        .message()
        .contains("Conflicts"));
    }

    #[test]
    fn test_push_result_message() {
        assert!(PushResult::Success { commits: 2 }.message().contains('2'));
        assert_eq!(PushResult::NothingToPush.message(), "Nothing to push");
        assert!(PushResult::Rejected.message().contains("rejected"));
    }
}
