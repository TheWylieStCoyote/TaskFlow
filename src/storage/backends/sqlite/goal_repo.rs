//! GoalRepository implementation for SQLite backend.

use rusqlite::{params, OptionalExtension};

use crate::domain::{Goal, GoalId, GoalStatus};
use crate::storage::{GoalRepository, StorageError, StorageResult};

use super::rows::goal_from_row;
use super::SqliteBackend;

impl GoalRepository for SqliteBackend {
    fn create_goal(&mut self, goal: &Goal) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        let quarter_json = goal
            .quarter
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| StorageError::serialization(e.to_string()))?;

        conn.execute(
            r"INSERT INTO goals (id, name, description, status, start_date, due_date,
                quarter, manual_progress, color, icon, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                goal.id.0.to_string(),
                goal.name,
                goal.description,
                goal_status_to_str(goal.status),
                goal.start_date.map(|d| d.format("%Y-%m-%d").to_string()),
                goal.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
                quarter_json,
                goal.manual_progress.map(i32::from),
                goal.color,
                goal.icon,
                goal.created_at.to_rfc3339(),
                goal.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    fn get_goal(&self, id: &GoalId) -> StorageResult<Option<Goal>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM goals WHERE id = ?1")?;

        Ok(stmt
            .query_row(params![id.0.to_string()], goal_from_row)
            .optional()?)
    }

    fn update_goal(&mut self, goal: &Goal) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        let quarter_json = goal
            .quarter
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| StorageError::serialization(e.to_string()))?;

        let rows = conn.execute(
            r"UPDATE goals SET name = ?2, description = ?3, status = ?4,
                start_date = ?5, due_date = ?6, quarter = ?7, manual_progress = ?8,
                color = ?9, icon = ?10, updated_at = ?11
            WHERE id = ?1",
            params![
                goal.id.0.to_string(),
                goal.name,
                goal.description,
                goal_status_to_str(goal.status),
                goal.start_date.map(|d| d.format("%Y-%m-%d").to_string()),
                goal.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
                quarter_json,
                goal.manual_progress.map(i32::from),
                goal.color,
                goal.icon,
                goal.updated_at.to_rfc3339(),
            ],
        )?;

        if rows == 0 {
            return Err(StorageError::not_found("Goal", goal.id.0.to_string()));
        }

        Ok(())
    }

    fn delete_goal(&mut self, id: &GoalId) -> StorageResult<()> {
        let conn = self.inner.conn()?;

        // KeyResults are deleted via ON DELETE CASCADE
        let rows = conn.execute("DELETE FROM goals WHERE id = ?1", params![id.0.to_string()])?;

        if rows == 0 {
            return Err(StorageError::not_found("Goal", id.0.to_string()));
        }

        Ok(())
    }

    fn list_goals(&self) -> StorageResult<Vec<Goal>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM goals ORDER BY name")?;

        let goals: Vec<Goal> = stmt
            .query_map([], goal_from_row)?
            .filter_map(Result::ok)
            .collect();

        Ok(goals)
    }

    fn list_active_goals(&self) -> StorageResult<Vec<Goal>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM goals WHERE status = 'active' ORDER BY name")?;

        let goals: Vec<Goal> = stmt
            .query_map([], goal_from_row)?
            .filter_map(Result::ok)
            .collect();

        Ok(goals)
    }
}

fn goal_status_to_str(status: GoalStatus) -> &'static str {
    match status {
        GoalStatus::Active => "active",
        GoalStatus::OnHold => "on_hold",
        GoalStatus::Completed => "completed",
        GoalStatus::Archived => "archived",
    }
}
