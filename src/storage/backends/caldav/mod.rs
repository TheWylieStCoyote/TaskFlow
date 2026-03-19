//! CalDAV storage backend.
//!
//! Syncs tasks with a CalDAV server (Google Calendar, Apple Calendar, Nextcloud,
//! Radicale, etc.) using the VTODO component of the iCalendar protocol.
//!
//! # Configuration
//!
//! The backend reads a TOML config file at the path provided to `new()`:
//!
//! ```toml
//! url = "https://caldav.example.com"
//! username = "alice"
//! password = "s3cret"
//! collection_path = "/dav/caldav/alice/tasks/"
//! ```
//!
//! # Feature Flag
//!
//! HTTP support is compiled in only when the `caldav-sync` cargo feature is
//! enabled.  Without that feature `initialize` / `flush` / `refresh` silently
//! no-op (returning an informative error only when network calls are attempted).

pub(crate) mod client;
mod storage;

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::domain::TaskId;
use crate::storage::{ExportData, StorageError, StorageResult};

/// Connection settings for a CalDAV server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalDavConfig {
    /// Base URL of the CalDAV server (e.g. `https://cloud.example.com`).
    pub url: String,
    /// Username for HTTP Basic authentication.
    pub username: String,
    /// Password for HTTP Basic authentication.
    pub password: String,
    /// Path to the VTODO collection on the server
    /// (e.g. `/dav/caldav/alice/tasks/`).
    pub collection_path: String,
}

/// CalDAV storage backend.
///
/// Tasks are stored as VTODO `.ics` objects in a CalDAV collection.
/// The backend maintains a local in-memory cache that is:
/// - populated from the server on [`initialize`](crate::storage::StorageBackend::initialize)
/// - pushed back to the server on [`flush`](crate::storage::StorageBackend::flush)
pub struct CalDavBackend {
    pub(crate) config: CalDavConfig,
    /// In-memory data store (tasks + projects + tags + …).
    pub(crate) mem: ExportData,
    /// True when any in-memory data has changed since the last flush.
    pub(crate) mem_dirty: bool,
    /// Task IDs mutated locally that need to be pushed on the next flush.
    pub(crate) dirty_ids: HashSet<TaskId>,
    /// Task IDs deleted locally that need a DELETE call on the next flush.
    pub(crate) deleted_ids: HashSet<TaskId>,
}

impl CalDavBackend {
    /// Create a new `CalDavBackend` by reading a TOML config file at `config_path`.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the config file cannot be read or parsed.
    pub fn new(config_path: &Path) -> StorageResult<Self> {
        let content = std::fs::read_to_string(config_path).map_err(|e| {
            StorageError::io(
                config_path,
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Cannot read CalDAV config: {e}"),
                ),
            )
        })?;

        let config: CalDavConfig = toml::from_str(&content)
            .map_err(|e| StorageError::serialization(format!("Invalid CalDAV config TOML: {e}")))?;

        Ok(Self::from_config(config))
    }

    /// Create a `CalDavBackend` directly from a config struct (useful for tests).
    #[must_use]
    pub fn from_config(config: CalDavConfig) -> Self {
        Self {
            config,
            mem: ExportData::default(),
            mem_dirty: false,
            dirty_ids: HashSet::new(),
            deleted_ids: HashSet::new(),
        }
    }
}
