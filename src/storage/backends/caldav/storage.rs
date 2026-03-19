//! `StorageBackend` implementation for the CalDAV backend.
//!
//! All repository traits are implemented directly against `self.mem`
//! (an [`ExportData`] in-memory store).  Task mutations also record the
//! affected task ID so that `flush` can push only changed VTODOs to the
//! CalDAV server instead of re-uploading everything.

use crate::domain::{
    Filter, Goal, GoalId, GoalStatus, Habit, HabitId, KeyResult, KeyResultId, PomodoroConfig,
    PomodoroSession, PomodoroStats, Project, ProjectId, Tag, Task, TaskId, TimeEntry, TimeEntryId,
    WorkLogEntry, WorkLogEntryId,
};
use crate::storage::{
    backends::filter_utils::task_matches_filter, ExportData, GoalRepository, HabitRepository,
    KeyResultRepository, ProjectRepository, StorageBackend, StorageError, StorageResult,
    TagRepository, TaskRepository, TimeEntryRepository, WorkLogRepository,
};

use super::{client, CalDavBackend};

// ── TaskRepository ────────────────────────────────────────────────────────────

impl TaskRepository for CalDavBackend {
    fn create_task(&mut self, task: &Task) -> StorageResult<()> {
        self.dirty_ids.insert(task.id);
        self.mem.tasks.insert(task.id, task.clone());
        self.mem_dirty = true;
        Ok(())
    }

    fn get_task(&self, id: &TaskId) -> StorageResult<Option<Task>> {
        Ok(self.mem.tasks.get(id).cloned())
    }

    fn update_task(&mut self, task: &Task) -> StorageResult<()> {
        self.dirty_ids.insert(task.id);
        self.mem.tasks.insert(task.id, task.clone());
        self.mem_dirty = true;
        Ok(())
    }

    fn delete_task(&mut self, id: &TaskId) -> StorageResult<()> {
        self.mem.tasks.remove(id);
        self.dirty_ids.remove(id);
        self.deleted_ids.insert(*id);
        self.mem_dirty = true;
        Ok(())
    }

    fn list_tasks(&self) -> StorageResult<Vec<Task>> {
        Ok(self.mem.tasks.values().cloned().collect())
    }

    fn list_tasks_filtered(&self, filter: &Filter) -> StorageResult<Vec<Task>> {
        Ok(self
            .mem
            .tasks
            .values()
            .filter(|t| task_matches_filter(t, filter))
            .cloned()
            .collect())
    }

    fn get_tasks_by_project(&self, project_id: &ProjectId) -> StorageResult<Vec<Task>> {
        Ok(self
            .mem
            .tasks
            .values()
            .filter(|t| t.project_id.as_ref() == Some(project_id))
            .cloned()
            .collect())
    }

    fn get_tasks_by_tag(&self, tag: &str) -> StorageResult<Vec<Task>> {
        Ok(self
            .mem
            .tasks
            .values()
            .filter(|t| t.tags.iter().any(|tg| tg == tag))
            .cloned()
            .collect())
    }
}

// ── ProjectRepository ─────────────────────────────────────────────────────────

impl ProjectRepository for CalDavBackend {
    fn create_project(&mut self, project: &Project) -> StorageResult<()> {
        self.mem_dirty = true;
        let id = project.id;
        self.mem.projects.retain(|p| p.id != id);
        self.mem.projects.push(project.clone());
        Ok(())
    }

    fn get_project(&self, id: &ProjectId) -> StorageResult<Option<Project>> {
        Ok(self.mem.projects.iter().find(|p| &p.id == id).cloned())
    }

    fn update_project(&mut self, project: &Project) -> StorageResult<()> {
        self.mem_dirty = true;
        if let Some(p) = self.mem.projects.iter_mut().find(|p| p.id == project.id) {
            *p = project.clone();
        }
        Ok(())
    }

    fn delete_project(&mut self, id: &ProjectId) -> StorageResult<()> {
        self.mem_dirty = true;
        self.mem.projects.retain(|p| &p.id != id);
        Ok(())
    }

    fn list_projects(&self) -> StorageResult<Vec<Project>> {
        Ok(self.mem.projects.clone())
    }

    fn get_subprojects(&self, parent_id: &ProjectId) -> StorageResult<Vec<Project>> {
        Ok(self
            .mem
            .projects
            .iter()
            .filter(|p| p.parent_id.as_ref() == Some(parent_id))
            .cloned()
            .collect())
    }
}

// ── TagRepository ─────────────────────────────────────────────────────────────

impl TagRepository for CalDavBackend {
    fn save_tag(&mut self, tag: &Tag) -> StorageResult<()> {
        self.mem_dirty = true;
        if let Some(existing) = self.mem.tags.iter_mut().find(|t| t.name == tag.name) {
            *existing = tag.clone();
        } else {
            self.mem.tags.push(tag.clone());
        }
        Ok(())
    }

    fn get_tag(&self, name: &str) -> StorageResult<Option<Tag>> {
        Ok(self.mem.tags.iter().find(|t| t.name == name).cloned())
    }

    fn delete_tag(&mut self, name: &str) -> StorageResult<()> {
        let before = self.mem.tags.len();
        self.mem.tags.retain(|t| t.name != name);
        if self.mem.tags.len() == before {
            return Err(StorageError::not_found("Tag", name));
        }
        self.mem_dirty = true;
        Ok(())
    }

    fn list_tags(&self) -> StorageResult<Vec<Tag>> {
        Ok(self.mem.tags.clone())
    }
}

// ── TimeEntryRepository ───────────────────────────────────────────────────────

impl TimeEntryRepository for CalDavBackend {
    fn create_time_entry(&mut self, entry: &TimeEntry) -> StorageResult<()> {
        if self.mem.time_entries.iter().any(|e| e.id == entry.id) {
            return Err(StorageError::already_exists(
                "TimeEntry",
                entry.id.0.to_string(),
            ));
        }
        self.mem.time_entries.push(entry.clone());
        self.mem_dirty = true;
        Ok(())
    }

    fn get_time_entry(&self, id: &TimeEntryId) -> StorageResult<Option<TimeEntry>> {
        Ok(self.mem.time_entries.iter().find(|e| &e.id == id).cloned())
    }

    fn update_time_entry(&mut self, entry: &TimeEntry) -> StorageResult<()> {
        if let Some(existing) = self.mem.time_entries.iter_mut().find(|e| e.id == entry.id) {
            *existing = entry.clone();
            self.mem_dirty = true;
            Ok(())
        } else {
            Err(StorageError::not_found("TimeEntry", entry.id.0.to_string()))
        }
    }

    fn delete_time_entry(&mut self, id: &TimeEntryId) -> StorageResult<()> {
        let before = self.mem.time_entries.len();
        self.mem.time_entries.retain(|e| &e.id != id);
        if self.mem.time_entries.len() == before {
            return Err(StorageError::not_found("TimeEntry", id.0.to_string()));
        }
        self.mem_dirty = true;
        Ok(())
    }

    fn get_entries_for_task(&self, task_id: &TaskId) -> StorageResult<Vec<TimeEntry>> {
        Ok(self
            .mem
            .time_entries
            .iter()
            .filter(|e| &e.task_id == task_id)
            .cloned()
            .collect())
    }

    fn get_active_entry(&self) -> StorageResult<Option<TimeEntry>> {
        Ok(self
            .mem
            .time_entries
            .iter()
            .find(|e| e.is_running())
            .cloned())
    }
}

// ── WorkLogRepository ─────────────────────────────────────────────────────────

impl WorkLogRepository for CalDavBackend {
    fn create_work_log(&mut self, entry: &WorkLogEntry) -> StorageResult<()> {
        if self.mem.work_logs.iter().any(|e| e.id == entry.id) {
            return Err(StorageError::already_exists(
                "WorkLogEntry",
                entry.id.0.to_string(),
            ));
        }
        self.mem.work_logs.push(entry.clone());
        self.mem_dirty = true;
        Ok(())
    }

    fn get_work_log(&self, id: &WorkLogEntryId) -> StorageResult<Option<WorkLogEntry>> {
        Ok(self.mem.work_logs.iter().find(|e| &e.id == id).cloned())
    }

    fn update_work_log(&mut self, entry: &WorkLogEntry) -> StorageResult<()> {
        if let Some(existing) = self.mem.work_logs.iter_mut().find(|e| e.id == entry.id) {
            *existing = entry.clone();
            self.mem_dirty = true;
            Ok(())
        } else {
            Err(StorageError::not_found(
                "WorkLogEntry",
                entry.id.0.to_string(),
            ))
        }
    }

    fn delete_work_log(&mut self, id: &WorkLogEntryId) -> StorageResult<()> {
        let before = self.mem.work_logs.len();
        self.mem.work_logs.retain(|e| &e.id != id);
        if self.mem.work_logs.len() == before {
            return Err(StorageError::not_found("WorkLogEntry", id.0.to_string()));
        }
        self.mem_dirty = true;
        Ok(())
    }

    fn get_work_logs_for_task(&self, task_id: &TaskId) -> StorageResult<Vec<WorkLogEntry>> {
        let mut logs: Vec<_> = self
            .mem
            .work_logs
            .iter()
            .filter(|e| &e.task_id == task_id)
            .cloned()
            .collect();
        logs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(logs)
    }

    fn list_work_logs(&self) -> StorageResult<Vec<WorkLogEntry>> {
        Ok(self.mem.work_logs.clone())
    }
}

// ── HabitRepository ───────────────────────────────────────────────────────────

impl HabitRepository for CalDavBackend {
    fn create_habit(&mut self, habit: &Habit) -> StorageResult<()> {
        if self.mem.habits.iter().any(|h| h.id == habit.id) {
            return Err(StorageError::already_exists("Habit", habit.id.to_string()));
        }
        self.mem.habits.push(habit.clone());
        self.mem_dirty = true;
        Ok(())
    }

    fn get_habit(&self, id: &HabitId) -> StorageResult<Option<Habit>> {
        Ok(self.mem.habits.iter().find(|h| &h.id == id).cloned())
    }

    fn update_habit(&mut self, habit: &Habit) -> StorageResult<()> {
        if let Some(existing) = self.mem.habits.iter_mut().find(|h| h.id == habit.id) {
            *existing = habit.clone();
            self.mem_dirty = true;
            Ok(())
        } else {
            Err(StorageError::not_found("Habit", habit.id.to_string()))
        }
    }

    fn delete_habit(&mut self, id: &HabitId) -> StorageResult<()> {
        let before = self.mem.habits.len();
        self.mem.habits.retain(|h| &h.id != id);
        if self.mem.habits.len() == before {
            return Err(StorageError::not_found("Habit", id.to_string()));
        }
        self.mem_dirty = true;
        Ok(())
    }

    fn list_habits(&self) -> StorageResult<Vec<Habit>> {
        Ok(self.mem.habits.clone())
    }

    fn list_active_habits(&self) -> StorageResult<Vec<Habit>> {
        Ok(self
            .mem
            .habits
            .iter()
            .filter(|h| !h.archived)
            .cloned()
            .collect())
    }
}

// ── GoalRepository ────────────────────────────────────────────────────────────

impl GoalRepository for CalDavBackend {
    fn create_goal(&mut self, goal: &Goal) -> StorageResult<()> {
        if self.mem.goals.iter().any(|g| g.id == goal.id) {
            return Err(StorageError::already_exists("Goal", goal.id.to_string()));
        }
        self.mem.goals.push(goal.clone());
        self.mem_dirty = true;
        Ok(())
    }

    fn get_goal(&self, id: &GoalId) -> StorageResult<Option<Goal>> {
        Ok(self.mem.goals.iter().find(|g| &g.id == id).cloned())
    }

    fn update_goal(&mut self, goal: &Goal) -> StorageResult<()> {
        if let Some(existing) = self.mem.goals.iter_mut().find(|g| g.id == goal.id) {
            *existing = goal.clone();
            self.mem_dirty = true;
            Ok(())
        } else {
            Err(StorageError::not_found("Goal", goal.id.to_string()))
        }
    }

    fn delete_goal(&mut self, id: &GoalId) -> StorageResult<()> {
        let before = self.mem.goals.len();
        self.mem.goals.retain(|g| &g.id != id);
        if self.mem.goals.len() == before {
            return Err(StorageError::not_found("Goal", id.to_string()));
        }
        self.mem_dirty = true;
        Ok(())
    }

    fn list_goals(&self) -> StorageResult<Vec<Goal>> {
        Ok(self.mem.goals.clone())
    }

    fn list_active_goals(&self) -> StorageResult<Vec<Goal>> {
        Ok(self
            .mem
            .goals
            .iter()
            .filter(|g| g.status == GoalStatus::Active)
            .cloned()
            .collect())
    }
}

// ── KeyResultRepository ───────────────────────────────────────────────────────

impl KeyResultRepository for CalDavBackend {
    fn create_key_result(&mut self, kr: &KeyResult) -> StorageResult<()> {
        if self.mem.key_results.iter().any(|k| k.id == kr.id) {
            return Err(StorageError::already_exists("KeyResult", kr.id.to_string()));
        }
        self.mem.key_results.push(kr.clone());
        self.mem_dirty = true;
        Ok(())
    }

    fn get_key_result(&self, id: &KeyResultId) -> StorageResult<Option<KeyResult>> {
        Ok(self.mem.key_results.iter().find(|k| &k.id == id).cloned())
    }

    fn update_key_result(&mut self, kr: &KeyResult) -> StorageResult<()> {
        if let Some(existing) = self.mem.key_results.iter_mut().find(|k| k.id == kr.id) {
            *existing = kr.clone();
            self.mem_dirty = true;
            Ok(())
        } else {
            Err(StorageError::not_found("KeyResult", kr.id.to_string()))
        }
    }

    fn delete_key_result(&mut self, id: &KeyResultId) -> StorageResult<()> {
        let before = self.mem.key_results.len();
        self.mem.key_results.retain(|k| &k.id != id);
        if self.mem.key_results.len() == before {
            return Err(StorageError::not_found("KeyResult", id.to_string()));
        }
        self.mem_dirty = true;
        Ok(())
    }

    fn list_key_results(&self) -> StorageResult<Vec<KeyResult>> {
        Ok(self.mem.key_results.clone())
    }

    fn get_key_results_for_goal(&self, goal_id: &GoalId) -> StorageResult<Vec<KeyResult>> {
        Ok(self
            .mem
            .key_results
            .iter()
            .filter(|k| &k.goal_id == goal_id)
            .cloned()
            .collect())
    }
}

// ── StorageBackend ────────────────────────────────────────────────────────────

impl StorageBackend for CalDavBackend {
    fn initialize(&mut self) -> StorageResult<()> {
        match client::fetch_all_vtodos(&self.config) {
            Ok(tasks) => {
                for task in tasks {
                    self.mem.tasks.insert(task.id, task);
                }
                tracing::info!(
                    "CalDAV: loaded {} tasks from {}",
                    self.mem.tasks.len(),
                    self.config.url
                );
            }
            Err(e) => {
                // Network unavailable at startup — log and continue with empty
                // cache rather than hard-crashing.
                tracing::warn!("CalDAV: failed to fetch tasks on init: {e}");
            }
        }
        Ok(())
    }

    fn flush(&mut self) -> StorageResult<()> {
        if !self.mem_dirty && self.deleted_ids.is_empty() {
            return Ok(());
        }

        for task_id in &self.dirty_ids {
            if let Some(task) = self.mem.tasks.get(task_id) {
                if let Err(e) = client::push_vtodo(&self.config, task) {
                    tracing::warn!("CalDAV: failed to push task {}: {e}", task_id.0);
                }
            }
        }

        for task_id in &self.deleted_ids {
            if let Err(e) = client::delete_vtodo(&self.config, task_id) {
                tracing::warn!("CalDAV: failed to delete task {}: {e}", task_id.0);
            }
        }

        self.dirty_ids.clear();
        self.deleted_ids.clear();
        self.mem_dirty = false;
        Ok(())
    }

    fn export_all(&self) -> StorageResult<ExportData> {
        Ok(self.mem.clone())
    }

    fn import_all(&mut self, data: &ExportData) -> StorageResult<()> {
        for task in data.tasks.values() {
            if let Err(e) = client::push_vtodo(&self.config, task) {
                tracing::warn!("CalDAV import: failed to push '{}': {e}", task.title);
            }
        }
        self.mem = data.clone();
        self.mem_dirty = false;
        self.dirty_ids.clear();
        self.deleted_ids.clear();
        Ok(())
    }

    fn backend_type(&self) -> &'static str {
        "caldav"
    }

    fn set_pomodoro_session(&mut self, session: Option<&PomodoroSession>) -> StorageResult<()> {
        self.mem.pomodoro_session = session.cloned();
        Ok(())
    }

    fn set_pomodoro_config(&mut self, config: &PomodoroConfig) -> StorageResult<()> {
        self.mem.pomodoro_config = Some(config.clone());
        Ok(())
    }

    fn set_pomodoro_stats(&mut self, stats: &PomodoroStats) -> StorageResult<()> {
        self.mem.pomodoro_stats = Some(stats.clone());
        Ok(())
    }

    fn refresh(&mut self) -> usize {
        match client::fetch_all_vtodos(&self.config) {
            Ok(remote_tasks) => {
                let mut changed = 0usize;
                let remote_ids: std::collections::HashSet<_> =
                    remote_tasks.iter().map(|t| t.id).collect();

                for task in &remote_tasks {
                    let outdated = self
                        .mem
                        .tasks
                        .get(&task.id)
                        .is_none_or(|local| local.updated_at < task.updated_at);
                    if outdated {
                        self.mem.tasks.insert(task.id, task.clone());
                        changed += 1;
                    }
                }

                let to_remove: Vec<_> = self
                    .mem
                    .tasks
                    .keys()
                    .filter(|id| !remote_ids.contains(id))
                    .copied()
                    .collect();
                changed += to_remove.len();
                for id in to_remove {
                    self.mem.tasks.remove(&id);
                }

                changed
            }
            Err(e) => {
                tracing::warn!("CalDAV refresh failed: {e}");
                0
            }
        }
    }
}
