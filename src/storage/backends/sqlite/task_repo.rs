//! TaskRepository implementation for SQLite backend.

use rusqlite::params;

use crate::domain::{Filter, ProjectId, Task, TaskId};
use crate::storage::{StorageError, StorageResult, TaskRepository};

/// Helper to serialize a value to JSON, propagating errors.
fn json_serialize<T: serde::Serialize>(value: &T) -> StorageResult<String> {
    serde_json::to_string(value)
        .map_err(|e| StorageError::serialization(format!("JSON serialization failed: {e}")))
}

use super::rows::task_from_row;
use super::SqliteBackend;

impl TaskRepository for SqliteBackend {
    fn create_task(&mut self, task: &Task) -> StorageResult<()> {
        let conn = self.inner.conn()?;

        // Serialize JSON fields, propagating errors
        let tags_json = json_serialize(&task.tags)?;
        let deps_json = json_serialize(
            &task
                .dependencies
                .iter()
                .map(|d| d.0.to_string())
                .collect::<Vec<_>>(),
        )?;
        let custom_fields_json = json_serialize(&task.custom_fields)?;
        let recurrence_json = task.recurrence.as_ref().map(json_serialize).transpose()?;

        conn.execute(
            r"INSERT INTO tasks (
                id, title, description, status, priority, project_id, parent_task_id,
                tags, dependencies, created_at, updated_at, due_date, scheduled_date,
                completed_at, recurrence, estimated_minutes, actual_minutes, sort_order, next_task_id, custom_fields
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            params![
                task.id.0.to_string(),
                task.title,
                task.description,
                task.status.as_str(),
                task.priority.as_str(),
                task.project_id.as_ref().map(|p| p.0.to_string()),
                task.parent_task_id.as_ref().map(|t| t.0.to_string()),
                tags_json,
                deps_json,
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
                task.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
                task.scheduled_date.map(|d| d.format("%Y-%m-%d").to_string()),
                task.completed_at.map(|d| d.to_rfc3339()),
                recurrence_json,
                task.estimated_minutes.map(|m| m as i32),
                task.actual_minutes as i32,
                task.sort_order,
                task.next_task_id.as_ref().map(|t| t.0.to_string()),
                custom_fields_json,
            ],
        )?;
        // Sync tags to junction table
        self.inner.sync_task_tags(&task.id, &task.tags)?;
        Ok(())
    }

    fn get_task(&self, id: &TaskId) -> StorageResult<Option<Task>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM tasks WHERE id = ?1")?;
        let task = stmt
            .query_row(params![id.0.to_string()], task_from_row)
            .optional()?;
        Ok(task)
    }

    fn update_task(&mut self, task: &Task) -> StorageResult<()> {
        let conn = self.inner.conn()?;

        // Serialize JSON fields, propagating errors
        let tags_json = json_serialize(&task.tags)?;
        let deps_json = json_serialize(
            &task
                .dependencies
                .iter()
                .map(|d| d.0.to_string())
                .collect::<Vec<_>>(),
        )?;
        let custom_fields_json = json_serialize(&task.custom_fields)?;
        let recurrence_json = task.recurrence.as_ref().map(json_serialize).transpose()?;

        let rows = conn.execute(
            r"UPDATE tasks SET
                title = ?2, description = ?3, status = ?4, priority = ?5,
                project_id = ?6, parent_task_id = ?7, tags = ?8, dependencies = ?9,
                updated_at = ?10, due_date = ?11, scheduled_date = ?12, completed_at = ?13,
                recurrence = ?14, estimated_minutes = ?15, actual_minutes = ?16, sort_order = ?17,
                next_task_id = ?18, custom_fields = ?19
            WHERE id = ?1",
            params![
                task.id.0.to_string(),
                task.title,
                task.description,
                task.status.as_str(),
                task.priority.as_str(),
                task.project_id.as_ref().map(|p| p.0.to_string()),
                task.parent_task_id.as_ref().map(|t| t.0.to_string()),
                tags_json,
                deps_json,
                task.updated_at.to_rfc3339(),
                task.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
                task.scheduled_date
                    .map(|d| d.format("%Y-%m-%d").to_string()),
                task.completed_at.map(|d| d.to_rfc3339()),
                recurrence_json,
                task.estimated_minutes.map(|m| m as i32),
                task.actual_minutes as i32,
                task.sort_order,
                task.next_task_id.as_ref().map(|t| t.0.to_string()),
                custom_fields_json,
            ],
        )?;
        if rows == 0 {
            return Err(StorageError::not_found("Task", task.id.to_string()));
        }
        // Sync tags to junction table
        self.inner.sync_task_tags(&task.id, &task.tags)?;
        Ok(())
    }

    fn delete_task(&mut self, id: &TaskId) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        let rows = conn.execute("DELETE FROM tasks WHERE id = ?1", params![id.0.to_string()])?;
        if rows == 0 {
            return Err(StorageError::not_found("Task", id.to_string()));
        }
        Ok(())
    }

    fn list_tasks(&self) -> StorageResult<Vec<Task>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM tasks")?;
        let tasks = stmt
            .query_map([], task_from_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(tasks)
    }

    fn list_tasks_filtered(&self, filter: &Filter) -> StorageResult<Vec<Task>> {
        let conn = self.inner.conn()?;

        // Build dynamic SQL query with WHERE clauses
        // Use DISTINCT because tag JOINs can produce duplicates
        let mut sql = String::from("SELECT DISTINCT t.* FROM tasks t");
        let mut params: Vec<String> = Vec::new();

        // Tag filtering via junction table
        if let Some(tags) = &filter.tags {
            if !tags.is_empty() {
                match filter.tags_mode {
                    crate::domain::TagFilterMode::Any => {
                        // ANY mode: task has at least one of the specified tags
                        let placeholders: Vec<String> = tags
                            .iter()
                            .enumerate()
                            .map(|(i, _)| format!("?{}", params.len() + i + 1))
                            .collect();
                        sql.push_str(&format!(
                            " INNER JOIN task_tags tt ON t.id = tt.task_id AND tt.tag_name IN ({})",
                            placeholders.join(",")
                        ));
                        for tag in tags {
                            params.push(tag.clone());
                        }
                    }
                    crate::domain::TagFilterMode::All => {
                        // ALL mode: task has ALL of the specified tags
                        // Use subquery with GROUP BY and HAVING COUNT
                        let placeholders: Vec<String> = tags
                            .iter()
                            .enumerate()
                            .map(|(i, _)| format!("?{}", params.len() + i + 1))
                            .collect();
                        sql.push_str(&format!(
                            " INNER JOIN (
                                SELECT task_id FROM task_tags
                                WHERE tag_name IN ({})
                                GROUP BY task_id
                                HAVING COUNT(DISTINCT tag_name) = {}
                            ) tt ON t.id = tt.task_id",
                            placeholders.join(","),
                            tags.len()
                        ));
                        for tag in tags {
                            params.push(tag.clone());
                        }
                    }
                }
            }
        }

        sql.push_str(" WHERE 1=1");

        // Status filter
        if let Some(ref statuses) = filter.status {
            if !statuses.is_empty() {
                let placeholders: Vec<String> = statuses
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("?{}", params.len() + i + 1))
                    .collect();
                sql.push_str(&format!(" AND t.status IN ({})", placeholders.join(",")));
                for s in statuses {
                    params.push(s.as_str().to_string());
                }
            }
        }

        // Priority filter
        if let Some(ref priorities) = filter.priority {
            if !priorities.is_empty() {
                let placeholders: Vec<String> = priorities
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("?{}", params.len() + i + 1))
                    .collect();
                sql.push_str(&format!(" AND t.priority IN ({})", placeholders.join(",")));
                for p in priorities {
                    params.push(p.as_str().to_string());
                }
            }
        }

        // Project filter
        if let Some(ref project_id) = filter.project_id {
            sql.push_str(&format!(" AND t.project_id = ?{}", params.len() + 1));
            params.push(project_id.0.to_string());
        }

        // Exclude completed tasks
        if !filter.include_completed {
            sql.push_str(" AND t.status NOT IN ('done', 'cancelled')");
        }

        // Due date filters
        if let Some(ref due_before) = filter.due_before {
            sql.push_str(&format!(
                " AND t.due_date IS NOT NULL AND t.due_date < ?{}",
                params.len() + 1
            ));
            params.push(due_before.to_string());
        }
        if let Some(ref due_after) = filter.due_after {
            sql.push_str(&format!(
                " AND t.due_date IS NOT NULL AND t.due_date > ?{}",
                params.len() + 1
            ));
            params.push(due_after.to_string());
        }

        // Search text (title and description)
        if let Some(ref search) = filter.search_text {
            let pattern = format!("%{}%", search.to_lowercase());
            sql.push_str(&format!(
                " AND (LOWER(t.title) LIKE ?{} OR LOWER(COALESCE(t.description, '')) LIKE ?{})",
                params.len() + 1,
                params.len() + 2
            ));
            params.push(pattern.clone());
            params.push(pattern);
        }

        // Execute query with parameters
        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let tasks = stmt
            .query_map(param_refs.as_slice(), task_from_row)?
            .filter_map(Result::ok)
            .collect();

        Ok(tasks)
    }

    fn get_tasks_by_project(&self, project_id: &ProjectId) -> StorageResult<Vec<Task>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM tasks WHERE project_id = ?1")?;
        let tasks = stmt
            .query_map(params![project_id.0.to_string()], task_from_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(tasks)
    }

    fn get_tasks_by_tag(&self, tag: &str) -> StorageResult<Vec<Task>> {
        let conn = self.inner.conn()?;
        // Use JOIN on task_tags junction table for efficient and reliable tag queries
        let mut stmt = conn.prepare(
            r"SELECT DISTINCT t.* FROM tasks t
              INNER JOIN task_tags tt ON t.id = tt.task_id
              WHERE tt.tag_name = ?1",
        )?;
        let tasks = stmt
            .query_map(params![tag], task_from_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(tasks)
    }
}

use rusqlite::OptionalExtension;
