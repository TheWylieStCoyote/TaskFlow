//! Estimation accuracy analytics.
//!
//! This module provides methods for computing estimation accuracy metrics,
//! including historical trends, per-project multipliers, and suggestions
//! for calibrating future estimates.
//!
//! # Overview
//!
//! The estimation system helps users improve their time estimates by:
//!
//! 1. **Tracking historical accuracy** - Comparing estimated vs actual time
//! 2. **Computing multipliers** - Deriving correction factors from past data
//! 3. **Providing suggestions** - Recommending adjusted estimates for new tasks
//!
//! # Usage
//!
//! ```rust,ignore
//! use taskflow::app::{Model, analytics::AnalyticsEngine};
//! use chrono::{Duration, Utc};
//!
//! let model = Model::new().with_sample_data();
//! let engine = AnalyticsEngine::new(&model);
//!
//! // Compute analytics for the last 90 days
//! let start = Utc::now().date_naive() - Duration::days(90);
//! let end = Utc::now().date_naive();
//! let analytics = engine.compute_estimation_analytics(start, end);
//!
//! // Get a suggestion for a new task
//! if let Some(suggestion) = engine.suggest_estimate(60, None, &[]) {
//!     println!("Suggested: {} minutes", suggestion.suggested_minutes);
//!     println!("Reason: {}", suggestion.explanation);
//! }
//! ```

use std::collections::HashMap;

use chrono::NaiveDate;

use crate::domain::analytics::{EstimationAnalytics, TimeSeriesPoint};
use crate::domain::{ProjectId, Task};

use super::AnalyticsEngine;

/// Information about a suggested estimate based on historical data.
///
/// When a user enters an estimate for a new task, the system can suggest
/// an adjusted estimate based on historical accuracy patterns. This struct
/// contains the suggestion along with metadata about how it was derived.
///
/// # Confidence Levels
///
/// - `0.0 - 0.3`: Low confidence (few historical data points)
/// - `0.3 - 0.7`: Medium confidence (some historical data)
/// - `0.7 - 1.0`: High confidence (many similar completed tasks)
///
/// # Priority Order
///
/// Suggestions are derived in this priority order:
/// 1. Explicit project multiplier (if set on the project)
/// 2. Calculated project accuracy (from completed tasks in same project)
/// 3. Tag-based accuracy (from completed tasks with similar tags)
/// 4. Global accuracy (from all completed tasks with estimates)
#[derive(Debug, Clone)]
pub struct EstimationSuggestion {
    /// Suggested estimate in minutes, adjusted from the raw estimate
    pub suggested_minutes: u32,
    /// Number of completed tasks this suggestion is based on
    pub based_on_count: u32,
    /// Project-specific multiplier if one was explicitly set
    pub project_multiplier: Option<f64>,
    /// Confidence level (0.0 - 1.0) based on amount of historical data
    pub confidence: f64,
    /// Human-readable explanation of how the suggestion was derived
    pub explanation: String,
}

impl AnalyticsEngine<'_> {
    /// Compute estimation accuracy analytics for the given date range.
    ///
    /// Analyzes completed tasks with both estimates and actual times to compute:
    /// - Accuracy trends over time
    /// - Per-project accuracy
    /// - Per-tag accuracy
    /// - Suggested global multiplier
    #[must_use]
    pub fn compute_estimation_analytics(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> EstimationAnalytics {
        let mut analytics = EstimationAnalytics::default();

        // Collect tasks with both estimate and actual time that are completed
        let tasks_with_data: Vec<&Task> = self
            .model
            .tasks
            .values()
            .filter(|t| {
                t.estimated_minutes.is_some()
                    && t.actual_minutes > 0
                    && t.status.is_complete()
                    && t.completed_at
                        .is_some_and(|c| c.date_naive() >= start && c.date_naive() <= end)
            })
            .collect();

        if tasks_with_data.is_empty() {
            return analytics;
        }

        analytics.tasks_with_estimates = tasks_with_data.len() as u32;

        // Calculate overall stats
        let mut total_variance: i64 = 0;
        let mut total_accuracy_sum: f64 = 0.0;

        // Group by date for trend
        let mut by_date: HashMap<NaiveDate, Vec<f64>> = HashMap::new();
        // Group by project
        let mut by_project: HashMap<Option<ProjectId>, (f64, u32)> = HashMap::new();
        // Group by tag
        let mut by_tag: HashMap<String, (f64, u32)> = HashMap::new();

        for task in &tasks_with_data {
            let estimated = task.estimated_minutes.unwrap_or(0);
            let actual = task.actual_minutes;
            let variance = i64::from(actual) - i64::from(estimated);
            let accuracy = if estimated > 0 {
                f64::from(actual) / f64::from(estimated) * 100.0
            } else {
                100.0
            };

            total_variance += variance;
            total_accuracy_sum += accuracy;

            // Categorize as over/under/on-target (within 10%)
            if (90.0..=110.0).contains(&accuracy) {
                analytics.on_target_count += 1;
            } else if accuracy > 110.0 {
                analytics.over_count += 1;
            } else {
                analytics.under_count += 1;
            }

            // Group by completion date
            if let Some(completed_at) = task.completed_at {
                let date = completed_at.date_naive();
                by_date.entry(date).or_default().push(accuracy);
            }

            // Group by project
            let entry = by_project.entry(task.project_id).or_insert((0.0, 0));
            entry.0 += accuracy;
            entry.1 += 1;

            // Group by tag
            for tag in &task.tags {
                let entry = by_tag.entry(tag.clone()).or_insert((0.0, 0));
                entry.0 += accuracy;
                entry.1 += 1;
            }
        }

        // Calculate averages
        let count = tasks_with_data.len() as f64;
        analytics.avg_variance_minutes = (total_variance as f64 / count).round() as i32;
        let avg_accuracy = total_accuracy_sum / count;

        // Calculate suggested multiplier (actual/estimated ratio)
        // If average accuracy is 130%, multiplier should be 1.3
        analytics.suggested_multiplier = avg_accuracy / 100.0;

        // Build accuracy over time trend (aggregate by week for smoother trend)
        let mut dates: Vec<NaiveDate> = by_date.keys().copied().collect();
        dates.sort();
        for date in dates {
            if let Some(accuracies) = by_date.get(&date) {
                let avg = accuracies.iter().sum::<f64>() / accuracies.len() as f64;
                analytics
                    .accuracy_over_time
                    .push(TimeSeriesPoint::new(date, avg));
            }
        }

        // Build per-project accuracy
        let mut project_data: Vec<_> = by_project
            .into_iter()
            .map(|(project_id, (sum, count))| (project_id, sum / f64::from(count), count))
            .collect();
        project_data.sort_by(|a, b| b.2.cmp(&a.2)); // Sort by count descending
        analytics.by_project = project_data;

        // Build per-tag accuracy
        let mut tag_data: Vec<_> = by_tag
            .into_iter()
            .map(|(tag, (sum, count))| (tag, sum / f64::from(count), count))
            .collect();
        tag_data.sort_by(|a, b| b.2.cmp(&a.2)); // Sort by count descending
        analytics.by_tag = tag_data;

        analytics
    }

    /// Suggest an estimate for a new task based on similar completed tasks.
    ///
    /// Takes into account:
    /// - Project-specific multipliers if the task belongs to a project
    /// - Historical averages for tasks with similar tags
    /// - Global estimation accuracy
    ///
    /// Returns `None` if there's insufficient historical data.
    #[must_use]
    pub fn suggest_estimate(
        &self,
        raw_estimate: u32,
        project_id: Option<ProjectId>,
        tags: &[String],
    ) -> Option<EstimationSuggestion> {
        // Need at least some historical data
        let analytics = self.compute_estimation_analytics(
            chrono::Local::now().date_naive() - chrono::Duration::days(90),
            chrono::Local::now().date_naive(),
        );

        if analytics.tasks_with_estimates < 3 {
            return None; // Not enough data
        }

        // Check for project-specific multiplier first
        let project_multiplier = project_id.and_then(|pid| {
            self.model
                .projects
                .get(&pid)
                .and_then(|p| p.estimation_multiplier)
        });

        // Look up project accuracy from analytics
        let project_accuracy = project_id.and_then(|pid| {
            analytics
                .by_project
                .iter()
                .find(|(p, _, _)| *p == Some(pid))
                .map(|(_, acc, _)| *acc)
        });

        // Calculate average tag accuracy if tags provided
        let tag_accuracy = if tags.is_empty() {
            None
        } else {
            let matching: Vec<f64> = analytics
                .by_tag
                .iter()
                .filter(|(t, _, _)| tags.contains(t))
                .map(|(_, acc, _)| *acc)
                .collect();
            if matching.is_empty() {
                None
            } else {
                Some(matching.iter().sum::<f64>() / matching.len() as f64)
            }
        };

        // Determine the best multiplier to use (prioritize explicit project multiplier)
        let effective_multiplier = if let Some(mult) = project_multiplier {
            mult
        } else if let Some(acc) = project_accuracy {
            acc / 100.0
        } else if let Some(acc) = tag_accuracy {
            acc / 100.0
        } else {
            analytics.suggested_multiplier
        };

        // Calculate suggested estimate
        let suggested = (f64::from(raw_estimate) * effective_multiplier).round() as u32;

        // Build explanation
        let explanation = if project_multiplier.is_some() {
            format!(
                "Based on project multiplier of {:.1}x ({} similar tasks)",
                effective_multiplier, analytics.tasks_with_estimates
            )
        } else if project_accuracy.is_some() {
            format!(
                "Tasks in this project typically take {:.0}% of estimates",
                effective_multiplier * 100.0
            )
        } else if tag_accuracy.is_some() {
            format!(
                "Tasks with similar tags typically take {:.0}% of estimates",
                effective_multiplier * 100.0
            )
        } else {
            format!(
                "Based on overall accuracy of {:.0}% ({} tasks)",
                effective_multiplier * 100.0,
                analytics.tasks_with_estimates
            )
        };

        // Calculate confidence (more data = higher confidence)
        let confidence = (f64::from(analytics.tasks_with_estimates) / 20.0).min(1.0);

        Some(EstimationSuggestion {
            suggested_minutes: suggested,
            based_on_count: analytics.tasks_with_estimates,
            project_multiplier,
            confidence,
            explanation,
        })
    }

    /// Calculate a suggested estimation multiplier for a specific project.
    ///
    /// Returns the ratio of actual time to estimated time for completed tasks
    /// in this project. Returns `None` if insufficient data.
    #[must_use]
    pub fn calculate_project_multiplier(&self, project_id: ProjectId) -> Option<f64> {
        let tasks: Vec<&Task> = self
            .model
            .tasks
            .values()
            .filter(|t| {
                t.project_id == Some(project_id)
                    && t.estimated_minutes.is_some()
                    && t.actual_minutes > 0
                    && t.status.is_complete()
            })
            .collect();

        if tasks.len() < 3 {
            return None; // Need at least 3 data points
        }

        let total_estimated: u32 = tasks.iter().filter_map(|t| t.estimated_minutes).sum();
        let total_actual: u32 = tasks.iter().map(|t| t.actual_minutes).sum();

        if total_estimated == 0 {
            return None;
        }

        Some(f64::from(total_actual) / f64::from(total_estimated))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Model;
    use crate::domain::{Project, Task, TaskStatus};
    use chrono::{Duration, Utc};

    fn create_completed_task_with_estimate(
        title: &str,
        estimated: u32,
        _actual: u32,
        project_id: Option<ProjectId>,
    ) -> Task {
        let completed_at = Utc::now() - Duration::days(1);
        Task::new(title)
            .with_estimated_minutes(estimated)
            .with_status(TaskStatus::Done)
            .with_completed_at(completed_at)
            .with_project_opt(project_id)
    }

    #[test]
    fn test_estimation_analytics_empty() {
        let model = Model::new();
        let engine = AnalyticsEngine::new(&model);
        let analytics = engine.compute_estimation_analytics(
            Utc::now().date_naive() - Duration::days(30),
            Utc::now().date_naive(),
        );

        assert_eq!(analytics.tasks_with_estimates, 0);
        assert!((analytics.suggested_multiplier - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_estimation_analytics_on_target() {
        let mut model = Model::new();

        // Create tasks that are exactly on target (actual = estimated)
        for i in 0..5 {
            let mut task = create_completed_task_with_estimate(&format!("Task {i}"), 60, 60, None);
            task.actual_minutes = 60; // Same as estimated
            model.tasks.insert(task.id, task);
        }

        let engine = AnalyticsEngine::new(&model);
        let analytics = engine.compute_estimation_analytics(
            Utc::now().date_naive() - Duration::days(30),
            Utc::now().date_naive(),
        );

        assert_eq!(analytics.tasks_with_estimates, 5);
        assert_eq!(analytics.on_target_count, 5);
        assert_eq!(analytics.over_count, 0);
        assert_eq!(analytics.under_count, 0);
        assert!((analytics.suggested_multiplier - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_estimation_analytics_over_estimate() {
        let mut model = Model::new();

        // Create tasks that take 30% longer than estimated
        for i in 0..5 {
            let mut task = create_completed_task_with_estimate(&format!("Task {i}"), 60, 78, None);
            task.actual_minutes = 78; // 30% over
            model.tasks.insert(task.id, task);
        }

        let engine = AnalyticsEngine::new(&model);
        let analytics = engine.compute_estimation_analytics(
            Utc::now().date_naive() - Duration::days(30),
            Utc::now().date_naive(),
        );

        assert_eq!(analytics.tasks_with_estimates, 5);
        assert_eq!(analytics.over_count, 5);
        // Multiplier should be ~1.3
        assert!((analytics.suggested_multiplier - 1.3).abs() < 0.01);
    }

    #[test]
    fn test_estimation_analytics_by_project() {
        let mut model = Model::new();

        let project = Project::new("Test Project");
        let project_id = project.id;
        model.projects.insert(project_id, project);

        // Create tasks in the project
        for i in 0..3 {
            let mut task = create_completed_task_with_estimate(
                &format!("Project Task {i}"),
                60,
                90,
                Some(project_id),
            );
            task.actual_minutes = 90; // 50% over
            model.tasks.insert(task.id, task);
        }

        // Create tasks without project
        for i in 0..3 {
            let mut task =
                create_completed_task_with_estimate(&format!("No Project Task {i}"), 60, 60, None);
            task.actual_minutes = 60;
            model.tasks.insert(task.id, task);
        }

        let engine = AnalyticsEngine::new(&model);
        let analytics = engine.compute_estimation_analytics(
            Utc::now().date_naive() - Duration::days(30),
            Utc::now().date_naive(),
        );

        assert_eq!(analytics.tasks_with_estimates, 6);
        // Should have 2 entries in by_project
        assert_eq!(analytics.by_project.len(), 2);
    }

    #[test]
    fn test_suggest_estimate_insufficient_data() {
        let model = Model::new();
        let engine = AnalyticsEngine::new(&model);

        let suggestion = engine.suggest_estimate(60, None, &[]);
        assert!(suggestion.is_none());
    }

    #[test]
    fn test_calculate_project_multiplier() {
        let mut model = Model::new();

        let project = Project::new("Test Project");
        let project_id = project.id;
        model.projects.insert(project_id, project);

        // Create 5 tasks that consistently take 20% longer
        for i in 0..5 {
            let mut task = create_completed_task_with_estimate(
                &format!("Task {i}"),
                100,
                120,
                Some(project_id),
            );
            task.actual_minutes = 120;
            model.tasks.insert(task.id, task);
        }

        let engine = AnalyticsEngine::new(&model);
        let multiplier = engine.calculate_project_multiplier(project_id);

        assert!(multiplier.is_some());
        assert!((multiplier.unwrap() - 1.2).abs() < 0.01);
    }
}
