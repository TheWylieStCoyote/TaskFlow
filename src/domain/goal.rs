//! Goal/OKR entity for objective tracking.
//!
//! Goals represent high-level objectives that can span quarters or custom
//! date ranges. They contain Key Results that measure progress toward
//! the objective.
//!
//! # OKR Workflow
//!
//! Goals and Key Results work together following the OKR (Objectives and
//! Key Results) methodology:
//!
//! 1. **Create a Goal** - The high-level objective you want to achieve
//! 2. **Add Key Results** - Measurable outcomes that indicate success
//! 3. **Track Progress** - Update key result values as work progresses
//! 4. **Review** - Assess goal completion based on key result progress
//!
//! ```
//! use taskflow::domain::{Goal, KeyResult, Quarter, GoalId};
//!
//! // 1. Create a quarterly goal
//! let goal = Goal::new("Improve customer satisfaction")
//!     .with_quarter(2025, Quarter::Q1)
//!     .with_description("Focus on support quality and response times");
//! let goal_id = goal.id;
//!
//! // 2. Add measurable key results
//! let kr1 = KeyResult::new(goal_id, "Increase NPS score")
//!     .with_target(50.0, Some("points"));
//!
//! let kr2 = KeyResult::new(goal_id, "Reduce response time")
//!     .with_target(4.0, Some("hours"));
//!
//! // 3. Track progress by updating current values
//! let mut kr1 = kr1.with_current_value(42.0); // 84% complete
//! let mut kr2 = kr2.with_current_value(3.0);  // 75% complete (lower is better here)
//!
//! // 4. Calculate overall progress
//! let avg_progress = (kr1.progress_percent() + kr2.progress_percent()) / 2;
//! ```
//!
//! See [`crate::domain::KeyResult`] for key result tracking details.
//!
//! # Quarter-based Planning
//!
//! Goals commonly align with fiscal quarters. Use [`Quarter`] for standard
//! Q1-Q4 timeframes:
//!
//! ```
//! use taskflow::domain::{Goal, Quarter};
//!
//! let q1_goal = Goal::new("Q1 Revenue Target").with_quarter(2025, Quarter::Q1);
//! assert_eq!(q1_goal.formatted_timeframe(), "Q1 2025");
//! ```
//!
//! # Examples
//!
//! ```
//! use taskflow::domain::{Goal, GoalStatus, Quarter};
//! use chrono::NaiveDate;
//!
//! // Create a quarterly goal
//! let goal = Goal::new("Launch mobile app")
//!     .with_quarter(2025, Quarter::Q1)
//!     .with_description("Ship iOS and Android versions");
//!
//! assert_eq!(goal.name, "Launch mobile app");
//! assert!(goal.is_active());
//!
//! // Create a goal with custom dates
//! let goal = Goal::new("Complete migration")
//!     .with_dates(
//!         Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
//!         Some(NaiveDate::from_ymd_opt(2025, 3, 31).unwrap()),
//!     );
//! ```

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a goal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GoalId(pub Uuid);

impl GoalId {
    /// Creates a new unique goal ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for GoalId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for GoalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a goal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GoalStatus {
    /// Goal is actively being worked on.
    #[default]
    Active,
    /// Goal is temporarily paused.
    OnHold,
    /// Goal has been achieved.
    Completed,
    /// Goal is no longer relevant.
    Archived,
}

impl GoalStatus {
    /// Returns the status as a lowercase string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::OnHold => "on_hold",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }

    /// Parses a status from a string, returning `Active` for unknown values.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "on_hold" => Self::OnHold,
            "completed" => Self::Completed,
            "archived" => Self::Archived,
            _ => Self::Active,
        }
    }

    /// Returns true if the goal is active.
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns true if the goal is complete.
    #[must_use]
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Completed)
    }

    /// Returns a symbol representing the status.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Active => "●",
            Self::OnHold => "◐",
            Self::Completed => "✓",
            Self::Archived => "○",
        }
    }
}

impl std::fmt::Display for GoalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "Active"),
            Self::OnHold => write!(f, "On Hold"),
            Self::Completed => write!(f, "Completed"),
            Self::Archived => write!(f, "Archived"),
        }
    }
}

/// Calendar quarter for OKR-style goal periods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Quarter {
    /// January - March
    Q1,
    /// April - June
    Q2,
    /// July - September
    Q3,
    /// October - December
    Q4,
}

impl Quarter {
    /// Returns the start month of this quarter (1-indexed).
    #[must_use]
    pub const fn start_month(self) -> u32 {
        match self {
            Self::Q1 => 1,
            Self::Q2 => 4,
            Self::Q3 => 7,
            Self::Q4 => 10,
        }
    }

    /// Returns the end month of this quarter (1-indexed).
    #[must_use]
    pub const fn end_month(self) -> u32 {
        match self {
            Self::Q1 => 3,
            Self::Q2 => 6,
            Self::Q3 => 9,
            Self::Q4 => 12,
        }
    }

    /// Returns the start date of this quarter for a given year.
    #[must_use]
    pub fn start_date(self, year: i32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(year, self.start_month(), 1)
    }

    /// Returns the end date of this quarter for a given year.
    #[must_use]
    pub fn end_date(self, year: i32) -> Option<NaiveDate> {
        let end_month = self.end_month();
        let last_day = match end_month {
            3 | 12 => 31,
            6 | 9 => 30,
            _ => 31,
        };
        NaiveDate::from_ymd_opt(year, end_month, last_day)
    }

    /// Returns the quarter containing a given date.
    #[must_use]
    pub fn from_date(date: NaiveDate) -> Self {
        match date.month() {
            1..=3 => Self::Q1,
            4..=6 => Self::Q2,
            7..=9 => Self::Q3,
            _ => Self::Q4,
        }
    }

    /// Returns the current quarter.
    #[must_use]
    pub fn current() -> (i32, Self) {
        let now = Utc::now().date_naive();
        (now.year(), Self::from_date(now))
    }
}

impl std::fmt::Display for Quarter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Q1 => write!(f, "Q1"),
            Self::Q2 => write!(f, "Q2"),
            Self::Q3 => write!(f, "Q3"),
            Self::Q4 => write!(f, "Q4"),
        }
    }
}

use chrono::Datelike;

/// A goal/objective in the OKR hierarchy.
///
/// Goals are high-level objectives that contain Key Results.
/// They can be tracked by quarter (OKR-style) or with custom dates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    /// Unique identifier.
    pub id: GoalId,
    /// Goal name/title.
    pub name: String,
    /// Optional longer description.
    pub description: Option<String>,
    /// Current status.
    pub status: GoalStatus,

    // Timeframe (flexible dates OR quarterly)
    /// Start date for custom date ranges.
    pub start_date: Option<NaiveDate>,
    /// Due/end date for custom date ranges.
    pub due_date: Option<NaiveDate>,
    /// Quarter for OKR-style tracking (year, quarter).
    pub quarter: Option<(i32, Quarter)>,

    // Progress
    /// Manual progress override (0-100). None = auto-calculate from key results.
    pub manual_progress: Option<u8>,

    // Metadata
    /// Optional color for UI display.
    pub color: Option<String>,
    /// Optional icon/emoji.
    pub icon: Option<String>,

    // Timestamps
    /// When this goal was created.
    pub created_at: DateTime<Utc>,
    /// When this goal was last modified.
    pub updated_at: DateTime<Utc>,
}

impl Goal {
    /// Creates a new goal with the given name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: GoalId::new(),
            name: name.into(),
            description: None,
            status: GoalStatus::default(),
            start_date: None,
            due_date: None,
            quarter: None,
            manual_progress: None,
            color: None,
            icon: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the goal description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the goal status.
    #[must_use]
    pub const fn with_status(mut self, status: GoalStatus) -> Self {
        self.status = status;
        self
    }

    /// Sets custom start and end dates.
    #[must_use]
    pub const fn with_dates(mut self, start: Option<NaiveDate>, end: Option<NaiveDate>) -> Self {
        self.start_date = start;
        self.due_date = end;
        self
    }

    /// Sets the quarter for OKR-style tracking.
    #[must_use]
    pub const fn with_quarter(mut self, year: i32, quarter: Quarter) -> Self {
        self.quarter = Some((year, quarter));
        self
    }

    /// Sets manual progress (0-100).
    #[must_use]
    pub fn with_manual_progress(mut self, progress: u8) -> Self {
        self.manual_progress = Some(progress.min(100));
        self
    }

    /// Sets the display color.
    #[must_use]
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the icon/emoji.
    #[must_use]
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Returns true if the goal is active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.status.is_active()
    }

    /// Returns true if the goal is complete.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.status.is_complete()
    }

    /// Returns the effective start date (from quarter or custom date).
    #[must_use]
    pub fn effective_start_date(&self) -> Option<NaiveDate> {
        if let Some((year, quarter)) = self.quarter {
            quarter.start_date(year)
        } else {
            self.start_date
        }
    }

    /// Returns the effective end date (from quarter or custom date).
    #[must_use]
    pub fn effective_end_date(&self) -> Option<NaiveDate> {
        if let Some((year, quarter)) = self.quarter {
            quarter.end_date(year)
        } else {
            self.due_date
        }
    }

    /// Returns a formatted timeframe string.
    #[must_use]
    pub fn formatted_timeframe(&self) -> String {
        if let Some((year, quarter)) = self.quarter {
            format!("{quarter} {year}")
        } else {
            match (self.start_date, self.due_date) {
                (Some(start), Some(end)) => {
                    format!("{} - {}", start.format("%b %d"), end.format("%b %d, %Y"))
                }
                (None, Some(end)) => format!("Due {}", end.format("%b %d, %Y")),
                (Some(start), None) => format!("From {}", start.format("%b %d, %Y")),
                (None, None) => "No timeframe".to_string(),
            }
        }
    }

    /// Marks the goal as complete.
    pub fn complete(&mut self) {
        self.status = GoalStatus::Completed;
        self.updated_at = Utc::now();
    }

    /// Marks the goal as archived.
    pub fn archive(&mut self) {
        self.status = GoalStatus::Archived;
        self.updated_at = Utc::now();
    }
}

impl std::fmt::Display for Goal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goal_new() {
        let goal = Goal::new("Test Goal");
        assert_eq!(goal.name, "Test Goal");
        assert_eq!(goal.status, GoalStatus::Active);
        assert!(goal.description.is_none());
        assert!(goal.quarter.is_none());
        assert!(goal.manual_progress.is_none());
    }

    #[test]
    fn test_goal_builder() {
        let goal = Goal::new("Launch Product")
            .with_description("Ship the MVP")
            .with_quarter(2025, Quarter::Q1)
            .with_color("#3498db")
            .with_icon("🚀");

        assert_eq!(goal.name, "Launch Product");
        assert_eq!(goal.description, Some("Ship the MVP".to_string()));
        assert_eq!(goal.quarter, Some((2025, Quarter::Q1)));
        assert_eq!(goal.color, Some("#3498db".to_string()));
        assert_eq!(goal.icon, Some("🚀".to_string()));
    }

    #[test]
    fn test_goal_with_dates() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 3, 31).unwrap();

        let goal = Goal::new("Custom Goal").with_dates(Some(start), Some(end));

        assert_eq!(goal.start_date, Some(start));
        assert_eq!(goal.due_date, Some(end));
        assert_eq!(goal.effective_start_date(), Some(start));
        assert_eq!(goal.effective_end_date(), Some(end));
    }

    #[test]
    fn test_goal_quarter_overrides_dates() {
        let goal = Goal::new("Q1 Goal").with_quarter(2025, Quarter::Q1);

        assert_eq!(
            goal.effective_start_date(),
            NaiveDate::from_ymd_opt(2025, 1, 1)
        );
        assert_eq!(
            goal.effective_end_date(),
            NaiveDate::from_ymd_opt(2025, 3, 31)
        );
    }

    #[test]
    fn test_goal_status() {
        let mut goal = Goal::new("Test");
        assert!(goal.is_active());
        assert!(!goal.is_complete());

        goal.complete();
        assert!(!goal.is_active());
        assert!(goal.is_complete());
    }

    #[test]
    fn test_goal_manual_progress_capped() {
        let goal = Goal::new("Test").with_manual_progress(150);
        assert_eq!(goal.manual_progress, Some(100));
    }

    #[test]
    fn test_quarter_dates() {
        assert_eq!(Quarter::Q1.start_month(), 1);
        assert_eq!(Quarter::Q1.end_month(), 3);
        assert_eq!(Quarter::Q2.start_month(), 4);
        assert_eq!(Quarter::Q3.start_month(), 7);
        assert_eq!(Quarter::Q4.start_month(), 10);

        assert_eq!(
            Quarter::Q1.start_date(2025),
            NaiveDate::from_ymd_opt(2025, 1, 1)
        );
        assert_eq!(
            Quarter::Q1.end_date(2025),
            NaiveDate::from_ymd_opt(2025, 3, 31)
        );
        assert_eq!(
            Quarter::Q2.end_date(2025),
            NaiveDate::from_ymd_opt(2025, 6, 30)
        );
    }

    #[test]
    fn test_quarter_from_date() {
        assert_eq!(
            Quarter::from_date(NaiveDate::from_ymd_opt(2025, 2, 15).unwrap()),
            Quarter::Q1
        );
        assert_eq!(
            Quarter::from_date(NaiveDate::from_ymd_opt(2025, 5, 1).unwrap()),
            Quarter::Q2
        );
        assert_eq!(
            Quarter::from_date(NaiveDate::from_ymd_opt(2025, 8, 20).unwrap()),
            Quarter::Q3
        );
        assert_eq!(
            Quarter::from_date(NaiveDate::from_ymd_opt(2025, 11, 30).unwrap()),
            Quarter::Q4
        );
    }

    #[test]
    fn test_goal_formatted_timeframe() {
        let q1_goal = Goal::new("Q1").with_quarter(2025, Quarter::Q1);
        assert_eq!(q1_goal.formatted_timeframe(), "Q1 2025");

        let no_dates = Goal::new("No dates");
        assert_eq!(no_dates.formatted_timeframe(), "No timeframe");
    }

    #[test]
    fn test_goal_status_symbol() {
        assert_eq!(GoalStatus::Active.symbol(), "●");
        assert_eq!(GoalStatus::OnHold.symbol(), "◐");
        assert_eq!(GoalStatus::Completed.symbol(), "✓");
        assert_eq!(GoalStatus::Archived.symbol(), "○");
    }

    #[test]
    fn test_goal_id_unique() {
        let id1 = GoalId::new();
        let id2 = GoalId::new();
        assert_ne!(id1, id2);
    }
}
