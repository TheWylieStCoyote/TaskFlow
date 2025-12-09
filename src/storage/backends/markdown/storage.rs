//! StorageBackend implementation for markdown backend.

use crate::domain::{PomodoroConfig, PomodoroSession, PomodoroStats};
use crate::storage::{
    ExportData, ProjectRepository, StorageBackend, StorageResult, TagRepository, TaskRepository,
};

use super::{MarkdownBackend, PomodoroState};

impl StorageBackend for MarkdownBackend {
    fn initialize(&mut self) -> StorageResult<()> {
        self.ensure_dirs()?;
        self.load_tasks()?;
        self.load_projects()?;
        self.load_tags()?;
        self.load_time_entries()?;
        self.load_work_logs()?;
        self.load_habits()?;
        self.load_pomodoro_state()?;
        Ok(())
    }

    fn flush(&mut self) -> StorageResult<()> {
        // Files are written immediately, but we save auxiliary data
        self.save_tags()?;
        self.save_time_entries()?;
        self.save_work_logs()?;
        self.save_habits()?;
        self.save_pomodoro_state()?;
        self.dirty = false;
        Ok(())
    }

    fn export_all(&self) -> StorageResult<ExportData> {
        Ok(ExportData {
            tasks: self.tasks_cache.values().cloned().collect(),
            projects: self.projects_cache.values().cloned().collect(),
            tags: self.tags.clone(),
            time_entries: self.time_entries.clone(),
            work_logs: self.work_logs.clone(),
            habits: self.habits.clone(),
            version: 1,
            pomodoro_session: self.pomodoro_state.session.clone(),
            pomodoro_config: self.pomodoro_state.config.clone(),
            pomodoro_stats: self.pomodoro_state.stats.clone(),
        })
    }

    #[allow(clippy::needless_collect)]
    fn import_all(&mut self, data: &ExportData) -> StorageResult<()> {
        // Clear existing data - collect needed to avoid borrow conflict
        for id in self.tasks_cache.keys().copied().collect::<Vec<_>>() {
            self.delete_task_file(&id)?;
        }
        for id in self.projects_cache.keys().copied().collect::<Vec<_>>() {
            self.delete_project_file(&id)?;
        }

        self.tasks_cache.clear();
        self.projects_cache.clear();
        self.tags.clear();
        self.time_entries.clear();
        self.work_logs.clear();
        self.habits.clear();

        // Import new data
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
            self.time_entries.push(entry.clone());
        }
        self.save_time_entries()?;

        // Import work logs
        for entry in &data.work_logs {
            self.work_logs.push(entry.clone());
        }
        self.save_work_logs()?;

        // Import habits
        for habit in &data.habits {
            self.habits.push(habit.clone());
        }
        self.save_habits()?;

        // Import Pomodoro state
        self.pomodoro_state = PomodoroState {
            session: data.pomodoro_session.clone(),
            config: data.pomodoro_config.clone(),
            stats: data.pomodoro_stats.clone(),
        };
        self.save_pomodoro_state()?;

        Ok(())
    }

    fn backend_type(&self) -> &'static str {
        "markdown"
    }

    fn set_pomodoro_session(&mut self, session: Option<&PomodoroSession>) -> StorageResult<()> {
        self.pomodoro_state.session = session.cloned();
        self.save_pomodoro_state()
    }

    fn set_pomodoro_config(&mut self, config: &PomodoroConfig) -> StorageResult<()> {
        self.pomodoro_state.config = Some(config.clone());
        self.save_pomodoro_state()
    }

    fn set_pomodoro_stats(&mut self, stats: &PomodoroStats) -> StorageResult<()> {
        self.pomodoro_state.stats = Some(stats.clone());
        self.save_pomodoro_state()
    }

    fn refresh(&mut self) -> usize {
        // Delegate to the existing refresh implementation
        let task_changes = self.scan_for_task_changes();
        let project_changes = self.scan_for_project_changes();
        task_changes + project_changes
    }
}
