//! Time tracking analytics and burndown charts.
//!
//! This module provides methods for computing:
//! - Time tracking analytics (by project, day of week, hour)
//! - Burndown charts for projects

use chrono::{Datelike, Local, NaiveDate, Timelike};
use std::collections::{HashMap, HashSet};

use crate::domain::analytics::{BurnChart, TaskTimeInfo, TimeAnalytics, TimeSeriesPoint};
use crate::domain::{ProjectId, Task, TaskId};

use super::AnalyticsEngine;
use crate::domain::LocalDate;

impl AnalyticsEngine<'_> {
    /// Compute time tracking analytics.
    ///
    /// Aggregates time spent on tasks by:
    /// - Project
    /// - Day of week
    /// - Hour of day
    #[must_use]
    pub fn compute_time_analytics(&self, start: NaiveDate, end: NaiveDate) -> TimeAnalytics {
        let mut analytics = TimeAnalytics::default();
        let mut tasks_with_entries: HashSet<TaskId> = HashSet::new();

        // Time entries are the source of truth, bucketed by local start date/hour.
        for entry in self.model.time_entries.values() {
            tasks_with_entries.insert(entry.task_id);
            let local_start = entry.started_at.with_timezone(&Local);
            let entry_date = local_start.date_naive();
            if entry_date < start || entry_date > end {
                continue;
            }
            let minutes = entry.calculated_duration_minutes();
            analytics.entry_count += 1;
            let task = self.model.tasks.get(&entry.task_id);
            Self::add_time_bucket(
                &mut analytics,
                task,
                entry.task_id,
                entry_date,
                local_start.hour() as usize,
                minutes,
                true,
            );
        }

        // Legacy/imported data: a task total with no entries behind it is
        // counted once, at its completion (or creation) date, at noon.
        for task in self.model.tasks.values() {
            if task.actual_minutes == 0 || tasks_with_entries.contains(&task.id) {
                continue;
            }
            let date = task.completed_at.unwrap_or(task.created_at).local_date();
            if date < start || date > end {
                continue;
            }
            let hour = task
                .completed_at
                .map_or(12, |c| c.with_timezone(&Local).hour()) as usize;
            Self::add_time_bucket(
                &mut analytics,
                Some(task),
                task.id,
                date,
                hour,
                task.actual_minutes,
                false,
            );
        }

        analytics
    }

    #[allow(clippy::too_many_arguments)]
    fn add_time_bucket(
        analytics: &mut TimeAnalytics,
        task: Option<&Task>,
        task_id: TaskId,
        date: NaiveDate,
        hour: usize,
        minutes: u32,
        from_entry: bool,
    ) {
        analytics.total_minutes += minutes;
        *analytics.by_task.entry(task_id).or_insert(0) += minutes;
        let info = analytics
            .task_info
            .entry(task_id)
            .or_insert_with(|| TaskTimeInfo {
                title: task.map_or_else(|| "(deleted task)".to_string(), |t| t.title.clone()),
                project_id: task.and_then(|t| t.project_id),
                entries: 0,
            });
        if from_entry {
            info.entries += 1;
        }
        *analytics.by_day.entry(date).or_insert(0) += minutes;
        let dow = date.weekday().num_days_from_monday() as usize;
        analytics.by_day_of_week[dow] += minutes;
        analytics.by_hour[hour.min(23)] += minutes;
        if let Some(task) = task {
            *analytics.by_project.entry(task.project_id).or_insert(0) += minutes;
            for tag in &task.tags {
                *analytics.by_tag.entry(tag.clone()).or_insert(0) += minutes;
            }
        } else {
            // Orphaned entry (task deleted): still real time, no project.
            *analytics.by_project.entry(None).or_insert(0) += minutes;
        }
    }

    #[must_use]
    pub fn compute_burn_charts(&self, start: NaiveDate, end: NaiveDate) -> Vec<BurnChart> {
        let mut charts = Vec::new();

        // Global burndown
        charts.push(self.compute_burn_chart_for_project(None, "All Tasks", start, end));

        // Per-project burndowns
        for project in self.model.projects.values() {
            charts.push(self.compute_burn_chart_for_project(
                Some(project.id),
                &project.name,
                start,
                end,
            ));
        }

        charts
    }

    /// Compute a burndown chart for a specific project (or all tasks if None).
    fn compute_burn_chart_for_project(
        &self,
        project_id: Option<ProjectId>,
        name: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> BurnChart {
        // Filter tasks for this project
        let tasks: Vec<&Task> = self
            .model
            .tasks
            .values()
            .filter(|t| match &project_id {
                Some(pid) => t.project_id.as_ref() == Some(pid),
                None => true,
            })
            .collect();

        let mut scope_by_day: HashMap<NaiveDate, i32> = HashMap::new();
        let mut completed_by_day: HashMap<NaiveDate, i32> = HashMap::new();

        // Initialize with starting values
        let mut initial_scope = 0i32;
        let mut initial_completed = 0i32;

        for task in &tasks {
            let created_date = task.created_at.local_date();
            if created_date < start {
                initial_scope += 1;
                if task.completed_at.is_some_and(|c| c.local_date() < start) {
                    initial_completed += 1;
                }
            }
        }

        // Track changes over time
        for task in &tasks {
            let created_date = task.created_at.local_date();
            if created_date >= start && created_date <= end {
                *scope_by_day.entry(created_date).or_insert(0) += 1;
            }

            if let Some(completed_at) = task.completed_at {
                let completed_date = completed_at.local_date();
                if completed_date >= start && completed_date <= end {
                    *completed_by_day.entry(completed_date).or_insert(0) += 1;
                }
            }
        }

        // Build cumulative lines
        let mut scope_line = Vec::new();
        let mut completed_line = Vec::new();
        let mut current_date = start;
        let mut running_scope = initial_scope;
        let mut running_completed = initial_completed;

        while current_date <= end {
            running_scope += *scope_by_day.get(&current_date).unwrap_or(&0);
            running_completed += *completed_by_day.get(&current_date).unwrap_or(&0);

            scope_line.push(TimeSeriesPoint::new(current_date, f64::from(running_scope)));
            completed_line.push(TimeSeriesPoint::new(
                current_date,
                f64::from(running_completed),
            ));

            current_date = current_date.succ_opt().unwrap_or(current_date);
        }

        BurnChart {
            project_name: name.to_string(),
            project_id,
            scope_line,
            completed_line,
            ideal_line: None,
        }
    }
}
