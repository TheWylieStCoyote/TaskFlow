//! SavedFilterRepository blanket implementation for in-memory backends.

use crate::domain::{SavedFilter, SavedFilterId};
use crate::storage::{SavedFilterRepository, StorageError, StorageResult};

use super::InMemoryBackend;

impl<B: InMemoryBackend> SavedFilterRepository for B {
    fn create_saved_filter(&mut self, filter: &SavedFilter) -> StorageResult<()> {
        if self.data().saved_filters.iter().any(|f| f.id == filter.id) {
            return Err(StorageError::already_exists(
                "SavedFilter",
                filter.id.0.clone(),
            ));
        }
        self.data_mut().saved_filters.push(filter.clone());
        self.mark_dirty();
        Ok(())
    }

    fn get_saved_filter(&self, id: &SavedFilterId) -> StorageResult<Option<SavedFilter>> {
        Ok(self
            .data()
            .saved_filters
            .iter()
            .find(|f| &f.id == id)
            .cloned())
    }

    fn update_saved_filter(&mut self, filter: &SavedFilter) -> StorageResult<()> {
        if let Some(existing) = self
            .data_mut()
            .saved_filters
            .iter_mut()
            .find(|f| f.id == filter.id)
        {
            *existing = filter.clone();
            self.mark_dirty();
            Ok(())
        } else {
            Err(StorageError::not_found("SavedFilter", filter.id.0.clone()))
        }
    }

    fn delete_saved_filter(&mut self, id: &SavedFilterId) -> StorageResult<()> {
        let len_before = self.data().saved_filters.len();
        self.data_mut().saved_filters.retain(|f| &f.id != id);
        if self.data().saved_filters.len() == len_before {
            return Err(StorageError::not_found("SavedFilter", id.0.clone()));
        }
        self.mark_dirty();
        Ok(())
    }

    fn list_saved_filters(&self) -> StorageResult<Vec<SavedFilter>> {
        Ok(self.data().saved_filters.clone())
    }
}
