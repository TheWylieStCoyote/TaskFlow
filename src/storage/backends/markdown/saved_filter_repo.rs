//! SavedFilterRepository implementation for markdown backend.
//!
//! Saved filters live in `saved_filters.yaml` next to the task files; the
//! file is rewritten (atomically) after every mutation, like the other
//! auxiliary YAML files in this backend.

use crate::domain::{SavedFilter, SavedFilterId};
use crate::storage::{SavedFilterRepository, StorageError, StorageResult};

use super::MarkdownBackend;

impl SavedFilterRepository for MarkdownBackend {
    fn create_saved_filter(&mut self, filter: &SavedFilter) -> StorageResult<()> {
        if self.saved_filters.iter().any(|f| f.id == filter.id) {
            return Err(StorageError::already_exists(
                "SavedFilter",
                filter.id.0.clone(),
            ));
        }
        self.saved_filters.push(filter.clone());
        self.save_saved_filters()?;
        Ok(())
    }

    fn get_saved_filter(&self, id: &SavedFilterId) -> StorageResult<Option<SavedFilter>> {
        Ok(self.saved_filters.iter().find(|f| &f.id == id).cloned())
    }

    fn update_saved_filter(&mut self, filter: &SavedFilter) -> StorageResult<()> {
        if let Some(existing) = self.saved_filters.iter_mut().find(|f| f.id == filter.id) {
            *existing = filter.clone();
            self.save_saved_filters()?;
            Ok(())
        } else {
            Err(StorageError::not_found("SavedFilter", filter.id.0.clone()))
        }
    }

    fn delete_saved_filter(&mut self, id: &SavedFilterId) -> StorageResult<()> {
        let len_before = self.saved_filters.len();
        self.saved_filters.retain(|f| &f.id != id);
        if self.saved_filters.len() == len_before {
            return Err(StorageError::not_found("SavedFilter", id.0.clone()));
        }
        self.save_saved_filters()?;
        Ok(())
    }

    fn list_saved_filters(&self) -> StorageResult<Vec<SavedFilter>> {
        Ok(self.saved_filters.clone())
    }
}
