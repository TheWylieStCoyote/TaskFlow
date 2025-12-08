//! JSON file-based storage backend.
//!
//! Stores all data in a single JSON file for simplicity.
//! Good for small to medium datasets and easy backup/version control.

mod project_repo;
mod storage;
mod tag_repo;
mod task_repo;
mod time_entry_repo;
mod work_log_repo;

#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};

use crate::storage::{ExportData, StorageError, StorageResult};

/// JSON file-based storage backend.
///
/// Stores all data in a single JSON file for simplicity.
/// Good for small to medium datasets and easy backup/version control.
pub struct JsonBackend {
    pub(crate) path: PathBuf,
    pub(crate) data: ExportData,
    pub(crate) dirty: bool,
}

impl JsonBackend {
    /// Creates a new JSON backend at the given path.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`] if the backend cannot be created.
    pub fn new(path: &Path) -> StorageResult<Self> {
        Ok(Self {
            path: path.to_path_buf(),
            data: ExportData::default(),
            dirty: false,
        })
    }

    /// Load data from the JSON file.
    pub(crate) fn load(&mut self) -> StorageResult<()> {
        if self.path.exists() {
            let content =
                fs::read_to_string(&self.path).map_err(|e| StorageError::io(&self.path, e))?;
            self.data = serde_json::from_str(&content)?;
        }
        self.dirty = false;
        Ok(())
    }

    /// Save data to the JSON file.
    pub(crate) fn save(&mut self) -> StorageResult<()> {
        if !self.dirty {
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| StorageError::io(parent, e))?;
        }

        let content = serde_json::to_string_pretty(&self.data)
            .map_err(|e| StorageError::serialization(e.to_string()))?;

        fs::write(&self.path, content).map_err(|e| StorageError::io(&self.path, e))?;

        self.dirty = false;
        Ok(())
    }

    /// Mark the data as modified.
    pub(crate) const fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}
