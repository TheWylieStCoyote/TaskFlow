//! WorkLogRepository implementation for SQLite backend.

use rusqlite::{params, OptionalExtension};

use crate::domain::{TaskId, WorkLogEntry, WorkLogEntryId};
use crate::storage::{StorageError, StorageResult, WorkLogRepository};

use super::rows::work_log_from_row;
use super::SqliteBackend;

impl WorkLogRepository for SqliteBackend {
    fn create_work_log(&mut self, entry: &WorkLogEntry) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        conn.execute(
            r"INSERT INTO work_logs (id, task_id, content, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                entry.id.0.to_string(),
                entry.task_id.0.to_string(),
                entry.content,
                entry.created_at.to_rfc3339(),
                entry.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn get_work_log(&self, id: &WorkLogEntryId) -> StorageResult<Option<WorkLogEntry>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM work_logs WHERE id = ?1")?;
        let entry = stmt
            .query_row(params![id.0.to_string()], work_log_from_row)
            .optional()?;
        Ok(entry)
    }

    fn update_work_log(&mut self, entry: &WorkLogEntry) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        let rows = conn.execute(
            r"UPDATE work_logs SET content = ?2, updated_at = ?3 WHERE id = ?1",
            params![
                entry.id.0.to_string(),
                entry.content,
                entry.updated_at.to_rfc3339(),
            ],
        )?;
        if rows == 0 {
            return Err(StorageError::not_found(
                "WorkLogEntry",
                entry.id.0.to_string(),
            ));
        }
        Ok(())
    }

    fn delete_work_log(&mut self, id: &WorkLogEntryId) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        let rows = conn.execute(
            "DELETE FROM work_logs WHERE id = ?1",
            params![id.0.to_string()],
        )?;
        if rows == 0 {
            return Err(StorageError::not_found("WorkLogEntry", id.0.to_string()));
        }
        Ok(())
    }

    fn get_work_logs_for_task(&self, task_id: &TaskId) -> StorageResult<Vec<WorkLogEntry>> {
        let conn = self.inner.conn()?;
        let mut stmt =
            conn.prepare("SELECT * FROM work_logs WHERE task_id = ?1 ORDER BY created_at DESC")?;
        let entries = stmt
            .query_map(params![task_id.0.to_string()], work_log_from_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(entries)
    }

    fn list_work_logs(&self) -> StorageResult<Vec<WorkLogEntry>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM work_logs ORDER BY created_at DESC")?;
        let entries = stmt
            .query_map([], work_log_from_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(entries)
    }
}
