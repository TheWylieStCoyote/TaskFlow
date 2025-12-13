//! Performance caches for expensive computations.
//!
//! These caches store pre-computed values that would otherwise require
//! O(n) or O(n²) operations during rendering. Caches are invalidated
//! when the underlying data changes.
//!
//! # Cache Types
//!
//! | Cache | Purpose | Rebuild Trigger |
//! |-------|---------|-----------------|
//! | [`FooterStats`] | Task counts for status bar | Task created/modified/deleted |
//! | [`TaskCache`] | Per-task metadata (depth, children, time sums) | Task/time entry changes |
//! | [`ReportCache`] | Analytics reports for dashboards | Task/time entry changes |
//!
//! # Invalidation Strategy
//!
//! Caches use a **full rebuild** strategy rather than incremental updates:
//!
//! - **Simplicity**: No complex delta tracking or partial invalidation
//! - **Correctness**: Avoids subtle bugs from stale partial state
//! - **Performance**: Rebuilds are fast (single pass over data) and infrequent
//!
//! Call [`Model::rebuild_caches()`](super::Model::rebuild_caches) after any data modification.
//!
//! # When to Invalidate
//!
//! Caches should be rebuilt after:
//! - Creating, updating, or deleting tasks
//! - Adding or modifying time entries
//! - Changing task parent relationships (affects hierarchy caches)
//! - Loading data from storage
//!
//! See also [`super::layout_cache::LayoutCache`] for UI layout caching.

use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;

use crate::domain::analytics::AnalyticsReport;
use crate::domain::{
    is_context_tag, ProjectId, Task, TaskId, TimeEntry, TimeEntryId, WorkLogEntry, WorkLogEntryId,
};

use super::hierarchy::traverse_ancestors;

/// Cached statistics for the footer display.
///
/// These values are computed once per data change rather than per frame.
#[derive(Debug, Clone, Default)]
pub struct FooterStats {
    /// Total number of completed tasks
    pub completed_count: usize,
    /// Number of overdue incomplete tasks
    pub overdue_count: usize,
    /// Number of tasks due today (incomplete)
    pub due_today_count: usize,
}

impl FooterStats {
    /// Rebuild footer statistics from tasks in a single pass.
    ///
    /// This is more efficient than computing each stat separately
    /// as it only iterates through tasks once.
    pub fn rebuild(&mut self, tasks: &HashMap<TaskId, Task>) {
        self.completed_count = 0;
        self.overdue_count = 0;
        self.due_today_count = 0;

        for task in tasks.values() {
            if task.status.is_complete() {
                self.completed_count += 1;
            } else {
                if task.is_overdue() {
                    self.overdue_count += 1;
                }
                if task.is_due_today() {
                    self.due_today_count += 1;
                }
            }
        }
    }
}

/// Cached per-task metadata.
///
/// Stores expensive-to-compute values for each task.
#[derive(Debug, Clone, Default)]
pub struct TaskCache {
    /// Total time tracked per task in minutes
    pub time_sums: HashMap<TaskId, u32>,
    /// Nesting depth per task (0 for root tasks)
    pub depths: HashMap<TaskId, usize>,
    /// Subtask progress per task: (completed, total)
    pub subtask_progress: HashMap<TaskId, (usize, usize)>,
    /// Child task IDs per parent task
    pub children: HashMap<TaskId, Vec<TaskId>>,
    /// Time entry IDs per task (for O(1) lookup by task)
    pub time_entries_by_task: HashMap<TaskId, Vec<TimeEntryId>>,
    /// Work log IDs per task (for O(1) lookup by task)
    pub work_logs_by_task: HashMap<TaskId, Vec<WorkLogEntryId>>,

    // Secondary indexes for fast lookups
    /// Task IDs grouped by project (None key for tasks without a project)
    pub tasks_by_project: HashMap<Option<ProjectId>, Vec<TaskId>>,
    /// Task IDs grouped by due date (None key for tasks without a due date)
    pub tasks_by_due_date: HashMap<Option<NaiveDate>, Vec<TaskId>>,
    /// Pre-computed set of context tags (tags starting with @)
    pub contexts: HashSet<String>,
}

impl TaskCache {
    /// Creates a new empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears all cached data.
    pub fn clear(&mut self) {
        self.time_sums.clear();
        self.depths.clear();
        self.subtask_progress.clear();
        self.children.clear();
        self.time_entries_by_task.clear();
        self.work_logs_by_task.clear();
        self.tasks_by_project.clear();
        self.tasks_by_due_date.clear();
        self.contexts.clear();
    }

    /// Rebuild time sum cache and time entry index from time entries.
    ///
    /// Groups time entries by task_id, sums durations, and builds a lookup index.
    pub fn rebuild_time_sums(&mut self, time_entries: &HashMap<TimeEntryId, TimeEntry>) {
        self.time_sums.clear();
        self.time_entries_by_task.clear();

        for (id, entry) in time_entries {
            *self.time_sums.entry(entry.task_id).or_insert(0) +=
                entry.calculated_duration_minutes();
            self.time_entries_by_task
                .entry(entry.task_id)
                .or_default()
                .push(*id);
        }
    }

    /// Rebuild work log index from work logs.
    ///
    /// Groups work logs by task_id for O(1) lookup.
    pub fn rebuild_work_logs_index(&mut self, work_logs: &HashMap<WorkLogEntryId, WorkLogEntry>) {
        self.work_logs_by_task.clear();

        for (id, entry) in work_logs {
            self.work_logs_by_task
                .entry(entry.task_id)
                .or_default()
                .push(*id);
        }
    }

    /// Rebuild hierarchy caches (depths, children, subtask progress) from tasks.
    ///
    /// This computes:
    /// - Child lists per parent (single pass)
    /// - Depth for each task (walks parent chain, cached)
    /// - Subtask progress for each task with children
    pub fn rebuild_hierarchy(&mut self, tasks: &HashMap<TaskId, Task>) {
        self.depths.clear();
        self.children.clear();
        self.subtask_progress.clear();

        // Build parent->children map in one pass
        for (id, task) in tasks {
            if let Some(parent_id) = task.parent_task_id {
                self.children.entry(parent_id).or_default().push(*id);
            }
        }

        // Compute depths for all tasks
        for task_id in tasks.keys() {
            self.compute_depth(*task_id, tasks);
        }

        // Compute subtask progress using bottom-up aggregation (O(n) instead of O(n²))
        // Sort tasks by depth descending so we process deepest tasks first
        let mut tasks_by_depth: Vec<_> = tasks.keys().copied().collect();
        tasks_by_depth.sort_by(|a, b| {
            let depth_a = self.depths.get(a).copied().unwrap_or(0);
            let depth_b = self.depths.get(b).copied().unwrap_or(0);
            depth_b.cmp(&depth_a) // Descending order (deepest first)
        });

        // Aggregate progress from children to parents (bottom-up)
        for task_id in tasks_by_depth {
            if let Some(children) = self.children.get(&task_id) {
                let mut completed = 0;
                let mut total = 0;

                for &child_id in children {
                    // Add this child
                    total += 1;
                    if tasks.get(&child_id).is_some_and(|t| t.status.is_complete()) {
                        completed += 1;
                    }

                    // Add child's descendants (already computed since we process bottom-up)
                    if let Some(&(child_completed, child_total)) =
                        self.subtask_progress.get(&child_id)
                    {
                        completed += child_completed;
                        total += child_total;
                    }
                }

                self.subtask_progress.insert(task_id, (completed, total));
            }
        }
    }

    /// Compute and cache depth for a single task.
    fn compute_depth(&mut self, task_id: TaskId, tasks: &HashMap<TaskId, Task>) -> usize {
        // Check cache first
        if let Some(&depth) = self.depths.get(&task_id) {
            return depth;
        }

        let depth = traverse_ancestors(
            task_id,
            |id| tasks.get(&id).and_then(|t| t.parent_task_id),
            |_| {}, // No-op visitor, we just need the count
        );

        self.depths.insert(task_id, depth);
        depth
    }

    /// Get cached time sum for a task, returning 0 if not cached.
    #[must_use]
    pub fn get_time_sum(&self, task_id: TaskId) -> u32 {
        self.time_sums.get(&task_id).copied().unwrap_or(0)
    }

    /// Get cached depth for a task, returning 0 if not cached.
    #[must_use]
    pub fn get_depth(&self, task_id: TaskId) -> usize {
        self.depths.get(&task_id).copied().unwrap_or(0)
    }

    /// Get cached subtask progress for a task, returning (0, 0) if not cached.
    #[must_use]
    pub fn get_subtask_progress(&self, task_id: TaskId) -> (usize, usize) {
        self.subtask_progress
            .get(&task_id)
            .copied()
            .unwrap_or((0, 0))
    }

    /// Rebuild the project index from tasks.
    ///
    /// Groups task IDs by their project_id for O(1) project-based lookups.
    pub fn rebuild_project_index(&mut self, tasks: &HashMap<TaskId, Task>) {
        self.tasks_by_project.clear();
        for (id, task) in tasks {
            self.tasks_by_project
                .entry(task.project_id)
                .or_default()
                .push(*id);
        }
    }

    /// Rebuild the due date index from tasks.
    ///
    /// Groups task IDs by their due_date for O(1) date-based lookups.
    pub fn rebuild_due_date_index(&mut self, tasks: &HashMap<TaskId, Task>) {
        self.tasks_by_due_date.clear();
        for (id, task) in tasks {
            self.tasks_by_due_date
                .entry(task.due_date)
                .or_default()
                .push(*id);
        }
    }

    /// Rebuild the context tags set from tasks.
    ///
    /// Extracts all unique @-prefixed tags for O(1) context listing.
    pub fn rebuild_contexts(&mut self, tasks: &HashMap<TaskId, Task>) {
        self.contexts.clear();
        for task in tasks.values() {
            for tag in &task.tags {
                if is_context_tag(tag) {
                    self.contexts.insert(tag.clone());
                }
            }
        }
    }

    /// Rebuild all secondary indexes from tasks.
    ///
    /// This is called by [`super::Model::rebuild_caches()`] to update
    /// project, due date, and context indexes.
    pub fn rebuild_secondary_indexes(&mut self, tasks: &HashMap<TaskId, Task>) {
        self.rebuild_project_index(tasks);
        self.rebuild_due_date_index(tasks);
        self.rebuild_contexts(tasks);
    }
}

/// Cached analytics reports for different time windows.
///
/// Reports are expensive to compute as they iterate through all tasks
/// and time entries. This cache stores pre-computed reports for common
/// time windows (30, 60, 90 days).
#[derive(Debug, Clone, Default)]
pub struct ReportCache {
    /// Report for 30-day window (used by overview, tags, time panels)
    pub report_30d: Option<AnalyticsReport>,
    /// Report for 60-day window (used by velocity panel)
    pub report_60d: Option<AnalyticsReport>,
    /// Report for 90-day window (used by insights panel)
    pub report_90d: Option<AnalyticsReport>,
}

impl ReportCache {
    /// Create a new empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all cached reports.
    ///
    /// Call this when tasks or time entries are modified.
    pub fn clear(&mut self) {
        self.report_30d = None;
        self.report_60d = None;
        self.report_90d = None;
    }

    /// Check if any reports are cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.report_30d.is_none() && self.report_60d.is_none() && self.report_90d.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Task, TaskStatus};

    #[test]
    fn test_footer_stats_default() {
        let stats = FooterStats::default();
        assert_eq!(stats.completed_count, 0);
        assert_eq!(stats.overdue_count, 0);
        assert_eq!(stats.due_today_count, 0);
    }

    #[test]
    fn test_footer_stats_rebuild() {
        let mut stats = FooterStats::default();
        let mut tasks = HashMap::new();

        // Add a completed task
        let t1 = Task::new("Completed").with_status(TaskStatus::Done);
        tasks.insert(t1.id, t1);

        // Add an incomplete task
        let t2 = Task::new("Todo");
        tasks.insert(t2.id, t2);

        stats.rebuild(&tasks);

        assert_eq!(stats.completed_count, 1);
        // Note: overdue/due_today depend on dates, so 0 here
        assert_eq!(stats.overdue_count, 0);
        assert_eq!(stats.due_today_count, 0);
    }

    #[test]
    fn test_task_cache_new() {
        let cache = TaskCache::new();
        assert!(cache.time_sums.is_empty());
        assert!(cache.depths.is_empty());
        assert!(cache.subtask_progress.is_empty());
        assert!(cache.children.is_empty());
    }

    #[test]
    fn test_task_cache_clear() {
        let mut cache = TaskCache::new();
        let task_id = TaskId::new();

        cache.time_sums.insert(task_id, 100);
        cache.depths.insert(task_id, 2);
        cache.subtask_progress.insert(task_id, (1, 3));
        cache.children.insert(task_id, vec![]);

        assert!(!cache.time_sums.is_empty());

        cache.clear();

        assert!(cache.time_sums.is_empty());
        assert!(cache.depths.is_empty());
        assert!(cache.subtask_progress.is_empty());
        assert!(cache.children.is_empty());
    }

    #[test]
    fn test_task_cache_rebuild_time_sums() {
        let mut cache = TaskCache::new();
        let mut entries = HashMap::new();

        let task_id = TaskId::new();

        // Add two time entries for the same task
        let mut e1 = TimeEntry::start(task_id);
        e1.stop(); // Will have ~0 duration in test
        entries.insert(e1.id, e1);

        cache.rebuild_time_sums(&entries);

        // Should have aggregated time for the task
        assert!(cache.time_sums.contains_key(&task_id));
    }

    #[test]
    fn test_task_cache_rebuild_hierarchy() {
        let mut cache = TaskCache::new();
        let mut tasks = HashMap::new();

        // Create parent task
        let parent = Task::new("Parent");
        let parent_id = parent.id;
        tasks.insert(parent.id, parent);

        // Create child task
        let child = Task::new("Child").with_parent(parent_id);
        let child_id = child.id;
        tasks.insert(child.id, child);

        // Create grandchild task
        let grandchild = Task::new("Grandchild").with_parent(child_id);
        let grandchild_id = grandchild.id;
        tasks.insert(grandchild.id, grandchild);

        cache.rebuild_hierarchy(&tasks);

        // Check depths
        assert_eq!(cache.get_depth(parent_id), 0);
        assert_eq!(cache.get_depth(child_id), 1);
        assert_eq!(cache.get_depth(grandchild_id), 2);

        // Check children
        assert!(cache.children.contains_key(&parent_id));
        assert_eq!(cache.children.get(&parent_id).unwrap().len(), 1);

        // Check subtask progress (parent has 2 descendants, 0 completed)
        let (completed, total) = cache.get_subtask_progress(parent_id);
        assert_eq!(total, 2);
        assert_eq!(completed, 0);
    }

    #[test]
    fn test_task_cache_getters_default() {
        let cache = TaskCache::new();
        let task_id = TaskId::new();

        // Getters should return defaults for missing entries
        assert_eq!(cache.get_time_sum(task_id), 0);
        assert_eq!(cache.get_depth(task_id), 0);
        assert_eq!(cache.get_subtask_progress(task_id), (0, 0));
    }

    // ========================================================================
    // FooterStats Tests
    // ========================================================================

    #[test]
    fn test_footer_stats_with_overdue_task() {
        use chrono::{Duration, Utc};

        let mut stats = FooterStats::default();
        let mut tasks = HashMap::new();

        // Add an overdue task (due yesterday)
        let mut overdue_task = Task::new("Overdue");
        overdue_task.due_date = Some(Utc::now().date_naive() - Duration::days(1));
        tasks.insert(overdue_task.id, overdue_task);

        stats.rebuild(&tasks);

        assert_eq!(stats.completed_count, 0);
        assert_eq!(stats.overdue_count, 1);
        assert_eq!(stats.due_today_count, 0);
    }

    #[test]
    fn test_footer_stats_with_due_today_task() {
        use chrono::Utc;

        let mut stats = FooterStats::default();
        let mut tasks = HashMap::new();

        // Add a task due today
        let mut today_task = Task::new("Due Today");
        today_task.due_date = Some(Utc::now().date_naive());
        tasks.insert(today_task.id, today_task);

        stats.rebuild(&tasks);

        assert_eq!(stats.completed_count, 0);
        assert_eq!(stats.overdue_count, 0);
        assert_eq!(stats.due_today_count, 1);
    }

    #[test]
    fn test_footer_stats_combined() {
        use chrono::{Duration, Utc};

        let mut stats = FooterStats::default();
        let mut tasks = HashMap::new();
        let today = Utc::now().date_naive();

        // Completed task
        let completed = Task::new("Completed").with_status(TaskStatus::Done);
        tasks.insert(completed.id, completed);

        // Overdue task
        let mut overdue = Task::new("Overdue");
        overdue.due_date = Some(today - Duration::days(3));
        tasks.insert(overdue.id, overdue);

        // Due today
        let mut due_today = Task::new("Due Today");
        due_today.due_date = Some(today);
        tasks.insert(due_today.id, due_today);

        // Future task (not counted in any special category)
        let mut future = Task::new("Future");
        future.due_date = Some(today + Duration::days(5));
        tasks.insert(future.id, future);

        stats.rebuild(&tasks);

        assert_eq!(stats.completed_count, 1);
        assert_eq!(stats.overdue_count, 1);
        assert_eq!(stats.due_today_count, 1);
    }

    // ========================================================================
    // TaskCache Additional Tests
    // ========================================================================

    #[test]
    fn test_task_cache_subtask_progress_with_completed() {
        let mut cache = TaskCache::new();
        let mut tasks = HashMap::new();

        // Parent task
        let parent = Task::new("Parent");
        let parent_id = parent.id;
        tasks.insert(parent.id, parent);

        // Two children - one completed
        let child1 = Task::new("Child 1")
            .with_parent(parent_id)
            .with_status(TaskStatus::Done);
        let child2 = Task::new("Child 2").with_parent(parent_id);
        tasks.insert(child1.id, child1);
        tasks.insert(child2.id, child2);

        cache.rebuild_hierarchy(&tasks);

        let (completed, total) = cache.get_subtask_progress(parent_id);
        assert_eq!(total, 2);
        assert_eq!(completed, 1);
    }

    #[test]
    fn test_task_cache_time_sums_aggregation() {
        let mut cache = TaskCache::new();
        let mut entries = HashMap::new();
        let task_id = TaskId::new();

        // Add multiple time entries for the same task
        let mut e1 = TimeEntry::start(task_id);
        e1.duration_minutes = Some(30);
        entries.insert(e1.id, e1);

        let mut e2 = TimeEntry::start(task_id);
        e2.duration_minutes = Some(45);
        entries.insert(e2.id, e2);

        cache.rebuild_time_sums(&entries);

        // Should sum up to 75 minutes
        assert_eq!(cache.get_time_sum(task_id), 75);
    }

    #[test]
    fn test_task_cache_deep_hierarchy() {
        let mut cache = TaskCache::new();
        let mut tasks = HashMap::new();

        // Create a deep hierarchy: root -> child -> grandchild -> great-grandchild
        let root = Task::new("Root");
        let root_id = root.id;
        tasks.insert(root.id, root);

        let child = Task::new("Child").with_parent(root_id);
        let child_id = child.id;
        tasks.insert(child.id, child);

        let grandchild = Task::new("Grandchild").with_parent(child_id);
        let grandchild_id = grandchild.id;
        tasks.insert(grandchild.id, grandchild);

        let great_grandchild = Task::new("Great-grandchild").with_parent(grandchild_id);
        let great_grandchild_id = great_grandchild.id;
        tasks.insert(great_grandchild.id, great_grandchild);

        cache.rebuild_hierarchy(&tasks);

        assert_eq!(cache.get_depth(root_id), 0);
        assert_eq!(cache.get_depth(child_id), 1);
        assert_eq!(cache.get_depth(grandchild_id), 2);
        assert_eq!(cache.get_depth(great_grandchild_id), 3);

        // Root should have 3 descendants
        let (_, total) = cache.get_subtask_progress(root_id);
        assert_eq!(total, 3);
    }
}
