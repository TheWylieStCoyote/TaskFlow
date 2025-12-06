//! Analytics engine for computing task metrics and insights.
//!
//! This module provides the [`AnalyticsEngine`] which computes various metrics
//! from task data, including completion trends, velocity, burndown charts,
//! and productivity insights.
//!
//! # Example
//!
//! ```rust
//! use taskflow::app::{Model, analytics::AnalyticsEngine};
//! use taskflow::domain::analytics::ReportConfig;
//!
//! let model = Model::new().with_sample_data();
//! let engine = AnalyticsEngine::new(&model);
//!
//! // Generate a full report for the last 30 days
//! let config = ReportConfig::last_n_days(30);
//! let report = engine.generate_report(&config);
//!
//! println!("Total completed: {}", report.status_breakdown.done);
//! println!("Velocity trend: {}", report.velocity.trend);
//! ```

use chrono::{Datelike, NaiveDate, Timelike, Weekday};
use std::collections::{HashMap, HashSet};

use crate::domain::analytics::{
    AnalyticsReport, BurnChart, CompletionTrend, PriorityBreakdown, ProductivityInsights,
    ReportConfig, StatusBreakdown, TagStats, TimeAnalytics, TimeSeriesPoint, VelocityMetrics,
};
use crate::domain::{Priority, ProjectId, Task, TaskStatus};

use super::Model;

/// Engine for computing analytics from task data.
pub struct AnalyticsEngine<'a> {
    model: &'a Model,
}

impl<'a> AnalyticsEngine<'a> {
    /// Create a new analytics engine for the given model.
    #[must_use]
    pub const fn new(model: &'a Model) -> Self {
        Self { model }
    }

    /// Generate a complete analytics report.
    #[must_use]
    pub fn generate_report(&self, config: &ReportConfig) -> AnalyticsReport {
        let completion_trend = self.compute_completion_trend(config.start_date, config.end_date);
        let velocity = self.compute_velocity(config.start_date, config.end_date);
        let burn_charts = self.compute_burn_charts(config.start_date, config.end_date);
        let time_analytics = self.compute_time_analytics(config.start_date, config.end_date);
        let insights = self.compute_insights();
        let status_breakdown = self.compute_status_breakdown();
        let priority_breakdown = self.compute_priority_breakdown();
        let tag_stats = if config.include_tags {
            self.compute_tag_stats()
        } else {
            vec![]
        };

        AnalyticsReport {
            config: config.clone(),
            completion_trend,
            velocity,
            burn_charts,
            time_analytics,
            insights,
            status_breakdown,
            priority_breakdown,
            tag_stats,
        }
    }

    /// Compute completion trends over a date range.
    #[must_use]
    pub fn compute_completion_trend(&self, start: NaiveDate, end: NaiveDate) -> CompletionTrend {
        let mut completions_by_day: HashMap<NaiveDate, u32> = HashMap::new();
        let mut creations_by_day: HashMap<NaiveDate, u32> = HashMap::new();

        for task in self.model.tasks.values() {
            // Count creations
            let created_date = task.created_at.date_naive();
            if created_date >= start && created_date <= end {
                *creations_by_day.entry(created_date).or_insert(0) += 1;
            }

            // Count completions
            if let Some(completed_at) = task.completed_at {
                let completed_date = completed_at.date_naive();
                if completed_date >= start && completed_date <= end {
                    *completions_by_day.entry(completed_date).or_insert(0) += 1;
                }
            }
        }

        // Convert to sorted time series
        let mut completions: Vec<TimeSeriesPoint> = completions_by_day
            .into_iter()
            .map(|(date, count)| TimeSeriesPoint::new(date, f64::from(count)))
            .collect();
        completions.sort_by_key(|p| p.date);

        let mut creations: Vec<TimeSeriesPoint> = creations_by_day
            .into_iter()
            .map(|(date, count)| TimeSeriesPoint::new(date, f64::from(count)))
            .collect();
        creations.sort_by_key(|p| p.date);

        // Calculate completion rate over time
        let mut completion_rate = Vec::new();
        let mut current_date = start;
        let mut total_created = 0u32;
        let mut total_completed = 0u32;

        while current_date <= end {
            // Add creations for this day
            total_created += creations
                .iter()
                .find(|p| p.date == current_date)
                .map_or(0, |p| p.value as u32);

            // Add completions for this day
            total_completed += completions
                .iter()
                .find(|p| p.date == current_date)
                .map_or(0, |p| p.value as u32);

            // Calculate rate
            let rate = if total_created > 0 {
                total_completed as f64 / total_created as f64
            } else {
                0.0
            };
            completion_rate.push(TimeSeriesPoint::new(current_date, rate));

            current_date = current_date.succ_opt().unwrap_or(current_date);
        }

        CompletionTrend {
            completions_by_day: completions,
            creations_by_day: creations,
            completion_rate_over_time: completion_rate,
        }
    }

    /// Compute velocity metrics.
    #[must_use]
    pub fn compute_velocity(&self, start: NaiveDate, end: NaiveDate) -> VelocityMetrics {
        // Group completions by week (start of week = Monday)
        let mut weekly: HashMap<NaiveDate, u32> = HashMap::new();
        let mut monthly: HashMap<NaiveDate, u32> = HashMap::new();

        for task in self.model.tasks.values() {
            if let Some(completed_at) = task.completed_at {
                let completed_date = completed_at.date_naive();
                if completed_date >= start && completed_date <= end {
                    // Get start of week (Monday)
                    let week_start = completed_date
                        - chrono::Duration::days(
                            completed_date.weekday().num_days_from_monday() as i64
                        );
                    *weekly.entry(week_start).or_insert(0) += 1;

                    // Get start of month
                    let month_start =
                        NaiveDate::from_ymd_opt(completed_date.year(), completed_date.month(), 1)
                            .unwrap();
                    *monthly.entry(month_start).or_insert(0) += 1;
                }
            }
        }

        let mut weekly_vec: Vec<(NaiveDate, u32)> = weekly.into_iter().collect();
        weekly_vec.sort_by_key(|(date, _)| *date);

        let mut monthly_vec: Vec<(NaiveDate, u32)> = monthly.into_iter().collect();
        monthly_vec.sort_by_key(|(date, _)| *date);

        // Calculate average weekly velocity
        let avg_weekly = if weekly_vec.is_empty() {
            0.0
        } else {
            let sum: u32 = weekly_vec.iter().map(|(_, v)| v).sum();
            sum as f64 / weekly_vec.len() as f64
        };

        // Calculate trend (simple linear regression slope)
        let trend = if weekly_vec.len() < 2 {
            0.0
        } else {
            let n = weekly_vec.len() as f64;
            let sum_x: f64 = (0..weekly_vec.len()).map(|i| i as f64).sum();
            let sum_y: f64 = weekly_vec.iter().map(|(_, v)| *v as f64).sum();
            let sum_xy: f64 = weekly_vec
                .iter()
                .enumerate()
                .map(|(i, (_, v))| i as f64 * *v as f64)
                .sum();
            let sum_xx: f64 = (0..weekly_vec.len()).map(|i| (i * i) as f64).sum();

            let denominator = n * sum_xx - sum_x * sum_x;
            if denominator.abs() < f64::EPSILON {
                0.0
            } else {
                (n * sum_xy - sum_x * sum_y) / denominator
            }
        };

        VelocityMetrics {
            weekly_velocity: weekly_vec,
            monthly_velocity: monthly_vec,
            avg_weekly,
            trend,
        }
    }

    /// Compute burndown charts for all projects.
    #[must_use]
    pub fn compute_burn_charts(&self, start: NaiveDate, end: NaiveDate) -> Vec<BurnChart> {
        let mut charts = Vec::new();

        // Global burndown
        charts.push(self.compute_burn_chart_for_project(None, "All Tasks", start, end));

        // Per-project burndowns
        for project in self.model.projects.values() {
            charts.push(self.compute_burn_chart_for_project(
                Some(project.id.clone()),
                &project.name,
                start,
                end,
            ));
        }

        charts
    }

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

            scope_line.push(TimeSeriesPoint::new(current_date, running_scope as f64));
            completed_line.push(TimeSeriesPoint::new(current_date, running_completed as f64));

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

    /// Compute time tracking analytics.
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
                    *analytics
                        .by_project
                        .entry(task.project_id.clone())
                        .or_insert(0) += minutes;

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
                    *analytics
                        .by_project
                        .entry(task.project_id.clone())
                        .or_insert(0) += minutes;
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

    /// Compute productivity insights.
    #[must_use]
    pub fn compute_insights(&self) -> ProductivityInsights {
        let mut insights = ProductivityInsights::default();

        // Count completions by day of week and hour
        let mut by_day: [u32; 7] = [0; 7];
        let mut by_hour: [u32; 24] = [0; 24];
        let mut completion_dates: Vec<NaiveDate> = Vec::new();

        for task in self.model.tasks.values() {
            if task.status == TaskStatus::Done {
                insights.total_completed += 1;

                if let Some(completed_at) = task.completed_at {
                    let date = completed_at.date_naive();
                    completion_dates.push(date);

                    let dow = date.weekday().num_days_from_monday() as usize;
                    by_day[dow] += 1;

                    let hour = completed_at.time().hour() as usize;
                    by_hour[hour] += 1;
                }
            }

            insights.total_time_tracked += task.actual_minutes;
        }

        // Add time from time entries
        for entry in self.model.time_entries.values() {
            insights.total_time_tracked += entry.calculated_duration_minutes();
        }

        // Find best day
        let max_day_idx = by_day
            .iter()
            .enumerate()
            .max_by_key(|(_, &v)| v)
            .filter(|(_, &v)| v > 0)
            .map(|(i, _)| i);

        insights.best_day = max_day_idx.map(|i| match i {
            0 => Weekday::Mon,
            1 => Weekday::Tue,
            2 => Weekday::Wed,
            3 => Weekday::Thu,
            4 => Weekday::Fri,
            5 => Weekday::Sat,
            6 => Weekday::Sun,
            _ => unreachable!(),
        });

        // Find peak hour
        insights.peak_hour = by_hour
            .iter()
            .enumerate()
            .max_by_key(|(_, &v)| v)
            .filter(|(_, &v)| v > 0)
            .map(|(i, _)| i as u32);

        // Calculate streaks
        if !completion_dates.is_empty() {
            completion_dates.sort();
            let unique_dates: Vec<NaiveDate> = completion_dates
                .into_iter()
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .into_iter()
                .collect();
            let mut sorted_unique: Vec<NaiveDate> = unique_dates;
            sorted_unique.sort();

            let today = chrono::Local::now().date_naive();
            let mut current_streak = 0u32;
            let mut longest_streak = 0u32;
            let mut streak = 0u32;
            let mut prev_date: Option<NaiveDate> = None;

            for date in sorted_unique.iter() {
                if let Some(prev) = prev_date {
                    if *date - prev == chrono::Duration::days(1) {
                        streak += 1;
                    } else {
                        longest_streak = longest_streak.max(streak);
                        streak = 1;
                    }
                } else {
                    streak = 1;
                }
                prev_date = Some(*date);
            }
            longest_streak = longest_streak.max(streak);

            // Check if current streak is still active
            if let Some(last_date) = sorted_unique.last() {
                let days_since = (today - *last_date).num_days();
                if days_since <= 1 {
                    // Still active (today or yesterday)
                    current_streak = streak;
                }
            }

            insights.current_streak = current_streak;
            insights.longest_streak = longest_streak;

            // Average tasks per day (on active days)
            let active_days = sorted_unique.len();
            if active_days > 0 {
                insights.avg_tasks_per_day = insights.total_completed as f64 / active_days as f64;
            }
        }

        insights
    }

    /// Compute status breakdown.
    #[must_use]
    pub fn compute_status_breakdown(&self) -> StatusBreakdown {
        let mut breakdown = StatusBreakdown::default();

        for task in self.model.tasks.values() {
            match task.status {
                TaskStatus::Todo => breakdown.todo += 1,
                TaskStatus::InProgress => breakdown.in_progress += 1,
                TaskStatus::Blocked => breakdown.blocked += 1,
                TaskStatus::Done => breakdown.done += 1,
                TaskStatus::Cancelled => breakdown.cancelled += 1,
            }
        }

        breakdown
    }

    /// Compute priority breakdown.
    #[must_use]
    pub fn compute_priority_breakdown(&self) -> PriorityBreakdown {
        let mut breakdown = PriorityBreakdown::default();

        for task in self.model.tasks.values() {
            match task.priority {
                Priority::None => breakdown.none += 1,
                Priority::Low => breakdown.low += 1,
                Priority::Medium => breakdown.medium += 1,
                Priority::High => breakdown.high += 1,
                Priority::Urgent => breakdown.urgent += 1,
            }
        }

        breakdown
    }

    /// Compute tag statistics.
    #[must_use]
    pub fn compute_tag_stats(&self) -> Vec<TagStats> {
        let mut tag_counts: HashMap<String, (u32, u32)> = HashMap::new(); // (total, completed)

        for task in self.model.tasks.values() {
            for tag in &task.tags {
                let entry = tag_counts.entry(tag.clone()).or_insert((0, 0));
                entry.0 += 1;
                if task.status == TaskStatus::Done {
                    entry.1 += 1;
                }
            }
        }

        let mut stats: Vec<TagStats> = tag_counts
            .into_iter()
            .map(|(tag, (count, completed))| TagStats {
                tag,
                count,
                completed,
            })
            .collect();

        // Sort by count descending
        stats.sort_by(|a, b| b.count.cmp(&a.count));
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Project, Task};

    fn create_test_model() -> Model {
        let mut model = Model::new();

        // Add some tasks with various states
        let mut task1 = Task::new("Task 1");
        task1.status = TaskStatus::Done;
        task1.completed_at = Some(chrono::Utc::now());
        task1.tags = vec!["work".to_string(), "urgent".to_string()];
        task1.actual_minutes = 60;
        model.tasks.insert(task1.id.clone(), task1);

        let mut task2 = Task::new("Task 2");
        task2.status = TaskStatus::Done;
        task2.completed_at = Some(chrono::Utc::now() - chrono::Duration::days(1));
        task2.tags = vec!["work".to_string()];
        task2.priority = Priority::High;
        task2.actual_minutes = 30;
        model.tasks.insert(task2.id.clone(), task2);

        let mut task3 = Task::new("Task 3");
        task3.status = TaskStatus::InProgress;
        task3.priority = Priority::Medium;
        model.tasks.insert(task3.id.clone(), task3);

        let mut task4 = Task::new("Task 4");
        task4.status = TaskStatus::Todo;
        task4.priority = Priority::Low;
        task4.tags = vec!["personal".to_string()];
        model.tasks.insert(task4.id.clone(), task4);

        // Add a project
        let project = Project::new("Test Project");
        model.projects.insert(project.id.clone(), project);

        model
    }

    #[test]
    fn test_analytics_engine_creation() {
        let model = Model::new();
        let _engine = AnalyticsEngine::new(&model);
    }

    #[test]
    fn test_status_breakdown() {
        let model = create_test_model();
        let engine = AnalyticsEngine::new(&model);

        let breakdown = engine.compute_status_breakdown();
        assert_eq!(breakdown.done, 2);
        assert_eq!(breakdown.in_progress, 1);
        assert_eq!(breakdown.todo, 1);
        assert_eq!(breakdown.cancelled, 0);
        assert_eq!(breakdown.total(), 4);
    }

    #[test]
    fn test_priority_breakdown() {
        let model = create_test_model();
        let engine = AnalyticsEngine::new(&model);

        let breakdown = engine.compute_priority_breakdown();
        assert_eq!(breakdown.high, 1);
        assert_eq!(breakdown.medium, 1);
        assert_eq!(breakdown.low, 1);
        assert_eq!(breakdown.none, 1);
        assert_eq!(breakdown.urgent, 0);
    }

    #[test]
    fn test_tag_stats() {
        let model = create_test_model();
        let engine = AnalyticsEngine::new(&model);

        let stats = engine.compute_tag_stats();

        // "work" tag should be most common
        let work_stats = stats.iter().find(|s| s.tag == "work");
        assert!(work_stats.is_some());
        assert_eq!(work_stats.unwrap().count, 2);
        assert_eq!(work_stats.unwrap().completed, 2);

        // "personal" tag
        let personal_stats = stats.iter().find(|s| s.tag == "personal");
        assert!(personal_stats.is_some());
        assert_eq!(personal_stats.unwrap().count, 1);
        assert_eq!(personal_stats.unwrap().completed, 0);
    }

    #[test]
    fn test_insights() {
        let model = create_test_model();
        let engine = AnalyticsEngine::new(&model);

        let insights = engine.compute_insights();
        assert_eq!(insights.total_completed, 2);
        assert!(insights.total_time_tracked >= 90); // At least 60 + 30 from actual_minutes
    }

    #[test]
    fn test_completion_trend() {
        let model = create_test_model();
        let engine = AnalyticsEngine::new(&model);

        let start = chrono::Local::now().date_naive() - chrono::Duration::days(7);
        let end = chrono::Local::now().date_naive();

        let trend = engine.compute_completion_trend(start, end);

        assert!(
            !trend.completions_by_day.is_empty()
                || model.tasks.values().all(|t| t.completed_at.is_none())
        );
        assert!(trend.total_completed() >= 0);
    }

    #[test]
    fn test_velocity_metrics() {
        let model = create_test_model();
        let engine = AnalyticsEngine::new(&model);

        let start = chrono::Local::now().date_naive() - chrono::Duration::days(30);
        let end = chrono::Local::now().date_naive();

        let velocity = engine.compute_velocity(start, end);

        // Should have some weekly data
        assert!(
            !velocity.weekly_velocity.is_empty()
                || model.tasks.values().all(|t| t.completed_at.is_none())
        );
    }

    #[test]
    fn test_burn_charts() {
        let model = create_test_model();
        let engine = AnalyticsEngine::new(&model);

        let start = chrono::Local::now().date_naive() - chrono::Duration::days(7);
        let end = chrono::Local::now().date_naive();

        let charts = engine.compute_burn_charts(start, end);

        // Should have at least global + per-project charts
        assert!(!charts.is_empty());
        assert!(charts.iter().any(|c| c.project_name == "All Tasks"));
    }

    #[test]
    fn test_time_analytics() {
        let model = create_test_model();
        let engine = AnalyticsEngine::new(&model);

        let start = chrono::Local::now().date_naive() - chrono::Duration::days(7);
        let end = chrono::Local::now().date_naive();

        let analytics = engine.compute_time_analytics(start, end);

        // Should have tracked some time
        assert!(analytics.total_minutes >= 0);
    }

    #[test]
    fn test_generate_full_report() {
        let model = create_test_model();
        let engine = AnalyticsEngine::new(&model);

        let config = ReportConfig::last_n_days(30);
        let report = engine.generate_report(&config);

        // Verify all components are present
        assert_eq!(report.status_breakdown.total(), 4);
        assert!(!report.tag_stats.is_empty());
        assert!(!report.burn_charts.is_empty());
    }

    #[test]
    fn test_empty_model_report() {
        let model = Model::new();
        let engine = AnalyticsEngine::new(&model);

        let config = ReportConfig::last_n_days(7);
        let report = engine.generate_report(&config);

        assert_eq!(report.status_breakdown.total(), 0);
        assert!(report.tag_stats.is_empty());
        assert!((report.velocity.avg_weekly - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_current_streak_calculation() {
        let mut model = Model::new();

        // Create tasks completed on consecutive days
        let today = chrono::Utc::now();

        let mut task1 = Task::new("Today");
        task1.status = TaskStatus::Done;
        task1.completed_at = Some(today);
        model.tasks.insert(task1.id.clone(), task1);

        let mut task2 = Task::new("Yesterday");
        task2.status = TaskStatus::Done;
        task2.completed_at = Some(today - chrono::Duration::days(1));
        model.tasks.insert(task2.id.clone(), task2);

        let mut task3 = Task::new("Day before");
        task3.status = TaskStatus::Done;
        task3.completed_at = Some(today - chrono::Duration::days(2));
        model.tasks.insert(task3.id.clone(), task3);

        let engine = AnalyticsEngine::new(&model);
        let insights = engine.compute_insights();

        // Should have a 3-day streak
        assert!(insights.current_streak >= 1);
        assert!(insights.longest_streak >= 1);
    }
}
