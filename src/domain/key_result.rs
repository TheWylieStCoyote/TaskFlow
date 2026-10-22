//! Key Result entity for measurable outcomes within goals.
//!
//! Key Results are specific, measurable outcomes that indicate progress
//! toward a Goal. They can track numeric targets or link to tasks/projects.
//!
//! # Tracking Methods
//!
//! Key Results support two tracking approaches:
//!
//! ## Numeric Targets
//!
//! Track progress toward a measurable number (revenue, users, score):
//!
//! ```
//! use taskflow::domain::{KeyResult, GoalId};
//!
//! let kr = KeyResult::new(GoalId::new(), "Acquire 1000 users")
//!     .with_target(1000.0, Some("users"))
//!     .with_current_value(450.0);
//!
//! assert_eq!(kr.progress_percent(), 45); // 450/1000 = 45%
//! ```
//!
//! ## Linked Tasks/Projects
//!
//! Track progress based on task or project completion:
//!
//! ```
//! use taskflow::domain::{KeyResult, GoalId, TaskId};
//!
//! let goal_id = GoalId::new();
//! let mut kr = KeyResult::new(goal_id, "Complete migration tasks");
//!
//! // Link tasks to this key result
//! kr.linked_task_ids.push(TaskId::new());
//! kr.linked_task_ids.push(TaskId::new());
//! kr.linked_task_ids.push(TaskId::new());
//!
//! // Progress is calculated from linked task completion
//! // When 2 of 3 tasks are done → 66.7% progress
//! // (Requires calling calculate_progress_from_tasks() to update)
//! ```
//!
//! # When to Use Each Approach
//!
//! | Use Case | Approach |
//! |----------|----------|
//! | Revenue, users, NPS scores | Numeric target |
//! | Completing a set of tasks | Linked tasks |
//! | Shipping a project | Linked project |
//! | Abstract goals (culture, learning) | Numeric with % target |
//!
//! # Status Transitions
//!
//! Key result status updates automatically based on progress:
//! - **NotStarted**: `current_value == 0`
//! - **InProgress**: `0 < progress < 100%`
//! - **Completed**: `progress >= 100%`
//! - **AtRisk**: Set manually when behind schedule
//!
//! See [`crate::domain::Goal`] for the parent goal entity.
//!
//! # Examples
//!
//! ```
//! use taskflow::domain::{KeyResult, GoalId};
//!
//! // Create a key result with a numeric target
//! let kr = KeyResult::new(GoalId::new(), "Acquire 1000 users")
//!     .with_target(1000.0, Some("users"))
//!     .with_current_value(450.0);
//!
//! assert_eq!(kr.name, "Acquire 1000 users");
//! assert_eq!(kr.progress_percent(), 45);
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::goal::GoalId;
use super::project::ProjectId;
use super::task::TaskId;

/// Unique identifier for a key result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyResultId(pub Uuid);

impl KeyResultId {
    /// Creates a new unique key result ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for KeyResultId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for KeyResultId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a key result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyResultStatus {
    /// Not yet started.
    #[default]
    NotStarted,
    /// Work in progress.
    InProgress,
    /// At risk of not being achieved.
    AtRisk,
    /// Successfully achieved.
    Completed,
}

impl KeyResultStatus {
    /// Returns the status as a lowercase string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::InProgress => "in_progress",
            Self::AtRisk => "at_risk",
            Self::Completed => "completed",
        }
    }

    /// Parses a status from a string, returning `NotStarted` for unknown values.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "in_progress" => Self::InProgress,
            "at_risk" => Self::AtRisk,
            "completed" => Self::Completed,
            _ => Self::NotStarted,
        }
    }

    /// Returns true if in progress (not done, not failed).
    #[must_use]
    pub const fn is_in_progress(self) -> bool {
        matches!(self, Self::InProgress | Self::AtRisk)
    }

    /// Returns true if completed.
    #[must_use]
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Completed)
    }

    /// Returns a symbol representing the status.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::NotStarted => "○",
            Self::InProgress => "◐",
            Self::AtRisk => "⚠",
            Self::Completed => "✓",
        }
    }
}

impl std::fmt::Display for KeyResultStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStarted => write!(f, "Not Started"),
            Self::InProgress => write!(f, "In Progress"),
            Self::AtRisk => write!(f, "At Risk"),
            Self::Completed => write!(f, "Completed"),
        }
    }
}

/// A measurable outcome within a Goal.
///
/// Key Results can track progress in two ways:
/// 1. Numeric target/current values (e.g., "100 users" with current 45)
/// 2. Linked tasks (progress = % of linked tasks completed)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyResult {
    /// Unique identifier.
    pub id: KeyResultId,
    /// Parent goal this belongs to.
    pub goal_id: GoalId,
    /// Key result name/description.
    pub name: String,
    /// Optional longer description.
    pub description: Option<String>,
    /// Current status.
    pub status: KeyResultStatus,

    // Measurable target
    /// Target value to achieve (e.g., 100).
    pub target_value: f64,
    /// Current progress value (e.g., 45).
    pub current_value: f64,
    /// Unit of measurement (e.g., "users", "%", "$").
    pub unit: Option<String>,

    // Manual progress override
    /// Manual progress (0-100). None = auto-calculate.
    pub manual_progress: Option<u8>,

    // Linked items
    /// Projects linked to this key result.
    pub linked_project_ids: Vec<ProjectId>,
    /// Tasks linked to this key result.
    pub linked_task_ids: Vec<TaskId>,

    // Timestamps
    /// When this key result was created.
    pub created_at: DateTime<Utc>,
    /// When this key result was last modified.
    pub updated_at: DateTime<Utc>,
}

impl KeyResult {
    /// Creates a new key result for a goal.
    #[must_use]
    pub fn new(goal_id: GoalId, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: KeyResultId::new(),
            goal_id,
            name: name.into(),
            description: None,
            status: KeyResultStatus::default(),
            target_value: 0.0,
            current_value: 0.0,
            unit: None,
            manual_progress: None,
            linked_project_ids: Vec::new(),
            linked_task_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the status.
    #[must_use]
    pub const fn with_status(mut self, status: KeyResultStatus) -> Self {
        self.status = status;
        self
    }

    /// Sets the target value and optional unit.
    #[must_use]
    pub fn with_target(mut self, target: f64, unit: Option<&str>) -> Self {
        self.target_value = target;
        self.unit = unit.map(String::from);
        self
    }

    /// Sets the current value.
    #[must_use]
    pub const fn with_current_value(mut self, value: f64) -> Self {
        self.current_value = value;
        self
    }

    /// Sets manual progress override (0-100).
    #[must_use]
    pub const fn with_manual_progress(mut self, progress: u8) -> Self {
        self.manual_progress = Some(if progress > 100 { 100 } else { progress });
        self
    }

    /// Links a project to this key result.
    pub fn link_project(&mut self, project_id: ProjectId) {
        if !self.linked_project_ids.contains(&project_id) {
            self.linked_project_ids.push(project_id);
            self.updated_at = Utc::now();
        }
    }

    /// Unlinks a project from this key result.
    pub fn unlink_project(&mut self, project_id: &ProjectId) {
        if let Some(pos) = self
            .linked_project_ids
            .iter()
            .position(|id| id == project_id)
        {
            self.linked_project_ids.remove(pos);
            self.updated_at = Utc::now();
        }
    }

    /// Links a task to this key result.
    pub fn link_task(&mut self, task_id: TaskId) {
        if !self.linked_task_ids.contains(&task_id) {
            self.linked_task_ids.push(task_id);
            self.updated_at = Utc::now();
        }
    }

    /// Unlinks a task from this key result.
    pub fn unlink_task(&mut self, task_id: &TaskId) {
        if let Some(pos) = self.linked_task_ids.iter().position(|id| id == task_id) {
            self.linked_task_ids.remove(pos);
            self.updated_at = Utc::now();
        }
    }

    /// Returns true if completed.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.status.is_complete()
    }

    /// Returns progress percentage (0-100) based on target/current values.
    ///
    /// Note: This only considers numeric values, not linked tasks.
    /// Use `Model::key_result_progress()` for full progress calculation.
    #[must_use]
    pub fn progress_percent(&self) -> u8 {
        if let Some(manual) = self.manual_progress {
            return manual;
        }

        if self.target_value > 0.0 {
            let pct = (self.current_value / self.target_value * 100.0).min(100.0);
            pct as u8
        } else {
            0
        }
    }

    /// Returns a formatted progress string (e.g., "45/100 users").
    #[must_use]
    #[allow(clippy::float_cmp)] // Intentional: checking for integer-like values
    pub fn formatted_progress(&self) -> String {
        let unit = self.unit.as_deref().unwrap_or("");
        if self.target_value > 0.0 {
            if self.target_value == self.target_value.floor()
                && self.current_value == self.current_value.floor()
            {
                // Integer display
                format!(
                    "{}/{} {}",
                    self.current_value as i64, self.target_value as i64, unit
                )
                .trim()
                .to_string()
            } else {
                // Decimal display
                format!(
                    "{:.1}/{:.1} {}",
                    self.current_value, self.target_value, unit
                )
                .trim()
                .to_string()
            }
        } else if !self.linked_task_ids.is_empty() {
            format!("{} linked tasks", self.linked_task_ids.len())
        } else {
            "No target set".to_string()
        }
    }

    /// Updates the current value.
    pub fn set_value(&mut self, value: f64) {
        self.current_value = value;
        self.updated_at = Utc::now();

        // Auto-update status based on progress
        if value >= self.target_value && self.target_value > 0.0 {
            self.status = KeyResultStatus::Completed;
        } else if value > 0.0 && self.status == KeyResultStatus::NotStarted {
            self.status = KeyResultStatus::InProgress;
        }
    }

    /// Marks the key result as complete.
    pub fn complete(&mut self) {
        self.status = KeyResultStatus::Completed;
        self.updated_at = Utc::now();
    }
}

impl std::fmt::Display for KeyResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_result_new() {
        let goal_id = GoalId::new();
        let kr = KeyResult::new(goal_id, "Test KR");

        assert_eq!(kr.name, "Test KR");
        assert_eq!(kr.goal_id, goal_id);
        assert_eq!(kr.status, KeyResultStatus::NotStarted);
        assert!((kr.target_value - 0.0).abs() < f64::EPSILON);
        assert!((kr.current_value - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_key_result_with_target() {
        let kr = KeyResult::new(GoalId::new(), "Users")
            .with_target(100.0, Some("users"))
            .with_current_value(45.0);

        assert!((kr.target_value - 100.0).abs() < f64::EPSILON);
        assert!((kr.current_value - 45.0).abs() < f64::EPSILON);
        assert_eq!(kr.unit, Some("users".to_string()));
        assert_eq!(kr.progress_percent(), 45);
    }

    #[test]
    fn test_key_result_progress_capped() {
        let kr = KeyResult::new(GoalId::new(), "Test")
            .with_target(50.0, None)
            .with_current_value(100.0);

        assert_eq!(kr.progress_percent(), 100); // Capped at 100
    }

    #[test]
    fn test_key_result_manual_progress() {
        let kr = KeyResult::new(GoalId::new(), "Test")
            .with_target(100.0, None)
            .with_current_value(25.0)
            .with_manual_progress(75);

        // Manual overrides calculated
        assert_eq!(kr.progress_percent(), 75);
    }

    #[test]
    fn test_key_result_manual_progress_capped() {
        let kr = KeyResult::new(GoalId::new(), "Test").with_manual_progress(150);

        assert_eq!(kr.manual_progress, Some(100));
    }

    #[test]
    fn test_key_result_formatted_progress() {
        let kr = KeyResult::new(GoalId::new(), "Users")
            .with_target(100.0, Some("users"))
            .with_current_value(45.0);

        assert_eq!(kr.formatted_progress(), "45/100 users");

        let kr_decimal = KeyResult::new(GoalId::new(), "Revenue")
            .with_target(1000.5, Some("$"))
            .with_current_value(500.25);

        assert_eq!(kr_decimal.formatted_progress(), "500.2/1000.5 $");
    }

    #[test]
    fn test_key_result_link_task() {
        let mut kr = KeyResult::new(GoalId::new(), "Test");
        let task_id = TaskId::new();

        kr.link_task(task_id);
        assert_eq!(kr.linked_task_ids.len(), 1);
        assert!(kr.linked_task_ids.contains(&task_id));

        // Linking same task again should not duplicate
        kr.link_task(task_id);
        assert_eq!(kr.linked_task_ids.len(), 1);

        kr.unlink_task(&task_id);
        assert!(kr.linked_task_ids.is_empty());
    }

    #[test]
    fn test_key_result_link_project() {
        let mut kr = KeyResult::new(GoalId::new(), "Test");
        let project_id = ProjectId::new();

        kr.link_project(project_id);
        assert_eq!(kr.linked_project_ids.len(), 1);

        kr.unlink_project(&project_id);
        assert!(kr.linked_project_ids.is_empty());
    }

    #[test]
    fn test_key_result_set_value_updates_status() {
        let mut kr = KeyResult::new(GoalId::new(), "Test").with_target(100.0, None);

        assert_eq!(kr.status, KeyResultStatus::NotStarted);

        kr.set_value(50.0);
        assert_eq!(kr.status, KeyResultStatus::InProgress);

        kr.set_value(100.0);
        assert_eq!(kr.status, KeyResultStatus::Completed);
    }

    #[test]
    fn test_key_result_status_symbols() {
        assert_eq!(KeyResultStatus::NotStarted.symbol(), "○");
        assert_eq!(KeyResultStatus::InProgress.symbol(), "◐");
        assert_eq!(KeyResultStatus::AtRisk.symbol(), "⚠");
        assert_eq!(KeyResultStatus::Completed.symbol(), "✓");
    }

    #[test]
    fn test_key_result_id_unique() {
        let id1 = KeyResultId::new();
        let id2 = KeyResultId::new();
        assert_ne!(id1, id2);
    }

    const ALL_STATUSES: [KeyResultStatus; 4] = [
        KeyResultStatus::NotStarted,
        KeyResultStatus::InProgress,
        KeyResultStatus::AtRisk,
        KeyResultStatus::Completed,
    ];

    #[test]
    fn test_key_result_status_display() {
        assert_eq!(KeyResultStatus::NotStarted.to_string(), "Not Started");
        assert_eq!(KeyResultStatus::InProgress.to_string(), "In Progress");
        assert_eq!(KeyResultStatus::AtRisk.to_string(), "At Risk");
        assert_eq!(KeyResultStatus::Completed.to_string(), "Completed");
    }

    #[test]
    fn test_key_result_status_as_str() {
        assert_eq!(KeyResultStatus::NotStarted.as_str(), "not_started");
        assert_eq!(KeyResultStatus::InProgress.as_str(), "in_progress");
        assert_eq!(KeyResultStatus::AtRisk.as_str(), "at_risk");
        assert_eq!(KeyResultStatus::Completed.as_str(), "completed");
    }

    #[test]
    fn test_key_result_status_from_str_lossy() {
        for status in ALL_STATUSES {
            assert_eq!(KeyResultStatus::from_str_lossy(status.as_str()), status);
        }
        assert_eq!(
            KeyResultStatus::from_str_lossy(""),
            KeyResultStatus::NotStarted
        );
        assert_eq!(
            KeyResultStatus::from_str_lossy("done"),
            KeyResultStatus::NotStarted
        );
        assert_eq!(
            KeyResultStatus::from_str_lossy("AT_RISK"),
            KeyResultStatus::NotStarted
        );
    }

    #[test]
    fn test_key_result_status_predicates_and_default() {
        assert_eq!(KeyResultStatus::default(), KeyResultStatus::NotStarted);

        assert!(!KeyResultStatus::NotStarted.is_in_progress());
        assert!(KeyResultStatus::InProgress.is_in_progress());
        assert!(KeyResultStatus::AtRisk.is_in_progress());
        assert!(!KeyResultStatus::Completed.is_in_progress());

        for status in ALL_STATUSES {
            assert_eq!(
                status.is_complete(),
                status == KeyResultStatus::Completed,
                "{status:?}"
            );
        }
    }

    #[test]
    fn test_key_result_status_serde_roundtrip() {
        for status in ALL_STATUSES {
            let json = serde_json::to_string(&status).unwrap();
            let back: KeyResultStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, status, "roundtrip of {json}");
        }
        assert_eq!(
            serde_json::to_string(&KeyResultStatus::AtRisk).unwrap(),
            "\"atrisk\""
        );
    }

    #[test]
    fn test_key_result_is_complete_and_complete() {
        let mut kr = KeyResult::new(GoalId::new(), "Ship");
        assert!(!kr.is_complete());

        kr.complete();
        assert!(kr.is_complete());
        assert_eq!(kr.status, KeyResultStatus::Completed);
        assert!(kr.updated_at >= kr.created_at);

        let at_risk = KeyResult::new(GoalId::new(), "Risky").with_status(KeyResultStatus::AtRisk);
        assert!(!at_risk.is_complete());
        assert_eq!(at_risk.status, KeyResultStatus::AtRisk);
    }

    #[test]
    fn test_key_result_builders_and_display() {
        let goal_id = GoalId::new();
        let kr = KeyResult::new(goal_id, "Reduce churn").with_description("Monthly churn below 2%");
        assert_eq!(kr.description.as_deref(), Some("Monthly churn below 2%"));
        assert_eq!(kr.goal_id, goal_id);
        assert_eq!(kr.to_string(), "Reduce churn");
        assert_eq!(format!("{kr}"), "Reduce churn");
        assert!(kr.linked_project_ids.is_empty());
        assert!(kr.linked_task_ids.is_empty());
        assert!(kr.manual_progress.is_none());
        assert!(kr.unit.is_none());
    }

    #[test]
    fn test_key_result_id_display_is_uuid() {
        let id = KeyResultId::new();
        assert_eq!(id.to_string(), id.0.to_string());
        assert_eq!(id.to_string().len(), 36);
        assert_ne!(KeyResultId::default(), KeyResultId::default());
    }

    #[test]
    fn test_key_result_progress_percent_without_target_is_zero() {
        let kr = KeyResult::new(GoalId::new(), "No target").with_current_value(5.0);
        assert_eq!(kr.progress_percent(), 0);

        let manual = KeyResult::new(GoalId::new(), "Manual").with_manual_progress(60);
        assert_eq!(manual.progress_percent(), 60);

        let partial = KeyResult::new(GoalId::new(), "Partial")
            .with_target(3.0, None)
            .with_current_value(1.0);
        assert_eq!(partial.progress_percent(), 33);
    }

    #[test]
    fn test_key_result_formatted_progress_fallbacks() {
        let none = KeyResult::new(GoalId::new(), "Nothing");
        assert_eq!(none.formatted_progress(), "No target set");

        let mut linked = KeyResult::new(GoalId::new(), "Linked");
        linked.link_task(TaskId::new());
        linked.link_task(TaskId::new());
        assert_eq!(linked.formatted_progress(), "2 linked tasks");

        // A numeric target wins over linked tasks and trims the missing unit.
        let numeric = linked.with_target(100.0, None).with_current_value(45.0);
        assert_eq!(numeric.formatted_progress(), "45/100");

        let decimal_no_unit = KeyResult::new(GoalId::new(), "Decimal")
            .with_target(2.5, None)
            .with_current_value(1.0);
        assert_eq!(decimal_no_unit.formatted_progress(), "1.0/2.5");
    }

    #[test]
    fn test_key_result_set_value_without_target_never_completes() {
        let mut kr = KeyResult::new(GoalId::new(), "Open ended");

        kr.set_value(0.0);
        assert_eq!(kr.status, KeyResultStatus::NotStarted);

        kr.set_value(5.0);
        assert_eq!(kr.status, KeyResultStatus::InProgress);
        assert!((kr.current_value - 5.0).abs() < f64::EPSILON);
        assert!(!kr.is_complete());
    }

    #[test]
    fn test_key_result_set_value_keeps_at_risk_until_target_reached() {
        let mut kr = KeyResult::new(GoalId::new(), "Risky")
            .with_target(100.0, None)
            .with_status(KeyResultStatus::AtRisk);

        kr.set_value(50.0);
        assert_eq!(kr.status, KeyResultStatus::AtRisk);

        kr.set_value(150.0);
        assert_eq!(kr.status, KeyResultStatus::Completed);
        assert_eq!(kr.progress_percent(), 100);
    }

    #[test]
    fn test_key_result_unlink_unknown_ids_is_noop() {
        let mut kr = KeyResult::new(GoalId::new(), "Links");
        let task_id = TaskId::new();
        let project_id = ProjectId::new();
        kr.link_task(task_id);
        kr.link_project(project_id);

        kr.unlink_task(&TaskId::new());
        kr.unlink_project(&ProjectId::new());
        assert_eq!(kr.linked_task_ids, vec![task_id]);
        assert_eq!(kr.linked_project_ids, vec![project_id]);

        // Linking the same project twice does not duplicate it.
        kr.link_project(project_id);
        assert_eq!(kr.linked_project_ids.len(), 1);
    }
}
