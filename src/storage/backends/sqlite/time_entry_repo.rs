//! TimeEntryRepository implementation for SQLite backend.

use rusqlite::{params, OptionalExtension};

use crate::domain::{TaskId, TimeEntry, TimeEntryId};
use crate::storage::{StorageError, StorageResult, TimeEntryRepository};

use super::rows::time_entry_from_row;
use super::SqliteBackend;

impl TimeEntryRepository for SqliteBackend {
    fn create_time_entry(&mut self, entry: &TimeEntry) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        conn.execute(
            r"INSERT INTO time_entries (id, task_id, description, started_at, ended_at, duration_minutes)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                entry.id.0.to_string(),
                entry.task_id.0.to_string(),
                entry.description,
                entry.started_at.to_rfc3339(),
                entry.ended_at.map(|d| d.to_rfc3339()),
                entry.duration_minutes.map(|m| m as i32),
            ],
        )?;
        Ok(())
    }

    fn get_time_entry(&self, id: &TimeEntryId) -> StorageResult<Option<TimeEntry>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM time_entries WHERE id = ?1")?;
        let entry = stmt
            .query_row(params![id.0.to_string()], time_entry_from_row)
            .optional()?;
        Ok(entry)
    }

    fn update_time_entry(&mut self, entry: &TimeEntry) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        let rows = conn.execute(
            r"UPDATE time_entries SET
                task_id = ?2, description = ?3, started_at = ?4, ended_at = ?5, duration_minutes = ?6
            WHERE id = ?1",
            params![
                entry.id.0.to_string(),
                entry.task_id.0.to_string(),
                entry.description,
                entry.started_at.to_rfc3339(),
                entry.ended_at.map(|d| d.to_rfc3339()),
                entry.duration_minutes.map(|m| m as i32),
            ],
        )?;
        if rows == 0 {
            return Err(StorageError::not_found("TimeEntry", entry.id.0.to_string()));
        }
        Ok(())
    }

    fn delete_time_entry(&mut self, id: &TimeEntryId) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        let rows = conn.execute(
            "DELETE FROM time_entries WHERE id = ?1",
            params![id.0.to_string()],
        )?;
        if rows == 0 {
            return Err(StorageError::not_found("TimeEntry", id.0.to_string()));
        }
        Ok(())
    }

    fn get_entries_for_task(&self, task_id: &TaskId) -> StorageResult<Vec<TimeEntry>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM time_entries WHERE task_id = ?1")?;
        let entries = stmt
            .query_map(params![task_id.0.to_string()], time_entry_from_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(entries)
    }

    fn get_active_entry(&self) -> StorageResult<Option<TimeEntry>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM time_entries WHERE ended_at IS NULL LIMIT 1")?;
        let entry = stmt
            .query_row([], |row| {
                let id: String = row.get("id")?;
                let task_id: String = row.get("task_id")?;
                let started_at: String = row.get("started_at")?;
                Ok(TimeEntry {
                    id: TimeEntryId(uuid::Uuid::parse_str(&id).unwrap_or_default()),
                    task_id: TaskId(uuid::Uuid::parse_str(&task_id).unwrap_or_default()),
                    description: row.get("description")?,
                    started_at: chrono::DateTime::parse_from_rfc3339(&started_at)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    ended_at: None,
                    duration_minutes: None,
                })
            })
            .optional()?;
        Ok(entry)
    }
}

impl SqliteBackend {
    /// List all time entries (used for export).
    pub(crate) fn list_all_time_entries(&self) -> StorageResult<Vec<TimeEntry>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM time_entries")?;
        let entries: Vec<TimeEntry> = stmt
            .query_map([], time_entry_from_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(entries)
    }
}
