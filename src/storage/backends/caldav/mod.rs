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
//! [`ExportData`] document plus two queues (the `Sidecar` struct); it is loaded in
//! [`initialize`](crate::storage::StorageBackend::initialize) before the
//! server is contacted and rewritten (atomically) on every
//! [`flush`](crate::storage::StorageBackend::flush) that has local changes.
//!
//! # Working offline
//!
//! The sidecar also carries a cache of the tasks last seen from the server
//! and the IDs of tasks whose push or delete has not reached the server yet.
//! When the server is unreachable the backend keeps working on the cache;
//! every flush writes the cache and the queues to disk first and only then
//! tries the network, so an edit made offline survives a restart and is
//! pushed by the first flush that succeeds. While a task is queued for push
//! the local copy wins over the server's; once pushed, the server's copy is
//! authoritative again. Tasks queued for delete are never resurrected from
//! the server.
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

/// On-disk shape of the sidecar file.
///
/// The [`ExportData`] fields are flattened in, so a sidecar written before the
/// queues existed still loads (both queues default to empty).
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct Sidecar {
    /// Local-only data plus the cached copy of the server's tasks.
    #[serde(flatten)]
    pub data: ExportData,
    /// Tasks created or edited locally whose push has not succeeded yet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_push: Vec<TaskId>,
    /// Tasks deleted locally whose server delete has not succeeded yet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_delete: Vec<TaskId>,
}

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
    /// In-memory data store (tasks + projects + tags + …). Tasks are the
    /// cached view of the server plus any local edits not yet pushed.
    pub(crate) mem: ExportData,
    /// True when any in-memory data has changed since the last flush.
    pub(crate) mem_dirty: bool,
    /// Task IDs mutated locally that still need to be pushed. Entries stay
    /// here across flushes (and restarts, via the sidecar) until a push
    /// succeeds.
    pub(crate) dirty_ids: HashSet<TaskId>,
    /// Task IDs deleted locally that still need a DELETE call on the server.
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

    /// Load the sidecar into `self.mem` and the pending queues.
    ///
    /// Every local-only collection is replaced by the sidecar's contents.
    /// Cached tasks fill in only where memory has none: a task already in
    /// memory came from the server and wins, unless the sidecar queued it
    /// for push, in which case the local edit wins. Returns `true` when a
    /// sidecar file was found and read.
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
        let sidecar: Sidecar = serde_json::from_str(&content).map_err(|e| {
            StorageError::deserialization(format!("Invalid CalDAV sidecar {}: {e}", path.display()))
        })?;
        let Sidecar {
            mut data,
            pending_push,
            pending_delete,
        } = sidecar;

        self.dirty_ids.extend(pending_push);
        self.deleted_ids.extend(pending_delete);

        let mut tasks = std::mem::take(&mut self.mem.tasks);
        for (id, cached) in data.tasks.drain() {
            if self.deleted_ids.contains(&id) {
                continue;
            }
            if self.dirty_ids.contains(&id) {
                tasks.insert(id, cached);
            } else {
                tasks.entry(id).or_insert(cached);
            }
        }
        for id in &self.deleted_ids {
            tasks.remove(id);
        }
        data.tasks = tasks;
        self.mem = data;
        Ok(true)
    }

    /// Write `self.mem` (including the task cache) and the pending queues to
    /// the sidecar.
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
        let mut pending_push: Vec<TaskId> = self.dirty_ids.iter().copied().collect();
        pending_push.sort_by_key(|id| id.0);
        let mut pending_delete: Vec<TaskId> = self.deleted_ids.iter().copied().collect();
        pending_delete.sort_by_key(|id| id.0);
        let sidecar = Sidecar {
            data: self.mem.clone(),
            pending_push,
            pending_delete,
        };
        let content = serde_json::to_string_pretty(&sidecar)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        atomic_write(path, content)
    }

    /// Number of task changes (pushes and deletes) still waiting for the
    /// server.
    #[must_use]
    pub fn pending_sync_count(&self) -> usize {
        self.dirty_ids.len() + self.deleted_ids.len()
    }

    /// Fold a fresh copy of the server's tasks into memory.
    ///
    /// Server tasks replace the cached copy when they are newer (or new).
    /// Tasks queued for push keep their local version; tasks queued for
    /// delete are not resurrected. Cached tasks the server no longer has are
    /// dropped unless they are queued for push (created offline). Returns the
    /// number of tasks that changed.
    pub(crate) fn merge_remote_tasks(&mut self, remote: Vec<crate::domain::Task>) -> usize {
        let mut changed = 0usize;
        let remote_ids: HashSet<TaskId> = remote.iter().map(|t| t.id).collect();

        for task in remote {
            if self.dirty_ids.contains(&task.id) || self.deleted_ids.contains(&task.id) {
                continue;
            }
            let outdated = self
                .mem
                .tasks
                .get(&task.id)
                .is_none_or(|local| local.updated_at < task.updated_at);
            if outdated {
                self.mem.tasks.insert(task.id, task);
                changed += 1;
            }
        }

        let to_remove: Vec<TaskId> = self
            .mem
            .tasks
            .keys()
            .filter(|id| !remote_ids.contains(id) && !self.dirty_ids.contains(id))
            .copied()
            .collect();
        changed += to_remove.len();
        for id in to_remove {
            self.mem.tasks.remove(&id);
        }

        // A delete the server already applied needs no retry.
        self.deleted_ids.retain(|id| remote_ids.contains(id));

        changed
    }
}
