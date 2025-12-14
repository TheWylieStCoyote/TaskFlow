//! Storage and import state types.

use std::path::PathBuf;

use crate::storage::{ImportResult, StorageBackend};

/// State for storage backend and persistence.
///
/// Groups fields related to data persistence and storage backend.
#[derive(Default)]
pub struct StorageState {
    /// Active storage backend (if configured)
    pub(crate) backend: Option<Box<dyn StorageBackend>>,
    /// Path to data file/directory
    pub data_path: Option<PathBuf>,
    /// Whether there are unsaved changes
    pub dirty: bool,
    /// Whether the model is in sample/demo data mode (no persistence)
    pub sample_data_mode: bool,
}

impl std::fmt::Debug for StorageState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageState")
            .field(
                "backend",
                &self.backend.as_ref().map(|_| "<StorageBackend>"),
            )
            .field("data_path", &self.data_path)
            .field("dirty", &self.dirty)
            .field("sample_data_mode", &self.sample_data_mode)
            .finish()
    }
}

impl Clone for StorageState {
    fn clone(&self) -> Self {
        // Storage backend cannot be cloned, so we create without backend
        Self {
            backend: None,
            data_path: self.data_path.clone(),
            dirty: self.dirty,
            sample_data_mode: self.sample_data_mode,
        }
    }
}

/// State for import operations.
///
/// Groups fields related to importing data from external sources.
#[derive(Debug, Clone, Default)]
pub struct ImportState {
    /// Pending import result awaiting confirmation
    pub pending: Option<ImportResult>,
    /// Whether import preview dialog is showing
    pub show_preview: bool,
}

impl ImportState {
    /// Returns true if there's a pending import.
    #[inline]
    #[must_use]
    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Clears the import state.
    pub fn clear(&mut self) {
        self.pending = None;
        self.show_preview = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_state_has_pending() {
        let mut state = ImportState::default();
        assert!(!state.has_pending());

        state.pending = Some(ImportResult {
            imported: Vec::new(),
            imported_events: Vec::new(),
            skipped: Vec::new(),
            errors: Vec::new(),
        });
        assert!(state.has_pending());
    }

    #[test]
    fn test_import_state_clear() {
        let mut state = ImportState {
            pending: Some(ImportResult {
                imported: Vec::new(),
                imported_events: Vec::new(),
                skipped: Vec::new(),
                errors: Vec::new(),
            }),
            show_preview: true,
        };

        state.clear();

        assert!(state.pending.is_none());
        assert!(!state.show_preview);
    }

    #[test]
    fn test_storage_state_debug() {
        let state = StorageState::default();
        let debug_str = format!("{state:?}");
        assert!(debug_str.contains("StorageState"));
    }

    #[test]
    fn test_storage_state_clone() {
        let state = StorageState {
            backend: None,
            data_path: Some(PathBuf::from("/tmp/test")),
            dirty: true,
            sample_data_mode: true,
        };

        let cloned = state.clone();
        assert!(cloned.backend.is_none()); // Backend doesn't clone
        assert_eq!(cloned.data_path, Some(PathBuf::from("/tmp/test")));
        assert!(cloned.dirty);
        assert!(cloned.sample_data_mode);
    }
}
