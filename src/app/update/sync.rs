//! Git synchronization message handlers.
//!
//! Handles all sync-related messages including:
//! - Status checks
//! - Commit, pull, push operations
//! - Conflict resolution

use crate::app::{Model, SyncMessage};
use crate::storage::sync::{GitSync, PullResult, PushResult};

/// Handle sync messages
pub fn handle_sync(model: &mut Model, msg: SyncMessage) {
    // Check if git sync is available
    if model.git_sync.is_none() {
        model.status_message =
            Some("Git sync not available (requires Markdown backend)".to_string());
        return;
    }

    match msg {
        SyncMessage::Status => handle_status(model),
        SyncMessage::Commit => handle_commit(model),
        SyncMessage::Pull => handle_pull(model),
        SyncMessage::Push => handle_push(model),
        SyncMessage::Sync => handle_full_sync(model),
        SyncMessage::ResolveConflict { path, resolution } => {
            handle_resolve_conflict(model, &path, resolution);
        }
        SyncMessage::AbortMerge => handle_abort_merge(model),
    }
}

fn handle_status(model: &mut Model) {
    let Some(ref git_sync) = model.git_sync else {
        return;
    };
    match git_sync.status() {
        Ok(status) => {
            let msg = if status.has_conflicts() {
                format!("Git: {} conflict(s)", status.conflicts.len())
            } else if status.is_synced() {
                "Git: Synced".to_string()
            } else {
                format!("Git: {}", status.short_status())
            };
            model.status_message = Some(msg);
            model.git_status = Some(status);
        }
        Err(e) => {
            model.status_message = Some(format!("Git status failed: {e}"));
        }
    }
}

fn handle_commit(model: &mut Model) {
    let Some(ref git_sync) = model.git_sync else {
        return;
    };
    match git_sync.commit_all("Update tasks") {
        Ok(_) => {
            model.status_message = Some("Changes committed".to_string());
            if let Ok(status) = git_sync.status() {
                model.git_status = Some(status);
            }
        }
        Err(e) => {
            model.status_message = Some(format!("Commit failed: {e}"));
        }
    }
}

fn handle_pull(model: &mut Model) {
    let Some(ref git_sync) = model.git_sync else {
        return;
    };

    // First commit any local changes
    let _ = git_sync.commit_all("Auto-commit before pull");

    let pull_result = git_sync.pull("origin", "main");
    let new_status = git_sync.status().ok();

    match pull_result {
        Ok(PullResult::UpToDate) => {
            model.status_message = Some("Already up to date".to_string());
        }
        Ok(PullResult::FastForward { commits }) => {
            model.status_message = Some(format!("Pulled {commits} commit(s)"));
            reload_after_sync(model);
        }
        Ok(PullResult::Merged { commits }) => {
            model.status_message = Some(format!("Merged {commits} commit(s)"));
            reload_after_sync(model);
        }
        Ok(PullResult::Conflicts { files }) => {
            model.status_message = Some(format!(
                "Conflicts in {} file(s). Resolve with gs resolve-ours/theirs",
                files.len()
            ));
        }
        Err(e) => {
            model.status_message = Some(format!("Pull failed: {e}"));
        }
    }

    model.git_status = new_status;
}

fn handle_push(model: &mut Model) {
    let Some(ref git_sync) = model.git_sync else {
        return;
    };

    // Commit any uncommitted changes first
    let _ = git_sync.commit_all("Auto-commit before push");

    let push_result = git_sync.push("origin", "main");
    let new_status = git_sync.status().ok();

    match push_result {
        Ok(PushResult::Success { commits }) => {
            model.status_message = Some(format!("Pushed {commits} commit(s)"));
        }
        Ok(PushResult::NothingToPush) => {
            model.status_message = Some("Nothing to push".to_string());
        }
        Ok(PushResult::Rejected) => {
            model.status_message = Some("Push rejected - pull first".to_string());
        }
        Err(e) => {
            model.status_message = Some(format!("Push failed: {e}"));
        }
    }

    model.git_status = new_status;
}

fn handle_full_sync(model: &mut Model) {
    let Some(ref git_sync) = model.git_sync else {
        return;
    };

    // Commit, pull, then push
    let _ = git_sync.commit_all("Auto-commit before sync");

    // Pull first
    let pull_result = git_sync.pull("origin", "main");
    let mut needs_reload = false;

    match pull_result {
        Ok(PullResult::Conflicts { files }) => {
            model.status_message =
                Some(format!("Sync paused: conflicts in {} file(s)", files.len()));
            model.git_status = git_sync.status().ok();
            return;
        }
        Ok(PullResult::UpToDate) => {}
        Ok(_) => {
            needs_reload = true;
        }
        Err(e) => {
            model.status_message = Some(format!("Sync failed during pull: {e}"));
            return;
        }
    }

    // Now push
    let push_result = git_sync.push("origin", "main");
    model.git_status = git_sync.status().ok();

    match push_result {
        Ok(PushResult::Success { commits }) => {
            model.status_message = Some(format!("Synced (pushed {commits} commit(s))"));
        }
        Ok(PushResult::NothingToPush) => {
            model.status_message = Some("Synced (up to date)".to_string());
        }
        Ok(PushResult::Rejected) => {
            model.status_message = Some("Sync failed: push rejected".to_string());
        }
        Err(e) => {
            model.status_message = Some(format!("Sync failed during push: {e}"));
        }
    }

    // Reload after sync if we pulled changes
    if needs_reload {
        reload_after_sync(model);
    }
}

fn handle_resolve_conflict(
    model: &mut Model,
    path: &std::path::Path,
    resolution: crate::storage::sync::ConflictResolution,
) {
    let Some(ref git_sync) = model.git_sync else {
        return;
    };

    match git_sync.resolve_conflict(path, resolution) {
        Ok(()) => {
            model.status_message = Some(format!("Resolved: {}", path.display()));
            if let Ok(status) = git_sync.status() {
                if status.conflicts.is_empty() {
                    let _ = git_sync.commit_all("Merge conflict resolution");
                    model.status_message =
                        Some("All conflicts resolved, merge complete".to_string());
                    reload_after_sync(model);
                }
                model.git_status = Some(status);
            }
        }
        Err(e) => {
            model.status_message = Some(format!("Failed to resolve conflict: {e}"));
        }
    }
}

fn handle_abort_merge(model: &mut Model) {
    let Some(ref git_sync) = model.git_sync else {
        return;
    };

    match git_sync.abort_merge() {
        Ok(()) => {
            model.status_message = Some("Merge aborted".to_string());
            model.git_status = git_sync.status().ok();
        }
        Err(e) => {
            model.status_message = Some(format!("Failed to abort merge: {e}"));
        }
    }
}

/// Reload task data from storage after a sync operation.
///
/// This is necessary when pull brings in changes from remote.
fn reload_after_sync(model: &mut Model) {
    // Re-initialize from backend to pick up changes
    if let Some(ref mut backend) = model.storage {
        // Reload tasks
        if let Ok(tasks) = backend.list_tasks() {
            model.tasks.clear();
            for task in tasks {
                model.tasks.insert(task.id, task);
            }
        }

        // Reload projects
        if let Ok(projects) = backend.list_projects() {
            model.projects.clear();
            for project in projects {
                model.projects.insert(project.id, project);
            }
        }

        // Reload habits
        if let Ok(habits) = backend.list_habits() {
            model.habits.clear();
            for habit in habits {
                model.habits.insert(habit.id, habit);
            }
        }

        model.refresh_visible_tasks();
        model.refresh_visible_habits();
    }
}

/// Initialize git sync for a model if using Markdown backend.
///
/// Call this during model initialization when storage path is known.
pub fn init_git_sync(model: &mut Model, storage_path: &std::path::Path) {
    match GitSync::open_or_init(storage_path) {
        Ok(sync) => {
            // Get initial status
            let status = sync.status().ok();
            model.git_status = status;
            model.git_sync = Some(sync);
        }
        Err(e) => {
            tracing::warn!("Failed to initialize git sync: {}", e);
        }
    }
}
