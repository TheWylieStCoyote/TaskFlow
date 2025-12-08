//! TaskRepository implementation for markdown backend.

use crate::domain::{Filter, ProjectId, Task, TaskId};
use crate::storage::backends::filter_utils::task_matches_filter;
use crate::storage::{StorageError, StorageResult, TaskRepository};

use super::MarkdownBackend;

impl TaskRepository for MarkdownBackend {
    fn create_task(&mut self, task: &Task) -> StorageResult<()> {
        if self.tasks_cache.contains_key(&task.id) {
            return Err(StorageError::already_exists("Task", task.id.to_string()));
        }
        self.write_task_file(task)?;
        self.tasks_cache.insert(task.id, task.clone());
        Ok(())
    }

    fn get_task(&self, id: &TaskId) -> StorageResult<Option<Task>> {
        Ok(self.tasks_cache.get(id).cloned())
    }

    fn update_task(&mut self, task: &Task) -> StorageResult<()> {
        if !self.tasks_cache.contains_key(&task.id) {
            return Err(StorageError::not_found("Task", task.id.to_string()));
        }
        self.write_task_file(task)?;
        self.tasks_cache.insert(task.id, task.clone());
        Ok(())
    }

    fn delete_task(&mut self, id: &TaskId) -> StorageResult<()> {
        if !self.tasks_cache.contains_key(id) {
            return Err(StorageError::not_found("Task", id.to_string()));
        }
        self.delete_task_file(id)?;
        self.tasks_cache.remove(id);
        Ok(())
    }

    fn list_tasks(&self) -> StorageResult<Vec<Task>> {
        Ok(self.tasks_cache.values().cloned().collect())
    }

    fn list_tasks_filtered(&self, filter: &Filter) -> StorageResult<Vec<Task>> {
        let tasks = self
            .tasks_cache
            .values()
            .filter(|task| task_matches_filter(task, filter))
            .cloned()
            .collect();
        Ok(tasks)
    }

    fn get_tasks_by_project(&self, project_id: &ProjectId) -> StorageResult<Vec<Task>> {
        Ok(self
            .tasks_cache
            .values()
            .filter(|t| t.project_id.as_ref() == Some(project_id))
            .cloned()
            .collect())
    }

    fn get_tasks_by_tag(&self, tag: &str) -> StorageResult<Vec<Task>> {
        Ok(self
            .tasks_cache
            .values()
            .filter(|t| t.tags.contains(&tag.to_string()))
            .cloned()
            .collect())
    }
}
