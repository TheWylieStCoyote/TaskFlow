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
//! # Local sidecar
//!
//! Only tasks have a CalDAV representation (VTODOs). Everything else the app
//! stores — projects, tags, time entries, work logs, habits, goals, key
//! results, Pomodoro state, saved filters and the audit log — has nowhere to
//! live on the server, so it is persisted to a JSON sidecar file next to the
//! config file: `caldav.toml` gets `caldav.local.json`. The sidecar is an
//! [`ExportData`] document with an empty `tasks` map; it is loaded in
//! [`initialize`](crate::storage::StorageBackend::initialize) before the
//! server is contacted and rewritten (atomically) on every
//! [`flush`](crate::storage::StorageBackend::flush) that has local changes.
//!
//! # Feature Flag
//!
//! HTTP support is compiled in only when the `caldav-sync` cargo feature is
//! enabled.  Without that feature `initialize` / `flush` / `refresh` silently
//! no-op (returning an informative error only when network calls are attempted).

pub(crate) mod client;
mod storage;

#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::TaskId;
use crate::storage::{atomic_write, ExportData, StorageError, StorageResult};

/// Extension appended to the config file's stem to name the local sidecar
/// (`caldav.toml` -> `caldav.local.json`).
pub(crate) const SIDECAR_EXTENSION: &str = "local.json";

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
///
/// Data with no CalDAV representation is persisted to a local sidecar file;
/// see the [module docs](self) for details.
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
    /// Where local-only data is persisted; `None` keeps it in memory only.
    pub(crate) sidecar_path: Option<PathBuf>,
}

impl CalDavBackend {
    /// Create a new `CalDavBackend` by reading a TOML config file at `config_path`.
    ///
    /// Local-only data is persisted to the sidecar file returned by
    /// [`sidecar_path_for`](Self::sidecar_path_for).
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

        Ok(Self::from_config(config).with_sidecar(Self::sidecar_path_for(config_path)))
    }

    /// Create a `CalDavBackend` directly from a config struct (useful for tests).
    ///
    /// The result has no sidecar, so local-only data lives in memory until
    /// [`with_sidecar`](Self::with_sidecar) is called.
    #[must_use]
    pub fn from_config(config: CalDavConfig) -> Self {
        Self {
            config,
            mem: ExportData::default(),
            mem_dirty: false,
            dirty_ids: HashSet::new(),
            deleted_ids: HashSet::new(),
            sidecar_path: None,
        }
    }

    /// Persist local-only data to `path` (see the [module docs](self)).
    #[must_use]
    pub fn with_sidecar(mut self, path: impl Into<PathBuf>) -> Self {
        self.sidecar_path = Some(path.into());
        self
    }

    /// The sidecar file used for a given config path:
    /// `/x/caldav.toml` -> `/x/caldav.local.json`.
    #[must_use]
    pub fn sidecar_path_for(config_path: &Path) -> PathBuf {
        config_path.with_extension(SIDECAR_EXTENSION)
    }

    /// Path of the local sidecar file, if persistence is enabled.
    #[must_use]
    pub fn sidecar_path(&self) -> Option<&Path> {
        self.sidecar_path.as_deref()
    }

    /// Load local-only data from the sidecar into `self.mem`.
    ///
    /// Tasks already in memory are kept (they come from the server); every
    /// other collection is replaced by the sidecar's contents. Returns `true`
    /// when a sidecar file was found and read.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the file exists but cannot be read or parsed.
    pub(crate) fn load_sidecar(&mut self) -> StorageResult<bool> {
        let Some(path) = self.sidecar_path.clone() else {
            return Ok(false);
        };
        if !path.exists() {
            return Ok(false);
        }
        let content = std::fs::read_to_string(&path).map_err(|e| StorageError::io(&path, e))?;
        let mut local: ExportData = serde_json::from_str(&content).map_err(|e| {
            StorageError::deserialization(format!("Invalid CalDAV sidecar {}: {e}", path.display()))
        })?;
        // Tasks are owned by the server; never resurrect them from the sidecar.
        local.tasks = std::mem::take(&mut self.mem.tasks);
        self.mem = local;
        Ok(true)
    }

    /// Write local-only data (everything in `self.mem` except tasks) to the sidecar.
    ///
    /// A no-op when no sidecar path is configured.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if serialization or the atomic write fails.
    pub(crate) fn save_sidecar(&self) -> StorageResult<()> {
        let Some(path) = &self.sidecar_path else {
            return Ok(());
        };
        let local = ExportData {
            tasks: std::collections::HashMap::new(),
            ..self.mem.clone()
        };
        let content = serde_json::to_string_pretty(&local)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        atomic_write(path, content)
    }
}
