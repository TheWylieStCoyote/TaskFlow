//! View-specific query methods for the Model.
//!
//! This module contains methods that return task collections for specific views:
//! - Kanban columns
//! - Eisenhower matrix quadrants
//! - Weekly planner days
//! - Network/dependency graph
//! - Project groupings
//! - Git TODO groupings

use std::collections::HashSet;

use chrono::{Datelike, Utc};

use crate::domain::{Priority, ProjectId, Task, TaskId, TaskStatus};

use super::Model;

impl Model {
    /// Returns task IDs for a specific Kanban column (by status).
    ///
    /// Column indices: 0=Todo, 1=InProgress, 2=Blocked, 3=Done
    #[must_use]
    pub fn kanban_column_tasks(&self, column: usize) -> Vec<TaskId> {
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
        let mut network_ids: HashSet<TaskId> = HashSet::new();

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
        use std::collections::HashMap;

        // Group visible tasks by project_id using HashMap for O(1) lookup
        let mut grouped: HashMap<Option<ProjectId>, Vec<TaskId>> = HashMap::new();

        for task_id in &self.visible_tasks {
            if let Some(task) = self.tasks.get(task_id) {
                grouped.entry(task.project_id).or_default().push(*task_id);
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
    /// and groups tasks by file. Returns `Vec<(file_path, Vec<(TaskId, line_num)>)>`.
    #[must_use]
    pub fn get_git_tasks_grouped_by_file(&self) -> Vec<(String, Vec<(TaskId, usize)>)> {
        let mut grouped: Vec<(String, Vec<(TaskId, usize)>)> = Vec::new();

        for task_id in &self.visible_tasks {
            if let Some(task) = self.tasks.get(task_id) {
                if let Some(ref desc) = task.description {
                    if let Some((file, line)) = extract_git_location(desc) {
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
}

/// Extract file path and line number from a git-todo task description.
///
/// Parses the `git:<file>:<line>` marker at the start of the description.
///
/// # Examples
///
/// ```
/// use taskflow::app::extract_git_location;
///
/// let desc = "git:src/main.rs:42\n\nSome content";
/// assert_eq!(extract_git_location(desc), Some(("src/main.rs".to_string(), 42)));
///
/// let desc = "No git marker";
/// assert_eq!(extract_git_location(desc), None);
/// ```
#[must_use]
pub fn extract_git_location(description: &str) -> Option<(String, usize)> {
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
