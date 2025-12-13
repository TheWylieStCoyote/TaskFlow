//! StorageBackend implementation for SQLite.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::{
    Filter, PomodoroConfig, PomodoroSession, PomodoroStats, SavedFilter, SavedFilterId, SortSpec,
};
use crate::storage::{
    ExportData, GoalRepository, HabitRepository, KeyResultRepository, ProjectRepository,
    StorageBackend, StorageError, StorageResult, TagRepository, TaskRepository,
    TimeEntryRepository, WorkLogRepository,
};

use super::SqliteBackendInner;

/// `SQLite` database storage backend
///
/// Best for larger datasets with complex queries. Provides ACID guarantees
/// and efficient indexing.
pub struct SqliteBackend {
    pub(crate) inner: SqliteBackendInner,
}

impl SqliteBackend {
    /// Creates a new `SQLite` backend at the given path.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageError`] if the backend cannot be created.
    pub fn new(path: &Path) -> StorageResult<Self> {
        Ok(Self {
            inner: SqliteBackendInner::new(path),
        })
    }

    fn get_pomodoro_value<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> StorageResult<Option<T>> {
        let conn = self.inner.conn()?;
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
        let conn = self.inner.conn()?;
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

    fn list_saved_filters(&self) -> StorageResult<Vec<SavedFilter>> {
        let conn = self.inner.conn()?;
        let mut stmt =
            conn.prepare("SELECT id, name, filter_json, sort_json, icon FROM saved_filters")?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let filter_json: String = row.get(2)?;
            let sort_json: String = row.get(3)?;
            let icon: Option<String> = row.get(4)?;
            Ok((id, name, filter_json, sort_json, icon))
        })?;

        let mut filters = Vec::new();
        for row in rows {
            let (id, name, filter_json, sort_json, icon) = row?;
            let filter: Filter = serde_json::from_str(&filter_json)
                .map_err(|e| StorageError::serialization(e.to_string()))?;
            let sort: SortSpec = serde_json::from_str(&sort_json)
                .map_err(|e| StorageError::serialization(e.to_string()))?;
            let mut saved = SavedFilter::new(name, filter, sort);
            saved.id = SavedFilterId(id);
            if let Some(i) = icon {
                saved = saved.with_icon(i);
            }
            filters.push(saved);
        }
        Ok(filters)
    }

    fn create_saved_filter(&self, filter: &SavedFilter) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        let filter_json = serde_json::to_string(&filter.filter)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        let sort_json = serde_json::to_string(&filter.sort)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO saved_filters (id, name, filter_json, sort_json, icon) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![filter.id.0, filter.name, filter_json, sort_json, filter.icon],
        )?;
        Ok(())
    }
}

impl StorageBackend for SqliteBackend {
    fn initialize(&mut self) -> StorageResult<()> {
        if let Some(parent) = self.inner.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| StorageError::io(parent, e))?;
        }
        self.inner.conn = Some(Connection::open(&self.inner.path)?);
        self.inner.create_tables()?;
        // Run migrations (all are idempotent)
        self.inner.migrate_tags_to_junction_table()?;
        self.inner.migrate_add_estimation_multiplier()?;
        self.inner.migrate_add_scheduled_time()
    }

    fn flush(&mut self) -> StorageResult<()> {
        // SQLite auto-commits, no explicit flush needed
        Ok(())
    }

    fn export_all(&self) -> StorageResult<ExportData> {
        Ok(ExportData {
            tasks: self.list_tasks()?.into_iter().map(|t| (t.id, t)).collect(),
            projects: self.list_projects()?,
            tags: self.list_tags()?,
            time_entries: self.list_all_time_entries()?,
            work_logs: self.list_work_logs()?,
            habits: self.list_habits()?,
            goals: self.list_goals()?,
            key_results: self.list_key_results()?,
            version: 1,
            pomodoro_session: self.get_pomodoro_value("session")?,
            pomodoro_config: self.get_pomodoro_value("config")?,
            pomodoro_stats: self.get_pomodoro_value("stats")?,
            saved_filters: self.list_saved_filters()?,
        })
    }

    fn import_all(&mut self, data: &ExportData) -> StorageResult<()> {
        let conn = self.inner.conn()?;

        // Clear existing data (habits cascade to habit_check_ins, goals cascade to key_results)
        conn.execute_batch(
            "DELETE FROM work_logs; DELETE FROM time_entries; DELETE FROM tasks; DELETE FROM projects; DELETE FROM tags; DELETE FROM habits; DELETE FROM key_results; DELETE FROM goals; DELETE FROM saved_filters;",
        )?;

        // Import data
        for project in &data.projects {
            self.create_project(project)?;
        }
        for task in data.tasks.values() {
            self.create_task(task)?;
        }
        for tag in &data.tags {
            self.save_tag(tag)?;
        }
        for entry in &data.time_entries {
            self.create_time_entry(entry)?;
        }
        for entry in &data.work_logs {
            self.create_work_log(entry)?;
        }
        for habit in &data.habits {
            self.create_habit(habit)?;
        }
        for goal in &data.goals {
            self.create_goal(goal)?;
        }
        for kr in &data.key_results {
            self.create_key_result(kr)?;
        }
        for filter in &data.saved_filters {
            self.create_saved_filter(filter)?;
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
