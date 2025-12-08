//! TaskRepository implementation for YAML backend.

use crate::domain::{Filter, ProjectId, Task, TaskId};
use crate::storage::backends::filter_utils::task_matches_filter;
use crate::storage::{StorageError, StorageResult, TaskRepository};

use super::YamlBackend;

impl TaskRepository for YamlBackend {
    fn create_task(&mut self, task: &Task) -> StorageResult<()> {
        if self.data.tasks.iter().any(|t| t.id == task.id) {
            return Err(StorageError::already_exists("Task", task.id.to_string()));
        }
        self.data.tasks.push(task.clone());
        self.mark_dirty();
        Ok(())
    }

    fn get_task(&self, id: &TaskId) -> StorageResult<Option<Task>> {
        Ok(self.data.tasks.iter().find(|t| &t.id == id).cloned())
    }

    fn update_task(&mut self, task: &Task) -> StorageResult<()> {
        if let Some(existing) = self.data.tasks.iter_mut().find(|t| t.id == task.id) {
            *existing = task.clone();
            self.mark_dirty();
            Ok(())
        } else {
            Err(StorageError::not_found("Task", task.id.to_string()))
        }
    }

    fn delete_task(&mut self, id: &TaskId) -> StorageResult<()> {
        let len_before = self.data.tasks.len();
        self.data.tasks.retain(|t| &t.id != id);
        if self.data.tasks.len() == len_before {
            return Err(StorageError::not_found("Task", id.to_string()));
        }
        self.mark_dirty();
        Ok(())
    }

    fn list_tasks(&self) -> StorageResult<Vec<Task>> {
        Ok(self.data.tasks.clone())
    }

    fn list_tasks_filtered(&self, filter: &Filter) -> StorageResult<Vec<Task>> {
        let tasks = self
            .data
            .tasks
            .iter()
            .filter(|task| task_matches_filter(task, filter))
            .cloned()
            .collect();
        Ok(tasks)
    }

    fn get_tasks_by_project(&self, project_id: &ProjectId) -> StorageResult<Vec<Task>> {
        Ok(self
            .data
            .tasks
            .iter()
            .filter(|t| t.project_id.as_ref() == Some(project_id))
            .cloned()
            .collect())
    }

    fn get_tasks_by_tag(&self, tag: &str) -> StorageResult<Vec<Task>> {
        Ok(self
            .data
            .tasks
            .iter()
            .filter(|t| t.tags.contains(&tag.to_string()))
            .cloned()
            .collect())
    }
}
