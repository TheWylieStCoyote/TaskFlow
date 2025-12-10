//! Task filtering and sorting methods for the Model.

use std::collections::HashMap;

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
        let sort_field = self.sort.field;
        let sort_order = self.sort.order;

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

        // Recursive helper to add a task and all its descendants
        fn add_with_descendants(
            task_id: &TaskId,
            subtasks_by_parent: &HashMap<&TaskId, Vec<&Task>>,
            result: &mut Vec<TaskId>,
        ) {
            result.push(*task_id);
            if let Some(children) = subtasks_by_parent.get(task_id) {
                for child in children {
                    add_with_descendants(&child.id, subtasks_by_parent, result);
                }
            }
        }

        for parent in parent_tasks {
            add_with_descendants(&parent.id, &subtasks_by_parent, &mut result);
        }

        // Handle orphaned subtasks (subtasks whose parent is not visible)
        // These are shown at the end
        for task in &filtered_tasks {
            if task.parent_task_id.is_some() && !result.contains(&task.id) {
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
        if !self.show_completed && task.status.is_complete() {
            return false;
        }

        // Filter out snoozed tasks unless viewing the Snoozed view
        if self.current_view != ViewId::Snoozed && task.is_snoozed() {
            return false;
        }

        // Filter by search text (case-insensitive, matches title or tags)
        if let Some(ref search) = self.filter.search_text {
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
        if let Some(ref filter_tags) = self.filter.tags {
            let has_tags = match self.filter.tags_mode {
                TagFilterMode::Any => {
                    // Task must have at least one of the filter tags
                    filter_tags.iter().any(|ft| {
                        task.tags
                            .iter()
                            .any(|t| t.to_lowercase() == ft.to_lowercase())
                    })
                }
                TagFilterMode::All => {
                    // Task must have all of the filter tags
                    filter_tags.iter().all(|ft| {
                        task.tags
                            .iter()
                            .any(|t| t.to_lowercase() == ft.to_lowercase())
                    })
                }
            };
            if !has_tags {
                return false;
            }
        }

        // Filter by priority (if set)
        if let Some(ref priorities) = self.filter.priority {
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
}
