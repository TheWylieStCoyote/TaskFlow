use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::{
    Filter, PomodoroConfig, PomodoroSession, PomodoroStats, Priority, Project, ProjectId,
    ProjectStatus, Tag, Task, TaskId, TaskStatus, TimeEntry, TimeEntryId,
};
use crate::storage::{
    ExportData, ProjectRepository, StorageBackend, StorageError, StorageResult, TagRepository,
    TaskRepository, TimeEntryRepository,
};

/// `SQLite` database storage backend
///
/// Best for larger datasets with complex queries. Provides ACID guarantees
/// and efficient indexing.
pub struct SqliteBackend {
    path: PathBuf,
    conn: Option<Connection>,
}

impl SqliteBackend {
    /// Creates a new `SQLite` backend at the given path.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`] if the backend cannot be created.
    pub fn new(path: &Path) -> StorageResult<Self> {
        Ok(Self {
            path: path.to_path_buf(),
            conn: None,
        })
    }

    fn conn(&self) -> StorageResult<&Connection> {
        self.conn.as_ref().ok_or(StorageError::NotInitialized)
    }

    fn create_tables(&self) -> StorageResult<()> {
        let conn = self.conn()?;

        conn.execute_batch(
            r"
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL DEFAULT 'todo',
                priority TEXT NOT NULL DEFAULT 'none',
                project_id TEXT,
                parent_task_id TEXT,
                tags TEXT NOT NULL DEFAULT '[]',
                dependencies TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                due_date TEXT,
                scheduled_date TEXT,
                completed_at TEXT,
                recurrence TEXT,
                estimated_minutes INTEGER,
                actual_minutes INTEGER NOT NULL DEFAULT 0,
                sort_order INTEGER,
                next_task_id TEXT,
                custom_fields TEXT NOT NULL DEFAULT '{}'
            );

            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                parent_id TEXT,
                color TEXT,
                icon TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                start_date TEXT,
                due_date TEXT,
                default_tags TEXT NOT NULL DEFAULT '[]',
                custom_fields TEXT NOT NULL DEFAULT '{}'
            );

            CREATE TABLE IF NOT EXISTS tags (
                name TEXT PRIMARY KEY,
                color TEXT,
                description TEXT
            );

            CREATE TABLE IF NOT EXISTS time_entries (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                description TEXT,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                duration_minutes INTEGER
            );

            CREATE INDEX IF NOT EXISTS idx_tasks_project ON tasks(project_id);
            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
            CREATE INDEX IF NOT EXISTS idx_tasks_due_date ON tasks(due_date);
            CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority);
            CREATE INDEX IF NOT EXISTS idx_time_entries_task ON time_entries(task_id);

            -- Junction table for normalized task-tag relationships
            -- Note: No foreign key on tag_name because tasks can use tags that
            -- aren't in the tags table (tags table only stores metadata like colors)
            CREATE TABLE IF NOT EXISTS task_tags (
                task_id TEXT NOT NULL,
                tag_name TEXT NOT NULL,
                PRIMARY KEY (task_id, tag_name),
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_task_tags_tag ON task_tags(tag_name);

            CREATE TABLE IF NOT EXISTS pomodoro_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )?;

        Ok(())
    }

    /// Migrate existing JSON tags to the junction table.
    ///
    /// This is idempotent - it only inserts tags that aren't already in the junction table.
    fn migrate_tags_to_junction_table(&self) -> StorageResult<()> {
        let conn = self.conn()?;

        // Check if migration is needed by seeing if task_tags is empty but tasks have tags
        let task_tags_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM task_tags", [], |row| row.get(0))?;
        let tasks_with_tags_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE tags != '[]' AND tags IS NOT NULL",
            [],
            |row| row.get(0),
        )?;

        // If junction table has data or no tasks have tags, skip migration
        if task_tags_count > 0 || tasks_with_tags_count == 0 {
            return Ok(());
        }

        // Migrate tags from JSON to junction table
        let mut stmt = conn.prepare("SELECT id, tags FROM tasks WHERE tags != '[]'")?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(Result::ok)
            .collect();

        for (task_id, tags_json) in rows {
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            for tag in tags {
                // Use INSERT OR IGNORE to handle duplicates gracefully
                conn.execute(
                    "INSERT OR IGNORE INTO task_tags (task_id, tag_name) VALUES (?1, ?2)",
                    params![task_id, tag],
                )?;
            }
        }

        Ok(())
    }

    /// Sync task tags to the junction table.
    ///
    /// Removes old tags and inserts new ones for the given task.
    fn sync_task_tags(&self, task_id: &TaskId, tags: &[String]) -> StorageResult<()> {
        let conn = self.conn()?;
        let task_id_str = task_id.0.to_string();

        // Delete existing tags for this task
        conn.execute(
            "DELETE FROM task_tags WHERE task_id = ?1",
            params![task_id_str],
        )?;

        // Insert new tags
        for tag in tags {
            conn.execute(
                "INSERT OR IGNORE INTO task_tags (task_id, tag_name) VALUES (?1, ?2)",
                params![task_id_str, tag],
            )?;
        }

        Ok(())
    }

    fn list_all_time_entries(&self) -> StorageResult<Vec<TimeEntry>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM time_entries")?;
        let entries: Vec<TimeEntry> = stmt
            .query_map([], |row| {
                let id: String = row.get("id")?;
                let task_id: String = row.get("task_id")?;
                let started_at: String = row.get("started_at")?;
                let ended_at: Option<String> = row.get("ended_at")?;
                Ok(TimeEntry {
                    id: TimeEntryId(uuid::Uuid::parse_str(&id).unwrap_or_default()),
                    task_id: TaskId(uuid::Uuid::parse_str(&task_id).unwrap_or_default()),
                    description: row.get("description")?,
                    started_at: chrono::DateTime::parse_from_rfc3339(&started_at)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    ended_at: ended_at.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .ok()
                    }),
                    duration_minutes: row
                        .get::<_, Option<i32>>("duration_minutes")?
                        .map(|m| m as u32),
                })
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(entries)
    }

    fn get_pomodoro_value<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> StorageResult<Option<T>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT value FROM pomodoro_state WHERE key = ?1")?;
        let value: Option<String> = stmt.query_row(params![key], |row| row.get(0)).optional()?;
        match value {
            Some(json) => Ok(serde_json::from_str(&json)?),
            None => Ok(None),
        }
    }

    fn set_pomodoro_value<T: serde::Serialize>(
        &self,
        key: &str,
        value: Option<&T>,
    ) -> StorageResult<()> {
        let conn = self.conn()?;
        match value {
            Some(v) => {
                let json = serde_json::to_string(v)
                    .map_err(|e| StorageError::serialization(e.to_string()))?;
                conn.execute(
                    "INSERT OR REPLACE INTO pomodoro_state (key, value) VALUES (?1, ?2)",
                    params![key, json],
                )?;
            }
            None => {
                conn.execute("DELETE FROM pomodoro_state WHERE key = ?1", params![key])?;
            }
        }
        Ok(())
    }

    fn task_from_row(row: &rusqlite::Row) -> rusqlite::Result<Task> {
        let id: String = row.get("id")?;
        let status_str: String = row.get("status")?;
        let priority_str: String = row.get("priority")?;
        let project_id: Option<String> = row.get("project_id")?;
        let parent_task_id: Option<String> = row.get("parent_task_id")?;
        let tags_json: String = row.get("tags")?;
        let deps_json: String = row.get("dependencies")?;
        let created_at: String = row.get("created_at")?;
        let updated_at: String = row.get("updated_at")?;
        let due_date: Option<String> = row.get("due_date")?;
        let scheduled_date: Option<String> = row.get("scheduled_date")?;
        let completed_at: Option<String> = row.get("completed_at")?;
        let recurrence_json: Option<String> = row.get("recurrence")?;
        let custom_fields_json: String = row.get("custom_fields")?;

        Ok(Task {
            id: TaskId(uuid::Uuid::parse_str(&id).unwrap_or_default()),
            title: row.get("title")?,
            description: row.get("description")?,
            status: match status_str.as_str() {
                "in_progress" => TaskStatus::InProgress,
                "blocked" => TaskStatus::Blocked,
                "done" => TaskStatus::Done,
                "cancelled" => TaskStatus::Cancelled,
                _ => TaskStatus::Todo,
            },
            priority: match priority_str.as_str() {
                "low" => Priority::Low,
                "medium" => Priority::Medium,
                "high" => Priority::High,
                "urgent" => Priority::Urgent,
                _ => Priority::None,
            },
            project_id: project_id
                .map(|s| ProjectId(uuid::Uuid::parse_str(&s).unwrap_or_default())),
            parent_task_id: parent_task_id
                .map(|s| TaskId(uuid::Uuid::parse_str(&s).unwrap_or_default())),
            tags: serde_json::from_str(&tags_json).unwrap_or_default(),
            dependencies: serde_json::from_str::<Vec<String>>(&deps_json)
                .unwrap_or_default()
                .into_iter()
                .map(|s| TaskId(uuid::Uuid::parse_str(&s).unwrap_or_default()))
                .collect(),
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            due_date: due_date.and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
            scheduled_date: scheduled_date
                .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
            completed_at: completed_at.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .ok()
            }),
            recurrence: recurrence_json.and_then(|s| serde_json::from_str(&s).ok()),
            estimated_minutes: row.get("estimated_minutes")?,
            actual_minutes: row.get::<_, i32>("actual_minutes")? as u32,
            sort_order: row.get("sort_order").ok().flatten(),
            next_task_id: row
                .get::<_, Option<String>>("next_task_id")
                .ok()
                .flatten()
                .and_then(|s| uuid::Uuid::parse_str(&s).ok())
                .map(TaskId),
            custom_fields: serde_json::from_str(&custom_fields_json).unwrap_or_default(),
            snooze_until: row
                .get::<_, Option<String>>("snooze_until")
                .ok()
                .flatten()
                .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        })
    }

    fn project_from_row(row: &rusqlite::Row) -> rusqlite::Result<Project> {
        let id: String = row.get("id")?;
        let status_str: String = row.get("status")?;
        let parent_id: Option<String> = row.get("parent_id")?;
        let created_at: String = row.get("created_at")?;
        let updated_at: String = row.get("updated_at")?;
        let start_date: Option<String> = row.get("start_date")?;
        let due_date: Option<String> = row.get("due_date")?;
        let default_tags_json: String = row.get("default_tags")?;
        let custom_fields_json: String = row.get("custom_fields")?;

        Ok(Project {
            id: ProjectId(uuid::Uuid::parse_str(&id).unwrap_or_default()),
            name: row.get("name")?,
            description: row.get("description")?,
            status: match status_str.as_str() {
                "on_hold" => ProjectStatus::OnHold,
                "completed" => ProjectStatus::Completed,
                "archived" => ProjectStatus::Archived,
                _ => ProjectStatus::Active,
            },
            parent_id: parent_id.map(|s| ProjectId(uuid::Uuid::parse_str(&s).unwrap_or_default())),
            color: row.get("color")?,
            icon: row.get("icon")?,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            start_date: start_date
                .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
            due_date: due_date.and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
            default_tags: serde_json::from_str(&default_tags_json).unwrap_or_default(),
            custom_fields: serde_json::from_str(&custom_fields_json).unwrap_or_default(),
        })
    }
}

impl TaskRepository for SqliteBackend {
    fn create_task(&mut self, task: &Task) -> StorageResult<()> {
        let conn = self.conn()?;
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
                serde_json::to_string(&task.tags).unwrap_or_default(),
                serde_json::to_string(&task.dependencies.iter().map(|d| d.0.to_string()).collect::<Vec<_>>()).unwrap_or_default(),
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
                task.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
                task.scheduled_date.map(|d| d.format("%Y-%m-%d").to_string()),
                task.completed_at.map(|d| d.to_rfc3339()),
                task.recurrence.as_ref().and_then(|r| serde_json::to_string(r).ok()),
                task.estimated_minutes.map(|m| m as i32),
                task.actual_minutes as i32,
                task.sort_order,
                task.next_task_id.as_ref().map(|t| t.0.to_string()),
                serde_json::to_string(&task.custom_fields).unwrap_or_default(),
            ],
        )?;
        // Sync tags to junction table
        self.sync_task_tags(&task.id, &task.tags)?;
        Ok(())
    }

    fn get_task(&self, id: &TaskId) -> StorageResult<Option<Task>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM tasks WHERE id = ?1")?;
        let task = stmt
            .query_row(params![id.0.to_string()], Self::task_from_row)
            .optional()?;
        Ok(task)
    }

    fn update_task(&mut self, task: &Task) -> StorageResult<()> {
        let conn = self.conn()?;
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
                serde_json::to_string(&task.tags).unwrap_or_default(),
                serde_json::to_string(
                    &task
                        .dependencies
                        .iter()
                        .map(|d| d.0.to_string())
                        .collect::<Vec<_>>()
                )
                .unwrap_or_default(),
                task.updated_at.to_rfc3339(),
                task.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
                task.scheduled_date
                    .map(|d| d.format("%Y-%m-%d").to_string()),
                task.completed_at.map(|d| d.to_rfc3339()),
                task.recurrence
                    .as_ref()
                    .and_then(|r| serde_json::to_string(r).ok()),
                task.estimated_minutes.map(|m| m as i32),
                task.actual_minutes as i32,
                task.sort_order,
                task.next_task_id.as_ref().map(|t| t.0.to_string()),
                serde_json::to_string(&task.custom_fields).unwrap_or_default(),
            ],
        )?;
        if rows == 0 {
            return Err(StorageError::not_found("Task", task.id.to_string()));
        }
        // Sync tags to junction table
        self.sync_task_tags(&task.id, &task.tags)?;
        Ok(())
    }

    fn delete_task(&mut self, id: &TaskId) -> StorageResult<()> {
        let conn = self.conn()?;
        let rows = conn.execute("DELETE FROM tasks WHERE id = ?1", params![id.0.to_string()])?;
        if rows == 0 {
            return Err(StorageError::not_found("Task", id.to_string()));
        }
        Ok(())
    }

    fn list_tasks(&self) -> StorageResult<Vec<Task>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM tasks")?;
        let tasks = stmt
            .query_map([], Self::task_from_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(tasks)
    }

    fn list_tasks_filtered(&self, filter: &Filter) -> StorageResult<Vec<Task>> {
        let conn = self.conn()?;

        // Build dynamic SQL query with WHERE clauses
        // Use DISTINCT because tag JOINs can produce duplicates
        let mut sql = String::from("SELECT DISTINCT t.* FROM tasks t");
        let mut params: Vec<String> = Vec::new();

        // Tag filtering via junction table
        let tag_filter_active = filter.tags.as_ref().is_some_and(|tags| !tags.is_empty());

        if tag_filter_active {
            let tags = filter.tags.as_ref().unwrap();
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
            .query_map(param_refs.as_slice(), Self::task_from_row)?
            .filter_map(Result::ok)
            .collect();

        Ok(tasks)
    }

    fn get_tasks_by_project(&self, project_id: &ProjectId) -> StorageResult<Vec<Task>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM tasks WHERE project_id = ?1")?;
        let tasks = stmt
            .query_map(params![project_id.0.to_string()], Self::task_from_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(tasks)
    }

    fn get_tasks_by_tag(&self, tag: &str) -> StorageResult<Vec<Task>> {
        let conn = self.conn()?;
        // Use JOIN on task_tags junction table for efficient and reliable tag queries
        let mut stmt = conn.prepare(
            r"SELECT DISTINCT t.* FROM tasks t
              INNER JOIN task_tags tt ON t.id = tt.task_id
              WHERE tt.tag_name = ?1",
        )?;
        let tasks = stmt
            .query_map(params![tag], Self::task_from_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(tasks)
    }
}

impl ProjectRepository for SqliteBackend {
    fn create_project(&mut self, project: &Project) -> StorageResult<()> {
        let conn = self.conn()?;
        conn.execute(
            r"INSERT INTO projects (
                id, name, description, status, parent_id, color, icon,
                created_at, updated_at, start_date, due_date, default_tags, custom_fields
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
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
            ],
        )?;
        Ok(())
    }

    fn get_project(&self, id: &ProjectId) -> StorageResult<Option<Project>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM projects WHERE id = ?1")?;
        let project = stmt
            .query_row(params![id.0.to_string()], Self::project_from_row)
            .optional()?;
        Ok(project)
    }

    fn update_project(&mut self, project: &Project) -> StorageResult<()> {
        let conn = self.conn()?;
        let rows = conn.execute(
            r"UPDATE projects SET
                name = ?2, description = ?3, status = ?4, parent_id = ?5, color = ?6, icon = ?7,
                updated_at = ?8, start_date = ?9, due_date = ?10, default_tags = ?11, custom_fields = ?12
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
            ],
        )?;
        if rows == 0 {
            return Err(StorageError::not_found("Project", project.id.to_string()));
        }
        Ok(())
    }

    fn delete_project(&mut self, id: &ProjectId) -> StorageResult<()> {
        let conn = self.conn()?;
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
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM projects")?;
        let projects = stmt
            .query_map([], Self::project_from_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(projects)
    }

    fn get_subprojects(&self, parent_id: &ProjectId) -> StorageResult<Vec<Project>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM projects WHERE parent_id = ?1")?;
        let projects = stmt
            .query_map(params![parent_id.0.to_string()], Self::project_from_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(projects)
    }
}

impl TagRepository for SqliteBackend {
    fn save_tag(&mut self, tag: &Tag) -> StorageResult<()> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO tags (name, color, description) VALUES (?1, ?2, ?3)",
            params![tag.name, tag.color, tag.description],
        )?;
        Ok(())
    }

    fn get_tag(&self, name: &str) -> StorageResult<Option<Tag>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM tags WHERE name = ?1")?;
        let tag = stmt
            .query_row(params![name], |row| {
                Ok(Tag {
                    name: row.get("name")?,
                    color: row.get("color")?,
                    description: row.get("description")?,
                })
            })
            .optional()?;
        Ok(tag)
    }

    fn delete_tag(&mut self, name: &str) -> StorageResult<()> {
        let conn = self.conn()?;
        let rows = conn.execute("DELETE FROM tags WHERE name = ?1", params![name])?;
        if rows == 0 {
            return Err(StorageError::not_found("Tag", name));
        }
        Ok(())
    }

    fn list_tags(&self) -> StorageResult<Vec<Tag>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM tags")?;
        let tags = stmt
            .query_map([], |row| {
                Ok(Tag {
                    name: row.get("name")?,
                    color: row.get("color")?,
                    description: row.get("description")?,
                })
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(tags)
    }
}

impl TimeEntryRepository for SqliteBackend {
    fn create_time_entry(&mut self, entry: &TimeEntry) -> StorageResult<()> {
        let conn = self.conn()?;
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
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM time_entries WHERE id = ?1")?;
        let entry = stmt
            .query_row(params![id.0.to_string()], |row| {
                let id: String = row.get("id")?;
                let task_id: String = row.get("task_id")?;
                let started_at: String = row.get("started_at")?;
                let ended_at: Option<String> = row.get("ended_at")?;
                Ok(TimeEntry {
                    id: TimeEntryId(uuid::Uuid::parse_str(&id).unwrap_or_default()),
                    task_id: TaskId(uuid::Uuid::parse_str(&task_id).unwrap_or_default()),
                    description: row.get("description")?,
                    started_at: chrono::DateTime::parse_from_rfc3339(&started_at)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    ended_at: ended_at.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .ok()
                    }),
                    duration_minutes: row
                        .get::<_, Option<i32>>("duration_minutes")?
                        .map(|m| m as u32),
                })
            })
            .optional()?;
        Ok(entry)
    }

    fn update_time_entry(&mut self, entry: &TimeEntry) -> StorageResult<()> {
        let conn = self.conn()?;
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
        let conn = self.conn()?;
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
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM time_entries WHERE task_id = ?1")?;
        let entries = stmt
            .query_map(params![task_id.0.to_string()], |row| {
                let id: String = row.get("id")?;
                let task_id: String = row.get("task_id")?;
                let started_at: String = row.get("started_at")?;
                let ended_at: Option<String> = row.get("ended_at")?;
                Ok(TimeEntry {
                    id: TimeEntryId(uuid::Uuid::parse_str(&id).unwrap_or_default()),
                    task_id: TaskId(uuid::Uuid::parse_str(&task_id).unwrap_or_default()),
                    description: row.get("description")?,
                    started_at: chrono::DateTime::parse_from_rfc3339(&started_at)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    ended_at: ended_at.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .ok()
                    }),
                    duration_minutes: row
                        .get::<_, Option<i32>>("duration_minutes")?
                        .map(|m| m as u32),
                })
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(entries)
    }

    fn get_active_entry(&self) -> StorageResult<Option<TimeEntry>> {
        let conn = self.conn()?;
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

impl StorageBackend for SqliteBackend {
    fn initialize(&mut self) -> StorageResult<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| StorageError::io(parent, e))?;
        }
        self.conn = Some(Connection::open(&self.path)?);
        self.create_tables()?;
        // Migrate existing JSON tags to junction table (idempotent)
        self.migrate_tags_to_junction_table()
    }

    fn flush(&mut self) -> StorageResult<()> {
        // SQLite auto-commits, no explicit flush needed
        Ok(())
    }

    fn export_all(&self) -> StorageResult<ExportData> {
        Ok(ExportData {
            tasks: self.list_tasks()?,
            projects: self.list_projects()?,
            tags: self.list_tags()?,
            time_entries: self.list_all_time_entries()?,
            version: 1,
            pomodoro_session: self.get_pomodoro_value("session")?,
            pomodoro_config: self.get_pomodoro_value("config")?,
            pomodoro_stats: self.get_pomodoro_value("stats")?,
        })
    }

    fn import_all(&mut self, data: &ExportData) -> StorageResult<()> {
        let conn = self.conn()?;

        // Clear existing data
        conn.execute_batch(
            "DELETE FROM time_entries; DELETE FROM tasks; DELETE FROM projects; DELETE FROM tags;",
        )?;

        // Import data
        for project in &data.projects {
            self.create_project(project)?;
        }
        for task in &data.tasks {
            self.create_task(task)?;
        }
        for tag in &data.tags {
            self.save_tag(tag)?;
        }
        for entry in &data.time_entries {
            self.create_time_entry(entry)?;
        }

        // Import Pomodoro state
        self.set_pomodoro_session(data.pomodoro_session.as_ref())?;
        if let Some(ref config) = data.pomodoro_config {
            self.set_pomodoro_config(config)?;
        }
        if let Some(ref stats) = data.pomodoro_stats {
            self.set_pomodoro_stats(stats)?;
        }

        Ok(())
    }

    fn backend_type(&self) -> &'static str {
        "sqlite"
    }

    fn set_pomodoro_session(&mut self, session: Option<&PomodoroSession>) -> StorageResult<()> {
        self.set_pomodoro_value("session", session)
    }

    fn set_pomodoro_config(&mut self, config: &PomodoroConfig) -> StorageResult<()> {
        self.set_pomodoro_value("config", Some(config))
    }

    fn set_pomodoro_stats(&mut self, stats: &PomodoroStats) -> StorageResult<()> {
        self.set_pomodoro_value("stats", Some(stats))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_backend() -> (tempfile::TempDir, SqliteBackend) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let mut backend = SqliteBackend::new(&path).unwrap();
        backend.initialize().unwrap();
        (dir, backend)
    }

    #[test]
    fn test_sqlite_initialize_creates_tables() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let mut backend = SqliteBackend::new(&path).unwrap();
        backend.initialize().unwrap();

        let conn = backend.conn().unwrap();

        // Check tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        assert!(tables.contains(&"tasks".to_string()));
        assert!(tables.contains(&"projects".to_string()));
        assert!(tables.contains(&"tags".to_string()));
        assert!(tables.contains(&"time_entries".to_string()));
    }

    #[test]
    fn test_sqlite_task_crud() {
        let (_dir, mut backend) = create_test_backend();

        // Create
        let task = Task::new("Test task");
        backend.create_task(&task).unwrap();

        // Read
        let retrieved = backend.get_task(&task.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test task");

        // Update
        let mut updated_task = task.clone();
        updated_task.title = "Updated task".to_string();
        backend.update_task(&updated_task).unwrap();

        let retrieved = backend.get_task(&task.id).unwrap().unwrap();
        assert_eq!(retrieved.title, "Updated task");

        // Delete
        backend.delete_task(&task.id).unwrap();
        assert!(backend.get_task(&task.id).unwrap().is_none());
    }

    #[test]
    fn test_sqlite_uuid_roundtrip() {
        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("UUID test");
        let original_id = task.id.clone();
        backend.create_task(&task).unwrap();

        let retrieved = backend.get_task(&original_id).unwrap().unwrap();
        assert_eq!(retrieved.id, original_id);
    }

    #[test]
    fn test_sqlite_datetime_roundtrip() {
        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("DateTime test");
        let original_created = task.created_at;
        backend.create_task(&task).unwrap();

        let retrieved = backend.get_task(&task.id).unwrap().unwrap();
        // Compare timestamps at second precision (RFC3339 may lose subseconds)
        assert_eq!(
            retrieved.created_at.timestamp(),
            original_created.timestamp()
        );
    }

    #[test]
    fn test_sqlite_enum_roundtrip() {
        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("Enum test")
            .with_priority(Priority::Urgent)
            .with_status(TaskStatus::InProgress);
        backend.create_task(&task).unwrap();

        let retrieved = backend.get_task(&task.id).unwrap().unwrap();
        assert_eq!(retrieved.priority, Priority::Urgent);
        assert_eq!(retrieved.status, TaskStatus::InProgress);
    }

    #[test]
    fn test_sqlite_json_fields_roundtrip() {
        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("JSON test").with_tags(vec!["tag1".to_string(), "tag2".to_string()]);
        backend.create_task(&task).unwrap();

        let retrieved = backend.get_task(&task.id).unwrap().unwrap();
        assert_eq!(retrieved.tags.len(), 2);
        assert!(retrieved.tags.contains(&"tag1".to_string()));
        assert!(retrieved.tags.contains(&"tag2".to_string()));
    }

    #[test]
    fn test_sqlite_null_optional_fields() {
        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("Null fields test");
        // task has None for: description, project_id, parent_task_id, due_date, etc.
        backend.create_task(&task).unwrap();

        let retrieved = backend.get_task(&task.id).unwrap().unwrap();
        assert!(retrieved.description.is_none());
        assert!(retrieved.project_id.is_none());
        assert!(retrieved.due_date.is_none());
        assert!(retrieved.completed_at.is_none());
    }

    #[test]
    fn test_sqlite_project_crud() {
        let (_dir, mut backend) = create_test_backend();

        let project = Project::new("Test project");
        backend.create_project(&project).unwrap();

        let retrieved = backend.get_project(&project.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test project");

        backend.delete_project(&project.id).unwrap();
        assert!(backend.get_project(&project.id).unwrap().is_none());
    }

    #[test]
    fn test_sqlite_tag_crud() {
        let (_dir, mut backend) = create_test_backend();

        let tag = Tag {
            name: "test-tag".to_string(),
            color: Some("#ff0000".to_string()),
            description: Some("A test tag".to_string()),
        };

        backend.save_tag(&tag).unwrap();

        let retrieved = backend.get_tag("test-tag").unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.color, Some("#ff0000".to_string()));
        assert_eq!(retrieved.description, Some("A test tag".to_string()));

        backend.delete_tag("test-tag").unwrap();
        assert!(backend.get_tag("test-tag").unwrap().is_none());
    }

    #[test]
    fn test_sqlite_time_entry_crud() {
        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("Task for time entry");
        backend.create_task(&task).unwrap();

        let entry = TimeEntry::start(task.id.clone());
        backend.create_time_entry(&entry).unwrap();

        let retrieved = backend.get_time_entry(&entry.id).unwrap();
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().is_running());

        backend.delete_time_entry(&entry.id).unwrap();
        assert!(backend.get_time_entry(&entry.id).unwrap().is_none());
    }

    #[test]
    fn test_sqlite_get_tasks_by_project() {
        let (_dir, mut backend) = create_test_backend();

        let project = Project::new("Test project");
        backend.create_project(&project).unwrap();

        let task1 = Task::new("Task 1").with_project(project.id.clone());
        let task2 = Task::new("Task 2").with_project(project.id.clone());
        let task3 = Task::new("Task 3"); // No project

        backend.create_task(&task1).unwrap();
        backend.create_task(&task2).unwrap();
        backend.create_task(&task3).unwrap();

        let project_tasks = backend.get_tasks_by_project(&project.id).unwrap();
        assert_eq!(project_tasks.len(), 2);
    }

    #[test]
    fn test_sqlite_get_active_entry() {
        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("Task");
        backend.create_task(&task).unwrap();

        // No active entry initially
        assert!(backend.get_active_entry().unwrap().is_none());

        // Start an entry
        let entry = TimeEntry::start(task.id.clone());
        backend.create_time_entry(&entry).unwrap();

        // Now there's an active entry
        let active = backend.get_active_entry().unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, entry.id);
    }

    #[test]
    fn test_sqlite_export_import_roundtrip() {
        let (_dir, mut backend) = create_test_backend();

        // Create sample data
        let task = Task::new("Test task").with_priority(Priority::High);
        let project = Project::new("Test project");
        let tag = Tag {
            name: "important".to_string(),
            color: Some("#ff0000".to_string()),
            description: None,
        };

        backend.create_task(&task).unwrap();
        backend.create_project(&project).unwrap();
        backend.save_tag(&tag).unwrap();

        // Export
        let exported = backend.export_all().unwrap();

        // Create new backend and import
        let dir2 = tempdir().unwrap();
        let path2 = dir2.path().join("import.db");
        let mut backend2 = SqliteBackend::new(&path2).unwrap();
        backend2.initialize().unwrap();
        backend2.import_all(&exported).unwrap();

        // Verify
        assert_eq!(backend2.list_tasks().unwrap().len(), 1);
        assert_eq!(backend2.list_projects().unwrap().len(), 1);
        assert_eq!(backend2.list_tags().unwrap().len(), 1);
    }

    #[test]
    fn test_sqlite_backend_type() {
        let (_dir, backend) = create_test_backend();
        assert_eq!(backend.backend_type(), "sqlite");
    }

    #[test]
    fn test_sqlite_update_task_not_found() {
        let (_dir, mut backend) = create_test_backend();

        let task = Task::new("Non-existent");
        let result = backend.update_task(&task);
        assert!(result.is_err());
    }

    #[test]
    fn test_sqlite_delete_task_not_found() {
        let (_dir, mut backend) = create_test_backend();

        let task_id = TaskId::new();
        let result = backend.delete_task(&task_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_sqlite_subprojects() {
        let (_dir, mut backend) = create_test_backend();

        let parent = Project::new("Parent");
        backend.create_project(&parent).unwrap();

        let child1 = Project::new("Child 1").with_parent(parent.id.clone());
        let child2 = Project::new("Child 2").with_parent(parent.id.clone());

        backend.create_project(&child1).unwrap();
        backend.create_project(&child2).unwrap();

        let subprojects = backend.get_subprojects(&parent.id).unwrap();
        assert_eq!(subprojects.len(), 2);
    }

    #[test]
    fn test_sqlite_get_tasks_by_tag_with_junction_table() {
        let (_dir, mut backend) = create_test_backend();

        // Create tasks with various tags
        let task1 = Task::new("Task 1").with_tags(vec!["work".to_string(), "urgent".to_string()]);
        let task2 = Task::new("Task 2").with_tags(vec!["work".to_string(), "low".to_string()]);
        let task3 = Task::new("Task 3").with_tags(vec!["personal".to_string()]);

        backend.create_task(&task1).unwrap();
        backend.create_task(&task2).unwrap();
        backend.create_task(&task3).unwrap();

        // Query by single tag
        let work_tasks = backend.get_tasks_by_tag("work").unwrap();
        assert_eq!(work_tasks.len(), 2);

        let personal_tasks = backend.get_tasks_by_tag("personal").unwrap();
        assert_eq!(personal_tasks.len(), 1);
        assert_eq!(personal_tasks[0].title, "Task 3");

        // Query non-existent tag
        let empty = backend.get_tasks_by_tag("nonexistent").unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_sqlite_tag_query_special_characters() {
        let (_dir, mut backend) = create_test_backend();

        // Create tasks with tags containing special characters
        // These would break the old LIKE-based pattern matching
        let task1 = Task::new("Task 1").with_tags(vec![
            "tag\"with\"quotes".to_string(),
            "tag,with,commas".to_string(),
        ]);
        let task2 = Task::new("Task 2")
            .with_tags(vec!["tag[with]brackets".to_string(), "100%".to_string()]);
        let task3 = Task::new("Task 3").with_tags(vec![
            "tag'with'apostrophes".to_string(),
            "tag\\with\\backslash".to_string(),
        ]);

        backend.create_task(&task1).unwrap();
        backend.create_task(&task2).unwrap();
        backend.create_task(&task3).unwrap();

        // Query by tags with special characters - should work correctly with junction table
        let quotes = backend.get_tasks_by_tag("tag\"with\"quotes").unwrap();
        assert_eq!(quotes.len(), 1);
        assert_eq!(quotes[0].title, "Task 1");

        let commas = backend.get_tasks_by_tag("tag,with,commas").unwrap();
        assert_eq!(commas.len(), 1);

        let brackets = backend.get_tasks_by_tag("tag[with]brackets").unwrap();
        assert_eq!(brackets.len(), 1);

        let percent = backend.get_tasks_by_tag("100%").unwrap();
        assert_eq!(percent.len(), 1);

        let apostrophes = backend.get_tasks_by_tag("tag'with'apostrophes").unwrap();
        assert_eq!(apostrophes.len(), 1);
    }

    #[test]
    fn test_sqlite_filtered_tags_any_mode() {
        let (_dir, mut backend) = create_test_backend();

        let task1 = Task::new("Task 1").with_tags(vec!["a".to_string(), "b".to_string()]);
        let task2 = Task::new("Task 2").with_tags(vec!["b".to_string(), "c".to_string()]);
        let task3 = Task::new("Task 3").with_tags(vec!["c".to_string(), "d".to_string()]);
        let task4 = Task::new("Task 4").with_tags(vec!["e".to_string()]);

        backend.create_task(&task1).unwrap();
        backend.create_task(&task2).unwrap();
        backend.create_task(&task3).unwrap();
        backend.create_task(&task4).unwrap();

        // Filter by tags in ANY mode (task has at least one of the tags)
        let filter = Filter {
            tags: Some(vec!["a".to_string(), "c".to_string()]),
            tags_mode: crate::domain::TagFilterMode::Any,
            include_completed: true,
            ..Default::default()
        };

        let filtered = backend.list_tasks_filtered(&filter).unwrap();
        // Should match task1 (has a), task2 (has c), task3 (has c)
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_sqlite_filtered_tags_all_mode() {
        let (_dir, mut backend) = create_test_backend();

        let task1 =
            Task::new("Task 1").with_tags(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        let task2 = Task::new("Task 2").with_tags(vec!["a".to_string(), "b".to_string()]);
        let task3 = Task::new("Task 3").with_tags(vec!["a".to_string()]);
        let task4 = Task::new("Task 4").with_tags(vec!["b".to_string(), "c".to_string()]);

        backend.create_task(&task1).unwrap();
        backend.create_task(&task2).unwrap();
        backend.create_task(&task3).unwrap();
        backend.create_task(&task4).unwrap();

        // Filter by tags in ALL mode (task has ALL of the tags)
        let filter = Filter {
            tags: Some(vec!["a".to_string(), "b".to_string()]),
            tags_mode: crate::domain::TagFilterMode::All,
            include_completed: true,
            ..Default::default()
        };

        let filtered = backend.list_tasks_filtered(&filter).unwrap();
        // Should match task1 (has a,b,c) and task2 (has a,b)
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_sqlite_tag_update_syncs_junction_table() {
        let (_dir, mut backend) = create_test_backend();

        // Create task with initial tags
        let mut task = Task::new("Test").with_tags(vec!["initial".to_string()]);
        backend.create_task(&task).unwrap();

        // Verify initial tag query works
        let initial_tasks = backend.get_tasks_by_tag("initial").unwrap();
        assert_eq!(initial_tasks.len(), 1);

        // Update task with new tags
        task.tags = vec!["updated".to_string(), "new".to_string()];
        backend.update_task(&task).unwrap();

        // Old tag should no longer match
        let old_tag_tasks = backend.get_tasks_by_tag("initial").unwrap();
        assert!(old_tag_tasks.is_empty());

        // New tags should match
        let updated_tasks = backend.get_tasks_by_tag("updated").unwrap();
        assert_eq!(updated_tasks.len(), 1);

        let new_tasks = backend.get_tasks_by_tag("new").unwrap();
        assert_eq!(new_tasks.len(), 1);
    }

    #[test]
    fn test_sqlite_task_tags_table_created() {
        let (_dir, backend) = create_test_backend();
        let conn = backend.conn().unwrap();

        // Check task_tags table exists
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        assert!(tables.contains(&"task_tags".to_string()));
    }

    #[test]
    fn test_sqlite_tag_migration_from_json() {
        // This test simulates opening an existing database with JSON tags
        // and verifying migration populates the junction table
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");

        // First, create database without migration (simulate old data)
        {
            let conn = Connection::open(&path).unwrap();
            // Create old schema without task_tags table
            conn.execute_batch(
                r"
                CREATE TABLE tasks (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    description TEXT,
                    status TEXT NOT NULL DEFAULT 'todo',
                    priority TEXT NOT NULL DEFAULT 'none',
                    project_id TEXT,
                    parent_task_id TEXT,
                    tags TEXT NOT NULL DEFAULT '[]',
                    dependencies TEXT NOT NULL DEFAULT '[]',
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    due_date TEXT,
                    scheduled_date TEXT,
                    completed_at TEXT,
                    recurrence TEXT,
                    estimated_minutes INTEGER,
                    actual_minutes INTEGER NOT NULL DEFAULT 0,
                    sort_order INTEGER,
                    next_task_id TEXT,
                    custom_fields TEXT NOT NULL DEFAULT '{}'
                );
                ",
            )
            .unwrap();

            // Insert task with JSON tags
            conn.execute(
                "INSERT INTO tasks (id, title, tags, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params!["test-id", "Test Task", r#"["tag1","tag2"]"#, "2024-01-01T00:00:00Z", "2024-01-01T00:00:00Z"],
            )
            .unwrap();
        }

        // Now open with our backend (should trigger migration)
        let mut backend = SqliteBackend::new(&path).unwrap();
        backend.initialize().unwrap();

        // Verify migration created the junction table and populated it
        let tag1_tasks = backend.get_tasks_by_tag("tag1").unwrap();
        assert_eq!(tag1_tasks.len(), 1);
        assert_eq!(tag1_tasks[0].title, "Test Task");

        let tag2_tasks = backend.get_tasks_by_tag("tag2").unwrap();
        assert_eq!(tag2_tasks.len(), 1);
    }
}
