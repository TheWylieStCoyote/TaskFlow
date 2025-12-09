//! Schema creation and migrations for SQLite backend.

use rusqlite::params;

use crate::domain::TaskId;
use crate::storage::StorageResult;

use super::SqliteBackendInner;

impl SqliteBackendInner {
    /// Create database tables and indices.
    pub(crate) fn create_tables(&self) -> StorageResult<()> {
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

            CREATE TABLE IF NOT EXISTS work_logs (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_work_logs_task ON work_logs(task_id);
            CREATE INDEX IF NOT EXISTS idx_work_logs_created ON work_logs(created_at);

            CREATE TABLE IF NOT EXISTS habits (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                frequency TEXT NOT NULL,
                start_date TEXT NOT NULL,
                end_date TEXT,
                color TEXT,
                icon TEXT,
                tags TEXT NOT NULL DEFAULT '[]',
                archived INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_habits_archived ON habits(archived);

            CREATE TABLE IF NOT EXISTS habit_check_ins (
                habit_id TEXT NOT NULL,
                date TEXT NOT NULL,
                completed INTEGER NOT NULL,
                note TEXT,
                checked_at TEXT NOT NULL,
                PRIMARY KEY (habit_id, date),
                FOREIGN KEY (habit_id) REFERENCES habits(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_habit_check_ins_date ON habit_check_ins(date);
            ",
        )?;

        Ok(())
    }

    /// Migrate existing JSON tags to the junction table.
    ///
    /// This is idempotent - it only inserts tags that aren't already in the junction table.
    pub(crate) fn migrate_tags_to_junction_table(&self) -> StorageResult<()> {
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
    pub(crate) fn sync_task_tags(&self, task_id: &TaskId, tags: &[String]) -> StorageResult<()> {
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
}
