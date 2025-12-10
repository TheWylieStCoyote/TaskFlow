//! Time tracking analytics and burndown charts.
//!
//! This module provides methods for computing:
//! - Time tracking analytics (by project, day of week, hour)
//! - Burndown charts for projects

use chrono::{Datelike, NaiveDate, Timelike};
use std::collections::HashMap;

use crate::domain::analytics::{BurnChart, TimeAnalytics, TimeSeriesPoint};
use crate::domain::{ProjectId, Task};

use super::AnalyticsEngine;

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

        for task in self.model.tasks.values() {
            // Use actual_minutes from task if available
            if task.actual_minutes > 0 {
                // Attribute to completion date or created date
                let date = task
                    .completed_at
                    .map_or_else(|| task.created_at, |c| c)
                    .date_naive();

                if date >= start && date <= end {
                    let minutes = task.actual_minutes;
                    analytics.total_minutes += minutes;

                    // By project
                    *analytics.by_project.entry(task.project_id).or_insert(0) += minutes;

                    // By day of week
                    let dow = date.weekday().num_days_from_monday() as usize;
                    analytics.by_day_of_week[dow] += minutes;

                    // By hour (use a default of noon if we don't have precise time)
                    let hour = task.completed_at.map_or(12, |c| c.time().hour()) as usize;
                    analytics.by_hour[hour] += minutes;
                }
            }
        }

        // Also count from time entries if available
        for entry in self.model.time_entries.values() {
            let entry_date = entry.started_at.date_naive();
            if entry_date >= start && entry_date <= end {
                let minutes = entry.calculated_duration_minutes();
                analytics.total_minutes += minutes;

                // Find the task to get project ID
                if let Some(task) = self.model.tasks.get(&entry.task_id) {
                    *analytics.by_project.entry(task.project_id).or_insert(0) += minutes;
                }

                // By day of week
                let dow = entry_date.weekday().num_days_from_monday() as usize;
                analytics.by_day_of_week[dow] += minutes;

                // By hour
                let hour = entry.started_at.time().hour() as usize;
                analytics.by_hour[hour] += minutes;
            }
        }

        analytics
    }

    /// Compute burndown charts for all projects.
    ///
    /// Generates burndown charts showing scope vs completion over time
    /// for the global task set and each individual project.
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
            let created_date = task.created_at.date_naive();
            if created_date < start {
                initial_scope += 1;
                if task.completed_at.is_some_and(|c| c.date_naive() < start) {
                    initial_completed += 1;
                }
            }
        }

        // Track changes over time
        for task in &tasks {
            let created_date = task.created_at.date_naive();
            if created_date >= start && created_date <= end {
                *scope_by_day.entry(created_date).or_insert(0) += 1;
            }

            if let Some(completed_at) = task.completed_at {
                let completed_date = completed_at.date_naive();
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
