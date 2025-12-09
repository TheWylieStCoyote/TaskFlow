//! Git-based synchronization for the Markdown backend.
//!
//! This module provides Git integration for syncing TaskFlow data
//! across machines. It works seamlessly with the Markdown backend
//! since tasks are stored as individual `.md` files.
//!
//! # Features
//!
//! - Initialize or open existing Git repositories
//! - Commit changes automatically
//! - Pull from and push to remotes
//! - Detect and report merge conflicts
//! - Track sync status (ahead/behind counts)
//!
//! # Example
//!
//! ```no_run
//! use taskflow::storage::sync::GitSync;
//! use std::path::Path;
//!
//! // Open an existing repo or initialize a new one
//! let sync = GitSync::open_or_init(Path::new("./tasks"))?;
//!
//! // Check status
//! let status = sync.status()?;
//! println!("Ahead: {}, Behind: {}", status.ahead, status.behind);
//!
//! // Commit local changes
//! sync.commit_all("Update tasks")?;
//!
//! // Sync with remote
//! sync.pull("origin", "main")?;
//! sync.push("origin", "main")?;
//! # Ok::<(), taskflow::storage::sync::SyncError>(())
//! ```

mod types;

pub use types::*;

use std::path::{Path, PathBuf};

use git2::{
    BranchType, Cred, FetchOptions, IndexAddOption, PushOptions, RemoteCallbacks, Repository,
    Signature, StatusOptions,
};

/// Git synchronization manager.
///
/// Wraps a git2 repository and provides high-level sync operations.
pub struct GitSync {
    repo: Repository,
    path: PathBuf,
}

impl GitSync {
    /// Opens an existing Git repository.
    ///
    /// # Errors
    ///
    /// Returns [`SyncError::NotARepository`] if the path is not a Git repository.
    pub fn open(path: &Path) -> SyncResult<Self> {
        let repo =
            Repository::open(path).map_err(|_| SyncError::NotARepository(path.to_owned()))?;
        Ok(Self {
            repo,
            path: path.to_owned(),
        })
    }

    /// Initializes a new Git repository.
    ///
    /// Creates a new repository if one doesn't exist, or returns the existing one.
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] if initialization fails.
    pub fn init(path: &Path) -> SyncResult<Self> {
        let repo = Repository::init(path)?;
        Ok(Self {
            repo,
            path: path.to_owned(),
        })
    }

    /// Opens an existing repository or initializes a new one.
    ///
    /// This is the recommended way to get a `GitSync` instance.
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] if neither opening nor initialization succeeds.
    pub fn open_or_init(path: &Path) -> SyncResult<Self> {
        Self::open(path).or_else(|_| Self::init(path))
    }

    /// Returns the path to the repository.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Gets the current repository status.
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] if status cannot be determined.
    pub fn status(&self) -> SyncResult<GitStatus> {
        let mut status = GitStatus {
            is_repo: true,
            ..Default::default()
        };

        // Get current branch
        if let Ok(head) = self.repo.head() {
            if let Some(name) = head.shorthand() {
                status.branch = Some(name.to_string());
            }
        }

        // Check for remote
        status.has_remote = self.repo.find_remote("origin").is_ok();

        // Get ahead/behind counts if we have upstream
        if let Ok(head) = self.repo.head() {
            if let Some(local_oid) = head.target() {
                // Try to find upstream branch
                if let Ok(upstream) = self
                    .repo
                    .find_branch(
                        &format!("origin/{}", status.branch.as_deref().unwrap_or("main")),
                        BranchType::Remote,
                    )
                    .and_then(|b| {
                        b.get()
                            .target()
                            .ok_or_else(|| git2::Error::from_str("no target"))
                    })
                {
                    if let Ok((ahead, behind)) = self.repo.graph_ahead_behind(local_oid, upstream) {
                        status.ahead = ahead;
                        status.behind = behind;
                    }
                }
            }
        }

        // Get file status
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false);

        if let Ok(statuses) = self.repo.statuses(Some(&mut opts)) {
            for entry in statuses.iter() {
                let s = entry.status();
                if s.is_index_new()
                    || s.is_index_modified()
                    || s.is_index_deleted()
                    || s.is_index_renamed()
                {
                    status.staged += 1;
                }
                if s.is_wt_modified() || s.is_wt_deleted() || s.is_wt_renamed() {
                    status.modified += 1;
                }
                if s.is_wt_new() {
                    status.untracked += 1;
                }
                if s.is_conflicted() {
                    if let Some(path) = entry.path() {
                        status.conflicts.push(PathBuf::from(path));
                    }
                }
            }
        }

        Ok(status)
    }

    /// Stages all changes (new, modified, deleted files).
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] if staging fails.
    pub fn add_all(&self) -> SyncResult<()> {
        let mut index = self.repo.index()?;
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    /// Creates a commit with all staged changes.
    ///
    /// Automatically stages all changes before committing.
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] if the commit fails.
    pub fn commit_all(&self, message: &str) -> SyncResult<git2::Oid> {
        // Stage all changes first
        self.add_all()?;

        let mut index = self.repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;

        let signature = self.get_signature()?;

        // Get parent commit (if any)
        let parent_commit = self.repo.head().ok().and_then(|h| h.peel_to_commit().ok());

        let parents: Vec<&git2::Commit<'_>> = parent_commit.iter().collect();

        let oid = self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )?;

        Ok(oid)
    }

    /// Fetches from a remote.
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] if the fetch fails.
    pub fn fetch(&self, remote_name: &str, branch: &str) -> SyncResult<()> {
        let mut remote = self
            .repo
            .find_remote(remote_name)
            .map_err(|_| SyncError::NoRemote(remote_name.to_string()))?;

        let callbacks = Self::get_callbacks();
        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        remote.fetch(&[branch], Some(&mut fetch_opts), None)?;
        Ok(())
    }

    /// Pulls from a remote (fetch + merge).
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] if the pull fails or conflicts are detected.
    pub fn pull(&self, remote_name: &str, branch: &str) -> SyncResult<PullResult> {
        // Fetch first
        self.fetch(remote_name, branch)?;

        // Find the fetch head
        let fetch_head = self.repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = self.repo.reference_to_annotated_commit(&fetch_head)?;

        // Perform merge analysis
        let (analysis, _) = self.repo.merge_analysis(&[&fetch_commit])?;

        if analysis.is_up_to_date() {
            return Ok(PullResult::UpToDate);
        }

        if analysis.is_fast_forward() {
            // Fast-forward merge
            let refname = format!("refs/heads/{branch}");
            let mut reference = self.repo.find_reference(&refname)?;
            reference.set_target(fetch_commit.id(), "Fast-forward")?;
            self.repo.set_head(&refname)?;
            self.repo
                .checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;

            // Count commits
            let commits =
                self.count_commits_between(self.repo.head()?.target(), Some(fetch_commit.id()));

            return Ok(PullResult::FastForward { commits });
        }

        // Normal merge required
        self.repo.merge(&[&fetch_commit], None, None)?;

        // Check for conflicts
        let mut index = self.repo.index()?;
        if index.has_conflicts() {
            let conflicts: Vec<PathBuf> = index
                .conflicts()?
                .filter_map(Result::ok)
                .filter_map(|c| c.our.or(c.their).or(c.ancestor))
                .filter_map(|e| String::from_utf8(e.path).ok())
                .map(PathBuf::from)
                .collect();

            return Ok(PullResult::Conflicts { files: conflicts });
        }

        // Complete the merge
        let signature = self.get_signature()?;
        let head = self.repo.head()?;
        let head_commit = head.peel_to_commit()?;
        let fetch_commit_obj = self.repo.find_commit(fetch_commit.id())?;

        let tree_id = index.write_tree_to(&self.repo)?;
        let tree = self.repo.find_tree(tree_id)?;

        self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &format!("Merge {remote_name}/{branch}"),
            &tree,
            &[&head_commit, &fetch_commit_obj],
        )?;

        self.repo.cleanup_state()?;

        Ok(PullResult::Merged { commits: 1 })
    }

    /// Pushes to a remote.
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] if the push fails.
    pub fn push(&self, remote_name: &str, branch: &str) -> SyncResult<PushResult> {
        // Check if there's anything to push
        let status = self.status()?;
        if status.ahead == 0 {
            return Ok(PushResult::NothingToPush);
        }

        let mut remote = self
            .repo
            .find_remote(remote_name)
            .map_err(|_| SyncError::NoRemote(remote_name.to_string()))?;

        let callbacks = Self::get_callbacks();
        let mut push_opts = PushOptions::new();
        push_opts.remote_callbacks(callbacks);

        let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");

        match remote.push(&[&refspec], Some(&mut push_opts)) {
            Ok(()) => Ok(PushResult::Success {
                commits: status.ahead,
            }),
            Err(e) if e.message().contains("non-fast-forward") => Ok(PushResult::Rejected),
            Err(e) => Err(e.into()),
        }
    }

    /// Resolves a conflict by choosing one side.
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] if resolution fails.
    pub fn resolve_conflict(&self, path: &Path, resolution: ConflictResolution) -> SyncResult<()> {
        let mut index = self.repo.index()?;

        // Get the conflicting entries
        let conflicts: Vec<_> = index.conflicts()?.filter_map(Result::ok).collect();

        for conflict in conflicts {
            let conflict_path = conflict
                .our
                .as_ref()
                .or(conflict.their.as_ref())
                .and_then(|e| String::from_utf8(e.path.clone()).ok());

            if conflict_path.as_deref() == Some(path.to_string_lossy().as_ref()) {
                let chosen = match resolution {
                    ConflictResolution::Ours => conflict.our,
                    ConflictResolution::Theirs => conflict.their,
                };

                if let Some(entry) = chosen {
                    // Read the blob content
                    let blob = self.repo.find_blob(entry.id)?;
                    let content = blob.content();

                    // Write the resolved file
                    std::fs::write(self.path.join(path), content)?;

                    // Stage the resolved file
                    index.add_path(path)?;
                }
            }
        }

        index.write()?;
        Ok(())
    }

    /// Aborts an in-progress merge.
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] if abort fails.
    pub fn abort_merge(&self) -> SyncResult<()> {
        self.repo.cleanup_state()?;
        self.repo
            .checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        Ok(())
    }

    /// Adds a remote to the repository.
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] if adding the remote fails.
    pub fn add_remote(&self, name: &str, url: &str) -> SyncResult<()> {
        self.repo.remote(name, url)?;
        Ok(())
    }

    /// Gets the URL of a remote.
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] if the remote is not found.
    pub fn get_remote_url(&self, name: &str) -> SyncResult<Option<String>> {
        match self.repo.find_remote(name) {
            Ok(remote) => Ok(remote.url().map(String::from)),
            Err(_) => Ok(None),
        }
    }

    // Helper methods

    fn get_signature(&self) -> SyncResult<Signature<'static>> {
        // Try to get from config, fall back to defaults
        let config = self.repo.config().ok();

        let name = config
            .as_ref()
            .and_then(|c| c.get_string("user.name").ok())
            .unwrap_or_else(|| "TaskFlow".to_string());

        let email = config
            .as_ref()
            .and_then(|c| c.get_string("user.email").ok())
            .unwrap_or_else(|| "taskflow@localhost".to_string());

        Ok(Signature::now(&name, &email)?)
    }

    fn get_callbacks() -> RemoteCallbacks<'static> {
        let mut callbacks = RemoteCallbacks::new();

        // SSH key authentication
        callbacks.credentials(|_url, username_from_url, allowed_types| {
            if allowed_types.contains(git2::CredentialType::SSH_KEY) {
                // Try SSH agent first, then default key locations
                if let Ok(cred) = Cred::ssh_key_from_agent(username_from_url.unwrap_or("git")) {
                    return Ok(cred);
                }

                // Try default SSH key locations
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                let key_path = PathBuf::from(&home).join(".ssh/id_rsa");
                let pub_path = PathBuf::from(&home).join(".ssh/id_rsa.pub");

                if key_path.exists() {
                    return Cred::ssh_key(
                        username_from_url.unwrap_or("git"),
                        Some(&pub_path),
                        &key_path,
                        None,
                    );
                }

                // Try ed25519
                let key_path = PathBuf::from(&home).join(".ssh/id_ed25519");
                let pub_path = PathBuf::from(&home).join(".ssh/id_ed25519.pub");

                if key_path.exists() {
                    return Cred::ssh_key(
                        username_from_url.unwrap_or("git"),
                        Some(&pub_path),
                        &key_path,
                        None,
                    );
                }
            }

            // Default credential
            Cred::default()
        });

        callbacks
    }

    fn count_commits_between(&self, from: Option<git2::Oid>, to: Option<git2::Oid>) -> usize {
        let (Some(from), Some(to)) = (from, to) else {
            return 0;
        };

        self.repo
            .graph_ahead_behind(from, to)
            .map(|(ahead, _)| ahead)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_repo() -> (TempDir, GitSync) {
        let temp = TempDir::new().unwrap();
        let sync = GitSync::init(temp.path()).unwrap();
        (temp, sync)
    }

    #[test]
    fn test_init_creates_repo() {
        let temp = TempDir::new().unwrap();
        let sync = GitSync::init(temp.path()).unwrap();
        assert!(sync.path().exists());

        let status = sync.status().unwrap();
        assert!(status.is_repo);
    }

    #[test]
    fn test_open_existing_repo() {
        let (temp, _) = create_test_repo();
        let sync = GitSync::open(temp.path()).unwrap();
        assert!(sync.status().unwrap().is_repo);
    }

    #[test]
    fn test_open_nonexistent_fails() {
        let result = GitSync::open(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_open_or_init_creates_new() {
        let temp = TempDir::new().unwrap();
        let sync = GitSync::open_or_init(temp.path()).unwrap();
        assert!(sync.status().unwrap().is_repo);
    }

    #[test]
    fn test_open_or_init_opens_existing() {
        let (temp, _) = create_test_repo();
        let sync = GitSync::open_or_init(temp.path()).unwrap();
        assert!(sync.status().unwrap().is_repo);
    }

    #[test]
    fn test_status_clean_repo() {
        let (_, sync) = create_test_repo();
        let status = sync.status().unwrap();

        assert!(status.is_repo);
        assert!(status.is_clean());
        assert_eq!(status.staged, 0);
        assert_eq!(status.modified, 0);
    }

    #[test]
    fn test_status_with_untracked_file() {
        let (temp, sync) = create_test_repo();

        // Create an untracked file
        std::fs::write(temp.path().join("test.txt"), "hello").unwrap();

        let status = sync.status().unwrap();
        assert_eq!(status.untracked, 1);
    }

    #[test]
    fn test_add_all() {
        let (temp, sync) = create_test_repo();

        // Create a file
        std::fs::write(temp.path().join("test.txt"), "hello").unwrap();

        // Add all
        sync.add_all().unwrap();

        let status = sync.status().unwrap();
        assert_eq!(status.staged, 1);
        assert_eq!(status.untracked, 0);
    }

    #[test]
    fn test_commit_all() {
        let (temp, sync) = create_test_repo();

        // Create a file
        std::fs::write(temp.path().join("test.txt"), "hello").unwrap();

        // Commit
        let oid = sync.commit_all("Initial commit").unwrap();
        assert!(!oid.is_zero());

        let status = sync.status().unwrap();
        assert!(status.is_clean());
    }

    #[test]
    fn test_status_has_no_remote_initially() {
        let (_, sync) = create_test_repo();
        let status = sync.status().unwrap();
        assert!(!status.has_remote);
    }

    #[test]
    fn test_add_remote() {
        let (temp, sync) = create_test_repo();

        sync.add_remote("origin", "https://github.com/test/repo.git")
            .unwrap();

        let url = sync.get_remote_url("origin").unwrap();
        assert_eq!(url, Some("https://github.com/test/repo.git".to_string()));

        let status = sync.status().unwrap();
        assert!(status.has_remote);

        drop(temp); // Keep temp dir alive until end
    }

    #[test]
    fn test_get_remote_url_not_found() {
        let (_, sync) = create_test_repo();
        let url = sync.get_remote_url("nonexistent").unwrap();
        assert!(url.is_none());
    }

    #[test]
    fn test_status_short_status_empty_repo() {
        let (_, sync) = create_test_repo();
        let status = sync.status().unwrap();
        // Empty repo is "synced"
        assert!(status.short_status().contains("synced"));
    }
}
