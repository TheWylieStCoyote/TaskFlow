//! Task hierarchy (subtask) methods for the Model.
//!
//! This module provides methods for navigating and querying task relationships:
//!
//! - **Parent-child relationships**: Tasks can have a `parent_task_id`, creating a tree structure
//! - **Subtask progress**: Track completion of descendant tasks
//! - **Dependency tracking**: Tasks can depend on other tasks (blocking relationships)
//!
//! # Cycle Detection
//!
//! All traversal methods include cycle detection to handle corrupted or malformed data
//! gracefully. This prevents infinite loops when:
//! - A task is its own ancestor (direct or indirect)
//! - Setting a parent that would create a circular reference
//!
//! # Performance
//!
//! - Ancestor traversal: O(d) where d = depth of task
//! - Descendant traversal: O(n) where n = number of descendants
//! - Cycle checks use `HashSet` for O(1) visited lookups
//! - Child lookups use cached `children` map for O(1) access
//!
//! # Example
//!
//! ```
//! use taskflow::app::Model;
//! use taskflow::domain::Task;
//!
//! let mut model = Model::new();
//!
//! // Create a task hierarchy
//! let parent = Task::new("Project");
//! let parent_id = parent.id;
//! model.tasks.insert(parent.id, parent);
//!
//! let subtask = Task::new("Subtask").with_parent(parent_id);
//! model.tasks.insert(subtask.id, subtask);
//!
//! // Rebuild caches to index the hierarchy
//! model.rebuild_caches();
//!
//! // Query hierarchy
//! assert_eq!(model.task_depth(&parent_id), 0); // Root task
//! assert!(model.has_subtasks(&parent_id));
//! ```

use std::collections::HashSet;

use crate::domain::{Task, TaskId};

use super::Model;

/// Traverses ancestors with cycle detection, calling the visitor for each ancestor.
///
/// This is a shared utility to avoid duplicating cycle detection logic.
/// Returns the number of ancestors visited (the depth).
pub(super) fn traverse_ancestors<F>(
    start_id: TaskId,
    get_parent: impl Fn(TaskId) -> Option<TaskId>,
    mut visitor: F,
) -> usize
where
    F: FnMut(TaskId),
{
    let mut current_id = start_id;
    let mut visited = HashSet::new();
    let mut count = 0;

    while let Some(parent_id) = get_parent(current_id) {
        if visited.contains(&parent_id) {
            break; // Cycle detected
        }
        visited.insert(current_id);
        visitor(parent_id);
        count += 1;
        current_id = parent_id;
    }
    count
}

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
    /// model.rebuild_caches(); // Rebuild cache to update hierarchy
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
        traverse_ancestors(
            *task_id,
            |id| self.tasks.get(&id).and_then(|t| t.parent_task_id),
            |_| {}, // No-op visitor, we just need the count
        )
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
        self.alerts.show_overdue = count > 0;
    }

    /// Returns all descendant task IDs (children, grandchildren, etc.)
    ///
    /// Uses iterative approach with cycle detection and the cached children map
    /// for O(d) complexity where d is the number of descendants.
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

            // Use cached children map for O(1) lookup instead of scanning all tasks
            if let Some(children) = self.task_cache.children.get(&current_id) {
                for child_id in children {
                    descendants.push(*child_id);
                    stack.push(*child_id);
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
        traverse_ancestors(
            *task_id,
            |id| self.tasks.get(&id).and_then(|t| t.parent_task_id),
            |parent_id| ancestors.push(parent_id),
        );
        ancestors
    }

    /// Checks if setting `new_parent_id` as parent of `task_id` would create a circular reference.
    #[must_use]
    pub fn would_create_cycle(&self, task_id: &TaskId, new_parent_id: &TaskId) -> bool {
        if task_id == new_parent_id {
            return true;
        }
        // Check if new_parent is a descendant of task_id (with early exit)
        self.has_descendant(task_id, new_parent_id)
    }

    /// Checks if `target_id` is a descendant of `task_id`.
    ///
    /// Uses early exit traversal - returns as soon as target is found.
    /// O(1) best case, O(d) worst case where d = number of descendants.
    #[must_use]
    fn has_descendant(&self, task_id: &TaskId, target_id: &TaskId) -> bool {
        let mut stack = vec![*task_id];
        let mut visited = HashSet::new();

        while let Some(current_id) = stack.pop() {
            if visited.contains(&current_id) {
                continue; // Prevent cycles
            }
            visited.insert(current_id);

            if let Some(children) = self.task_cache.children.get(&current_id) {
                for child_id in children {
                    if child_id == target_id {
                        return true; // Early exit!
                    }
                    stack.push(*child_id);
                }
            }
        }
        false
    }

    /// Returns true if the task has any subtasks (direct children).
    #[must_use]
    pub fn has_subtasks(&self, task_id: &TaskId) -> bool {
        self.task_cache
            .children
            .get(task_id)
            .is_some_and(|children| !children.is_empty())
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
