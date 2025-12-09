//! Task hierarchy (subtask) methods for the Model.

use std::collections::HashSet;

use crate::domain::{Task, TaskId};

use super::Model;

impl Model {
    /// Returns subtask completion progress for a task.
    ///
    /// Returns a tuple of (completed_count, total_count) for subtasks
    /// that have this task as their parent.
    ///
    /// # Examples
    ///
    /// ```
    /// use taskflow::app::Model;
    /// use taskflow::domain::Task;
    ///
    /// let mut model = Model::new();
    /// let parent = Task::new("Parent task");
    /// let parent_id = parent.id.clone();
    ///
    /// let subtask1 = Task::new("Subtask 1").with_parent(parent_id.clone());
    /// let subtask2 = Task::new("Subtask 2").with_parent(parent_id.clone());
    ///
    /// model.tasks.insert(parent.id.clone(), parent);
    /// model.tasks.insert(subtask1.id.clone(), subtask1);
    /// model.tasks.insert(subtask2.id.clone(), subtask2);
    ///
    /// let (completed, total) = model.subtask_progress(&parent_id);
    /// assert_eq!(total, 2);
    /// assert_eq!(completed, 0);
    /// ```
    #[must_use]
    pub fn subtask_progress(&self, task_id: &TaskId) -> (usize, usize) {
        let descendants = self.get_all_descendants(task_id);
        let total = descendants.len();
        let completed = descendants
            .iter()
            .filter(|id| self.tasks.get(*id).is_some_and(|t| t.status.is_complete()))
            .count();
        (completed, total)
    }

    /// Returns the nesting depth of a task (0 for root tasks, 1 for direct children, etc.)
    ///
    /// Includes cycle detection to prevent infinite loops from corrupted data.
    #[must_use]
    pub fn task_depth(&self, task_id: &TaskId) -> usize {
        let mut depth = 0;
        let mut current_id = *task_id;
        let mut visited = HashSet::new();

        while let Some(task) = self.tasks.get(&current_id) {
            if let Some(ref parent_id) = task.parent_task_id {
                if visited.contains(parent_id) {
                    // Circular reference detected - break to prevent infinite loop
                    break;
                }
                visited.insert(current_id);
                depth += 1;
                current_id = *parent_id;
            } else {
                break;
            }
        }
        depth
    }

    /// Returns overdue task count and the overdue tasks themselves
    #[must_use]
    pub fn overdue_summary(&self) -> (usize, Vec<&Task>) {
        let overdue_tasks: Vec<_> = self.tasks.values().filter(|t| t.is_overdue()).collect();
        (overdue_tasks.len(), overdue_tasks)
    }

    /// Check if there are overdue tasks and show alert if so.
    /// Call this after loading tasks from storage.
    pub fn check_overdue_alert(&mut self) {
        let (count, _) = self.overdue_summary();
        self.show_overdue_alert = count > 0;
    }

    /// Returns all descendant task IDs (children, grandchildren, etc.)
    ///
    /// Uses iterative approach with cycle detection.
    #[must_use]
    pub fn get_all_descendants(&self, task_id: &TaskId) -> Vec<TaskId> {
        let mut descendants = Vec::new();
        let mut stack = vec![*task_id];
        let mut visited = HashSet::new();

        while let Some(current_id) = stack.pop() {
            if visited.contains(&current_id) {
                continue; // Prevent cycles
            }
            visited.insert(current_id);

            for (id, task) in &self.tasks {
                if task.parent_task_id.as_ref() == Some(&current_id) {
                    descendants.push(*id);
                    stack.push(*id);
                }
            }
        }
        descendants
    }

    /// Returns all ancestor task IDs (parent, grandparent, etc.)
    ///
    /// Uses iterative approach with cycle detection.
    #[must_use]
    pub fn get_all_ancestors(&self, task_id: &TaskId) -> Vec<TaskId> {
        let mut ancestors = Vec::new();
        let mut current_id = *task_id;
        let mut visited = HashSet::new();

        while let Some(task) = self.tasks.get(&current_id) {
            if let Some(ref parent_id) = task.parent_task_id {
                if visited.contains(parent_id) {
                    break; // Circular reference
                }
                visited.insert(current_id);
                ancestors.push(*parent_id);
                current_id = *parent_id;
            } else {
                break;
            }
        }
        ancestors
    }

    /// Checks if setting `new_parent_id` as parent of `task_id` would create a circular reference.
    #[must_use]
    pub fn would_create_cycle(&self, task_id: &TaskId, new_parent_id: &TaskId) -> bool {
        if task_id == new_parent_id {
            return true;
        }
        // Check if new_parent is a descendant of task_id
        self.get_all_descendants(task_id).contains(new_parent_id)
    }

    /// Returns true if the task has any subtasks (direct children).
    #[must_use]
    pub fn has_subtasks(&self, task_id: &TaskId) -> bool {
        self.tasks
            .values()
            .any(|t| t.parent_task_id.as_ref() == Some(task_id))
    }

    /// Returns recursive subtask completion as a percentage (0-100).
    ///
    /// Returns `None` if the task has no subtasks.
    #[must_use]
    pub fn subtask_percentage(&self, task_id: &TaskId) -> Option<u8> {
        let (completed, total) = self.subtask_progress(task_id);
        if total == 0 {
            None
        } else {
            Some(((completed * 100) / total) as u8)
        }
    }

    /// Returns true if the task has any incomplete dependencies (is blocked).
    ///
    /// A task is blocked if any of its dependencies (tasks it depends on)
    /// are not yet completed.
    #[must_use]
    pub fn is_task_blocked(&self, task_id: &TaskId) -> bool {
        if let Some(task) = self.tasks.get(task_id) {
            task.dependencies.iter().any(|dep_id| {
                self.tasks
                    .get(dep_id)
                    .is_some_and(|dep| !dep.status.is_complete())
            })
        } else {
            false
        }
    }

    /// Returns the count of incomplete dependencies for a task.
    #[must_use]
    pub fn incomplete_dependency_count(&self, task_id: &TaskId) -> usize {
        if let Some(task) = self.tasks.get(task_id) {
            task.dependencies
                .iter()
                .filter(|dep_id| {
                    self.tasks
                        .get(dep_id)
                        .is_some_and(|dep| !dep.status.is_complete())
                })
                .count()
        } else {
            0
        }
    }

    /// Returns true if this task has any dependencies (regardless of completion).
    #[must_use]
    pub fn has_dependencies(&self, task_id: &TaskId) -> bool {
        self.tasks
            .get(task_id)
            .is_some_and(|task| !task.dependencies.is_empty())
    }
}
