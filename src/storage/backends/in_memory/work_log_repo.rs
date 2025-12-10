//! WorkLogRepository blanket implementation for in-memory backends.

use crate::domain::{TaskId, WorkLogEntry, WorkLogEntryId};
use crate::storage::{StorageError, StorageResult, WorkLogRepository};

use super::InMemoryBackend;

impl<B: InMemoryBackend> WorkLogRepository for B {
    fn create_work_log(&mut self, entry: &WorkLogEntry) -> StorageResult<()> {
        if self.data().work_logs.iter().any(|e| e.id == entry.id) {
            return Err(StorageError::already_exists(
                "WorkLogEntry",
                entry.id.0.to_string(),
            ));
        }
        self.data_mut().work_logs.push(entry.clone());
        self.mark_dirty();
        Ok(())
    }

    fn get_work_log(&self, id: &WorkLogEntryId) -> StorageResult<Option<WorkLogEntry>> {
        Ok(self.data().work_logs.iter().find(|e| &e.id == id).cloned())
    }

    fn update_work_log(&mut self, entry: &WorkLogEntry) -> StorageResult<()> {
        if let Some(existing) = self
            .data_mut()
            .work_logs
            .iter_mut()
            .find(|e| e.id == entry.id)
        {
            *existing = entry.clone();
            self.mark_dirty();
            Ok(())
        } else {
            Err(StorageError::not_found(
                "WorkLogEntry",
                entry.id.0.to_string(),
            ))
        }
    }

    fn delete_work_log(&mut self, id: &WorkLogEntryId) -> StorageResult<()> {
        let len_before = self.data().work_logs.len();
        self.data_mut().work_logs.retain(|e| &e.id != id);
        if self.data().work_logs.len() == len_before {
            return Err(StorageError::not_found("WorkLogEntry", id.0.to_string()));
        }
        self.mark_dirty();
        Ok(())
    }

    fn get_work_logs_for_task(&self, task_id: &TaskId) -> StorageResult<Vec<WorkLogEntry>> {
        let mut logs: Vec<_> = self
            .data()
            .work_logs
            .iter()
            .filter(|e| &e.task_id == task_id)
            .cloned()
            .collect();
        // Sort by creation time, newest first
        logs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(logs)
    }

    fn list_work_logs(&self) -> StorageResult<Vec<WorkLogEntry>> {
        Ok(self.data().work_logs.clone())
    }
}
