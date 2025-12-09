//! Performance caches for expensive computations.
//!
//! These caches store pre-computed values that would otherwise require
//! O(n) or O(n²) operations during rendering. Caches are invalidated
//! when the underlying data changes.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use ratatui::layout::Rect;

use crate::domain::{Task, TaskId, TimeEntry, TimeEntryId};

/// Internal layout data for mouse hit-testing.
#[derive(Debug, Clone, Default)]
struct LayoutData {
    /// Sidebar area (if visible)
    sidebar_area: Option<Rect>,
    /// Main content area (task list, calendar, etc.)
    main_area: Option<Rect>,
    /// Task list area within main area
    task_list_area: Option<Rect>,
    /// Calendar grid area (calendar view only)
    calendar_area: Option<Rect>,
    /// Individual kanban column areas
    kanban_columns: [Option<Rect>; 4],
    /// Eisenhower quadrant areas
    eisenhower_quadrants: [Option<Rect>; 4],
    /// Weekly planner day column areas
    weekly_planner_days: [Option<Rect>; 7],
    /// Reports tabs area (for clicking on tabs)
    reports_tabs_area: Option<Rect>,
    /// Individual report tab areas for precise click detection
    reports_tab_rects: [Option<Rect>; 7],
    /// Header height offset for task list (border + header row)
    task_list_header_offset: u16,
    /// Scroll offset for task list (how many rows scrolled)
    scroll_offset: usize,
    /// Last click position and time for double-click detection
    last_click: Option<(u16, u16, Instant)>,
}

/// Cached layout rectangles for mouse hit-testing.
///
/// Uses interior mutability so layout can be updated during rendering
/// even when the Model is borrowed immutably.
#[derive(Debug, Default)]
pub struct LayoutCache {
    data: RefCell<LayoutData>,
}

impl Clone for LayoutCache {
    fn clone(&self) -> Self {
        Self {
            data: RefCell::new(self.data.borrow().clone()),
        }
    }
}

impl LayoutCache {
    /// Clear all cached layout areas.
    pub fn clear(&self) {
        let mut data = self.data.borrow_mut();
        data.sidebar_area = None;
        data.main_area = None;
        data.task_list_area = None;
        data.calendar_area = None;
        data.kanban_columns = [None; 4];
        data.eisenhower_quadrants = [None; 4];
        data.weekly_planner_days = [None; 7];
        data.reports_tabs_area = None;
        data.reports_tab_rects = [None; 7];
    }

    /// Check if a point is within a rectangle.
    #[must_use]
    pub fn is_in_rect(x: u16, y: u16, rect: Rect) -> bool {
        x >= rect.x
            && x < rect.x.saturating_add(rect.width)
            && y >= rect.y
            && y < rect.y.saturating_add(rect.height)
    }

    /// Record a click for double-click detection.
    pub fn record_click(&self, x: u16, y: u16) {
        self.data.borrow_mut().last_click = Some((x, y, Instant::now()));
    }

    /// Check if the current click is a double-click.
    #[must_use]
    pub fn is_double_click(&self, x: u16, y: u16) -> bool {
        let data = self.data.borrow();
        if let Some((last_x, last_y, last_time)) = data.last_click {
            let same_position =
                (x as i16 - last_x as i16).abs() <= 1 && (y as i16 - last_y as i16).abs() <= 1;
            let within_time = last_time.elapsed().as_millis() < 500;
            same_position && within_time
        } else {
            false
        }
    }

    // Layout setters (for use during rendering)

    /// Set the sidebar area.
    pub fn set_sidebar_area(&self, area: Rect) {
        self.data.borrow_mut().sidebar_area = Some(area);
    }

    /// Set the main content area.
    pub fn set_main_area(&self, area: Rect) {
        self.data.borrow_mut().main_area = Some(area);
    }

    /// Set the task list area.
    pub fn set_task_list_area(&self, area: Rect, header_offset: u16, scroll_offset: usize) {
        let mut data = self.data.borrow_mut();
        data.task_list_area = Some(area);
        data.task_list_header_offset = header_offset;
        data.scroll_offset = scroll_offset;
    }

    /// Set the calendar grid area.
    pub fn set_calendar_area(&self, area: Rect) {
        self.data.borrow_mut().calendar_area = Some(area);
    }

    /// Set a kanban column area.
    pub fn set_kanban_column(&self, index: usize, area: Rect) {
        if index < 4 {
            self.data.borrow_mut().kanban_columns[index] = Some(area);
        }
    }

    /// Set an eisenhower quadrant area.
    pub fn set_eisenhower_quadrant(&self, index: usize, area: Rect) {
        if index < 4 {
            self.data.borrow_mut().eisenhower_quadrants[index] = Some(area);
        }
    }

    /// Set a weekly planner day column area.
    pub fn set_weekly_planner_day(&self, index: usize, area: Rect) {
        if index < 7 {
            self.data.borrow_mut().weekly_planner_days[index] = Some(area);
        }
    }

    /// Set the reports tabs area.
    pub fn set_reports_tabs_area(&self, area: Rect) {
        self.data.borrow_mut().reports_tabs_area = Some(area);
    }

    /// Set an individual report tab area.
    pub fn set_reports_tab_rect(&self, index: usize, area: Rect) {
        if index < 7 {
            self.data.borrow_mut().reports_tab_rects[index] = Some(area);
        }
    }

    // Layout getters (for use in mouse handling)

    /// Get the sidebar area.
    #[must_use]
    pub fn sidebar_area(&self) -> Option<Rect> {
        self.data.borrow().sidebar_area
    }

    /// Get the main content area.
    #[must_use]
    pub fn main_area(&self) -> Option<Rect> {
        self.data.borrow().main_area
    }

    /// Get the task list area.
    #[must_use]
    pub fn task_list_area(&self) -> Option<Rect> {
        self.data.borrow().task_list_area
    }

    /// Get the task list header offset.
    #[must_use]
    pub fn task_list_header_offset(&self) -> u16 {
        self.data.borrow().task_list_header_offset
    }

    /// Get the task list scroll offset.
    #[must_use]
    pub fn scroll_offset(&self) -> usize {
        self.data.borrow().scroll_offset
    }

    /// Get the calendar grid area.
    #[must_use]
    pub fn calendar_area(&self) -> Option<Rect> {
        self.data.borrow().calendar_area
    }

    /// Get a kanban column area.
    #[must_use]
    pub fn kanban_column(&self, index: usize) -> Option<Rect> {
        if index < 4 {
            self.data.borrow().kanban_columns[index]
        } else {
            None
        }
    }

    /// Get all kanban column areas.
    #[must_use]
    pub fn kanban_columns(&self) -> [Option<Rect>; 4] {
        self.data.borrow().kanban_columns
    }

    /// Get an eisenhower quadrant area.
    #[must_use]
    pub fn eisenhower_quadrant(&self, index: usize) -> Option<Rect> {
        if index < 4 {
            self.data.borrow().eisenhower_quadrants[index]
        } else {
            None
        }
    }

    /// Get all eisenhower quadrant areas.
    #[must_use]
    pub fn eisenhower_quadrants(&self) -> [Option<Rect>; 4] {
        self.data.borrow().eisenhower_quadrants
    }

    /// Get a weekly planner day column area.
    #[must_use]
    pub fn weekly_planner_day(&self, index: usize) -> Option<Rect> {
        if index < 7 {
            self.data.borrow().weekly_planner_days[index]
        } else {
            None
        }
    }

    /// Get all weekly planner day column areas.
    #[must_use]
    pub fn weekly_planner_days(&self) -> [Option<Rect>; 7] {
        self.data.borrow().weekly_planner_days
    }

    /// Get the reports tabs area.
    #[must_use]
    pub fn reports_tabs_area(&self) -> Option<Rect> {
        self.data.borrow().reports_tabs_area
    }

    /// Get all report tab areas.
    #[must_use]
    pub fn reports_tab_rects(&self) -> [Option<Rect>; 7] {
        self.data.borrow().reports_tab_rects
    }
}

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
    }

    /// Rebuild time sum cache from time entries.
    ///
    /// Groups time entries by task_id and sums durations in a single pass.
    pub fn rebuild_time_sums(&mut self, time_entries: &HashMap<TimeEntryId, TimeEntry>) {
        self.time_sums.clear();

        for entry in time_entries.values() {
            *self.time_sums.entry(entry.task_id).or_insert(0) +=
                entry.calculated_duration_minutes();
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

        // Compute subtask progress for tasks with children
        for parent_id in self.children.keys() {
            let descendants = self.get_all_descendants_cached(*parent_id);
            let total = descendants.len();
            let completed = descendants
                .iter()
                .filter(|id| tasks.get(id).is_some_and(|t| t.status.is_complete()))
                .count();
            self.subtask_progress.insert(*parent_id, (completed, total));
        }
    }

    /// Compute and cache depth for a single task.
    fn compute_depth(&mut self, task_id: TaskId, tasks: &HashMap<TaskId, Task>) -> usize {
        // Check cache first
        if let Some(&depth) = self.depths.get(&task_id) {
            return depth;
        }

        let mut depth = 0;
        let mut current_id = task_id;
        let mut visited = HashSet::new();

        while let Some(task) = tasks.get(&current_id) {
            if let Some(parent_id) = task.parent_task_id {
                if visited.contains(&parent_id) {
                    break; // Cycle detected
                }
                visited.insert(current_id);
                depth += 1;
                current_id = parent_id;
            } else {
                break;
            }
        }

        self.depths.insert(task_id, depth);
        depth
    }

    /// Get all descendants using the cached children map.
    fn get_all_descendants_cached(&self, task_id: TaskId) -> Vec<TaskId> {
        let mut descendants = Vec::new();
        let mut stack = vec![task_id];
        let mut visited = HashSet::new();

        while let Some(current_id) = stack.pop() {
            if visited.contains(&current_id) {
                continue;
            }
            visited.insert(current_id);

            if let Some(children) = self.children.get(&current_id) {
                for child_id in children {
                    descendants.push(*child_id);
                    stack.push(*child_id);
                }
            }
        }
        descendants
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
}
