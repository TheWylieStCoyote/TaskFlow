//! Task filtering and sorting methods for the Model.

use std::collections::HashMap;

use chrono::{Datelike, NaiveDate, Utc};

use crate::domain::{ProjectId, SortField, SortOrder, TagFilterMode, Task, TaskId};

use super::{Model, ViewId};

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
        match self.current_view {
            // TaskList and Dashboard show all tasks
            ViewId::TaskList | ViewId::Dashboard => true,
            ViewId::Today => {
                // Show tasks due today
                task.due_date.is_some_and(|d| d == Utc::now().date_naive())
            }
            ViewId::Upcoming => {
                // Show tasks with future due dates
                task.due_date.is_some_and(|d| d > Utc::now().date_naive())
            }
            ViewId::Overdue => {
                // Show tasks with past due dates (before today)
                task.due_date.is_some_and(|d| d < Utc::now().date_naive())
            }
            ViewId::Scheduled => {
                // Show tasks with scheduled dates
                task.scheduled_date.is_some()
            }
            ViewId::Calendar => {
                // Show tasks for the selected day in calendar (if any)
                self.calendar_state.selected_day.map_or_else(
                    || {
                        // No day selected, show tasks for the entire month
                        task.due_date.is_some_and(|d| {
                            d.year() == self.calendar_state.year
                                && d.month() == self.calendar_state.month
                        })
                    },
                    |selected_day| {
                        NaiveDate::from_ymd_opt(
                            self.calendar_state.year,
                            self.calendar_state.month,
                            selected_day,
                        )
                        .is_some_and(|date| task.due_date == Some(date))
                    },
                )
            }
            ViewId::Projects => {
                // Show tasks that belong to a project
                task.project_id.is_some()
            }
            ViewId::Blocked => {
                // Show tasks with incomplete dependencies
                !task.dependencies.is_empty()
                    && task.dependencies.iter().any(|dep_id| {
                        self.tasks
                            .get(dep_id)
                            .is_none_or(|d| !d.status.is_complete())
                    })
            }
            ViewId::Untagged => {
                // Show tasks without any tags
                task.tags.is_empty()
            }
            ViewId::NoProject => {
                // Show tasks not assigned to any project
                task.project_id.is_none()
            }
            ViewId::RecentlyModified => {
                // Show tasks modified in the last 7 days
                let week_ago = Utc::now() - chrono::Duration::days(7);
                task.updated_at >= week_ago
            }
            ViewId::Reports => {
                // Reports view shows all tasks (used for analytics)
                true
            }
            ViewId::Kanban => {
                // Kanban view shows all tasks (grouped by status in the UI)
                true
            }
            ViewId::Eisenhower => {
                // Eisenhower matrix shows all tasks (grouped by urgency/importance in the UI)
                true
            }
            ViewId::WeeklyPlanner => {
                // Weekly planner shows tasks with due dates or scheduled dates in the current week
                let today = Utc::now().date_naive();
                let week_start =
                    today - chrono::Duration::days(today.weekday().num_days_from_monday().into());
                let week_end = week_start + chrono::Duration::days(6);

                task.due_date
                    .is_some_and(|d| d >= week_start && d <= week_end)
                    || task
                        .scheduled_date
                        .is_some_and(|d| d >= week_start && d <= week_end)
            }
            ViewId::Snoozed => {
                // Show only snoozed tasks
                task.is_snoozed()
            }
            ViewId::Habits => {
                // Habits view shows habits, not tasks - filter out all tasks
                false
            }
            ViewId::Timeline => {
                // Timeline shows tasks with at least one date (scheduled or due)
                task.scheduled_date.is_some() || task.due_date.is_some()
            }
            ViewId::Heatmap | ViewId::Forecast | ViewId::Network | ViewId::Burndown => {
                // Analytics views show all tasks
                true
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
