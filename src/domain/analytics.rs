//! Analytics domain types for task tracking insights.
//!
//! This module provides data structures for analyzing task completion trends,
//! velocity metrics, burndown charts, and productivity insights.
//!
//! # Example
//!
//! ```rust
//! use taskflow::domain::analytics::{TimeSeriesPoint, CompletionTrend, VelocityMetrics};
//! use chrono::NaiveDate;
//!
//! // Create a time series point
//! let point = TimeSeriesPoint {
//!     date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
//!     value: 5.0,
//! };
//! ```

use chrono::{Datelike, NaiveDate, Weekday};
use std::collections::HashMap;

use super::{ProjectId, TaskId};

/// A single point in a time series.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeSeriesPoint {
    /// The date for this data point
    pub date: NaiveDate,
    /// The value at this date
    pub value: f64,
}

impl TimeSeriesPoint {
    /// Create a new time series point.
    #[must_use]
    pub const fn new(date: NaiveDate, value: f64) -> Self {
        Self { date, value }
    }
}

/// Task completion trends over time.
#[derive(Debug, Clone, Default)]
pub struct CompletionTrend {
    /// Number of tasks completed per day
    pub completions_by_day: Vec<TimeSeriesPoint>,
    /// Number of tasks created per day
    pub creations_by_day: Vec<TimeSeriesPoint>,
    /// Completion rate (completed / created) over time
    pub completion_rate_over_time: Vec<TimeSeriesPoint>,
}

impl CompletionTrend {
    /// Returns the average daily completion rate.
    #[must_use]
    pub fn average_completion_rate(&self) -> f64 {
        if self.completion_rate_over_time.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.completion_rate_over_time.iter().map(|p| p.value).sum();
        sum / self.completion_rate_over_time.len() as f64
    }

    /// Returns the total tasks completed in the period.
    #[must_use]
    pub fn total_completed(&self) -> u32 {
        self.completions_by_day.iter().map(|p| p.value as u32).sum()
    }

    /// Returns the total tasks created in the period.
    #[must_use]
    pub fn total_created(&self) -> u32 {
        self.creations_by_day.iter().map(|p| p.value as u32).sum()
    }
}

/// Velocity metrics for measuring productivity over time.
#[derive(Debug, Clone, Default)]
pub struct VelocityMetrics {
    /// Tasks completed per week (date is start of week)
    pub weekly_velocity: Vec<(NaiveDate, u32)>,
    /// Tasks completed per month (date is start of month)
    pub monthly_velocity: Vec<(NaiveDate, u32)>,
    /// Average weekly velocity
    pub avg_weekly: f64,
    /// Velocity trend: positive = improving, negative = declining
    pub trend: f64,
}

impl VelocityMetrics {
    /// Returns the most productive week.
    #[must_use]
    pub fn best_week(&self) -> Option<(NaiveDate, u32)> {
        self.weekly_velocity.iter().max_by_key(|(_, v)| v).copied()
    }

    /// Returns the least productive week.
    #[must_use]
    pub fn worst_week(&self) -> Option<(NaiveDate, u32)> {
        self.weekly_velocity
            .iter()
            .filter(|(_, v)| *v > 0)
            .min_by_key(|(_, v)| v)
            .copied()
    }

    /// Returns whether velocity is improving.
    #[must_use]
    pub fn is_improving(&self) -> bool {
        self.trend > 0.0
    }
}

/// Burndown chart data for a project.
#[derive(Debug, Clone)]
pub struct BurnChart {
    /// Project name (or "All Tasks" for global)
    pub project_name: String,
    /// Project ID (None for global burndown)
    pub project_id: Option<ProjectId>,
    /// Total scope (tasks) over time
    pub scope_line: Vec<TimeSeriesPoint>,
    /// Completed tasks over time
    pub completed_line: Vec<TimeSeriesPoint>,
    /// Ideal burndown line (optional, for sprint planning)
    pub ideal_line: Option<Vec<TimeSeriesPoint>>,
}

impl BurnChart {
    /// Returns the current remaining work.
    #[must_use]
    pub fn remaining_work(&self) -> f64 {
        let scope = self.scope_line.last().map_or(0.0, |p| p.value);
        let completed = self.completed_line.last().map_or(0.0, |p| p.value);
        scope - completed
    }

    /// Returns the completion percentage.
    #[must_use]
    pub fn completion_percentage(&self) -> f64 {
        let scope = self.scope_line.last().map_or(0.0, |p| p.value);
        if scope == 0.0 {
            return 100.0;
        }
        let completed = self.completed_line.last().map_or(0.0, |p| p.value);
        (completed / scope) * 100.0
    }
}

/// Time analytics showing when tasks are completed.
#[derive(Debug, Clone, Default)]
pub struct TimeAnalytics {
    /// Minutes tracked per project
    pub by_project: HashMap<Option<ProjectId>, u32>,
    /// Minutes tracked per day of week (0 = Monday, 6 = Sunday)
    pub by_day_of_week: [u32; 7],
    /// Minutes tracked per hour of day (0-23)
    pub by_hour: [u32; 24],
    /// Total tracked time in minutes
    pub total_minutes: u32,
}

/// Maps array index to weekday (0 = Monday, 6 = Sunday).
const WEEKDAYS: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

impl TimeAnalytics {
    /// Returns the most productive day of the week.
    #[must_use]
    pub fn most_productive_day(&self) -> Option<Weekday> {
        self.by_day_of_week
            .iter()
            .enumerate()
            .max_by_key(|(_, &v)| v)
            .filter(|(_, &v)| v > 0)
            .map(|(i, _)| WEEKDAYS[i])
    }

    /// Returns the peak productivity hour (0-23).
    #[must_use]
    pub fn peak_hour(&self) -> Option<u32> {
        self.by_hour
            .iter()
            .enumerate()
            .max_by_key(|(_, &v)| v)
            .filter(|(_, &v)| v > 0)
            .map(|(i, _)| i as u32)
    }

    /// Returns total hours tracked.
    #[must_use]
    pub fn total_hours(&self) -> f64 {
        f64::from(self.total_minutes) / 60.0
    }
}

/// Productivity insights and achievements.
#[derive(Debug, Clone, Default)]
pub struct ProductivityInsights {
    /// Most productive day of the week
    pub best_day: Option<Weekday>,
    /// Most productive hour of the day (0-23)
    pub peak_hour: Option<u32>,
    /// Current streak of days with completed tasks
    pub current_streak: u32,
    /// Longest streak ever
    pub longest_streak: u32,
    /// Average tasks completed per day (active days only)
    pub avg_tasks_per_day: f64,
    /// Total tasks completed all time
    pub total_completed: u32,
    /// Total time tracked in minutes
    pub total_time_tracked: u32,
}

impl ProductivityInsights {
    /// Returns whether currently on a streak.
    #[must_use]
    pub fn is_on_streak(&self) -> bool {
        self.current_streak > 0
    }

    /// Returns whether current streak is the best ever.
    #[must_use]
    pub fn is_best_streak(&self) -> bool {
        self.current_streak > 0 && self.current_streak >= self.longest_streak
    }
}

/// Task status breakdown.
#[derive(Debug, Clone, Default)]
pub struct StatusBreakdown {
    /// Number of tasks in each status
    pub todo: u32,
    pub in_progress: u32,
    pub blocked: u32,
    pub done: u32,
    pub cancelled: u32,
}

impl StatusBreakdown {
    /// Returns total task count.
    #[must_use]
    pub fn total(&self) -> u32 {
        self.todo + self.in_progress + self.blocked + self.done + self.cancelled
    }

    /// Returns completion rate (done / total).
    #[must_use]
    pub fn completion_rate(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }
        f64::from(self.done) / f64::from(total)
    }
}

/// Priority breakdown.
#[derive(Debug, Clone, Default)]
pub struct PriorityBreakdown {
    pub none: u32,
    pub low: u32,
    pub medium: u32,
    pub high: u32,
    pub urgent: u32,
}

impl PriorityBreakdown {
    /// Returns total task count.
    #[must_use]
    pub fn total(&self) -> u32 {
        self.none + self.low + self.medium + self.high + self.urgent
    }

    /// Returns percentage of high-priority tasks (high + urgent).
    #[must_use]
    pub fn high_priority_percentage(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }
        f64::from(self.high + self.urgent) / f64::from(total) * 100.0
    }
}

/// Tag usage statistics.
#[derive(Debug, Clone)]
pub struct TagStats {
    /// Tag name
    pub tag: String,
    /// Number of tasks with this tag
    pub count: u32,
    /// Number of completed tasks with this tag
    pub completed: u32,
}

impl TagStats {
    /// Returns completion rate for this tag.
    #[must_use]
    pub fn completion_rate(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        f64::from(self.completed) / f64::from(self.count)
    }
}

/// Report configuration for customizing analytics output.
#[derive(Debug, Clone)]
pub struct ReportConfig {
    /// Start date for the report period
    pub start_date: NaiveDate,
    /// End date for the report period
    pub end_date: NaiveDate,
    /// Whether to include project breakdowns
    pub include_projects: bool,
    /// Whether to include tag analysis
    pub include_tags: bool,
    /// Specific project IDs to include (None = all)
    pub project_filter: Option<Vec<ProjectId>>,
    /// Specific task IDs to include (None = all)
    pub task_filter: Option<Vec<TaskId>>,
}

impl ReportConfig {
    /// Create a report config for the last N days.
    #[must_use]
    pub fn last_n_days(days: i64) -> Self {
        let end = chrono::Local::now().date_naive();
        let start = end - chrono::Duration::days(days);
        Self {
            start_date: start,
            end_date: end,
            include_projects: true,
            include_tags: true,
            project_filter: None,
            task_filter: None,
        }
    }

    /// Create a report config for the current month.
    #[must_use]
    pub fn current_month() -> Self {
        let today = chrono::Local::now().date_naive();
        let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
            .expect("day 1 of current month always exists");
        Self {
            start_date: start,
            end_date: today,
            include_projects: true,
            include_tags: true,
            project_filter: None,
            task_filter: None,
        }
    }

    /// Create a report config for a specific date range.
    #[must_use]
    pub const fn custom(start_date: NaiveDate, end_date: NaiveDate) -> Self {
        Self {
            start_date,
            end_date,
            include_projects: true,
            include_tags: true,
            project_filter: None,
            task_filter: None,
        }
    }
}

/// Estimation accuracy analytics over time.
///
/// Tracks how accurate time estimates have been historically,
/// broken down by project, tag, and time period. Used to suggest
/// calibration adjustments for future estimates.
///
/// # Accuracy Calculation
///
/// Accuracy is calculated as `actual_minutes / estimated_minutes * 100`:
/// - 100% = perfect estimate
/// - 130% = task took 30% longer than estimated (overrun)
/// - 80% = task took 20% less than estimated (under)
///
/// # On-Target Threshold
///
/// Tasks are considered "on target" if actual time is within 10% of estimate
/// (90% - 110% accuracy range).
///
/// # Example
///
/// ```
/// use taskflow::domain::analytics::EstimationAnalytics;
///
/// let mut analytics = EstimationAnalytics::default();
/// analytics.tasks_with_estimates = 10;
/// analytics.on_target_count = 6;
/// analytics.over_count = 3;
/// analytics.under_count = 1;
/// analytics.avg_variance_minutes = 15;
///
/// assert_eq!(analytics.on_target_percentage(), 60.0);
/// assert!(analytics.accuracy_summary().contains("over"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct EstimationAnalytics {
    /// Accuracy trend points over time (date, accuracy percentage)
    /// where 100% = perfect accuracy, >100% = overrun, <100% = under
    pub accuracy_over_time: Vec<TimeSeriesPoint>,
    /// Per-project accuracy: (project_id, average accuracy %, task count)
    pub by_project: Vec<(Option<ProjectId>, f64, u32)>,
    /// Per-tag accuracy: (tag name, average accuracy %, task count)
    pub by_tag: Vec<(String, f64, u32)>,
    /// Suggested global estimation multiplier based on historical data
    /// (e.g., 1.3 means tasks typically take 30% longer than estimated)
    pub suggested_multiplier: f64,
    /// Total tasks with both estimates and actual times
    pub tasks_with_estimates: u32,
    /// Tasks that were on target (within 10% of estimate)
    pub on_target_count: u32,
    /// Tasks that went over estimate
    pub over_count: u32,
    /// Tasks that came in under estimate
    pub under_count: u32,
    /// Average variance in minutes (positive = overrun)
    pub avg_variance_minutes: i32,
}

impl EstimationAnalytics {
    /// Returns the percentage of tasks that were on target.
    #[must_use]
    pub fn on_target_percentage(&self) -> f64 {
        if self.tasks_with_estimates == 0 {
            return 0.0;
        }
        f64::from(self.on_target_count) / f64::from(self.tasks_with_estimates) * 100.0
    }

    /// Returns a human-readable summary of estimation accuracy.
    #[must_use]
    pub fn accuracy_summary(&self) -> String {
        if self.tasks_with_estimates == 0 {
            return "No tasks with estimates".to_string();
        }
        let direction = if self.avg_variance_minutes > 0 {
            "over"
        } else if self.avg_variance_minutes < 0 {
            "under"
        } else {
            "on target"
        };
        if direction == "on target" {
            "You estimate perfectly on average".to_string()
        } else {
            let abs_mins = self.avg_variance_minutes.abs();
            let hours = abs_mins / 60;
            let mins = abs_mins % 60;
            if hours > 0 {
                format!("You tend to estimate {hours}h {mins}m {direction}")
            } else {
                format!("You tend to estimate {mins}m {direction}")
            }
        }
    }
}

/// Complete analytics report aggregating all metrics.
#[derive(Debug, Clone)]
pub struct AnalyticsReport {
    /// The configuration used to generate this report
    pub config: ReportConfig,
    /// Completion trends over the period
    pub completion_trend: CompletionTrend,
    /// Velocity metrics
    pub velocity: VelocityMetrics,
    /// Burndown charts (one per project + global)
    pub burn_charts: Vec<BurnChart>,
    /// Time tracking analytics
    pub time_analytics: TimeAnalytics,
    /// Productivity insights
    pub insights: ProductivityInsights,
    /// Status breakdown
    pub status_breakdown: StatusBreakdown,
    /// Priority breakdown
    pub priority_breakdown: PriorityBreakdown,
    /// Tag statistics (sorted by count descending)
    pub tag_stats: Vec<TagStats>,
    /// Estimation accuracy analytics
    pub estimation_analytics: EstimationAnalytics,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_series_point() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let point = TimeSeriesPoint::new(date, 5.0);
        assert_eq!(point.date, date);
        assert!((point.value - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_completion_trend_average() {
        let trend = CompletionTrend {
            completions_by_day: vec![
                TimeSeriesPoint::new(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(), 5.0),
                TimeSeriesPoint::new(NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(), 3.0),
            ],
            creations_by_day: vec![
                TimeSeriesPoint::new(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(), 10.0),
                TimeSeriesPoint::new(NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(), 6.0),
            ],
            completion_rate_over_time: vec![
                TimeSeriesPoint::new(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(), 0.5),
                TimeSeriesPoint::new(NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(), 0.5),
            ],
        };
        assert!((trend.average_completion_rate() - 0.5).abs() < f64::EPSILON);
        assert_eq!(trend.total_completed(), 8);
        assert_eq!(trend.total_created(), 16);
    }

    #[test]
    fn test_completion_trend_empty() {
        let trend = CompletionTrend::default();
        assert!((trend.average_completion_rate() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_velocity_metrics_best_week() {
        let velocity = VelocityMetrics {
            weekly_velocity: vec![
                (NaiveDate::from_ymd_opt(2025, 1, 6).unwrap(), 5),
                (NaiveDate::from_ymd_opt(2025, 1, 13).unwrap(), 10),
                (NaiveDate::from_ymd_opt(2025, 1, 20).unwrap(), 7),
            ],
            monthly_velocity: vec![],
            avg_weekly: 7.33,
            trend: 0.5,
        };
        let best = velocity.best_week();
        assert!(best.is_some());
        assert_eq!(best.unwrap().1, 10);
        assert!(velocity.is_improving());
    }

    #[test]
    fn test_velocity_metrics_worst_week() {
        let velocity = VelocityMetrics {
            weekly_velocity: vec![
                (NaiveDate::from_ymd_opt(2025, 1, 6).unwrap(), 5),
                (NaiveDate::from_ymd_opt(2025, 1, 13).unwrap(), 0),
                (NaiveDate::from_ymd_opt(2025, 1, 20).unwrap(), 7),
            ],
            monthly_velocity: vec![],
            avg_weekly: 4.0,
            trend: -0.5,
        };
        let worst = velocity.worst_week();
        assert!(worst.is_some());
        assert_eq!(worst.unwrap().1, 5); // 0 is excluded
        assert!(!velocity.is_improving());
    }

    #[test]
    fn test_burn_chart_remaining() {
        let chart = BurnChart {
            project_name: "Test".to_string(),
            project_id: None,
            scope_line: vec![TimeSeriesPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                100.0,
            )],
            completed_line: vec![TimeSeriesPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                75.0,
            )],
            ideal_line: None,
        };
        assert!((chart.remaining_work() - 25.0).abs() < f64::EPSILON);
        assert!((chart.completion_percentage() - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_burn_chart_empty_scope() {
        let chart = BurnChart {
            project_name: "Empty".to_string(),
            project_id: None,
            scope_line: vec![],
            completed_line: vec![],
            ideal_line: None,
        };
        assert!((chart.completion_percentage() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_time_analytics_most_productive_day() {
        let mut analytics = TimeAnalytics::default();
        analytics.by_day_of_week[2] = 100; // Wednesday
        analytics.by_day_of_week[4] = 150; // Friday

        assert_eq!(analytics.most_productive_day(), Some(Weekday::Fri));
    }

    #[test]
    fn test_time_analytics_peak_hour() {
        let mut analytics = TimeAnalytics::default();
        analytics.by_hour[14] = 60; // 2 PM
        analytics.by_hour[9] = 30; // 9 AM

        assert_eq!(analytics.peak_hour(), Some(14));
    }

    #[test]
    fn test_time_analytics_total_hours() {
        let analytics = TimeAnalytics {
            total_minutes: 150,
            ..TimeAnalytics::default()
        };

        assert!((analytics.total_hours() - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_productivity_insights_streak() {
        let insights = ProductivityInsights {
            current_streak: 5,
            longest_streak: 10,
            ..Default::default()
        };
        assert!(insights.is_on_streak());
        assert!(!insights.is_best_streak());

        let best_ever = ProductivityInsights {
            current_streak: 15,
            longest_streak: 10,
            ..Default::default()
        };
        assert!(best_ever.is_best_streak());
    }

    #[test]
    fn test_status_breakdown() {
        let breakdown = StatusBreakdown {
            todo: 10,
            in_progress: 5,
            blocked: 0,
            done: 15,
            cancelled: 0,
        };
        assert_eq!(breakdown.total(), 30);
        assert!((breakdown.completion_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_status_breakdown_empty() {
        let breakdown = StatusBreakdown::default();
        assert!((breakdown.completion_rate() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_priority_breakdown() {
        let breakdown = PriorityBreakdown {
            none: 5,
            low: 10,
            medium: 10,
            high: 5,
            urgent: 0,
        };
        assert_eq!(breakdown.total(), 30);
        // 5 high + 0 urgent = 5 out of 30 = 16.67%
        assert!((breakdown.high_priority_percentage() - 16.666_666).abs() < 0.001);
    }

    #[test]
    fn test_tag_stats() {
        let stats = TagStats {
            tag: "work".to_string(),
            count: 10,
            completed: 7,
        };
        assert!((stats.completion_rate() - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_tag_stats_empty() {
        let stats = TagStats {
            tag: "empty".to_string(),
            count: 0,
            completed: 0,
        };
        assert!((stats.completion_rate() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_report_config_last_n_days() {
        let config = ReportConfig::last_n_days(7);
        assert!(config.include_projects);
        assert!(config.include_tags);
        assert!(config.project_filter.is_none());
        assert!(config.task_filter.is_none());
        // Should span 7 days
        let days = (config.end_date - config.start_date).num_days();
        assert_eq!(days, 7);
    }

    #[test]
    fn test_report_config_current_month() {
        let config = ReportConfig::current_month();
        assert_eq!(config.start_date.day(), 1);
    }

    #[test]
    fn test_report_config_custom() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
        let config = ReportConfig::custom(start, end);
        assert_eq!(config.start_date, start);
        assert_eq!(config.end_date, end);
    }

    #[test]
    fn test_estimation_analytics_default() {
        let analytics = EstimationAnalytics::default();
        assert_eq!(analytics.tasks_with_estimates, 0);
        assert!((analytics.on_target_percentage() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_estimation_analytics_on_target_percentage() {
        let analytics = EstimationAnalytics {
            tasks_with_estimates: 10,
            on_target_count: 7,
            over_count: 2,
            under_count: 1,
            ..Default::default()
        };
        assert!((analytics.on_target_percentage() - 70.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_estimation_analytics_accuracy_summary_over() {
        let analytics = EstimationAnalytics {
            tasks_with_estimates: 10,
            avg_variance_minutes: 45, // 45 minutes over
            ..Default::default()
        };
        assert_eq!(
            analytics.accuracy_summary(),
            "You tend to estimate 45m over"
        );
    }

    #[test]
    fn test_estimation_analytics_accuracy_summary_under() {
        let analytics = EstimationAnalytics {
            tasks_with_estimates: 10,
            avg_variance_minutes: -30, // 30 minutes under
            ..Default::default()
        };
        assert_eq!(
            analytics.accuracy_summary(),
            "You tend to estimate 30m under"
        );
    }

    #[test]
    fn test_estimation_analytics_accuracy_summary_perfect() {
        let analytics = EstimationAnalytics {
            tasks_with_estimates: 10,
            avg_variance_minutes: 0,
            ..Default::default()
        };
        assert_eq!(
            analytics.accuracy_summary(),
            "You estimate perfectly on average"
        );
    }

    #[test]
    fn test_estimation_analytics_accuracy_summary_hours() {
        let analytics = EstimationAnalytics {
            tasks_with_estimates: 10,
            avg_variance_minutes: 90, // 1h 30m over
            ..Default::default()
        };
        assert_eq!(
            analytics.accuracy_summary(),
            "You tend to estimate 1h 30m over"
        );
    }
}
