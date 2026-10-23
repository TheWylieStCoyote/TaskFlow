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

use chrono::Datelike;

use crate::domain::{Priority, ProjectId, Task, TaskId, TaskStatus};

use super::Model;
use crate::domain::today;
use chrono::{Days, NaiveDate};

impl Model {
    // ── Notification queries ─────────────────────────────────────────────────

    /// Count incomplete tasks due after `today` and no later than `today + days`.
    ///
    /// Tasks due on `today` itself are excluded: the due-today notification
    /// already reports them. `days == 0` therefore always yields zero.
    #[must_use]
    pub fn count_due_soon_from(&self, today: NaiveDate, days: u32) -> usize {
        let cutoff = today
            .checked_add_days(Days::new(u64::from(days)))
            .unwrap_or(NaiveDate::MAX);
        self.tasks
            .values()
            .filter(|t| {
                !t.status.is_complete() && t.due_date.is_some_and(|d| d > today && d <= cutoff)
            })
            .count()
    }

    /// Count incomplete tasks due within the next `days` days (after today).
    ///
    /// Drives the "due soon" desktop notification; see [`Self::count_due_soon_from`].
    #[must_use]
    pub fn count_due_soon(&self, days: u32) -> usize {
        self.count_due_soon_from(today(), days)
    }

    /// Count incomplete tasks whose snooze ended on `date` (`snooze_until == date`).
    ///
    /// A task is snoozed while `snooze_until` is after today, so on the day it
    /// equals the task becomes visible again; the user may not notice without
    /// a notification. Tasks snoozed into the future or whose snooze ended
    /// earlier are not counted.
    #[must_use]
    pub fn count_snooze_expired_on(&self, date: NaiveDate) -> usize {
        self.tasks
            .values()
            .filter(|t| !t.status.is_complete() && t.snooze_until == Some(date))
            .count()
    }

    /// Count incomplete tasks whose snooze ended today.
    ///
    /// Drives the "snooze expired" desktop notification; see
    /// [`Self::count_snooze_expired_on`].
    #[must_use]
    pub fn count_snooze_expired_today(&self) -> usize {
        self.count_snooze_expired_on(today())
    }

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
        let today = today();

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
        let today = today();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Priority, Project, Task, TaskStatus};

    fn make_model() -> Model {
        Model::new()
    }

    fn add_task(model: &mut Model, title: &str) -> TaskId {
        let task = Task::new(title);
        let id = task.id;
        model.tasks.insert(id, task);
        model.refresh_visible_tasks();
        id
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn add_task_due(model: &mut Model, title: &str, due: Option<NaiveDate>) -> TaskId {
        let mut task = Task::new(title);
        task.due_date = due;
        let id = task.id;
        model.tasks.insert(id, task);
        id
    }

    fn add_task_snoozed(model: &mut Model, title: &str, until: Option<NaiveDate>) -> TaskId {
        let mut task = Task::new(title);
        task.snooze_until = until;
        let id = task.id;
        model.tasks.insert(id, task);
        id
    }

    // ── count_due_soon ───────────────────────────────────────────────────────

    #[test]
    fn test_count_due_soon_from_window_is_exclusive_of_today_inclusive_of_cutoff() {
        let mut model = make_model();
        let today = date(2026, 3, 10);
        add_task_due(&mut model, "Yesterday", Some(date(2026, 3, 9)));
        add_task_due(&mut model, "Today", Some(today));
        add_task_due(&mut model, "Tomorrow", Some(date(2026, 3, 11)));
        add_task_due(&mut model, "In three days", Some(date(2026, 3, 13)));
        add_task_due(&mut model, "In four days", Some(date(2026, 3, 14)));
        add_task_due(&mut model, "No due date", None);

        assert_eq!(model.count_due_soon_from(today, 0), 0);
        assert_eq!(model.count_due_soon_from(today, 1), 1);
        assert_eq!(model.count_due_soon_from(today, 2), 1);
        assert_eq!(model.count_due_soon_from(today, 3), 2);
        assert_eq!(model.count_due_soon_from(today, 4), 3);
        assert_eq!(model.count_due_soon_from(today, 30), 3);
    }

    #[test]
    fn test_count_due_soon_from_ignores_completed_tasks() {
        let mut model = make_model();
        let today = date(2026, 3, 10);
        let done = add_task_due(&mut model, "Done tomorrow", Some(date(2026, 3, 11)));
        model.tasks.get_mut(&done).unwrap().status = TaskStatus::Done;
        add_task_due(&mut model, "Open tomorrow", Some(date(2026, 3, 11)));

        assert_eq!(model.count_due_soon_from(today, 1), 1);
    }

    #[test]
    fn test_count_due_soon_from_crosses_month_and_year_boundaries() {
        let mut model = make_model();
        add_task_due(&mut model, "New year", Some(date(2027, 1, 1)));
        add_task_due(&mut model, "Early Jan", Some(date(2027, 1, 3)));

        assert_eq!(model.count_due_soon_from(date(2026, 12, 31), 1), 1);
        assert_eq!(model.count_due_soon_from(date(2026, 12, 31), 3), 2);
    }

    #[test]
    fn test_count_due_soon_from_huge_window_does_not_overflow() {
        let mut model = make_model();
        add_task_due(&mut model, "Far away", Some(date(2099, 1, 1)));
        assert_eq!(model.count_due_soon_from(date(2026, 3, 10), u32::MAX), 1);
    }

    #[test]
    fn test_count_due_soon_is_relative_to_local_today() {
        let mut model = make_model();
        let tomorrow = today().succ_opt().unwrap();
        add_task_due(&mut model, "Today", Some(today()));
        add_task_due(&mut model, "Tomorrow", Some(tomorrow));

        assert_eq!(model.count_due_soon(1), 1);
        assert_eq!(model.count_due_soon(0), 0);
    }

    #[test]
    fn test_count_due_soon_empty_model() {
        let model = make_model();
        assert_eq!(model.count_due_soon(7), 0);
        assert_eq!(model.count_due_soon_from(date(2026, 3, 10), 7), 0);
    }

    // ── count_snooze_expired ─────────────────────────────────────────────────

    #[test]
    fn test_count_snooze_expired_on_matches_only_that_date() {
        let mut model = make_model();
        let day = date(2026, 3, 10);
        add_task_snoozed(&mut model, "Expired yesterday", Some(date(2026, 3, 9)));
        add_task_snoozed(&mut model, "Expires today", Some(day));
        add_task_snoozed(&mut model, "Also today", Some(day));
        add_task_snoozed(&mut model, "Still snoozed", Some(date(2026, 3, 11)));
        add_task_snoozed(&mut model, "Never snoozed", None);

        assert_eq!(model.count_snooze_expired_on(day), 2);
        assert_eq!(model.count_snooze_expired_on(date(2026, 3, 11)), 1);
        assert_eq!(model.count_snooze_expired_on(date(2026, 3, 12)), 0);
    }

    #[test]
    fn test_count_snooze_expired_on_ignores_completed_tasks() {
        let mut model = make_model();
        let day = date(2026, 3, 10);
        let done = add_task_snoozed(&mut model, "Done", Some(day));
        model.tasks.get_mut(&done).unwrap().status = TaskStatus::Done;
        add_task_snoozed(&mut model, "Open", Some(day));

        assert_eq!(model.count_snooze_expired_on(day), 1);
    }

    #[test]
    fn test_count_snooze_expired_today_uses_local_today() {
        let mut model = make_model();
        add_task_snoozed(&mut model, "Expires today", Some(today()));
        add_task_snoozed(&mut model, "Tomorrow", today().succ_opt());

        assert_eq!(model.count_snooze_expired_today(), 1);
    }

    #[test]
    fn test_count_snooze_expired_empty_model() {
        let model = make_model();
        assert_eq!(model.count_snooze_expired_today(), 0);
    }

    // ── kanban_column_tasks ──────────────────────────────────────────────────

    #[test]
    fn test_kanban_column_todo() {
        let mut model = make_model();
        let id = add_task(&mut model, "Task A");
        let col = model.kanban_column_tasks(0);
        assert!(col.contains(&id));
    }

    #[test]
    fn test_kanban_column_in_progress() {
        let mut model = make_model();
        let mut task = Task::new("In progress");
        task.status = TaskStatus::InProgress;
        let id = task.id;
        model.tasks.insert(id, task);
        model.refresh_visible_tasks();
        let col = model.kanban_column_tasks(1);
        assert!(col.contains(&id));
    }

    #[test]
    fn test_kanban_column_blocked() {
        let mut model = make_model();
        let mut task = Task::new("Blocked");
        task.status = TaskStatus::Blocked;
        let id = task.id;
        model.tasks.insert(id, task);
        model.refresh_visible_tasks();
        let col = model.kanban_column_tasks(2);
        assert!(col.contains(&id));
    }

    #[test]
    fn test_kanban_column_done() {
        let mut model = make_model();
        let mut task = Task::new("Done");
        task.status = TaskStatus::Done;
        let id = task.id;
        model.tasks.insert(id, task);
        model.filtering.show_completed = true;
        model.refresh_visible_tasks();
        let col = model.kanban_column_tasks(3);
        assert!(col.contains(&id));
    }

    #[test]
    fn test_kanban_column_out_of_range_returns_empty() {
        let mut model = make_model();
        add_task(&mut model, "Task");
        assert!(model.kanban_column_tasks(99).is_empty());
    }

    // ── eisenhower_quadrant_tasks ────────────────────────────────────────────

    #[test]
    fn test_eisenhower_eliminate_quadrant() {
        let mut model = make_model();
        // Low priority, no due date → "Eliminate" (3)
        let mut task = Task::new("Low priority, no deadline");
        task.priority = Priority::Low;
        let id = task.id;
        model.tasks.insert(id, task);
        model.refresh_visible_tasks();
        let q = model.eisenhower_quadrant_tasks(3);
        assert!(q.contains(&id));
    }

    #[test]
    fn test_eisenhower_schedule_quadrant() {
        let mut model = make_model();
        // High priority, no due date → "Schedule" (1)
        let mut task = Task::new("Important, no deadline");
        task.priority = Priority::High;
        let id = task.id;
        model.tasks.insert(id, task);
        model.refresh_visible_tasks();
        let q = model.eisenhower_quadrant_tasks(1);
        assert!(q.contains(&id));
    }

    #[test]
    fn test_eisenhower_do_first_quadrant() {
        let mut model = make_model();
        // Urgent priority + due today → "Do First" (0)
        let mut task = Task::new("Urgent and important");
        task.priority = Priority::Urgent;
        task.due_date = Some(today());
        let id = task.id;
        model.tasks.insert(id, task);
        model.refresh_visible_tasks();
        let q = model.eisenhower_quadrant_tasks(0);
        assert!(q.contains(&id));
    }

    #[test]
    fn test_eisenhower_delegate_quadrant() {
        let mut model = make_model();
        // Low priority + due today → "Delegate" (2)
        let mut task = Task::new("Urgent but not important");
        task.priority = Priority::Low;
        task.due_date = Some(today());
        let id = task.id;
        model.tasks.insert(id, task);
        model.refresh_visible_tasks();
        let q = model.eisenhower_quadrant_tasks(2);
        assert!(q.contains(&id));
    }

    #[test]
    fn test_eisenhower_out_of_range_empty() {
        let mut model = make_model();
        add_task(&mut model, "Task");
        assert!(model.eisenhower_quadrant_tasks(99).is_empty());
    }

    #[test]
    fn test_eisenhower_excludes_completed_tasks() {
        let mut model = make_model();
        let mut task = Task::new("Done high prio");
        task.priority = Priority::High;
        task.status = TaskStatus::Done;
        let id = task.id;
        model.tasks.insert(id, task);
        model.filtering.show_completed = true;
        model.refresh_visible_tasks();
        // Done tasks should be excluded from all quadrants
        for q in 0..4 {
            assert!(!model.eisenhower_quadrant_tasks(q).contains(&id));
        }
    }

    // ── weekly_planner_day_tasks ─────────────────────────────────────────────

    #[test]
    fn test_weekly_planner_out_of_range_empty() {
        let model = make_model();
        assert!(model.weekly_planner_day_tasks(7).is_empty());
    }

    #[test]
    fn test_weekly_planner_task_on_monday() {
        let mut model = make_model();
        let today = today();
        let week_start =
            today - chrono::Duration::days(today.weekday().num_days_from_monday().into());

        let mut task = Task::new("Monday task");
        task.due_date = Some(week_start); // Monday
        let id = task.id;
        model.tasks.insert(id, task);
        model.refresh_visible_tasks();

        let day_tasks = model.weekly_planner_day_tasks(0);
        assert!(day_tasks.contains(&id));
    }

    #[test]
    fn test_weekly_planner_scheduled_date() {
        let mut model = make_model();
        let today = today();
        let week_start =
            today - chrono::Duration::days(today.weekday().num_days_from_monday().into());
        let wednesday = week_start + chrono::Duration::days(2);

        let mut task = Task::new("Scheduled Wednesday");
        task.scheduled_date = Some(wednesday);
        let id = task.id;
        model.tasks.insert(id, task);
        model.refresh_visible_tasks();

        let day_tasks = model.weekly_planner_day_tasks(2);
        assert!(day_tasks.contains(&id));
    }

    // ── network_tasks ────────────────────────────────────────────────────────

    #[test]
    fn test_network_tasks_empty_model() {
        let model = make_model();
        assert!(model.network_tasks().is_empty());
    }

    #[test]
    fn test_network_tasks_with_dependency() {
        let mut model = make_model();
        let task_a = Task::new("A");
        let task_b_id = task_a.id;
        let mut task_c = Task::new("C depends on A");
        task_c.dependencies.push(task_b_id);
        let task_c_id = task_c.id;

        model.tasks.insert(task_a.id, task_a);
        model.tasks.insert(task_c.id, task_c);
        model.refresh_visible_tasks();

        let net = model.network_tasks();
        assert!(net.contains(&task_b_id));
        assert!(net.contains(&task_c_id));
    }

    #[test]
    fn test_network_tasks_with_chain() {
        let mut model = make_model();
        let task_a = Task::new("Chain start");
        let task_b = Task::new("Chain end");
        let a_id = task_a.id;
        let b_id = task_b.id;
        // task_a.next_task_id = task_b.id — add via tasks field
        let mut task_a2 = task_a;
        task_a2.next_task_id = Some(b_id);
        model.tasks.insert(a_id, task_a2);
        model.tasks.insert(b_id, task_b);
        model.refresh_visible_tasks();

        let net = model.network_tasks();
        assert!(net.contains(&a_id));
        assert!(net.contains(&b_id));
    }

    // ── get_tasks_grouped_by_project ─────────────────────────────────────────

    #[test]
    fn test_grouped_by_project_no_project() {
        let mut model = make_model();
        let id = add_task(&mut model, "Unassigned task");
        let groups = model.get_tasks_grouped_by_project();
        let no_project = groups.iter().find(|(pid, _, _)| pid.is_none());
        assert!(no_project.is_some());
        assert!(no_project.unwrap().2.contains(&id));
    }

    #[test]
    fn test_grouped_by_project_with_project() {
        let mut model = make_model();
        let project = Project::new("My Project");
        let pid = project.id;
        model.projects.insert(pid, project);

        let mut task = Task::new("Project task");
        task.project_id = Some(pid);
        let tid = task.id;
        model.tasks.insert(tid, task);
        model.refresh_visible_tasks();

        let groups = model.get_tasks_grouped_by_project();
        let proj_group = groups.iter().find(|(p, _, _)| *p == Some(pid));
        assert!(proj_group.is_some(), "should have project group");
        assert!(proj_group.unwrap().2.contains(&tid));
        assert_eq!(proj_group.unwrap().1, "My Project");
    }

    #[test]
    fn test_grouped_by_project_sorts_no_project_last() {
        let mut model = make_model();
        let project = Project::new("Alpha Project");
        let pid = project.id;
        model.projects.insert(pid, project);

        let mut t1 = Task::new("Project task");
        t1.project_id = Some(pid);
        model.tasks.insert(t1.id, t1);

        let t2 = Task::new("Unassigned");
        model.tasks.insert(t2.id, t2);

        model.refresh_visible_tasks();

        let groups = model.get_tasks_grouped_by_project();
        if groups.len() >= 2 {
            // "No Project" should come last
            assert!(groups.last().unwrap().0.is_none());
        }
    }

    // ── get_git_tasks_grouped_by_file ─────────────────────────────────────────

    #[test]
    fn test_git_tasks_grouped_empty() {
        let model = make_model();
        assert!(model.get_git_tasks_grouped_by_file().is_empty());
    }

    #[test]
    fn test_git_tasks_grouped_with_tasks() {
        let mut model = make_model();
        let mut task = Task::new("Fix auth bug");
        task.description = Some("git:src/auth.rs:42".to_string());
        let id = task.id;
        model.tasks.insert(id, task);
        model.refresh_visible_tasks();

        let groups = model.get_git_tasks_grouped_by_file();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0, "src/auth.rs");
        assert!(groups[0]
            .1
            .iter()
            .any(|(tid, line)| *tid == id && *line == 42));
    }

    #[test]
    fn test_git_tasks_grouped_multiple_files() {
        let mut model = make_model();

        let mut t1 = Task::new("Auth fix");
        t1.description = Some("git:src/auth.rs:10".to_string());
        let mut t2 = Task::new("Main fix");
        t2.description = Some("git:src/main.rs:5".to_string());
        let mut t3 = Task::new("Auth fix 2");
        t3.description = Some("git:src/auth.rs:20".to_string());

        model.tasks.insert(t1.id, t1);
        model.tasks.insert(t2.id, t2);
        model.tasks.insert(t3.id, t3);
        model.refresh_visible_tasks();

        let groups = model.get_git_tasks_grouped_by_file();
        assert_eq!(groups.len(), 2);

        let auth_group = groups.iter().find(|(f, _)| f == "src/auth.rs");
        assert!(auth_group.is_some());
        assert_eq!(auth_group.unwrap().1.len(), 2);
        // Should be sorted by line number
        assert_eq!(auth_group.unwrap().1[0].1, 10);
        assert_eq!(auth_group.unwrap().1[1].1, 20);
    }

    #[test]
    fn test_git_tasks_no_description_excluded() {
        let mut model = make_model();
        let task = Task::new("Regular task, no description");
        model.tasks.insert(task.id, task);
        model.refresh_visible_tasks();
        assert!(model.get_git_tasks_grouped_by_file().is_empty());
    }
}
