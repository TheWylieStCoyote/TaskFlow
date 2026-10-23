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

use super::TimeWindow;
use crate::domain::analytics::AnalyticsReport;
use crate::domain::{
    is_context_tag, ProjectId, Task, TaskId, TimeEntry, TimeEntryId, WorkLogEntry, WorkLogEntryId,
};

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
    /// Task IDs grouped by scheduled date (only tasks with a scheduled_date)
    pub tasks_by_scheduled_date: HashMap<NaiveDate, Vec<TaskId>>,
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
        self.tasks_by_scheduled_date.clear();
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
    /// - Depth for each task (BFS from roots, O(n) total)
    /// - Subtask progress for each task with children
    pub fn rebuild_hierarchy(&mut self, tasks: &HashMap<TaskId, Task>) {
        use std::collections::VecDeque;

        self.depths.clear();
        self.children.clear();
        self.subtask_progress.clear();

        // Build parent->children map in one pass
        for (id, task) in tasks {
            if let Some(parent_id) = task.parent_task_id {
                self.children.entry(parent_id).or_default().push(*id);
            }
        }

        // Compute depths via BFS from root tasks — O(n) total.
        // The previous approach called traverse_ancestors() per task which was O(n²)
        // for deep chains (each call walked all the way to the root without reusing
        // previously computed depths along the path).
        let mut queue = VecDeque::new();
        for (id, task) in tasks {
            if task.parent_task_id.is_none() {
                self.depths.insert(*id, 0);
                queue.push_back(*id);
            }
        }
        while let Some(parent_id) = queue.pop_front() {
            let parent_depth = self.depths.get(&parent_id).copied().unwrap_or(0);
            if let Some(children) = self.children.get(&parent_id) {
                for &child_id in children {
                    if let std::collections::hash_map::Entry::Vacant(e) =
                        self.depths.entry(child_id)
                    {
                        e.insert(parent_depth + 1);
                        queue.push_back(child_id);
                    }
                }
            }
        }
        // Assign depth 0 to any orphaned tasks whose parent doesn't exist in the map
        for id in tasks.keys() {
            self.depths.entry(*id).or_insert(0);
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

    /// Rebuild the scheduled date index from tasks.
    ///
    /// Groups task IDs by their `scheduled_date` for O(1) date-based lookups.
    /// Each day's tasks are sorted by `scheduled_start_time` with untimed tasks
    /// last; ties fall back to creation time so the order is stable across
    /// rebuilds (the source map iterates in arbitrary order).
    pub fn rebuild_scheduled_date_index(&mut self, tasks: &HashMap<TaskId, Task>) {
        self.tasks_by_scheduled_date.clear();
        for (id, task) in tasks {
            if let Some(date) = task.scheduled_date {
                self.tasks_by_scheduled_date
                    .entry(date)
                    .or_default()
                    .push(*id);
            }
        }
        for ids in self.tasks_by_scheduled_date.values_mut() {
            ids.sort_by_key(|id| {
                tasks.get(id).map(|t| {
                    (
                        t.scheduled_start_time.is_none(),
                        t.scheduled_start_time,
                        t.created_at,
                    )
                })
            });
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
        self.rebuild_scheduled_date_index(tasks);
        self.rebuild_contexts(tasks);
    }
}

/// Cached analytics reports for different time windows.
///
/// Reports are expensive to compute as they iterate through all tasks
/// and time entries. This cache stores pre-computed reports for common
/// time windows (7, 30, 60, 90 days).
#[derive(Debug, Clone, Default)]
pub struct ReportCache {
    /// Report for 7-day window (used by the time panel's 7-day view)
    pub report_7d: Option<AnalyticsReport>,
    /// Report for 30-day window (used by overview, tags, time panels, dashboard)
    pub report_30d: Option<AnalyticsReport>,
    /// Report for 60-day window (used by velocity panel)
    pub report_60d: Option<AnalyticsReport>,
    /// Report for 90-day window (used by insights and time panels)
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
        self.report_7d = None;
        self.report_30d = None;
        self.report_60d = None;
        self.report_90d = None;
    }

    /// Check if any reports are cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.report_7d.is_none()
            && self.report_30d.is_none()
            && self.report_60d.is_none()
            && self.report_90d.is_none()
    }

    /// The cached report backing a Reports → Time window, if generated.
    #[must_use]
    pub fn report_for_window(&self, window: TimeWindow) -> Option<&AnalyticsReport> {
        match window {
            TimeWindow::Days7 => self.report_7d.as_ref(),
            TimeWindow::Days30 => self.report_30d.as_ref(),
            TimeWindow::Days90 => self.report_90d.as_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{today, Task, TaskStatus};

    #[test]
    fn test_footer_stats_default() {
        let stats = FooterStats::default();
        assert_eq!(stats.completed_count, 0);
        assert_eq!(stats.overdue_count, 0);
        assert_eq!(stats.due_today_count, 0);
    }

    fn sample_report(days: i64) -> AnalyticsReport {
        use crate::app::analytics::AnalyticsEngine;
        use crate::app::Model;
        use crate::domain::analytics::ReportConfig;

        let model = Model::new();
        AnalyticsEngine::new(&model).generate_report(&ReportConfig::last_n_days(days))
    }

    #[test]
    fn test_report_cache_window_lookup_and_clear() {
        let mut cache = ReportCache::new();
        assert!(cache.is_empty());
        assert!(cache.report_for_window(TimeWindow::Days7).is_none());

        cache.report_7d = Some(sample_report(7));
        assert!(!cache.is_empty(), "a 7-day report counts as cached");
        assert!(cache.report_for_window(TimeWindow::Days7).is_some());
        assert!(cache.report_for_window(TimeWindow::Days30).is_none());
        assert!(cache.report_for_window(TimeWindow::Days90).is_none());

        cache.report_30d = Some(sample_report(30));
        cache.report_90d = Some(sample_report(90));
        let span = |report: &AnalyticsReport| {
            (report.config.end_date - report.config.start_date).num_days()
        };
        assert_eq!(
            cache.report_for_window(TimeWindow::Days30).map(span),
            Some(29),
            "30 calendar dates, today included"
        );
        assert_eq!(
            cache.report_for_window(TimeWindow::Days90).map(span),
            Some(89)
        );

        cache.clear();
        assert!(cache.is_empty());
        assert!(cache.report_7d.is_none());
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
        use chrono::Duration;

        let mut stats = FooterStats::default();
        let mut tasks = HashMap::new();

        // Add an overdue task (due yesterday)
        let mut overdue_task = Task::new("Overdue");
        overdue_task.due_date = Some(today() - Duration::days(1));
        tasks.insert(overdue_task.id, overdue_task);

        stats.rebuild(&tasks);

        assert_eq!(stats.completed_count, 0);
        assert_eq!(stats.overdue_count, 1);
        assert_eq!(stats.due_today_count, 0);
    }

    #[test]
    fn test_footer_stats_with_due_today_task() {
        let mut stats = FooterStats::default();
        let mut tasks = HashMap::new();

        // Add a task due today
        let mut today_task = Task::new("Due Today");
        today_task.due_date = Some(today());
        tasks.insert(today_task.id, today_task);

        stats.rebuild(&tasks);

        assert_eq!(stats.completed_count, 0);
        assert_eq!(stats.overdue_count, 0);
        assert_eq!(stats.due_today_count, 1);
    }

    #[test]
    fn test_footer_stats_combined() {
        use chrono::Duration;

        let mut stats = FooterStats::default();
        let mut tasks = HashMap::new();
        let today = today();

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

    fn scheduled(title: &str, date: NaiveDate, start: Option<(u32, u32)>) -> Task {
        let mut task = Task::new(title);
        task.scheduled_date = Some(date);
        task.scheduled_start_time =
            start.and_then(|(h, m)| chrono::NaiveTime::from_hms_opt(h, m, 0));
        task
    }

    #[test]
    fn test_scheduled_date_index_orders_by_start_time_with_untimed_last() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 14).unwrap();
        let mut tasks = HashMap::new();
        let untimed = scheduled("Untimed", date, None);
        let late = scheduled("Late", date, Some((15, 0)));
        let early = scheduled("Early", date, Some((8, 30)));
        let other_day = scheduled("Other day", date.succ_opt().unwrap(), Some((7, 0)));
        let unscheduled = Task::new("Unscheduled");
        let (untimed_id, late_id, early_id) = (untimed.id, late.id, early.id);
        for t in [untimed, late, early, other_day, unscheduled] {
            tasks.insert(t.id, t);
        }

        let mut cache = TaskCache::new();
        cache.rebuild_scheduled_date_index(&tasks);

        assert_eq!(
            cache.tasks_by_scheduled_date.get(&date).cloned(),
            Some(vec![early_id, late_id, untimed_id])
        );
        assert_eq!(
            cache.tasks_by_scheduled_date.len(),
            2,
            "one entry per scheduled day"
        );
        assert!(
            !cache
                .tasks_by_scheduled_date
                .values()
                .flatten()
                .any(|id| { tasks[id].scheduled_date.is_none() }),
            "tasks without a scheduled date are not indexed"
        );
    }

    #[test]
    fn test_scheduled_date_index_ties_are_stable_across_rebuilds() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 14).unwrap();
        let mut tasks = HashMap::new();
        for i in 0..8 {
            let mut task = scheduled(&format!("Block {i}"), date, Some((9, 0)));
            task.created_at = chrono::Utc::now() + chrono::Duration::seconds(i);
            tasks.insert(task.id, task);
        }

        let mut cache = TaskCache::new();
        cache.rebuild_scheduled_date_index(&tasks);
        let first = cache.tasks_by_scheduled_date[&date].clone();
        let titles: Vec<_> = first.iter().map(|id| tasks[id].title.clone()).collect();
        assert_eq!(
            titles,
            (0..8).map(|i| format!("Block {i}")).collect::<Vec<_>>(),
            "same start time falls back to creation order"
        );
        for _ in 0..5 {
            cache.rebuild_scheduled_date_index(&tasks);
            assert_eq!(cache.tasks_by_scheduled_date[&date], first);
        }
    }

    #[test]
    fn test_scheduled_date_index_follows_mutations_and_clear() {
        let day_a = NaiveDate::from_ymd_opt(2024, 3, 14).unwrap();
        let day_b = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
        let mut tasks = HashMap::new();
        let task = scheduled("Movable", day_a, Some((10, 0)));
        let id = task.id;
        tasks.insert(id, task);

        let mut cache = TaskCache::new();
        cache.rebuild_secondary_indexes(&tasks);
        assert_eq!(cache.tasks_by_scheduled_date.get(&day_a), Some(&vec![id]));

        // Rescheduling moves the id to the new day
        tasks.get_mut(&id).unwrap().scheduled_date = Some(day_b);
        cache.rebuild_secondary_indexes(&tasks);
        assert!(!cache.tasks_by_scheduled_date.contains_key(&day_a));
        assert_eq!(cache.tasks_by_scheduled_date.get(&day_b), Some(&vec![id]));

        // Unscheduling drops it
        tasks.get_mut(&id).unwrap().scheduled_date = None;
        cache.rebuild_secondary_indexes(&tasks);
        assert!(cache.tasks_by_scheduled_date.is_empty());

        tasks.get_mut(&id).unwrap().scheduled_date = Some(day_a);
        cache.rebuild_secondary_indexes(&tasks);
        cache.clear();
        assert!(cache.tasks_by_scheduled_date.is_empty());
    }
}
