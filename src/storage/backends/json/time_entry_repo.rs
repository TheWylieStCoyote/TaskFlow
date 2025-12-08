//! TimeEntryRepository implementation for JSON backend.

use crate::domain::{TaskId, TimeEntry, TimeEntryId};
use crate::storage::{StorageError, StorageResult, TimeEntryRepository};

use super::JsonBackend;

impl TimeEntryRepository for JsonBackend {
    fn create_time_entry(&mut self, entry: &TimeEntry) -> StorageResult<()> {
        if self.data.time_entries.iter().any(|e| e.id == entry.id) {
            return Err(StorageError::already_exists(
                "TimeEntry",
                entry.id.0.to_string(),
            ));
        }
        self.data.time_entries.push(entry.clone());
        self.mark_dirty();
        Ok(())
    }

    fn get_time_entry(&self, id: &TimeEntryId) -> StorageResult<Option<TimeEntry>> {
        Ok(self.data.time_entries.iter().find(|e| &e.id == id).cloned())
    }

    fn update_time_entry(&mut self, entry: &TimeEntry) -> StorageResult<()> {
        if let Some(existing) = self.data.time_entries.iter_mut().find(|e| e.id == entry.id) {
            *existing = entry.clone();
            self.mark_dirty();
            Ok(())
        } else {
            Err(StorageError::not_found("TimeEntry", entry.id.0.to_string()))
        }
    }

    fn delete_time_entry(&mut self, id: &TimeEntryId) -> StorageResult<()> {
        let len_before = self.data.time_entries.len();
        self.data.time_entries.retain(|e| &e.id != id);
        if self.data.time_entries.len() == len_before {
            return Err(StorageError::not_found("TimeEntry", id.0.to_string()));
        }
        self.mark_dirty();
        Ok(())
    }

    fn get_entries_for_task(&self, task_id: &TaskId) -> StorageResult<Vec<TimeEntry>> {
        Ok(self
            .data
            .time_entries
            .iter()
            .filter(|e| &e.task_id == task_id)
            .cloned()
            .collect())
    }

    fn get_active_entry(&self) -> StorageResult<Option<TimeEntry>> {
        Ok(self
            .data
            .time_entries
            .iter()
            .find(|e| e.is_running())
            .cloned())
    }
}
