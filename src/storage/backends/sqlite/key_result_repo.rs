//! KeyResultRepository implementation for SQLite backend.

use rusqlite::{params, OptionalExtension};

use crate::domain::{GoalId, KeyResult, KeyResultId, KeyResultStatus};
use crate::storage::{KeyResultRepository, StorageError, StorageResult};

use super::rows::key_result_from_row;
use super::SqliteBackend;

impl KeyResultRepository for SqliteBackend {
    fn create_key_result(&mut self, kr: &KeyResult) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        let linked_project_ids_json = serde_json::to_string(
            &kr.linked_project_ids
                .iter()
                .map(|id| id.0.to_string())
                .collect::<Vec<_>>(),
        )
        .map_err(|e| StorageError::serialization(e.to_string()))?;
        let linked_task_ids_json = serde_json::to_string(
            &kr.linked_task_ids
                .iter()
                .map(|id| id.0.to_string())
                .collect::<Vec<_>>(),
        )
        .map_err(|e| StorageError::serialization(e.to_string()))?;

        conn.execute(
            r"INSERT INTO key_results (id, goal_id, name, description, status,
                target_value, current_value, unit, manual_progress,
                linked_project_ids, linked_task_ids, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                kr.id.0.to_string(),
                kr.goal_id.0.to_string(),
                kr.name,
                kr.description,
                key_result_status_to_str(kr.status),
                kr.target_value,
                kr.current_value,
                kr.unit,
                kr.manual_progress.map(i32::from),
                linked_project_ids_json,
                linked_task_ids_json,
                kr.created_at.to_rfc3339(),
                kr.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    fn get_key_result(&self, id: &KeyResultId) -> StorageResult<Option<KeyResult>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM key_results WHERE id = ?1")?;

        Ok(stmt
            .query_row(params![id.0.to_string()], key_result_from_row)
            .optional()?)
    }

    fn update_key_result(&mut self, kr: &KeyResult) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        let linked_project_ids_json = serde_json::to_string(
            &kr.linked_project_ids
                .iter()
                .map(|id| id.0.to_string())
                .collect::<Vec<_>>(),
        )
        .map_err(|e| StorageError::serialization(e.to_string()))?;
        let linked_task_ids_json = serde_json::to_string(
            &kr.linked_task_ids
                .iter()
                .map(|id| id.0.to_string())
                .collect::<Vec<_>>(),
        )
        .map_err(|e| StorageError::serialization(e.to_string()))?;

        let rows = conn.execute(
            r"UPDATE key_results SET goal_id = ?2, name = ?3, description = ?4,
                status = ?5, target_value = ?6, current_value = ?7, unit = ?8,
                manual_progress = ?9, linked_project_ids = ?10, linked_task_ids = ?11,
                updated_at = ?12
            WHERE id = ?1",
            params![
                kr.id.0.to_string(),
                kr.goal_id.0.to_string(),
                kr.name,
                kr.description,
                key_result_status_to_str(kr.status),
                kr.target_value,
                kr.current_value,
                kr.unit,
                kr.manual_progress.map(i32::from),
                linked_project_ids_json,
                linked_task_ids_json,
                kr.updated_at.to_rfc3339(),
            ],
        )?;

        if rows == 0 {
            return Err(StorageError::not_found("KeyResult", kr.id.0.to_string()));
        }

        Ok(())
    }

    fn delete_key_result(&mut self, id: &KeyResultId) -> StorageResult<()> {
        let conn = self.inner.conn()?;

        let rows = conn.execute(
            "DELETE FROM key_results WHERE id = ?1",
            params![id.0.to_string()],
        )?;

        if rows == 0 {
            return Err(StorageError::not_found("KeyResult", id.0.to_string()));
        }

        Ok(())
    }

    fn list_key_results(&self) -> StorageResult<Vec<KeyResult>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM key_results ORDER BY name")?;

        let key_results: Vec<KeyResult> = stmt
            .query_map([], key_result_from_row)?
            .filter_map(Result::ok)
            .collect();

        Ok(key_results)
    }

    fn get_key_results_for_goal(&self, goal_id: &GoalId) -> StorageResult<Vec<KeyResult>> {
        let conn = self.inner.conn()?;
        let mut stmt =
            conn.prepare("SELECT * FROM key_results WHERE goal_id = ?1 ORDER BY name")?;

        let key_results: Vec<KeyResult> = stmt
            .query_map(params![goal_id.0.to_string()], key_result_from_row)?
            .filter_map(Result::ok)
            .collect();

        Ok(key_results)
    }
}

fn key_result_status_to_str(status: KeyResultStatus) -> &'static str {
    match status {
        KeyResultStatus::NotStarted => "not_started",
        KeyResultStatus::InProgress => "in_progress",
        KeyResultStatus::AtRisk => "at_risk",
        KeyResultStatus::Completed => "completed",
    }
}
