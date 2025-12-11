//! ProjectRepository implementation for SQLite backend.

use rusqlite::{params, OptionalExtension};

use crate::domain::{Project, ProjectId};
use crate::storage::{ProjectRepository, StorageError, StorageResult};

use super::rows::project_from_row;
use super::SqliteBackend;

impl ProjectRepository for SqliteBackend {
    fn create_project(&mut self, project: &Project) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        conn.execute(
            r"INSERT INTO projects (
                id, name, description, status, parent_id, color, icon,
                created_at, updated_at, start_date, due_date, default_tags, custom_fields,
                estimation_multiplier
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                project.id.0.to_string(),
                project.name,
                project.description,
                project.status.as_str(),
                project.parent_id.as_ref().map(|p| p.0.to_string()),
                project.color,
                project.icon,
                project.created_at.to_rfc3339(),
                project.updated_at.to_rfc3339(),
                project.start_date.map(|d| d.format("%Y-%m-%d").to_string()),
                project.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
                serde_json::to_string(&project.default_tags).unwrap_or_default(),
                serde_json::to_string(&project.custom_fields).unwrap_or_default(),
                project.estimation_multiplier,
            ],
        )?;
        Ok(())
    }

    fn get_project(&self, id: &ProjectId) -> StorageResult<Option<Project>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM projects WHERE id = ?1")?;
        let project = stmt
            .query_row(params![id.0.to_string()], project_from_row)
            .optional()?;
        Ok(project)
    }

    fn update_project(&mut self, project: &Project) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        let rows = conn.execute(
            r"UPDATE projects SET
                name = ?2, description = ?3, status = ?4, parent_id = ?5, color = ?6, icon = ?7,
                updated_at = ?8, start_date = ?9, due_date = ?10, default_tags = ?11, custom_fields = ?12,
                estimation_multiplier = ?13
            WHERE id = ?1",
            params![
                project.id.0.to_string(),
                project.name,
                project.description,
                project.status.as_str(),
                project.parent_id.as_ref().map(|p| p.0.to_string()),
                project.color,
                project.icon,
                project.updated_at.to_rfc3339(),
                project.start_date.map(|d| d.format("%Y-%m-%d").to_string()),
                project.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
                serde_json::to_string(&project.default_tags).unwrap_or_default(),
                serde_json::to_string(&project.custom_fields).unwrap_or_default(),
                project.estimation_multiplier,
            ],
        )?;
        if rows == 0 {
            return Err(StorageError::not_found("Project", project.id.to_string()));
        }
        Ok(())
    }

    fn delete_project(&mut self, id: &ProjectId) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        let rows = conn.execute(
            "DELETE FROM projects WHERE id = ?1",
            params![id.0.to_string()],
        )?;
        if rows == 0 {
            return Err(StorageError::not_found("Project", id.to_string()));
        }
        Ok(())
    }

    fn list_projects(&self) -> StorageResult<Vec<Project>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM projects")?;
        let projects = stmt
            .query_map([], project_from_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(projects)
    }

    fn get_subprojects(&self, parent_id: &ProjectId) -> StorageResult<Vec<Project>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM projects WHERE parent_id = ?1")?;
        let projects = stmt
            .query_map(params![parent_id.0.to_string()], project_from_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(projects)
    }
}
