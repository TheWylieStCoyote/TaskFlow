//! Cache management and external change detection for markdown backend.
//!
//! Handles detecting when files have been modified externally (e.g., by a text editor
//! or git operation) and refreshing the in-memory cache accordingly.

use std::collections::HashSet;
use std::fs;

use crate::domain::{ProjectId, TaskId};

use super::MarkdownBackend;

impl MarkdownBackend {
    /// Check if a task file was modified externally and reload if needed.
    /// Returns true if the cache was updated.
    pub(crate) fn check_task_modified(&mut self, id: &TaskId) -> bool {
        let path = self.tasks_dir.join(format!("{}.md", id.0));

        // Check current file mtime
        let current_mtime = match fs::metadata(&path).and_then(|m| m.modified()) {
            Ok(mtime) => mtime,
            Err(_) => return false, // File doesn't exist or can't read metadata
        };

        // Compare with cached mtime
        let cached_mtime = self.task_mtimes.get(id);
        let needs_reload = cached_mtime.is_none_or(|cached| *cached != current_mtime);

        if needs_reload {
            // Reload the file
            if let Ok(task) = self.parse_task_file(&path) {
                self.task_mtimes.insert(*id, current_mtime);
                self.tasks_cache.insert(*id, task);
                return true;
            }
        }

        false
    }

    /// Check if a project file was modified externally and reload if needed.
    /// Returns true if the cache was updated.
    pub(crate) fn check_project_modified(&mut self, id: &ProjectId) -> bool {
        let path = self.projects_dir.join(format!("{}.md", id.0));

        // Check current file mtime
        let current_mtime = match fs::metadata(&path).and_then(|m| m.modified()) {
            Ok(mtime) => mtime,
            Err(_) => return false, // File doesn't exist or can't read metadata
        };

        // Compare with cached mtime
        let cached_mtime = self.project_mtimes.get(id);
        let needs_reload = cached_mtime.is_none_or(|cached| *cached != current_mtime);

        if needs_reload {
            // Reload the file
            if let Ok(project) = self.parse_project_file(&path) {
                self.project_mtimes.insert(*id, current_mtime);
                self.projects_cache.insert(*id, project);
                return true;
            }
        }

        false
    }

    /// Check for any externally added or removed task files.
    /// Returns the number of changes detected.
    pub(crate) fn scan_for_task_changes(&mut self) -> usize {
        let mut changes = 0;

        if !self.tasks_dir.exists() {
            return 0;
        }

        // Collect current file IDs from disk
        let mut disk_ids: HashSet<TaskId> = HashSet::new();

        if let Ok(entries) = fs::read_dir(&self.tasks_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    // Extract ID from filename (uuid.md)
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(uuid) = uuid::Uuid::parse_str(stem) {
                            disk_ids.insert(TaskId(uuid));
                        }
                    }
                }
            }
        }

        // Check for new files (on disk but not in cache)
        for id in &disk_ids {
            if !self.tasks_cache.contains_key(id) {
                let path = self.tasks_dir.join(format!("{}.md", id.0));
                if let Ok(task) = self.parse_task_file(&path) {
                    if let Ok(mtime) = fs::metadata(&path).and_then(|m| m.modified()) {
                        self.task_mtimes.insert(*id, mtime);
                    }
                    self.tasks_cache.insert(*id, task);
                    changes += 1;
                }
            }
        }

        // Check for deleted files (in cache but not on disk)
        let cached_ids: Vec<TaskId> = self.tasks_cache.keys().copied().collect();
        for id in cached_ids {
            if !disk_ids.contains(&id) {
                self.tasks_cache.remove(&id);
                self.task_mtimes.remove(&id);
                changes += 1;
            }
        }

        // Check for modified files
        for id in &disk_ids {
            if self.check_task_modified(id) {
                changes += 1;
            }
        }

        changes
    }

    /// Check for any externally added or removed project files.
    /// Returns the number of changes detected.
    pub(crate) fn scan_for_project_changes(&mut self) -> usize {
        let mut changes = 0;

        if !self.projects_dir.exists() {
            return 0;
        }

        // Collect current file IDs from disk
        let mut disk_ids: HashSet<ProjectId> = HashSet::new();

        if let Ok(entries) = fs::read_dir(&self.projects_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    // Extract ID from filename (uuid.md)
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(uuid) = uuid::Uuid::parse_str(stem) {
                            disk_ids.insert(ProjectId(uuid));
                        }
                    }
                }
            }
        }

        // Check for new files (on disk but not in cache)
        for id in &disk_ids {
            if !self.projects_cache.contains_key(id) {
                let path = self.projects_dir.join(format!("{}.md", id.0));
                if let Ok(project) = self.parse_project_file(&path) {
                    if let Ok(mtime) = fs::metadata(&path).and_then(|m| m.modified()) {
                        self.project_mtimes.insert(*id, mtime);
                    }
                    self.projects_cache.insert(*id, project);
                    changes += 1;
                }
            }
        }

        // Check for deleted files (in cache but not on disk)
        let cached_ids: Vec<ProjectId> = self.projects_cache.keys().copied().collect();
        for id in cached_ids {
            if !disk_ids.contains(&id) {
                self.projects_cache.remove(&id);
                self.project_mtimes.remove(&id);
                changes += 1;
            }
        }

        // Check for modified files
        for id in &disk_ids {
            if self.check_project_modified(id) {
                changes += 1;
            }
        }

        changes
    }
}
