//! Task visibility and cache management methods.

use std::collections::{HashMap, HashSet};

use crate::domain::{SortField, SortOrder, Task, TaskId};

use super::super::Model;
use super::FilterCache;

impl Model {
    /// Rebuilds all performance caches.
    ///
    /// Should be called when:
    /// - Tasks are added, removed, or modified
    /// - Time entries change
    /// - Task hierarchy changes
    pub fn rebuild_caches(&mut self) {
        self.footer_stats.rebuild(&self.tasks);
        self.task_cache.rebuild_time_sums(&self.time_entries);
        self.task_cache.rebuild_hierarchy(&self.tasks);
    }

    /// Recalculates visible tasks based on current filters and sort.
    ///
    /// This should be called after any change that affects which tasks
    /// are visible (adding/removing tasks, changing filters, switching views).
    /// Updates `visible_tasks` with the filtered and sorted task IDs.
    ///
    /// Also rebuilds performance caches to ensure UI data is current.
    ///
    /// Subtasks are displayed directly after their parent task.
    pub fn refresh_visible_tasks(&mut self) {
        // Rebuild caches when task list changes
        self.rebuild_caches();

        // Pre-compute filter values once (avoids repeated allocations per task)
        let cache = FilterCache::new(self);

        // Collect all tasks that pass the filter
        let filtered_tasks: Vec<_> = self
            .tasks
            .values()
            .filter(|task| self.task_matches_filter_cached(task, &cache))
            .collect();

        // Separate into parent tasks and subtasks
        let mut parent_tasks: Vec<_> = filtered_tasks
            .iter()
            .filter(|t| t.parent_task_id.is_none())
            .copied()
            .collect();

        // Build a map of parent_id -> subtasks for quick lookup
        let mut subtasks_by_parent: HashMap<&TaskId, Vec<&Task>> = HashMap::new();
        for task in &filtered_tasks {
            if let Some(ref parent_id) = task.parent_task_id {
                subtasks_by_parent.entry(parent_id).or_default().push(task);
            }
        }

        // Sort parent tasks based on SortSpec
        let sort_field = self.filtering.sort.field;
        let sort_order = self.filtering.sort.order;

        let sort_fn = |a: &&Task, b: &&Task| {
            let primary_cmp = match sort_field {
                SortField::CreatedAt => a.created_at.cmp(&b.created_at),
                SortField::UpdatedAt => a.updated_at.cmp(&b.updated_at),
                SortField::DueDate => {
                    // Tasks with no due date go last
                    match (a.due_date, b.due_date) {
                        (Some(da), Some(db)) => da.cmp(&db),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                }
                SortField::Priority => {
                    let priority_order = |p: &crate::domain::Priority| match p {
                        crate::domain::Priority::Urgent => 0,
                        crate::domain::Priority::High => 1,
                        crate::domain::Priority::Medium => 2,
                        crate::domain::Priority::Low => 3,
                        crate::domain::Priority::None => 4,
                    };
                    priority_order(&a.priority).cmp(&priority_order(&b.priority))
                }
                SortField::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                SortField::Status => {
                    let status_order = |s: &crate::domain::TaskStatus| match s {
                        crate::domain::TaskStatus::InProgress => 0,
                        crate::domain::TaskStatus::Todo => 1,
                        crate::domain::TaskStatus::Blocked => 2,
                        crate::domain::TaskStatus::Done => 3,
                        crate::domain::TaskStatus::Cancelled => 4,
                    };
                    status_order(&a.status).cmp(&status_order(&b.status))
                }
            };

            // Use sort_order as secondary sort key when primary values are equal
            // Tasks with sort_order come before tasks without
            let cmp = if primary_cmp == std::cmp::Ordering::Equal {
                match (a.sort_order, b.sort_order) {
                    (Some(oa), Some(ob)) => oa.cmp(&ob),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            } else {
                primary_cmp
            };

            match sort_order {
                SortOrder::Ascending => cmp,
                SortOrder::Descending => cmp.reverse(),
            }
        };

        parent_tasks.sort_by(sort_fn);

        // Also sort subtasks within each parent group
        for subtasks in subtasks_by_parent.values_mut() {
            subtasks.sort_by(sort_fn);
        }

        // Build final list: parent followed by its subtasks (recursively)
        let mut result = Vec::new();
        // Track seen IDs to avoid O(n²) contains() calls
        let mut seen_ids: HashSet<TaskId> = HashSet::new();

        // Recursive helper to add a task and all its descendants
        fn add_with_descendants(
            task_id: &TaskId,
            subtasks_by_parent: &HashMap<&TaskId, Vec<&Task>>,
            result: &mut Vec<TaskId>,
            seen_ids: &mut HashSet<TaskId>,
        ) {
            result.push(*task_id);
            seen_ids.insert(*task_id);
            if let Some(children) = subtasks_by_parent.get(task_id) {
                for child in children {
                    add_with_descendants(&child.id, subtasks_by_parent, result, seen_ids);
                }
            }
        }

        for parent in parent_tasks {
            add_with_descendants(&parent.id, &subtasks_by_parent, &mut result, &mut seen_ids);
        }

        // Handle orphaned subtasks (subtasks whose parent is not visible)
        // These are shown at the end - O(1) lookup with HashSet
        for task in &filtered_tasks {
            if task.parent_task_id.is_some() && !seen_ids.contains(&task.id) {
                result.push(task.id);
            }
        }

        self.visible_tasks = result;

        // Adjust selection if needed
        if self.selected_index >= self.visible_tasks.len() && !self.visible_tasks.is_empty() {
            self.selected_index = self.visible_tasks.len() - 1;
        }
    }
}
