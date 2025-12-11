//! TaskRepository blanket implementation for in-memory backends.

use crate::domain::{Filter, ProjectId, Task, TaskId};
use crate::storage::backends::filter_utils::task_matches_filter;
use crate::storage::{StorageError, StorageResult, TaskRepository};

use super::InMemoryBackend;

impl<B: InMemoryBackend> TaskRepository for B {
    fn create_task(&mut self, task: &Task) -> StorageResult<()> {
        if self.data().tasks.iter().any(|t| t.id == task.id) {
            return Err(StorageError::already_exists("Task", task.id.to_string()));
        }
        self.data_mut().tasks.push(task.clone());
        self.mark_dirty();
        Ok(())
    }

    fn get_task(&self, id: &TaskId) -> StorageResult<Option<Task>> {
        Ok(self.data().tasks.iter().find(|t| &t.id == id).cloned())
    }

    fn update_task(&mut self, task: &Task) -> StorageResult<()> {
        if let Some(existing) = self.data_mut().tasks.iter_mut().find(|t| t.id == task.id) {
            *existing = task.clone();
            self.mark_dirty();
            Ok(())
        } else {
            Err(StorageError::not_found("Task", task.id.to_string()))
        }
    }

    fn delete_task(&mut self, id: &TaskId) -> StorageResult<()> {
        let len_before = self.data().tasks.len();
        self.data_mut().tasks.retain(|t| &t.id != id);
        if self.data().tasks.len() == len_before {
            return Err(StorageError::not_found("Task", id.to_string()));
        }
        self.mark_dirty();
        Ok(())
    }

    fn list_tasks(&self) -> StorageResult<Vec<Task>> {
        Ok(self.data().tasks.clone())
    }

    fn list_tasks_filtered(&self, filter: &Filter) -> StorageResult<Vec<Task>> {
        let tasks = self
            .data()
            .tasks
            .iter()
            .filter(|task| task_matches_filter(task, filter))
            .cloned()
            .collect();
        Ok(tasks)
    }

    fn get_tasks_by_project(&self, project_id: &ProjectId) -> StorageResult<Vec<Task>> {
        Ok(self
            .data()
            .tasks
            .iter()
            .filter(|t| t.project_id.as_ref() == Some(project_id))
            .cloned()
            .collect())
    }

    fn get_tasks_by_tag(&self, tag: &str) -> StorageResult<Vec<Task>> {
        Ok(self
            .data()
            .tasks
            .iter()
            .filter(|t| t.tags.contains(&tag.to_string()))
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Project, TaskStatus};
    use crate::storage::ExportData;

    /// Test backend implementation for testing
    struct TestBackend {
        data: ExportData,
        dirty: bool,
    }

    impl TestBackend {
        fn new() -> Self {
            Self {
                data: ExportData::default(),
                dirty: false,
            }
        }
    }

    impl InMemoryBackend for TestBackend {
        fn data(&self) -> &ExportData {
            &self.data
        }
        fn data_mut(&mut self) -> &mut ExportData {
            &mut self.data
        }
        fn mark_dirty(&mut self) {
            self.dirty = true;
        }
    }

    // ========================================================================
    // CRUD Tests
    // ========================================================================

    #[test]
    fn test_create_task() {
        let mut backend = TestBackend::new();
        let task = Task::new("Test task");

        let result = backend.create_task(&task);
        assert!(result.is_ok());
        assert!(backend.dirty);
        assert_eq!(backend.data.tasks.len(), 1);
    }

    #[test]
    fn test_create_duplicate_task_fails() {
        let mut backend = TestBackend::new();
        let task = Task::new("Test task");

        backend.create_task(&task).unwrap();
        let result = backend.create_task(&task);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_task() {
        let mut backend = TestBackend::new();
        let task = Task::new("Test task");
        let task_id = task.id;

        backend.create_task(&task).unwrap();

        let found = backend.get_task(&task_id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Test task");
    }

    #[test]
    fn test_get_task_not_found() {
        let backend = TestBackend::new();
        let random_id = TaskId::new();

        let found = backend.get_task(&random_id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_update_task() {
        let mut backend = TestBackend::new();
        let mut task = Task::new("Original title");
        let task_id = task.id;

        backend.create_task(&task).unwrap();
        backend.dirty = false;

        task.title = "Updated title".to_string();
        let result = backend.update_task(&task);

        assert!(result.is_ok());
        assert!(backend.dirty);

        let found = backend.get_task(&task_id).unwrap().unwrap();
        assert_eq!(found.title, "Updated title");
    }

    #[test]
    fn test_update_nonexistent_task_fails() {
        let mut backend = TestBackend::new();
        let task = Task::new("Nonexistent");

        let result = backend.update_task(&task);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_task() {
        let mut backend = TestBackend::new();
        let task = Task::new("To delete");
        let task_id = task.id;

        backend.create_task(&task).unwrap();
        backend.dirty = false;

        let result = backend.delete_task(&task_id);
        assert!(result.is_ok());
        assert!(backend.dirty);
        assert!(backend.data.tasks.is_empty());
    }

    #[test]
    fn test_delete_nonexistent_task_fails() {
        let mut backend = TestBackend::new();
        let random_id = TaskId::new();

        let result = backend.delete_task(&random_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_tasks() {
        let mut backend = TestBackend::new();

        backend.create_task(&Task::new("Task 1")).unwrap();
        backend.create_task(&Task::new("Task 2")).unwrap();
        backend.create_task(&Task::new("Task 3")).unwrap();

        let tasks = backend.list_tasks().unwrap();
        assert_eq!(tasks.len(), 3);
    }

    // ========================================================================
    // Filter Tests
    // ========================================================================

    #[test]
    fn test_get_tasks_by_project() {
        let mut backend = TestBackend::new();

        let project = Project::new("Test Project");
        let project_id = project.id;
        backend.data.projects.push(project);

        let mut task1 = Task::new("In project");
        task1.project_id = Some(project_id);
        let task2 = Task::new("No project");

        backend.create_task(&task1).unwrap();
        backend.create_task(&task2).unwrap();

        let project_tasks = backend.get_tasks_by_project(&project_id).unwrap();
        assert_eq!(project_tasks.len(), 1);
        assert_eq!(project_tasks[0].title, "In project");
    }

    #[test]
    fn test_get_tasks_by_tag() {
        let mut backend = TestBackend::new();

        let mut task1 = Task::new("Has tag");
        task1.tags.push("important".to_string());
        let task2 = Task::new("No tag");

        backend.create_task(&task1).unwrap();
        backend.create_task(&task2).unwrap();

        let tagged_tasks = backend.get_tasks_by_tag("important").unwrap();
        assert_eq!(tagged_tasks.len(), 1);
        assert_eq!(tagged_tasks[0].title, "Has tag");
    }

    #[test]
    fn test_list_tasks_filtered_by_status() {
        let mut backend = TestBackend::new();

        backend
            .create_task(&Task::new("Todo").with_status(TaskStatus::Todo))
            .unwrap();
        backend
            .create_task(&Task::new("Done").with_status(TaskStatus::Done))
            .unwrap();
        backend
            .create_task(&Task::new("Also Todo").with_status(TaskStatus::Todo))
            .unwrap();

        let filter = Filter {
            status: Some(vec![TaskStatus::Todo]),
            ..Filter::default()
        };

        let filtered = backend.list_tasks_filtered(&filter).unwrap();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_list_tasks_filtered_empty_filter() {
        let mut backend = TestBackend::new();

        backend.create_task(&Task::new("Task 1")).unwrap();
        backend.create_task(&Task::new("Task 2")).unwrap();

        let filter = Filter::default();
        let filtered = backend.list_tasks_filtered(&filter).unwrap();
        assert_eq!(filtered.len(), 2);
    }
}
