//! Status, priority, and tag breakdown calculations.
//!
//! This module provides methods for computing breakdowns by:
//! - Task status (todo, in progress, blocked, done, cancelled)
//! - Task priority (urgent, high, medium, low, none)
//! - Tags (count and completion rate per tag)

use std::collections::HashMap;

use crate::domain::analytics::{PriorityBreakdown, StatusBreakdown, TagStats};
use crate::domain::{Priority, TaskStatus};

use super::AnalyticsEngine;

impl AnalyticsEngine<'_> {
    /// Compute status breakdown.
    ///
    /// Counts the number of tasks in each status category.
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
    ///
    /// Counts the number of tasks at each priority level.
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
    ///
    /// For each tag, counts total tasks and completed tasks,
    /// sorted by count descending.
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

        // Sort by count descending, then alphabetically by tag name (case-insensitive)
        stats.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| a.tag.to_lowercase().cmp(&b.tag.to_lowercase()))
        });
        stats
    }
}
