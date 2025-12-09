//! Analytics engine for computing task metrics and insights.
//!
//! This module provides the [`AnalyticsEngine`] which computes various metrics
//! from task data, including completion trends, velocity, burndown charts,
//! and productivity insights.
//!
//! # Architecture
//!
//! The analytics module is organized into specialized sub-modules:
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `trends` | Completion trends and velocity calculations |
//! | `breakdowns` | Status, priority, and tag breakdowns |
//! | `insights` | Productivity insights (streaks, best day, peak hour) |
//! | `time` | Time tracking analytics and burndown charts |
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

mod breakdowns;
mod insights;
mod time;
mod trends;

#[cfg(test)]
mod tests;

use chrono::Weekday;

use crate::domain::analytics::{AnalyticsReport, ReportConfig};

use super::Model;

/// Maps array index to weekday (0 = Monday, 6 = Sunday).
pub(crate) const WEEKDAYS: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

/// Engine for computing analytics from task data.
///
/// The analytics engine provides methods for computing various metrics
/// and insights from task data, including:
///
/// - Completion trends over time
/// - Velocity metrics (weekly/monthly)
/// - Burndown charts for projects
/// - Time tracking analytics
/// - Productivity insights (streaks, peak hours, best days)
/// - Status and priority breakdowns
/// - Tag statistics
pub struct AnalyticsEngine<'a> {
    pub(crate) model: &'a Model,
}

impl<'a> AnalyticsEngine<'a> {
    /// Create a new analytics engine for the given model.
    #[must_use]
    pub const fn new(model: &'a Model) -> Self {
        Self { model }
    }

    /// Generate a complete analytics report.
    ///
    /// This method orchestrates the generation of all analytics components
    /// based on the provided configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Report configuration specifying date range and options
    ///
    /// # Returns
    ///
    /// A complete [`AnalyticsReport`] containing all computed metrics.
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
}
