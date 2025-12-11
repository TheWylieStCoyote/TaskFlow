//! Task filtering and sorting methods for the Model.

use std::collections::{HashMap, HashSet};

use chrono::{Datelike, NaiveDate, Utc};

use crate::domain::{ProjectId, SortField, SortOrder, TagFilterMode, Task, TaskId};

use super::{Model, ViewId};

impl Model {
    // ========================================================================
    // Selection Helpers
    // ========================================================================

    /// Returns the TaskId of the currently selected task, if any.
    ///
    /// Returns `None` if no task is selected or the selection index is out of bounds.
    #[inline]
    pub fn selected_task_id(&self) -> Option<TaskId> {
        self.visible_tasks.get(self.selected_index).copied()
    }

    // ========================================================================
    // Cache Management
    // ========================================================================

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
        // Collect all tasks that pass the filter
        let filtered_tasks: Vec<_> = self
            .tasks
            .values()
            .filter(|task| self.task_matches_filter(task))
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

    fn task_matches_filter(&self, task: &Task) -> bool {
        // Filter out completed tasks unless show_completed is true
        if !self.filtering.show_completed && task.status.is_complete() {
            return false;
        }

        // Filter out snoozed tasks unless viewing the Snoozed view
        if self.current_view != ViewId::Snoozed && task.is_snoozed() {
            return false;
        }

        // Filter by search text (case-insensitive, matches title or tags)
        if let Some(ref search) = self.filtering.filter.search_text {
            let search_lower = search.to_lowercase();
            let title_matches = task.title.to_lowercase().contains(&search_lower);
            let tags_match = task
                .tags
                .iter()
                .any(|t| t.to_lowercase().contains(&search_lower));
            if !title_matches && !tags_match {
                return false;
            }
        }

        // Filter by tags (if set)
        if let Some(ref filter_tags) = self.filtering.filter.tags {
            // Pre-compute lowercased filter tags to avoid repeated allocations
            let filter_tags_lower: Vec<String> =
                filter_tags.iter().map(|t| t.to_lowercase()).collect();
            // Pre-compute lowercased task tags
            let task_tags_lower: Vec<String> = task.tags.iter().map(|t| t.to_lowercase()).collect();

            let has_tags = match self.filtering.filter.tags_mode {
                TagFilterMode::Any => {
                    // Task must have at least one of the filter tags
                    filter_tags_lower
                        .iter()
                        .any(|ft| task_tags_lower.iter().any(|t| t == ft))
                }
                TagFilterMode::All => {
                    // Task must have all of the filter tags
                    filter_tags_lower
                        .iter()
                        .all(|ft| task_tags_lower.iter().any(|t| t == ft))
                }
            };
            if !has_tags {
                return false;
            }
        }

        // Filter by priority (if set)
        if let Some(ref priorities) = self.filtering.filter.priority {
            if !priorities.contains(&task.priority) {
                return false;
            }
        }

        // Filter by selected project if any
        if let Some(ref project_id) = self.selected_project {
            if task.project_id.as_ref() != Some(project_id) {
                return false;
            }
        }

        // Filter by current view
        self.task_matches_view(task)
    }

    /// Checks if a task matches the current view's criteria.
    ///
    /// Views are grouped by behavior:
    /// - Aggregate views (TaskList, Dashboard, etc.): show all tasks
    /// - Date-based views (Today, Upcoming, etc.): filter by dates
    /// - Property views (Projects, Untagged, etc.): filter by task properties
    fn task_matches_view(&self, task: &Task) -> bool {
        let today = Utc::now().date_naive();

        match self.current_view {
            // Aggregate views - show all tasks (UI groups/filters them)
            ViewId::TaskList
            | ViewId::Dashboard
            | ViewId::Reports
            | ViewId::Kanban
            | ViewId::Eisenhower
            | ViewId::Heatmap
            | ViewId::Forecast
            | ViewId::Network
            | ViewId::Burndown => true,

            // Git TODOs view - tasks with git-todo tag
            ViewId::GitTodos => task.tags.iter().any(|t| t == "git-todo"),

            // Non-task view - filter out all tasks
            ViewId::Habits => false,

            // Date-based views
            ViewId::Today => task.due_date == Some(today),
            ViewId::Upcoming => task.due_date.is_some_and(|d| d > today),
            ViewId::Overdue => task.due_date.is_some_and(|d| d < today),
            ViewId::Scheduled => task.scheduled_date.is_some(),
            ViewId::Snoozed => task.is_snoozed(),
            ViewId::Timeline => task.scheduled_date.is_some() || task.due_date.is_some(),
            ViewId::RecentlyModified => {
                let week_ago = Utc::now() - chrono::Duration::days(7);
                task.updated_at >= week_ago
            }
            ViewId::WeeklyPlanner => {
                let week_start =
                    today - chrono::Duration::days(today.weekday().num_days_from_monday().into());
                let week_end = week_start + chrono::Duration::days(6);
                let in_week = |d: NaiveDate| d >= week_start && d <= week_end;
                task.due_date.is_some_and(in_week) || task.scheduled_date.is_some_and(in_week)
            }
            ViewId::Calendar => self.calendar_state.selected_day.map_or_else(
                || {
                    // No day selected - show tasks for the entire month
                    task.due_date.is_some_and(|d| {
                        d.year() == self.calendar_state.year
                            && d.month() == self.calendar_state.month
                    })
                },
                |day| {
                    NaiveDate::from_ymd_opt(
                        self.calendar_state.year,
                        self.calendar_state.month,
                        day,
                    )
                    .is_some_and(|date| task.due_date == Some(date))
                },
            ),

            // Property-based views
            ViewId::Projects => task.project_id.is_some(),
            ViewId::NoProject => task.project_id.is_none(),
            ViewId::Untagged => task.tags.is_empty(),
            ViewId::Blocked => {
                !task.dependencies.is_empty()
                    && task.dependencies.iter().any(|dep_id| {
                        self.tasks
                            .get(dep_id)
                            .is_none_or(|d| !d.status.is_complete())
                    })
            }
        }
    }

    /// Returns the currently selected task, if any.
    ///
    /// Returns `None` if no tasks are visible or the selection is invalid.
    #[must_use]
    pub fn selected_task(&self) -> Option<&Task> {
        self.visible_tasks
            .get(self.selected_index)
            .and_then(|id| self.tasks.get(id))
    }

    /// Returns the currently selected task mutably, if any.
    #[must_use]
    pub fn selected_task_mut(&mut self) -> Option<&mut Task> {
        let id = *self.visible_tasks.get(self.selected_index)?;
        self.tasks.get_mut(&id)
    }

    /// Returns task IDs for a specific Kanban column (by status).
    ///
    /// Column indices: 0=Todo, 1=InProgress, 2=Blocked, 3=Done
    #[must_use]
    pub fn kanban_column_tasks(&self, column: usize) -> Vec<TaskId> {
        use crate::domain::TaskStatus;

        let status = match column {
            0 => TaskStatus::Todo,
            1 => TaskStatus::InProgress,
            2 => TaskStatus::Blocked,
            3 => TaskStatus::Done,
            _ => return Vec::new(),
        };

        self.visible_tasks
            .iter()
            .filter_map(|id| self.tasks.get(id).map(|t| (id, t)))
            .filter(|(_, t)| t.status == status)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Returns task IDs for a specific Eisenhower quadrant.
    ///
    /// Quadrant indices:
    /// - 0: Urgent + Important (Do First)
    /// - 1: Not Urgent + Important (Schedule)
    /// - 2: Urgent + Not Important (Delegate)
    /// - 3: Not Urgent + Not Important (Eliminate)
    ///
    /// Urgency is determined by due date (within 2 days or overdue).
    /// Importance is determined by priority (High or Urgent).
    #[must_use]
    pub fn eisenhower_quadrant_tasks(&self, quadrant: usize) -> Vec<TaskId> {
        use crate::domain::Priority;

        let today = Utc::now().date_naive();

        // Helper to check if task is urgent (due within 2 days or overdue)
        let is_urgent = |task: &Task| {
            task.due_date.is_some_and(|due| {
                let days_until = (due - today).num_days();
                days_until <= 2
            })
        };

        // Helper to check if task is important (High or Urgent priority)
        let is_important = |task: &Task| matches!(task.priority, Priority::High | Priority::Urgent);

        self.visible_tasks
            .iter()
            .filter_map(|id| self.tasks.get(id).map(|t| (*id, t)))
            .filter(|(_, t)| !t.status.is_complete())
            .filter(|(_, task)| {
                let urgent = is_urgent(task);
                let important = is_important(task);
                match quadrant {
                    0 => urgent && important,   // Do First
                    1 => !urgent && important,  // Schedule
                    2 => urgent && !important,  // Delegate
                    3 => !urgent && !important, // Eliminate
                    _ => false,
                }
            })
            .map(|(id, _)| id)
            .collect()
    }

    /// Returns task IDs for a specific day in the Weekly Planner.
    ///
    /// Day indices: 0=Monday, 1=Tuesday, ..., 6=Sunday
    /// Returns tasks that have due_date or scheduled_date on that day of the current week.
    #[must_use]
    pub fn weekly_planner_day_tasks(&self, day: usize) -> Vec<TaskId> {
        if day > 6 {
            return Vec::new();
        }

        // Get the start of the current week (Monday)
        let today = Utc::now().date_naive();
        let week_start =
            today - chrono::Duration::days(today.weekday().num_days_from_monday().into());
        let target_date = week_start + chrono::Duration::days(day as i64);

        self.visible_tasks
            .iter()
            .filter_map(|id| self.tasks.get(id).map(|t| (*id, t)))
            .filter(|(_, task)| {
                task.due_date == Some(target_date) || task.scheduled_date == Some(target_date)
            })
            .map(|(id, _)| id)
            .collect()
    }

    /// Get all tasks that are part of the dependency network (have dependencies or are depended upon).
    #[must_use]
    pub fn network_tasks(&self) -> Vec<TaskId> {
        // Collect all task IDs that have dependencies or are depended upon or have chain links
        let mut network_ids: std::collections::HashSet<TaskId> = std::collections::HashSet::new();

        for task in self.tasks.values() {
            // Include tasks that have dependencies
            if !task.dependencies.is_empty() {
                network_ids.insert(task.id);
                for dep_id in &task.dependencies {
                    network_ids.insert(*dep_id);
                }
            }
            // Include tasks that are part of chains
            if task.next_task_id.is_some() {
                network_ids.insert(task.id);
            }
        }

        // Also include tasks that are pointed to by next_task_id
        for task in self.tasks.values() {
            if let Some(next_id) = task.next_task_id {
                network_ids.insert(next_id);
            }
        }

        // Return as a sorted vector for consistent ordering
        let mut result: Vec<TaskId> = network_ids.into_iter().collect();
        result.sort_by_key(|id| {
            self.tasks
                .get(id)
                .map(|t| t.title.clone())
                .unwrap_or_default()
        });
        result
    }

    /// Returns visible tasks grouped by project.
    ///
    /// Returns a `Vec` of (`Option<ProjectId>`, `project_name`, `Vec<TaskId>`).
    /// Projects are sorted alphabetically, with "No Project" last.
    /// Tasks within each project follow the current sort order.
    #[must_use]
    pub fn get_tasks_grouped_by_project(&self) -> Vec<(Option<ProjectId>, String, Vec<TaskId>)> {
        // Group visible tasks by project_id using a Vec to preserve order
        let mut grouped: Vec<(Option<ProjectId>, Vec<TaskId>)> = Vec::new();

        for task_id in &self.visible_tasks {
            if let Some(task) = self.tasks.get(task_id) {
                let project_id = task.project_id;
                // Find existing group or create new one
                if let Some(group) = grouped.iter_mut().find(|(pid, _)| *pid == project_id) {
                    group.1.push(*task_id);
                } else {
                    grouped.push((project_id, vec![*task_id]));
                }
            }
        }

        // Convert to vec with project names
        let mut result: Vec<(Option<ProjectId>, String, Vec<TaskId>)> = grouped
            .into_iter()
            .map(|(project_id, task_ids)| {
                let name = project_id
                    .as_ref()
                    .and_then(|pid| self.projects.get(pid))
                    .map_or_else(|| "No Project".to_string(), |p| p.name.clone());
                (project_id, name, task_ids)
            })
            .collect();

        // Sort by project name (No Project goes last)
        result.sort_by(|a, b| match (&a.0, &b.0) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Greater, // No Project last
            (Some(_), None) => std::cmp::Ordering::Less,
            (Some(_), Some(_)) => a.1.to_lowercase().cmp(&b.1.to_lowercase()),
        });

        result
    }

    /// Returns git-todo tasks grouped by source file path.
    ///
    /// Parses the `git:<file>:<line>` marker from task descriptions
    /// and groups tasks by file. Returns `Vec<(file_path, line_num, Vec<TaskId>)>`.
    #[must_use]
    pub fn get_git_tasks_grouped_by_file(&self) -> Vec<(String, Vec<(TaskId, usize)>)> {
        let mut grouped: Vec<(String, Vec<(TaskId, usize)>)> = Vec::new();

        for task_id in &self.visible_tasks {
            if let Some(task) = self.tasks.get(task_id) {
                if let Some(ref desc) = task.description {
                    if let Some((file, line)) = Self::extract_git_location(desc) {
                        if let Some(group) = grouped.iter_mut().find(|(f, _)| f == &file) {
                            group.1.push((*task_id, line));
                        } else {
                            grouped.push((file, vec![(*task_id, line)]));
                        }
                    }
                }
            }
        }

        // Sort by file path
        grouped.sort_by(|a, b| a.0.cmp(&b.0));

        // Sort tasks within each group by line number
        for (_, tasks) in &mut grouped {
            tasks.sort_by_key(|(_, line)| *line);
        }

        grouped
    }

    /// Extract file path and line number from a git-todo task description.
    ///
    /// Parses the `git:<file>:<line>` marker at the start of the description.
    fn extract_git_location(description: &str) -> Option<(String, usize)> {
        let first_line = description.lines().next()?;
        if first_line.starts_with("git:") {
            let parts: Vec<&str> = first_line.splitn(3, ':').collect();
            if parts.len() >= 3 {
                let file = parts[1].to_string();
                let line = parts[2].parse().ok()?;
                return Some((file, line));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Priority, Project, Task, TaskStatus};
    use chrono::Duration;

    // ========================================================================
    // View-Specific Task Filtering Tests
    // ========================================================================

    #[test]
    fn test_kanban_column_tasks() {
        let mut model = Model::new();

        let todo = Task::new("Todo task").with_status(TaskStatus::Todo);
        let in_progress = Task::new("In progress").with_status(TaskStatus::InProgress);
        let blocked = Task::new("Blocked").with_status(TaskStatus::Blocked);
        let done = Task::new("Done").with_status(TaskStatus::Done);

        model.tasks.insert(todo.id, todo.clone());
        model.tasks.insert(in_progress.id, in_progress.clone());
        model.tasks.insert(blocked.id, blocked.clone());
        model.tasks.insert(done.id, done.clone());
        model.visible_tasks = vec![todo.id, in_progress.id, blocked.id, done.id];

        assert_eq!(model.kanban_column_tasks(0).len(), 1); // Todo
        assert_eq!(model.kanban_column_tasks(1).len(), 1); // InProgress
        assert_eq!(model.kanban_column_tasks(2).len(), 1); // Blocked
        assert_eq!(model.kanban_column_tasks(3).len(), 1); // Done
        assert!(model.kanban_column_tasks(4).is_empty()); // Invalid column
    }

    #[test]
    fn test_eisenhower_quadrant_tasks() {
        let mut model = Model::new();
        let today = Utc::now().date_naive();

        // Q0: Urgent + Important (due within 2 days, high priority)
        let mut urgent_important = Task::new("Urgent Important");
        urgent_important.priority = Priority::High;
        urgent_important.due_date = Some(today + Duration::days(1));
        model
            .tasks
            .insert(urgent_important.id, urgent_important.clone());

        // Q1: Not Urgent + Important (due later, high priority)
        let mut not_urgent_important = Task::new("Not Urgent Important");
        not_urgent_important.priority = Priority::High;
        not_urgent_important.due_date = Some(today + Duration::days(10));
        model
            .tasks
            .insert(not_urgent_important.id, not_urgent_important.clone());

        // Q2: Urgent + Not Important (due soon, low priority)
        let mut urgent_not_important = Task::new("Urgent Not Important");
        urgent_not_important.priority = Priority::Low;
        urgent_not_important.due_date = Some(today);
        model
            .tasks
            .insert(urgent_not_important.id, urgent_not_important.clone());

        // Q3: Not Urgent + Not Important
        let mut not_urgent_not_important = Task::new("Not Urgent Not Important");
        not_urgent_not_important.priority = Priority::Low;
        not_urgent_not_important.due_date = Some(today + Duration::days(30));
        model.tasks.insert(
            not_urgent_not_important.id,
            not_urgent_not_important.clone(),
        );

        model.visible_tasks = vec![
            urgent_important.id,
            not_urgent_important.id,
            urgent_not_important.id,
            not_urgent_not_important.id,
        ];

        assert_eq!(model.eisenhower_quadrant_tasks(0).len(), 1); // Urgent + Important
        assert_eq!(model.eisenhower_quadrant_tasks(1).len(), 1); // Not Urgent + Important
        assert_eq!(model.eisenhower_quadrant_tasks(2).len(), 1); // Urgent + Not Important
        assert_eq!(model.eisenhower_quadrant_tasks(3).len(), 1); // Not Urgent + Not Important
        assert!(model.eisenhower_quadrant_tasks(5).is_empty()); // Invalid quadrant
    }

    #[test]
    fn test_weekly_planner_day_tasks() {
        let mut model = Model::new();
        let today = Utc::now().date_naive();
        let days_since_monday = today.weekday().num_days_from_monday();
        let monday = today - Duration::days(i64::from(days_since_monday));

        // Task due on Monday (day 0)
        let mut monday_task = Task::new("Monday task");
        monday_task.due_date = Some(monday);
        model.tasks.insert(monday_task.id, monday_task.clone());

        // Task scheduled for Tuesday (day 1)
        let mut tuesday_task = Task::new("Tuesday task");
        tuesday_task.scheduled_date = Some(monday + Duration::days(1));
        model.tasks.insert(tuesday_task.id, tuesday_task.clone());

        model.visible_tasks = vec![monday_task.id, tuesday_task.id];

        assert_eq!(model.weekly_planner_day_tasks(0).len(), 1); // Monday
        assert_eq!(model.weekly_planner_day_tasks(1).len(), 1); // Tuesday
        assert!(model.weekly_planner_day_tasks(2).is_empty()); // Wednesday
        assert!(model.weekly_planner_day_tasks(7).is_empty()); // Invalid day
    }

    #[test]
    fn test_network_tasks() {
        let mut model = Model::new();

        // Task with dependency
        let dep_target = Task::new("Dependency target");
        let mut dependent = Task::new("Dependent task");
        dependent.dependencies.push(dep_target.id);

        // Task in a chain
        let mut chain_start = Task::new("Chain start");
        let chain_end = Task::new("Chain end");
        chain_start.next_task_id = Some(chain_end.id);

        // Standalone task (should not be in network)
        let standalone = Task::new("Standalone");

        model.tasks.insert(dep_target.id, dep_target);
        model.tasks.insert(dependent.id, dependent);
        model.tasks.insert(chain_start.id, chain_start);
        model.tasks.insert(chain_end.id, chain_end);
        model.tasks.insert(standalone.id, standalone);

        let network_tasks = model.network_tasks();

        // Should include dep_target, dependent, chain_start, chain_end
        // but not standalone
        assert_eq!(network_tasks.len(), 4);
    }

    #[test]
    fn test_get_tasks_grouped_by_project() {
        let mut model = Model::new();

        let project1 = Project::new("Alpha Project");
        let project2 = Project::new("Beta Project");
        model.projects.insert(project1.id, project1.clone());
        model.projects.insert(project2.id, project2.clone());

        let mut task1 = Task::new("Task in Alpha");
        task1.project_id = Some(project1.id);
        let mut task2 = Task::new("Task in Beta");
        task2.project_id = Some(project2.id);
        let task3 = Task::new("No project task");

        model.tasks.insert(task1.id, task1.clone());
        model.tasks.insert(task2.id, task2.clone());
        model.tasks.insert(task3.id, task3.clone());
        model.visible_tasks = vec![task1.id, task2.id, task3.id];

        let grouped = model.get_tasks_grouped_by_project();

        // Should have 3 groups: Alpha, Beta, No Project
        assert_eq!(grouped.len(), 3);

        // Alpha Project should come first (alphabetically)
        assert_eq!(grouped[0].1, "Alpha Project");

        // No Project should come last
        assert_eq!(grouped[2].1, "No Project");
    }

    #[test]
    fn test_selected_task_id() {
        let mut model = Model::new();

        let task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");
        let task1_id = task1.id;
        let task2_id = task2.id;

        model.tasks.insert(task1.id, task1);
        model.tasks.insert(task2.id, task2);
        model.visible_tasks = vec![task1_id, task2_id];

        model.selected_index = 0;
        assert_eq!(model.selected_task_id(), Some(task1_id));

        model.selected_index = 1;
        assert_eq!(model.selected_task_id(), Some(task2_id));

        model.selected_index = 10; // Out of bounds
        assert!(model.selected_task_id().is_none());
    }

    #[test]
    fn test_selected_task() {
        let mut model = Model::new();

        let task = Task::new("Selected task");
        let task_id = task.id;
        model.tasks.insert(task.id, task);
        model.visible_tasks = vec![task_id];
        model.selected_index = 0;

        let selected = model.selected_task();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().title, "Selected task");
    }

    #[test]
    fn test_selected_task_mut() {
        let mut model = Model::new();

        let task = Task::new("Mutable task");
        let task_id = task.id;
        model.tasks.insert(task.id, task);
        model.visible_tasks = vec![task_id];
        model.selected_index = 0;

        if let Some(selected) = model.selected_task_mut() {
            selected.title = "Modified title".to_string();
        }

        assert_eq!(model.tasks.get(&task_id).unwrap().title, "Modified title");
    }

    #[test]
    fn test_refresh_visible_tasks_with_sort() {
        let mut model = Model::new();
        model.filtering.sort.field = SortField::Title;
        model.filtering.sort.order = SortOrder::Ascending;

        let task_c = Task::new("Charlie");
        let task_a = Task::new("Alpha");
        let task_b = Task::new("Bravo");

        model.tasks.insert(task_c.id, task_c.clone());
        model.tasks.insert(task_a.id, task_a.clone());
        model.tasks.insert(task_b.id, task_b.clone());

        model.refresh_visible_tasks();

        // Should be sorted alphabetically
        assert_eq!(model.visible_tasks.len(), 3);
        assert_eq!(
            model.tasks.get(&model.visible_tasks[0]).unwrap().title,
            "Alpha"
        );
        assert_eq!(
            model.tasks.get(&model.visible_tasks[1]).unwrap().title,
            "Bravo"
        );
        assert_eq!(
            model.tasks.get(&model.visible_tasks[2]).unwrap().title,
            "Charlie"
        );
    }

    #[test]
    fn test_refresh_visible_tasks_with_subtasks() {
        let mut model = Model::new();

        let parent = Task::new("Parent");
        let parent_id = parent.id;
        let child1 = Task::new("Child 1").with_parent(parent_id);
        let child2 = Task::new("Child 2").with_parent(parent_id);

        model.tasks.insert(parent.id, parent);
        model.tasks.insert(child1.id, child1.clone());
        model.tasks.insert(child2.id, child2.clone());

        model.refresh_visible_tasks();

        // Parent should come before children
        let parent_idx = model
            .visible_tasks
            .iter()
            .position(|id| *id == parent_id)
            .unwrap();
        let child1_idx = model
            .visible_tasks
            .iter()
            .position(|id| *id == child1.id)
            .unwrap();
        let child2_idx = model
            .visible_tasks
            .iter()
            .position(|id| *id == child2.id)
            .unwrap();

        assert!(parent_idx < child1_idx);
        assert!(parent_idx < child2_idx);
    }

    #[test]
    fn test_refresh_visible_tasks_hides_completed() {
        let mut model = Model::new();
        model.filtering.show_completed = false;

        let todo = Task::new("Todo");
        let done = Task::new("Done").with_status(TaskStatus::Done);

        model.tasks.insert(todo.id, todo.clone());
        model.tasks.insert(done.id, done);

        model.refresh_visible_tasks();

        // Should only show incomplete task
        assert_eq!(model.visible_tasks.len(), 1);
        assert_eq!(model.visible_tasks[0], todo.id);
    }

    #[test]
    fn test_refresh_visible_tasks_shows_completed_when_enabled() {
        let mut model = Model::new();
        model.filtering.show_completed = true;

        let todo = Task::new("Todo");
        let done = Task::new("Done").with_status(TaskStatus::Done);

        model.tasks.insert(todo.id, todo);
        model.tasks.insert(done.id, done);

        model.refresh_visible_tasks();

        // Should show both tasks
        assert_eq!(model.visible_tasks.len(), 2);
    }

    #[test]
    fn test_refresh_visible_tasks_by_priority_sort() {
        let mut model = Model::new();
        model.filtering.sort.field = SortField::Priority;
        model.filtering.sort.order = SortOrder::Ascending;

        let low = Task::new("Low priority").with_priority(Priority::Low);
        let urgent = Task::new("Urgent").with_priority(Priority::Urgent);
        let medium = Task::new("Medium").with_priority(Priority::Medium);

        model.tasks.insert(low.id, low.clone());
        model.tasks.insert(urgent.id, urgent.clone());
        model.tasks.insert(medium.id, medium.clone());

        model.refresh_visible_tasks();

        // Urgent should come first
        assert_eq!(
            model.tasks.get(&model.visible_tasks[0]).unwrap().priority,
            Priority::Urgent
        );
    }

    #[test]
    fn test_refresh_visible_tasks_by_due_date_sort() {
        let mut model = Model::new();
        model.filtering.sort.field = SortField::DueDate;
        model.filtering.sort.order = SortOrder::Ascending;
        let today = Utc::now().date_naive();

        let mut soon = Task::new("Due soon");
        soon.due_date = Some(today + Duration::days(1));
        let mut later = Task::new("Due later");
        later.due_date = Some(today + Duration::days(10));
        let no_due = Task::new("No due date");

        model.tasks.insert(soon.id, soon.clone());
        model.tasks.insert(later.id, later.clone());
        model.tasks.insert(no_due.id, no_due);

        model.refresh_visible_tasks();

        // Soon should come first, no due date last
        assert_eq!(
            model.tasks.get(&model.visible_tasks[0]).unwrap().title,
            "Due soon"
        );
        assert_eq!(
            model.tasks.get(&model.visible_tasks[1]).unwrap().title,
            "Due later"
        );
        assert_eq!(
            model.tasks.get(&model.visible_tasks[2]).unwrap().title,
            "No due date"
        );
    }
}
