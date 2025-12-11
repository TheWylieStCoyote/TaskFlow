//! Abstract syntax tree types for the filter DSL.
//!
//! This module defines the data structures that represent parsed filter expressions.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::domain::{Priority, TaskStatus};

/// A parsed filter expression that can be evaluated against tasks.
///
/// Filter expressions form a tree structure supporting boolean operators
/// (AND, OR, NOT) and leaf conditions like `priority:high` or `status:todo`.
///
/// # Examples
///
/// ```
/// use taskflow::domain::filter_dsl::{FilterExpr, Condition, FilterField, FilterValue};
/// use taskflow::domain::Priority;
///
/// // Simple condition: priority:high
/// let expr = FilterExpr::Condition(Condition {
///     field: FilterField::Priority,
///     value: FilterValue::Priority(Priority::High),
/// });
///
/// // Compound expression: priority:high AND status:todo
/// let and_expr = FilterExpr::And(
///     Box::new(expr),
///     Box::new(FilterExpr::Condition(Condition {
///         field: FilterField::Status,
///         value: FilterValue::Status(taskflow::domain::TaskStatus::Todo),
///     })),
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilterExpr {
    /// Logical AND of two expressions (both must match).
    And(Box<FilterExpr>, Box<FilterExpr>),

    /// Logical OR of two expressions (either must match).
    Or(Box<FilterExpr>, Box<FilterExpr>),

    /// Logical NOT (negation) of an expression.
    Not(Box<FilterExpr>),

    /// A leaf condition (e.g., `priority:high`).
    Condition(Condition),
}

impl FilterExpr {
    /// Create an AND expression from two expressions.
    #[must_use]
    pub fn and(left: FilterExpr, right: FilterExpr) -> Self {
        Self::And(Box::new(left), Box::new(right))
    }

    /// Create an OR expression from two expressions.
    #[must_use]
    pub fn or(left: FilterExpr, right: FilterExpr) -> Self {
        Self::Or(Box::new(left), Box::new(right))
    }

    /// Create a NOT expression wrapping another expression.
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn not(inner: FilterExpr) -> Self {
        Self::Not(Box::new(inner))
    }

    /// Create a condition expression.
    #[must_use]
    pub fn condition(field: FilterField, value: FilterValue) -> Self {
        Self::Condition(Condition { field, value })
    }
}

/// A single filter condition (field:value pair).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Condition {
    /// The field to filter on.
    pub field: FilterField,
    /// The value to match against.
    pub value: FilterValue,
}

/// Supported filter fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterField {
    /// Task priority (none, low, medium, high, urgent).
    Priority,
    /// Task status (todo, in_progress, blocked, done, cancelled).
    Status,
    /// Task tags.
    Tags,
    /// Project name (partial match, case-insensitive).
    Project,
    /// Due date conditions (today, tomorrow, overdue, etc.).
    Due,
    /// Full-text search in title and description.
    Search,
    /// Check for presence of a field (has:due, has:project, etc.).
    Has,
    /// Title text (partial match).
    Title,
    /// Creation date conditions (today, thisweek, before/after date, etc.).
    Created,
    /// Scheduled date conditions (today, tomorrow, thisweek, etc.).
    Scheduled,
    /// Completion date conditions (today, yesterday, thisweek, etc.).
    Completed,
    /// Modification date conditions (today, yesterday, thisweek, etc.).
    Modified,
    /// Time estimate in minutes (numeric comparisons).
    Estimate,
    /// Actual time tracked in minutes (numeric comparisons).
    Actual,
}

impl FilterField {
    /// Get the field name as it appears in DSL syntax.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Priority => "priority",
            Self::Status => "status",
            Self::Tags => "tags",
            Self::Project => "project",
            Self::Due => "due",
            Self::Search => "search",
            Self::Has => "has",
            Self::Title => "title",
            Self::Created => "created",
            Self::Scheduled => "scheduled",
            Self::Completed => "completed",
            Self::Modified => "modified",
            Self::Estimate => "estimate",
            Self::Actual => "actual",
        }
    }
}

/// Filter condition values (typed per field).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum FilterValue {
    /// Priority values: none, low, medium, high, urgent.
    Priority(Priority),

    /// Status values: todo, in_progress, blocked, done, cancelled.
    Status(TaskStatus),

    /// Tag name (case-insensitive match).
    Tag(String),

    /// Project name (partial match, case-insensitive).
    ProjectName(String),

    /// Due date conditions.
    Due(DueFilter),

    /// Full-text search query.
    SearchText(String),

    /// Has-field conditions.
    Has(HasField),

    /// Title text (partial match).
    TitleText(String),

    /// Creation date conditions.
    Created(CreatedFilter),

    /// Scheduled date conditions.
    Scheduled(ScheduledFilter),

    /// Completion date conditions.
    Completed(CreatedFilter),

    /// Modification date conditions.
    Modified(CreatedFilter),

    /// Time estimate filter (numeric comparisons).
    Estimate(NumericFilter),

    /// Actual time tracked filter (numeric comparisons).
    Actual(NumericFilter),
}

/// Due date filter options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DueFilter {
    /// Due today.
    Today,
    /// Due tomorrow.
    Tomorrow,
    /// Due this week (Monday to Sunday).
    ThisWeek,
    /// Due next week.
    NextWeek,
    /// Past due and not completed.
    Overdue,
    /// Has no due date.
    None,
    /// Due on a specific date.
    On(NaiveDate),
    /// Due before a specific date (exclusive).
    Before(NaiveDate),
    /// Due after a specific date (exclusive).
    After(NaiveDate),
    /// Due within a date range (inclusive).
    Between(NaiveDate, NaiveDate),
    /// Due on or after a date (inclusive, for `start..` syntax).
    OnOrAfter(NaiveDate),
    /// Due on or before a date (inclusive, for `..end` syntax).
    OnOrBefore(NaiveDate),
}

/// Creation date filter options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CreatedFilter {
    /// Created today.
    Today,
    /// Created yesterday.
    Yesterday,
    /// Created this week (Monday to Sunday).
    ThisWeek,
    /// Created last week.
    LastWeek,
    /// Created on a specific date.
    On(NaiveDate),
    /// Created before a specific date (exclusive).
    Before(NaiveDate),
    /// Created after a specific date (exclusive).
    After(NaiveDate),
    /// Created within a date range (inclusive).
    Between(NaiveDate, NaiveDate),
    /// Created on or after a date (inclusive, for `start..` syntax).
    OnOrAfter(NaiveDate),
    /// Created on or before a date (inclusive, for `..end` syntax).
    OnOrBefore(NaiveDate),
}

/// Scheduled date filter options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScheduledFilter {
    /// Scheduled for today.
    Today,
    /// Scheduled for tomorrow.
    Tomorrow,
    /// Scheduled this week (Monday to Sunday).
    ThisWeek,
    /// Scheduled next week.
    NextWeek,
    /// Has no scheduled date.
    None,
    /// Scheduled on a specific date.
    On(NaiveDate),
    /// Scheduled before a specific date (exclusive).
    Before(NaiveDate),
    /// Scheduled after a specific date (exclusive).
    After(NaiveDate),
    /// Scheduled within a date range (inclusive).
    Between(NaiveDate, NaiveDate),
    /// Scheduled on or after a date (inclusive, for `start..` syntax).
    OnOrAfter(NaiveDate),
    /// Scheduled on or before a date (inclusive, for `..end` syntax).
    OnOrBefore(NaiveDate),
}

/// Numeric comparison filter for time estimates and actuals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NumericFilter {
    /// Equals a specific value.
    Equals(u32),
    /// Greater than a value.
    GreaterThan(u32),
    /// Less than a value.
    LessThan(u32),
    /// Greater than or equal to a value.
    GreaterOrEqual(u32),
    /// Less than or equal to a value.
    LessOrEqual(u32),
    /// Has no value (None).
    None,
    /// Value within a range (inclusive).
    Between(u32, u32),
}

/// Fields that can be checked for presence with `has:`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HasField {
    /// Task has a due date.
    Due,
    /// Task is assigned to a project.
    Project,
    /// Task has at least one tag.
    Tags,
    /// Task has a time estimate.
    Estimate,
    /// Task has a description.
    Description,
    /// Task has a recurrence pattern.
    Recurrence,
    /// Task has a scheduled date.
    Scheduled,
    /// Task has dependencies.
    Dependencies,
    /// Task has a parent (is a subtask).
    Parent,
    /// Task has tracked time (actual_minutes > 0).
    Tracked,
}

impl HasField {
    /// Get the field name as it appears in DSL syntax.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Due => "due",
            Self::Project => "project",
            Self::Tags => "tags",
            Self::Estimate => "estimate",
            Self::Description => "description",
            Self::Recurrence => "recurrence",
            Self::Scheduled => "scheduled",
            Self::Dependencies => "dependencies",
            Self::Parent => "parent",
            Self::Tracked => "tracked",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_expr_and() {
        let left =
            FilterExpr::condition(FilterField::Priority, FilterValue::Priority(Priority::High));
        let right =
            FilterExpr::condition(FilterField::Status, FilterValue::Status(TaskStatus::Todo));
        let expr = FilterExpr::and(left, right);
        assert!(matches!(expr, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_filter_expr_or() {
        let left =
            FilterExpr::condition(FilterField::Status, FilterValue::Status(TaskStatus::Todo));
        let right = FilterExpr::condition(
            FilterField::Status,
            FilterValue::Status(TaskStatus::InProgress),
        );
        let expr = FilterExpr::or(left, right);
        assert!(matches!(expr, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_filter_expr_not() {
        let inner =
            FilterExpr::condition(FilterField::Status, FilterValue::Status(TaskStatus::Done));
        let expr = FilterExpr::not(inner);
        assert!(matches!(expr, FilterExpr::Not(_)));
    }

    #[test]
    fn test_filter_field_as_str() {
        assert_eq!(FilterField::Priority.as_str(), "priority");
        assert_eq!(FilterField::Status.as_str(), "status");
        assert_eq!(FilterField::Tags.as_str(), "tags");
        assert_eq!(FilterField::Project.as_str(), "project");
        assert_eq!(FilterField::Due.as_str(), "due");
        assert_eq!(FilterField::Search.as_str(), "search");
        assert_eq!(FilterField::Has.as_str(), "has");
        assert_eq!(FilterField::Title.as_str(), "title");
    }

    #[test]
    fn test_has_field_as_str() {
        assert_eq!(HasField::Due.as_str(), "due");
        assert_eq!(HasField::Project.as_str(), "project");
        assert_eq!(HasField::Tags.as_str(), "tags");
        assert_eq!(HasField::Estimate.as_str(), "estimate");
        assert_eq!(HasField::Description.as_str(), "description");
    }

    #[test]
    fn test_filter_expr_serialization() {
        let expr =
            FilterExpr::condition(FilterField::Priority, FilterValue::Priority(Priority::High));
        let json = serde_json::to_string(&expr).unwrap();
        let restored: FilterExpr = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, restored);
    }

    #[test]
    fn test_complex_expr_serialization() {
        let expr = FilterExpr::and(
            FilterExpr::condition(FilterField::Priority, FilterValue::Priority(Priority::High)),
            FilterExpr::not(FilterExpr::condition(
                FilterField::Status,
                FilterValue::Status(TaskStatus::Done),
            )),
        );
        let json = serde_json::to_string(&expr).unwrap();
        let restored: FilterExpr = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, restored);
    }

    #[test]
    fn test_due_filter_variants() {
        let filters = vec![
            DueFilter::Today,
            DueFilter::Tomorrow,
            DueFilter::ThisWeek,
            DueFilter::NextWeek,
            DueFilter::Overdue,
            DueFilter::None,
        ];
        for filter in filters {
            let json = serde_json::to_string(&filter).unwrap();
            let restored: DueFilter = serde_json::from_str(&json).unwrap();
            assert_eq!(filter, restored);
        }
    }
}
