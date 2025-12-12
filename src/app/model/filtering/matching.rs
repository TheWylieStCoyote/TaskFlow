//! Task filter matching methods.

use std::collections::HashSet;

use chrono::{Datelike, NaiveDate, Utc};

use crate::domain::{TagFilterMode, Task};

use super::super::{Model, ViewId};
use super::FilterCache;

impl Model {
    /// Check if a task matches the current filter using pre-computed cache.
    pub(super) fn task_matches_filter_cached(&self, task: &Task, cache: &FilterCache) -> bool {
        // Filter out completed tasks unless show_completed is true
        if !self.filtering.show_completed && task.status.is_complete() {
            return false;
        }

        // Filter out snoozed tasks unless viewing the Snoozed view
        if self.current_view != ViewId::Snoozed && task.is_snoozed() {
            return false;
        }

        // Filter by search text (case-insensitive, matches title or tags)
        if let Some(ref search_lower) = cache.search_lower {
            let title_matches = task.title.to_lowercase().contains(search_lower);
            let tags_match = task
                .tags
                .iter()
                .any(|t| t.to_lowercase().contains(search_lower));
            if !title_matches && !tags_match {
                return false;
            }
        }

        // Filter by tags (if set) - uses pre-computed lowercase filter tags
        if let Some(ref filter_tags_lower) = cache.filter_tags_lower {
            // Use HashSet for O(1) lookup instead of O(n) iteration
            let task_tags_lower: HashSet<String> =
                task.tags.iter().map(|t| t.to_lowercase()).collect();

            let has_tags = match self.filtering.filter.tags_mode {
                TagFilterMode::Any => {
                    // Task must have at least one of the filter tags
                    filter_tags_lower
                        .iter()
                        .any(|ft| task_tags_lower.contains(ft))
                }
                TagFilterMode::All => {
                    // Task must have all of the filter tags
                    filter_tags_lower
                        .iter()
                        .all(|ft| task_tags_lower.contains(ft))
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

    /// Check if a task matches the current filter (convenience wrapper).
    ///
    /// Note: For bulk filtering, use `task_matches_filter_cached` with a
    /// pre-built `FilterCache` to avoid repeated string allocations.
    #[allow(dead_code)]
    pub(super) fn task_matches_filter(&self, task: &Task) -> bool {
        let cache = FilterCache::new(self);
        self.task_matches_filter_cached(task, &cache)
    }

    /// Checks if a task matches the current view's criteria.
    ///
    /// Views are grouped by behavior:
    /// - Aggregate views (TaskList, Dashboard, etc.): show all tasks
    /// - Date-based views (Today, Upcoming, etc.): filter by dates
    /// - Property views (Projects, Untagged, etc.): filter by task properties
    pub(super) fn task_matches_view(&self, task: &Task) -> bool {
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

            // Non-task views - filter out all tasks (they use their own data)
            ViewId::Habits | ViewId::Goals | ViewId::Duplicates => false,
            // Git TODOs view - tasks with git-todo tag
            ViewId::GitTodos => task.tags.iter().any(|t| t == "git-todo"),

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
}
