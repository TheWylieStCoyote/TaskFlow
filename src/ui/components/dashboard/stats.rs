//! Dashboard statistics calculations
//!
//! This module contains methods for calculating various statistics
//! displayed on the dashboard, including completion rates, time tracking,
//! and estimation accuracy.

use chrono::{Datelike, Utc};

use crate::app::Model;
use crate::domain::{Priority, TaskStatus, TimeEntry};

/// Statistics calculator for the dashboard
pub struct DashboardStats<'a> {
    pub(crate) model: &'a Model,
}

impl<'a> DashboardStats<'a> {
    /// Create a new statistics calculator
    #[must_use]
    pub const fn new(model: &'a Model) -> Self {
        Self { model }
    }

    /// Calculate completion rate as percentage
    #[must_use]
    pub fn completion_rate(&self) -> f32 {
        let total = self.model.tasks.len();
        if total == 0 {
            return 0.0;
        }
        let completed = self
            .model
            .tasks
            .values()
            .filter(|t| t.status.is_complete())
            .count();
        (completed as f32 / total as f32) * 100.0
    }

    /// Calculate completion rate by priority
    #[must_use]
    pub fn completion_by_priority(&self, priority: Priority) -> (usize, usize) {
        let tasks: Vec<_> = self
            .model
            .tasks
            .values()
            .filter(|t| t.priority == priority)
            .collect();
        let total = tasks.len();
        let completed = tasks.iter().filter(|t| t.status.is_complete()).count();
        (completed, total)
    }

    /// Get total time tracked across all tasks
    #[must_use]
    pub fn total_time_tracked(&self) -> u32 {
        self.model
            .time_entries
            .values()
            .map(TimeEntry::calculated_duration_minutes)
            .sum()
    }

    /// Get count of overdue tasks
    #[must_use]
    pub fn overdue_count(&self) -> usize {
        let today = Utc::now().date_naive();
        self.model
            .tasks
            .values()
            .filter(|t| {
                t.due_date
                    .is_some_and(|d| d < today && !t.status.is_complete())
            })
            .count()
    }

    /// Count tasks by status
    /// Returns (todo, in_progress, blocked, done, cancelled)
    #[must_use]
    pub fn status_counts(&self) -> (usize, usize, usize, usize, usize) {
        let mut todo = 0;
        let mut in_progress = 0;
        let mut blocked = 0;
        let mut done = 0;
        let mut cancelled = 0;

        for task in self.model.tasks.values() {
            match task.status {
                TaskStatus::Todo => todo += 1,
                TaskStatus::InProgress => in_progress += 1,
                TaskStatus::Blocked => blocked += 1,
                TaskStatus::Done => done += 1,
                TaskStatus::Cancelled => cancelled += 1,
            }
        }

        (todo, in_progress, blocked, done, cancelled)
    }

    /// Get tasks created this week
    #[must_use]
    pub fn tasks_this_week(&self) -> usize {
        let today = Utc::now().date_naive();
        let week_start =
            today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);

        self.model
            .tasks
            .values()
            .filter(|t| t.created_at.date_naive() >= week_start)
            .count()
    }

    /// Get tasks completed this week
    #[must_use]
    pub fn completed_this_week(&self) -> usize {
        let today = Utc::now().date_naive();
        let week_start =
            today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);

        self.model
            .tasks
            .values()
            .filter(|t| t.completed_at.is_some_and(|d| d.date_naive() >= week_start))
            .count()
    }

    /// Calculate estimation accuracy statistics
    /// Returns (total_estimated, total_actual, over_count, under_count, on_target_count, avg_accuracy)
    #[must_use]
    pub fn estimation_stats(&self) -> (u32, u32, usize, usize, usize, Option<f64>) {
        let mut total_estimated: u32 = 0;
        let mut total_actual: u32 = 0;
        let mut over_count = 0;
        let mut under_count = 0;
        let mut on_target_count = 0;
        let mut accuracies: Vec<f64> = Vec::new();

        for task in self.model.tasks.values() {
            if let Some(est) = task.estimated_minutes {
                total_estimated = total_estimated.saturating_add(est);
                total_actual = total_actual.saturating_add(task.actual_minutes);

                if let Some(variance) = task.time_variance() {
                    match variance.cmp(&0) {
                        std::cmp::Ordering::Greater => over_count += 1,
                        std::cmp::Ordering::Less => under_count += 1,
                        std::cmp::Ordering::Equal => on_target_count += 1,
                    }
                }

                if let Some(accuracy) = task.estimation_accuracy() {
                    accuracies.push(accuracy);
                }
            }
        }

        let avg_accuracy = if accuracies.is_empty() {
            None
        } else {
            Some(accuracies.iter().sum::<f64>() / accuracies.len() as f64)
        };

        (
            total_estimated,
            total_actual,
            over_count,
            under_count,
            on_target_count,
            avg_accuracy,
        )
    }
}

/// Format minutes as hours and minutes
#[must_use]
pub fn format_duration(minutes: u32) -> String {
    let hours = minutes / 60;
    let mins = minutes % 60;
    if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}
