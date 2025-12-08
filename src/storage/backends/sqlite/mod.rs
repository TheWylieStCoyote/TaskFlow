//! `SQLite` database storage backend.
//!
//! Best for larger datasets with complex queries. Provides ACID guarantees
//! and efficient indexing.

mod project_repo;
mod rows;
mod schema;
mod storage;
mod tag_repo;
mod task_repo;
#[cfg(test)]
mod tests;
mod time_entry_repo;
mod work_log_repo;

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::storage::{StorageError, StorageResult};

pub use self::storage::SqliteBackend;

/// Internal struct for `SQLite` backend state
pub(crate) struct SqliteBackendInner {
    pub(crate) path: PathBuf,
    pub(crate) conn: Option<Connection>,
}

impl SqliteBackendInner {
    pub(crate) fn new(path: &Path) -> StorageResult<Self> {
        Ok(Self {
            path: path.to_path_buf(),
            conn: None,
        })
    }

    pub(crate) fn conn(&self) -> StorageResult<&Connection> {
        self.conn.as_ref().ok_or(StorageError::NotInitialized)
    }
}
