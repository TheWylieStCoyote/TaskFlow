//! Evaluator for filter DSL expressions.
//!
//! Evaluates parsed filter expressions against tasks to determine matches.
//!
//! # Date and Time Handling
//!
//! ## Timezone Behavior
//!
//! The evaluator uses **UTC** for determining "today" via [`Utc::now()`]:
//!
//! - `today`, `tomorrow`, `yesterday` - Based on UTC date
//! - Task timestamps (`created_at`, `completed_at`, `updated_at`) are stored in UTC
//! - Due dates and scheduled dates are stored as [`NaiveDate`] (no timezone)
//!
//! **Note:** Users in timezones far from UTC may see unexpected behavior near midnight.
//! For example, at 11 PM EST (4 AM UTC next day), `due:today` uses the UTC date.
//!
//! ## Week Boundaries
//!
//! Week-based filters use **Monday as the start of the week**:
//!
//! | Keyword | Definition |
//! |---------|------------|
//! | `thisweek` | Monday through Sunday of the current week |
//! | `nextweek` | Monday through Sunday of next week |
//! | `lastweek` | Monday through Sunday of last week |
//!
//! Week boundaries are calculated using [`chrono::Weekday::num_days_from_monday()`].
//!
//! ### Week Calculation Example
//!
//! If today is Wednesday, June 18, 2025:
//!
//! ```text
//! thisweek: Mon Jun 16 - Sun Jun 22 (includes today)
//! nextweek: Mon Jun 23 - Sun Jun 29
//! lastweek: Mon Jun  9 - Sun Jun 15
//! ```
//!
//! # Range Semantics
//!
//! All range boundaries are **inclusive**:
//!
//! | Syntax | Meaning | Boundary Behavior |
//! |--------|---------|-------------------|
//! | `2025-01-01..2025-12-31` | Between dates | Both inclusive |
//! | `2025-06-01..` | On or after | Start inclusive |
//! | `..2025-12-31` | On or before | End inclusive |
//! | `>2025-01-01` | Strictly after | Exclusive |
//! | `<2025-12-31` | Strictly before | Exclusive |
//!
//! **Key distinction:** Range syntax (`..`) is always inclusive, while comparison
//! operators (`<`, `>`) are exclusive. Use `<=` or `>=` for inclusive comparisons.
//!
//! # Text Matching
//!
//! Text fields use **case-insensitive substring matching**:
//!
//! - `search:` - Matches against title OR description
//! - `title:` - Matches against title only
//! - `tags:` - Exact tag name match (case-insensitive)
//! - `project:` - Partial project name match (case-insensitive)
//!
//! # Example
//!
//! ```
//! use std::collections::HashMap;
//! use taskflow::domain::{Task, Priority};
//! use taskflow::domain::filter_dsl::{parse, evaluate, EvalContext};
//!
//! let task = Task::new("Fix login bug").with_priority(Priority::High);
//! let expr = parse("priority:high AND title:login").unwrap();
//! let projects = HashMap::new();
//! let ctx = EvalContext::new(&projects);
//!
//! assert!(evaluate(&expr, &task, &ctx));
//! ```

use std::collections::HashMap;

use chrono::{Datelike, NaiveDate, Utc};

use crate::domain::{Project, ProjectId, Task};

use super::ast::{
    Condition, CreatedFilter, DueFilter, FilterExpr, FilterValue, HasField, NumericFilter,
    ScheduledFilter,
};

/// Pre-computed lowercase task data for efficient filtering.
///
/// Avoids repeated `to_lowercase()` allocations during filter evaluation.
/// Create once per task and reuse across multiple filter condition evaluations.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use taskflow::domain::Task;
/// use taskflow::domain::filter_dsl::{parse, evaluate_with_cache, EvalContext, TaskLowerCache};
///
/// let task = Task::new("Fix Login Bug");
/// let cache = TaskLowerCache::new(&task);
/// let expr = parse("title:login").unwrap();
/// let projects = HashMap::new();
/// let ctx = EvalContext::new(&projects);
///
/// assert!(evaluate_with_cache(&expr, &cache, &ctx));
/// ```
pub struct TaskLowerCache<'a> {
    /// Pre-computed lowercase title.
    pub title_lower: String,
    /// Pre-computed lowercase description (if present).
    pub description_lower: Option<String>,
    /// Pre-computed lowercase tags.
    pub tags_lower: Vec<String>,
    /// Reference to the original task for non-text fields.
    pub task: &'a Task,
}

impl<'a> TaskLowerCache<'a> {
    /// Create a new lowercase cache for a task.
    #[must_use]
    pub fn new(task: &'a Task) -> Self {
        Self {
            title_lower: task.title.to_lowercase(),
            description_lower: task.description.as_ref().map(|d| d.to_lowercase()),
            tags_lower: task.tags.iter().map(|t| t.to_lowercase()).collect(),
            task,
        }
    }
}

/// Context for evaluating filter expressions.
///
/// Provides access to projects for name lookups and the current date
/// for relative date comparisons.
pub struct EvalContext<'a> {
    /// Map of project IDs to projects (for project name lookups).
    pub projects: &'a HashMap<ProjectId, Project>,
    /// Today's date (for due date comparisons).
    pub today: NaiveDate,
}

impl<'a> EvalContext<'a> {
    /// Create a new evaluation context.
    #[must_use]
    pub fn new(projects: &'a HashMap<ProjectId, Project>) -> Self {
        Self {
            projects,
            today: Utc::now().date_naive(),
        }
    }

    /// Create an evaluation context with a specific date (for testing).
    #[cfg(test)]
    #[must_use]
    pub fn with_date(projects: &'a HashMap<ProjectId, Project>, today: NaiveDate) -> Self {
        Self { projects, today }
    }
}

/// Evaluate a filter expression against a task.
///
/// Returns `true` if the task matches the filter expression.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use taskflow::domain::{Task, Priority};
/// use taskflow::domain::filter_dsl::{parse, evaluate, EvalContext};
///
/// let task = Task::new("Bug fix").with_priority(Priority::High);
/// let expr = parse("priority:high").unwrap();
/// let projects = HashMap::new();
/// let ctx = EvalContext::new(&projects);
///
/// assert!(evaluate(&expr, &task, &ctx));
/// ```
#[must_use]
pub fn evaluate(expr: &FilterExpr, task: &Task, ctx: &EvalContext<'_>) -> bool {
    match expr {
        FilterExpr::And(left, right) => evaluate(left, task, ctx) && evaluate(right, task, ctx),
        FilterExpr::Or(left, right) => evaluate(left, task, ctx) || evaluate(right, task, ctx),
        FilterExpr::Not(inner) => !evaluate(inner, task, ctx),
        FilterExpr::Condition(cond) => evaluate_condition(cond, task, ctx),
    }
}

/// Evaluate a filter expression against a task with pre-computed lowercase cache.
///
/// More efficient than [`evaluate()`] when the filter contains text-matching
/// conditions (tags, search, title). The cache avoids repeated `to_lowercase()`
/// allocations during evaluation.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use taskflow::domain::{Task, Priority};
/// use taskflow::domain::filter_dsl::{parse, evaluate_with_cache, EvalContext, TaskLowerCache};
///
/// let task = Task::new("Bug fix").with_priority(Priority::High);
/// let cache = TaskLowerCache::new(&task);
/// let expr = parse("priority:high AND title:bug").unwrap();
/// let projects = HashMap::new();
/// let ctx = EvalContext::new(&projects);
///
/// assert!(evaluate_with_cache(&expr, &cache, &ctx));
/// ```
#[must_use]
pub fn evaluate_with_cache(
    expr: &FilterExpr,
    cache: &TaskLowerCache<'_>,
    ctx: &EvalContext<'_>,
) -> bool {
    match expr {
        FilterExpr::And(left, right) => {
            evaluate_with_cache(left, cache, ctx) && evaluate_with_cache(right, cache, ctx)
        }
        FilterExpr::Or(left, right) => {
            evaluate_with_cache(left, cache, ctx) || evaluate_with_cache(right, cache, ctx)
        }
        FilterExpr::Not(inner) => !evaluate_with_cache(inner, cache, ctx),
        FilterExpr::Condition(cond) => evaluate_condition_cached(cond, cache, ctx),
    }
}

/// Evaluate a single condition against a task.
fn evaluate_condition(cond: &Condition, task: &Task, ctx: &EvalContext<'_>) -> bool {
    match &cond.value {
        FilterValue::Priority(priority) => task.priority == *priority,

        FilterValue::Status(status) => task.status == *status,

        FilterValue::Tag(tag) => {
            let tag_lower = tag.to_lowercase();
            task.tags.iter().any(|t| t.to_lowercase() == tag_lower)
        }

        FilterValue::ProjectName(name) => {
            let name_lower = name.to_lowercase();
            task.project_id
                .and_then(|pid| ctx.projects.get(&pid))
                .is_some_and(|p| p.name.to_lowercase().contains(&name_lower))
        }

        FilterValue::Due(due_filter) => evaluate_due(due_filter, task, ctx.today),

        FilterValue::SearchText(query) => {
            let query_lower = query.to_lowercase();
            task.title.to_lowercase().contains(&query_lower)
                || task
                    .description
                    .as_ref()
                    .is_some_and(|d| d.to_lowercase().contains(&query_lower))
        }

        FilterValue::Has(has_field) => evaluate_has(*has_field, task),

        FilterValue::TitleText(text) => {
            let text_lower = text.to_lowercase();
            task.title.to_lowercase().contains(&text_lower)
        }

        FilterValue::Created(created_filter) => evaluate_created(created_filter, task, ctx.today),

        FilterValue::Scheduled(scheduled_filter) => {
            evaluate_scheduled(scheduled_filter, task, ctx.today)
        }

        FilterValue::Completed(completed_filter) => {
            evaluate_completed(completed_filter, task, ctx.today)
        }

        FilterValue::Modified(modified_filter) => {
            evaluate_modified(modified_filter, task, ctx.today)
        }

        FilterValue::Estimate(numeric_filter) => {
            evaluate_numeric(numeric_filter, task.estimated_minutes)
        }

        FilterValue::Actual(numeric_filter) => {
            evaluate_numeric(numeric_filter, Some(task.actual_minutes))
        }
    }
}

/// Evaluate a single condition using cached lowercase data.
///
/// Uses pre-computed lowercase strings from the cache for text fields,
/// avoiding repeated allocations.
fn evaluate_condition_cached(
    cond: &Condition,
    cache: &TaskLowerCache<'_>,
    ctx: &EvalContext<'_>,
) -> bool {
    let task = cache.task;

    match &cond.value {
        // Non-text fields: delegate directly to task
        FilterValue::Priority(priority) => task.priority == *priority,
        FilterValue::Status(status) => task.status == *status,
        FilterValue::Due(due_filter) => evaluate_due(due_filter, task, ctx.today),
        FilterValue::Has(has_field) => evaluate_has(*has_field, task),
        FilterValue::Created(created_filter) => evaluate_created(created_filter, task, ctx.today),
        FilterValue::Scheduled(scheduled_filter) => {
            evaluate_scheduled(scheduled_filter, task, ctx.today)
        }
        FilterValue::Completed(completed_filter) => {
            evaluate_completed(completed_filter, task, ctx.today)
        }
        FilterValue::Modified(modified_filter) => {
            evaluate_modified(modified_filter, task, ctx.today)
        }
        FilterValue::Estimate(numeric_filter) => {
            evaluate_numeric(numeric_filter, task.estimated_minutes)
        }
        FilterValue::Actual(numeric_filter) => {
            evaluate_numeric(numeric_filter, Some(task.actual_minutes))
        }

        // Text fields: use cached lowercase values
        FilterValue::Tag(tag) => {
            let tag_lower = tag.to_lowercase();
            cache.tags_lower.iter().any(|t| t == &tag_lower)
        }

        FilterValue::SearchText(query) => {
            let query_lower = query.to_lowercase();
            cache.title_lower.contains(&query_lower)
                || cache
                    .description_lower
                    .as_ref()
                    .is_some_and(|d| d.contains(&query_lower))
        }

        FilterValue::TitleText(text) => {
            let text_lower = text.to_lowercase();
            cache.title_lower.contains(&text_lower)
        }

        FilterValue::ProjectName(name) => {
            let name_lower = name.to_lowercase();
            task.project_id
                .and_then(|pid| ctx.projects.get(&pid))
                .is_some_and(|p| p.name.to_lowercase().contains(&name_lower))
        }
    }
}

/// Evaluate a due date filter against a task.
fn evaluate_due(filter: &DueFilter, task: &Task, today: NaiveDate) -> bool {
    match filter {
        DueFilter::Today => task.due_date == Some(today),

        DueFilter::Tomorrow => {
            let tomorrow = today.succ_opt().unwrap_or(today);
            task.due_date == Some(tomorrow)
        }

        DueFilter::ThisWeek => {
            // Monday to Sunday of current week
            let days_since_monday = today.weekday().num_days_from_monday();
            let week_start = today - chrono::Duration::days(days_since_monday.into());
            let week_end = week_start + chrono::Duration::days(6);
            task.due_date
                .is_some_and(|d| d >= week_start && d <= week_end)
        }

        DueFilter::NextWeek => {
            let days_since_monday = today.weekday().num_days_from_monday();
            let this_monday = today - chrono::Duration::days(days_since_monday.into());
            let next_monday = this_monday + chrono::Duration::days(7);
            let next_sunday = next_monday + chrono::Duration::days(6);
            task.due_date
                .is_some_and(|d| d >= next_monday && d <= next_sunday)
        }

        DueFilter::Overdue => {
            task.due_date.is_some_and(|d| d < today) && !task.status.is_complete()
        }

        DueFilter::None => task.due_date.is_none(),

        DueFilter::On(date) => task.due_date == Some(*date),

        DueFilter::Before(date) => task.due_date.is_some_and(|d| d < *date),

        DueFilter::After(date) => task.due_date.is_some_and(|d| d > *date),

        DueFilter::Between(start, end) => task.due_date.is_some_and(|d| d >= *start && d <= *end),

        DueFilter::OnOrAfter(date) => task.due_date.is_some_and(|d| d >= *date),

        DueFilter::OnOrBefore(date) => task.due_date.is_some_and(|d| d <= *date),
    }
}

/// Evaluate a creation date filter against a task.
fn evaluate_created(filter: &CreatedFilter, task: &Task, today: NaiveDate) -> bool {
    let created_date = task.created_at.date_naive();

    match filter {
        CreatedFilter::Today => created_date == today,

        CreatedFilter::Yesterday => {
            let yesterday = today.pred_opt().unwrap_or(today);
            created_date == yesterday
        }

        CreatedFilter::ThisWeek => {
            // Monday to Sunday of current week
            let days_since_monday = today.weekday().num_days_from_monday();
            let week_start = today - chrono::Duration::days(days_since_monday.into());
            let week_end = week_start + chrono::Duration::days(6);
            created_date >= week_start && created_date <= week_end
        }

        CreatedFilter::LastWeek => {
            let days_since_monday = today.weekday().num_days_from_monday();
            let this_monday = today - chrono::Duration::days(days_since_monday.into());
            let last_monday = this_monday - chrono::Duration::days(7);
            let last_sunday = last_monday + chrono::Duration::days(6);
            created_date >= last_monday && created_date <= last_sunday
        }

        CreatedFilter::On(date) => created_date == *date,

        CreatedFilter::Before(date) => created_date < *date,

        CreatedFilter::After(date) => created_date > *date,

        CreatedFilter::Between(start, end) => created_date >= *start && created_date <= *end,

        CreatedFilter::OnOrAfter(date) => created_date >= *date,

        CreatedFilter::OnOrBefore(date) => created_date <= *date,
    }
}

/// Evaluate a "has" field condition.
fn evaluate_has(field: HasField, task: &Task) -> bool {
    match field {
        HasField::Due => task.due_date.is_some(),
        HasField::Project => task.project_id.is_some(),
        HasField::Tags => !task.tags.is_empty(),
        HasField::Estimate => task.estimated_minutes.is_some(),
        HasField::Description => task.description.is_some(),
        HasField::Recurrence => task.recurrence.is_some(),
        HasField::Scheduled => task.scheduled_date.is_some(),
        HasField::Dependencies => !task.dependencies.is_empty(),
        HasField::Parent => task.parent_task_id.is_some(),
        HasField::Tracked => task.actual_minutes > 0,
    }
}

/// Evaluate a scheduled date filter against a task.
fn evaluate_scheduled(filter: &ScheduledFilter, task: &Task, today: NaiveDate) -> bool {
    match filter {
        ScheduledFilter::Today => task.scheduled_date == Some(today),

        ScheduledFilter::Tomorrow => {
            let tomorrow = today.succ_opt().unwrap_or(today);
            task.scheduled_date == Some(tomorrow)
        }

        ScheduledFilter::ThisWeek => {
            let days_since_monday = today.weekday().num_days_from_monday();
            let week_start = today - chrono::Duration::days(days_since_monday.into());
            let week_end = week_start + chrono::Duration::days(6);
            task.scheduled_date
                .is_some_and(|d| d >= week_start && d <= week_end)
        }

        ScheduledFilter::NextWeek => {
            let days_since_monday = today.weekday().num_days_from_monday();
            let this_monday = today - chrono::Duration::days(days_since_monday.into());
            let next_monday = this_monday + chrono::Duration::days(7);
            let next_sunday = next_monday + chrono::Duration::days(6);
            task.scheduled_date
                .is_some_and(|d| d >= next_monday && d <= next_sunday)
        }

        ScheduledFilter::None => task.scheduled_date.is_none(),

        ScheduledFilter::On(date) => task.scheduled_date == Some(*date),

        ScheduledFilter::Before(date) => task.scheduled_date.is_some_and(|d| d < *date),

        ScheduledFilter::After(date) => task.scheduled_date.is_some_and(|d| d > *date),

        ScheduledFilter::Between(start, end) => task
            .scheduled_date
            .is_some_and(|d| d >= *start && d <= *end),

        ScheduledFilter::OnOrAfter(date) => task.scheduled_date.is_some_and(|d| d >= *date),

        ScheduledFilter::OnOrBefore(date) => task.scheduled_date.is_some_and(|d| d <= *date),
    }
}

/// Evaluate a completion date filter against a task.
fn evaluate_completed(filter: &CreatedFilter, task: &Task, today: NaiveDate) -> bool {
    let Some(completed_at) = task.completed_at else {
        return false;
    };
    let completed_date = completed_at.date_naive();

    match filter {
        CreatedFilter::Today => completed_date == today,

        CreatedFilter::Yesterday => {
            let yesterday = today.pred_opt().unwrap_or(today);
            completed_date == yesterday
        }

        CreatedFilter::ThisWeek => {
            let days_since_monday = today.weekday().num_days_from_monday();
            let week_start = today - chrono::Duration::days(days_since_monday.into());
            let week_end = week_start + chrono::Duration::days(6);
            completed_date >= week_start && completed_date <= week_end
        }

        CreatedFilter::LastWeek => {
            let days_since_monday = today.weekday().num_days_from_monday();
            let this_monday = today - chrono::Duration::days(days_since_monday.into());
            let last_monday = this_monday - chrono::Duration::days(7);
            let last_sunday = last_monday + chrono::Duration::days(6);
            completed_date >= last_monday && completed_date <= last_sunday
        }

        CreatedFilter::On(date) => completed_date == *date,

        CreatedFilter::Before(date) => completed_date < *date,

        CreatedFilter::After(date) => completed_date > *date,

        CreatedFilter::Between(start, end) => completed_date >= *start && completed_date <= *end,

        CreatedFilter::OnOrAfter(date) => completed_date >= *date,

        CreatedFilter::OnOrBefore(date) => completed_date <= *date,
    }
}

/// Evaluate a modification date filter against a task.
fn evaluate_modified(filter: &CreatedFilter, task: &Task, today: NaiveDate) -> bool {
    let modified_date = task.updated_at.date_naive();

    match filter {
        CreatedFilter::Today => modified_date == today,

        CreatedFilter::Yesterday => {
            let yesterday = today.pred_opt().unwrap_or(today);
            modified_date == yesterday
        }

        CreatedFilter::ThisWeek => {
            let days_since_monday = today.weekday().num_days_from_monday();
            let week_start = today - chrono::Duration::days(days_since_monday.into());
            let week_end = week_start + chrono::Duration::days(6);
            modified_date >= week_start && modified_date <= week_end
        }

        CreatedFilter::LastWeek => {
            let days_since_monday = today.weekday().num_days_from_monday();
            let this_monday = today - chrono::Duration::days(days_since_monday.into());
            let last_monday = this_monday - chrono::Duration::days(7);
            let last_sunday = last_monday + chrono::Duration::days(6);
            modified_date >= last_monday && modified_date <= last_sunday
        }

        CreatedFilter::On(date) => modified_date == *date,

        CreatedFilter::Before(date) => modified_date < *date,

        CreatedFilter::After(date) => modified_date > *date,

        CreatedFilter::Between(start, end) => modified_date >= *start && modified_date <= *end,

        CreatedFilter::OnOrAfter(date) => modified_date >= *date,

        CreatedFilter::OnOrBefore(date) => modified_date <= *date,
    }
}

/// Evaluate a numeric filter against an optional value.
fn evaluate_numeric(filter: &NumericFilter, value: Option<u32>) -> bool {
    match filter {
        NumericFilter::None => value.is_none(),
        NumericFilter::Equals(n) => value == Some(*n),
        NumericFilter::GreaterThan(n) => value.is_some_and(|v| v > *n),
        NumericFilter::LessThan(n) => value.is_some_and(|v| v < *n),
        NumericFilter::GreaterOrEqual(n) => value.is_some_and(|v| v >= *n),
        NumericFilter::LessOrEqual(n) => value.is_some_and(|v| v <= *n),
        NumericFilter::Between(min, max) => value.is_some_and(|v| v >= *min && v <= *max),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Priority, TaskStatus};

    fn empty_ctx() -> EvalContext<'static> {
        static EMPTY: std::sync::LazyLock<HashMap<ProjectId, Project>> =
            std::sync::LazyLock::new(HashMap::new);
        EvalContext::new(&EMPTY)
    }

    fn ctx_with_date(date: NaiveDate) -> EvalContext<'static> {
        static EMPTY: std::sync::LazyLock<HashMap<ProjectId, Project>> =
            std::sync::LazyLock::new(HashMap::new);
        EvalContext::with_date(&EMPTY, date)
    }

    #[test]
    fn test_eval_priority() {
        let task = Task::new("Test").with_priority(Priority::High);
        let ctx = empty_ctx();

        let expr = crate::domain::filter_dsl::parse("priority:high").unwrap();
        assert!(evaluate(&expr, &task, &ctx));

        let expr = crate::domain::filter_dsl::parse("priority:low").unwrap();
        assert!(!evaluate(&expr, &task, &ctx));
    }

    #[test]
    fn test_eval_status() {
        let mut task = Task::new("Test");
        task.status = TaskStatus::InProgress;
        let ctx = empty_ctx();

        let expr = crate::domain::filter_dsl::parse("status:in_progress").unwrap();
        assert!(evaluate(&expr, &task, &ctx));

        let expr = crate::domain::filter_dsl::parse("status:done").unwrap();
        assert!(!evaluate(&expr, &task, &ctx));
    }

    #[test]
    fn test_eval_tags() {
        let task = Task::new("Test").with_tags(vec!["bug".to_string(), "urgent".to_string()]);
        let ctx = empty_ctx();

        let expr = crate::domain::filter_dsl::parse("tags:bug").unwrap();
        assert!(evaluate(&expr, &task, &ctx));

        let expr = crate::domain::filter_dsl::parse("tags:BUG").unwrap(); // case insensitive
        assert!(evaluate(&expr, &task, &ctx));

        let expr = crate::domain::filter_dsl::parse("tags:feature").unwrap();
        assert!(!evaluate(&expr, &task, &ctx));
    }

    #[test]
    fn test_eval_search() {
        let task = Task::new("Fix login bug").with_description("Users cannot log in".to_string());
        let ctx = empty_ctx();

        // Search in title
        let expr = crate::domain::filter_dsl::parse(r#"search:"login""#).unwrap();
        assert!(evaluate(&expr, &task, &ctx));

        // Search in description
        let expr = crate::domain::filter_dsl::parse(r#"search:"users""#).unwrap();
        assert!(evaluate(&expr, &task, &ctx));

        // Not found
        let expr = crate::domain::filter_dsl::parse(r#"search:"password""#).unwrap();
        assert!(!evaluate(&expr, &task, &ctx));
    }

    #[test]
    fn test_eval_title() {
        let task = Task::new("Refactor authentication module");
        let ctx = empty_ctx();

        let expr = crate::domain::filter_dsl::parse("title:refactor").unwrap();
        assert!(evaluate(&expr, &task, &ctx));

        let expr = crate::domain::filter_dsl::parse("title:auth").unwrap();
        assert!(evaluate(&expr, &task, &ctx));

        let expr = crate::domain::filter_dsl::parse("title:database").unwrap();
        assert!(!evaluate(&expr, &task, &ctx));
    }

    #[test]
    fn test_eval_has_due() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

        let task_with_due = Task::new("Test").with_due_date(today);
        let task_without_due = Task::new("Test");
        let ctx = ctx_with_date(today);

        let expr = crate::domain::filter_dsl::parse("has:due").unwrap();
        assert!(evaluate(&expr, &task_with_due, &ctx));
        assert!(!evaluate(&expr, &task_without_due, &ctx));
    }

    #[test]
    fn test_eval_has_project() {
        let task_with_project = {
            let mut t = Task::new("Test");
            t.project_id = Some(ProjectId::new());
            t
        };
        let task_without_project = Task::new("Test");
        let ctx = empty_ctx();

        let expr = crate::domain::filter_dsl::parse("has:project").unwrap();
        assert!(evaluate(&expr, &task_with_project, &ctx));
        assert!(!evaluate(&expr, &task_without_project, &ctx));
    }

    #[test]
    fn test_eval_has_tags() {
        let task_with_tags = Task::new("Test").with_tags(vec!["bug".to_string()]);
        let task_without_tags = Task::new("Test");
        let ctx = empty_ctx();

        let expr = crate::domain::filter_dsl::parse("has:tags").unwrap();
        assert!(evaluate(&expr, &task_with_tags, &ctx));
        assert!(!evaluate(&expr, &task_without_tags, &ctx));
    }

    #[test]
    fn test_eval_has_estimate() {
        let task_with_estimate = {
            let mut t = Task::new("Test");
            t.estimated_minutes = Some(60);
            t
        };
        let task_without_estimate = Task::new("Test");
        let ctx = empty_ctx();

        let expr = crate::domain::filter_dsl::parse("has:estimate").unwrap();
        assert!(evaluate(&expr, &task_with_estimate, &ctx));
        assert!(!evaluate(&expr, &task_without_estimate, &ctx));
    }

    #[test]
    fn test_eval_has_description() {
        let task_with_desc = Task::new("Test").with_description("Details".to_string());
        let task_without_desc = Task::new("Test");
        let ctx = empty_ctx();

        let expr = crate::domain::filter_dsl::parse("has:description").unwrap();
        assert!(evaluate(&expr, &task_with_desc, &ctx));
        assert!(!evaluate(&expr, &task_without_desc, &ctx));
    }

    #[test]
    fn test_eval_due_today() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let ctx = ctx_with_date(today);

        let task_due_today = Task::new("Test").with_due_date(today);
        let task_due_tomorrow = Task::new("Test").with_due_date(today.succ_opt().unwrap());

        let expr = crate::domain::filter_dsl::parse("due:today").unwrap();
        assert!(evaluate(&expr, &task_due_today, &ctx));
        assert!(!evaluate(&expr, &task_due_tomorrow, &ctx));
    }

    #[test]
    fn test_eval_due_tomorrow() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let tomorrow = today.succ_opt().unwrap();
        let ctx = ctx_with_date(today);

        let task_due_tomorrow = Task::new("Test").with_due_date(tomorrow);
        let task_due_today = Task::new("Test").with_due_date(today);

        let expr = crate::domain::filter_dsl::parse("due:tomorrow").unwrap();
        assert!(evaluate(&expr, &task_due_tomorrow, &ctx));
        assert!(!evaluate(&expr, &task_due_today, &ctx));
    }

    #[test]
    fn test_eval_due_overdue() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let yesterday = today.pred_opt().unwrap();
        let ctx = ctx_with_date(today);

        let mut task_overdue = Task::new("Test");
        task_overdue.due_date = Some(yesterday);
        task_overdue.status = TaskStatus::Todo;

        let mut task_overdue_but_done = Task::new("Test");
        task_overdue_but_done.due_date = Some(yesterday);
        task_overdue_but_done.status = TaskStatus::Done;

        let expr = crate::domain::filter_dsl::parse("due:overdue").unwrap();
        assert!(evaluate(&expr, &task_overdue, &ctx));
        assert!(!evaluate(&expr, &task_overdue_but_done, &ctx)); // Done tasks aren't overdue
    }

    #[test]
    fn test_eval_due_none() {
        let task_no_due = Task::new("Test");
        let task_with_due =
            Task::new("Test").with_due_date(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());
        let ctx = empty_ctx();

        let expr = crate::domain::filter_dsl::parse("due:none").unwrap();
        assert!(evaluate(&expr, &task_no_due, &ctx));
        assert!(!evaluate(&expr, &task_with_due, &ctx));
    }

    #[test]
    fn test_eval_due_thisweek() {
        // Wednesday, June 18, 2025
        let today = NaiveDate::from_ymd_opt(2025, 6, 18).unwrap();
        let ctx = ctx_with_date(today);

        // This week is Mon June 16 - Sun June 22
        let monday = NaiveDate::from_ymd_opt(2025, 6, 16).unwrap();
        let sunday = NaiveDate::from_ymd_opt(2025, 6, 22).unwrap();
        let next_monday = NaiveDate::from_ymd_opt(2025, 6, 23).unwrap();

        let task_monday = Task::new("Test").with_due_date(monday);
        let task_sunday = Task::new("Test").with_due_date(sunday);
        let task_next_monday = Task::new("Test").with_due_date(next_monday);

        let expr = crate::domain::filter_dsl::parse("due:thisweek").unwrap();
        assert!(evaluate(&expr, &task_monday, &ctx));
        assert!(evaluate(&expr, &task_sunday, &ctx));
        assert!(!evaluate(&expr, &task_next_monday, &ctx));
    }

    #[test]
    fn test_eval_and() {
        let task = Task::new("Bug fix")
            .with_priority(Priority::High)
            .with_tags(vec!["bug".to_string()]);
        let ctx = empty_ctx();

        let expr = crate::domain::filter_dsl::parse("priority:high AND tags:bug").unwrap();
        assert!(evaluate(&expr, &task, &ctx));

        let expr = crate::domain::filter_dsl::parse("priority:high AND tags:feature").unwrap();
        assert!(!evaluate(&expr, &task, &ctx));

        let expr = crate::domain::filter_dsl::parse("priority:low AND tags:bug").unwrap();
        assert!(!evaluate(&expr, &task, &ctx));
    }

    #[test]
    fn test_eval_or() {
        let task = Task::new("Bug fix").with_priority(Priority::High);
        let ctx = empty_ctx();

        let expr = crate::domain::filter_dsl::parse("priority:high OR priority:urgent").unwrap();
        assert!(evaluate(&expr, &task, &ctx));

        let expr = crate::domain::filter_dsl::parse("priority:low OR priority:high").unwrap();
        assert!(evaluate(&expr, &task, &ctx));

        let expr = crate::domain::filter_dsl::parse("priority:low OR priority:medium").unwrap();
        assert!(!evaluate(&expr, &task, &ctx));
    }

    #[test]
    fn test_eval_not() {
        let mut task = Task::new("In progress");
        task.status = TaskStatus::InProgress;
        let ctx = empty_ctx();

        let expr = crate::domain::filter_dsl::parse("!status:done").unwrap();
        assert!(evaluate(&expr, &task, &ctx));

        let expr = crate::domain::filter_dsl::parse("!status:in_progress").unwrap();
        assert!(!evaluate(&expr, &task, &ctx));
    }

    #[test]
    fn test_eval_complex_expression() {
        let task = Task::new("Urgent bug")
            .with_priority(Priority::Urgent)
            .with_tags(vec!["bug".to_string()]);
        let mut task2 = task.clone();
        task2.status = TaskStatus::Done;

        let ctx = empty_ctx();

        // (priority:high OR priority:urgent) AND !status:done AND tags:bug
        let expr = crate::domain::filter_dsl::parse(
            "(priority:high OR priority:urgent) AND !status:done AND tags:bug",
        )
        .unwrap();

        assert!(evaluate(&expr, &task, &ctx));
        assert!(!evaluate(&expr, &task2, &ctx)); // Done, so !status:done fails
    }

    #[test]
    fn test_eval_project_name() {
        let project = Project::new("Backend Services");
        let project_id = project.id;

        let mut projects = HashMap::new();
        projects.insert(project_id, project);

        let mut task_with_project = Task::new("Test");
        task_with_project.project_id = Some(project_id);

        let task_without_project = Task::new("Test");

        let ctx = EvalContext::new(&projects);

        // Partial match
        let expr = crate::domain::filter_dsl::parse("project:backend").unwrap();
        assert!(evaluate(&expr, &task_with_project, &ctx));
        assert!(!evaluate(&expr, &task_without_project, &ctx));

        // Case insensitive
        let expr = crate::domain::filter_dsl::parse("project:BACKEND").unwrap();
        assert!(evaluate(&expr, &task_with_project, &ctx));

        // Full name
        let expr = crate::domain::filter_dsl::parse(r#"project:"Backend Services""#).unwrap();
        assert!(evaluate(&expr, &task_with_project, &ctx));

        // No match
        let expr = crate::domain::filter_dsl::parse("project:frontend").unwrap();
        assert!(!evaluate(&expr, &task_with_project, &ctx));
    }

    #[test]
    fn test_eval_due_before() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let ctx = ctx_with_date(today);

        let task_before =
            Task::new("Test").with_due_date(NaiveDate::from_ymd_opt(2025, 6, 10).unwrap());
        let task_on =
            Task::new("Test").with_due_date(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());
        let task_after =
            Task::new("Test").with_due_date(NaiveDate::from_ymd_opt(2025, 6, 20).unwrap());

        let expr = crate::domain::filter_dsl::parse("due:<2025-06-15").unwrap();
        assert!(evaluate(&expr, &task_before, &ctx));
        assert!(!evaluate(&expr, &task_on, &ctx)); // Not strictly before
        assert!(!evaluate(&expr, &task_after, &ctx));
    }

    #[test]
    fn test_eval_due_after() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let ctx = ctx_with_date(today);

        let task_before =
            Task::new("Test").with_due_date(NaiveDate::from_ymd_opt(2025, 6, 10).unwrap());
        let task_on =
            Task::new("Test").with_due_date(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());
        let task_after =
            Task::new("Test").with_due_date(NaiveDate::from_ymd_opt(2025, 6, 20).unwrap());

        let expr = crate::domain::filter_dsl::parse("due:>2025-06-15").unwrap();
        assert!(!evaluate(&expr, &task_before, &ctx));
        assert!(!evaluate(&expr, &task_on, &ctx)); // Not strictly after
        assert!(evaluate(&expr, &task_after, &ctx));
    }

    // Helper to create a task with a specific creation date
    fn task_created_on(date: NaiveDate) -> Task {
        use chrono::TimeZone;
        let mut task = Task::new("Test");
        task.created_at = Utc.from_utc_datetime(&date.and_hms_opt(12, 0, 0).unwrap());
        task
    }

    #[test]
    fn test_eval_created_today() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let ctx = ctx_with_date(today);

        let task_today = task_created_on(today);
        let task_yesterday = task_created_on(today.pred_opt().unwrap());

        let expr = crate::domain::filter_dsl::parse("created:today").unwrap();
        assert!(evaluate(&expr, &task_today, &ctx));
        assert!(!evaluate(&expr, &task_yesterday, &ctx));
    }

    #[test]
    fn test_eval_created_yesterday() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let yesterday = today.pred_opt().unwrap();
        let ctx = ctx_with_date(today);

        let task_today = task_created_on(today);
        let task_yesterday = task_created_on(yesterday);

        let expr = crate::domain::filter_dsl::parse("created:yesterday").unwrap();
        assert!(!evaluate(&expr, &task_today, &ctx));
        assert!(evaluate(&expr, &task_yesterday, &ctx));
    }

    #[test]
    fn test_eval_created_thisweek() {
        // Wednesday, June 18, 2025
        let today = NaiveDate::from_ymd_opt(2025, 6, 18).unwrap();
        let ctx = ctx_with_date(today);

        // This week is Mon June 16 - Sun June 22
        let monday = NaiveDate::from_ymd_opt(2025, 6, 16).unwrap();
        let sunday = NaiveDate::from_ymd_opt(2025, 6, 22).unwrap();
        let last_sunday = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

        let task_monday = task_created_on(monday);
        let task_sunday = task_created_on(sunday);
        let task_last_sunday = task_created_on(last_sunday);

        let expr = crate::domain::filter_dsl::parse("created:thisweek").unwrap();
        assert!(evaluate(&expr, &task_monday, &ctx));
        assert!(evaluate(&expr, &task_sunday, &ctx));
        assert!(!evaluate(&expr, &task_last_sunday, &ctx));
    }

    #[test]
    fn test_eval_created_lastweek() {
        // Wednesday, June 18, 2025
        let today = NaiveDate::from_ymd_opt(2025, 6, 18).unwrap();
        let ctx = ctx_with_date(today);

        // Last week is Mon June 9 - Sun June 15
        let last_monday = NaiveDate::from_ymd_opt(2025, 6, 9).unwrap();
        let last_sunday = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let this_monday = NaiveDate::from_ymd_opt(2025, 6, 16).unwrap();

        let task_last_monday = task_created_on(last_monday);
        let task_last_sunday = task_created_on(last_sunday);
        let task_this_monday = task_created_on(this_monday);

        let expr = crate::domain::filter_dsl::parse("created:lastweek").unwrap();
        assert!(evaluate(&expr, &task_last_monday, &ctx));
        assert!(evaluate(&expr, &task_last_sunday, &ctx));
        assert!(!evaluate(&expr, &task_this_monday, &ctx));
    }

    #[test]
    fn test_eval_created_before() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let ctx = ctx_with_date(today);

        let task_before = task_created_on(NaiveDate::from_ymd_opt(2025, 6, 10).unwrap());
        let task_on = task_created_on(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());
        let task_after = task_created_on(NaiveDate::from_ymd_opt(2025, 6, 20).unwrap());

        let expr = crate::domain::filter_dsl::parse("created:<2025-06-15").unwrap();
        assert!(evaluate(&expr, &task_before, &ctx));
        assert!(!evaluate(&expr, &task_on, &ctx)); // Not strictly before
        assert!(!evaluate(&expr, &task_after, &ctx));
    }

    #[test]
    fn test_eval_created_after() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let ctx = ctx_with_date(today);

        let task_before = task_created_on(NaiveDate::from_ymd_opt(2025, 6, 10).unwrap());
        let task_on = task_created_on(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());
        let task_after = task_created_on(NaiveDate::from_ymd_opt(2025, 6, 20).unwrap());

        let expr = crate::domain::filter_dsl::parse("created:>2025-06-15").unwrap();
        assert!(!evaluate(&expr, &task_before, &ctx));
        assert!(!evaluate(&expr, &task_on, &ctx)); // Not strictly after
        assert!(evaluate(&expr, &task_after, &ctx));
    }

    // ========================================================================
    // Range evaluation tests
    // ========================================================================

    #[test]
    fn test_eval_due_date_range() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let ctx = ctx_with_date(today);

        let start = NaiveDate::from_ymd_opt(2025, 6, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 6, 30).unwrap();

        // Task within range
        let task_in =
            Task::new("Test").with_due_date(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());
        let expr = crate::domain::filter_dsl::parse("due:2025-06-01..2025-06-30").unwrap();
        assert!(evaluate(&expr, &task_in, &ctx));

        // Task before range
        let task_before =
            Task::new("Test").with_due_date(NaiveDate::from_ymd_opt(2025, 5, 15).unwrap());
        assert!(!evaluate(&expr, &task_before, &ctx));

        // Task after range
        let task_after =
            Task::new("Test").with_due_date(NaiveDate::from_ymd_opt(2025, 7, 15).unwrap());
        assert!(!evaluate(&expr, &task_after, &ctx));

        // Task on start boundary (inclusive)
        let task_start = Task::new("Test").with_due_date(start);
        assert!(evaluate(&expr, &task_start, &ctx));

        // Task on end boundary (inclusive)
        let task_end = Task::new("Test").with_due_date(end);
        assert!(evaluate(&expr, &task_end, &ctx));
    }

    #[test]
    fn test_eval_due_on_or_after() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let ctx = ctx_with_date(today);

        let boundary = NaiveDate::from_ymd_opt(2025, 6, 10).unwrap();

        // Task on boundary (inclusive)
        let task_on = Task::new("Test").with_due_date(boundary);
        let expr = crate::domain::filter_dsl::parse("due:2025-06-10..").unwrap();
        assert!(evaluate(&expr, &task_on, &ctx));

        // Task after boundary
        let task_after =
            Task::new("Test").with_due_date(NaiveDate::from_ymd_opt(2025, 6, 20).unwrap());
        assert!(evaluate(&expr, &task_after, &ctx));

        // Task before boundary
        let task_before =
            Task::new("Test").with_due_date(NaiveDate::from_ymd_opt(2025, 6, 5).unwrap());
        assert!(!evaluate(&expr, &task_before, &ctx));
    }

    #[test]
    fn test_eval_due_on_or_before() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let ctx = ctx_with_date(today);

        let boundary = NaiveDate::from_ymd_opt(2025, 6, 20).unwrap();

        // Task on boundary (inclusive)
        let task_on = Task::new("Test").with_due_date(boundary);
        let expr = crate::domain::filter_dsl::parse("due:..2025-06-20").unwrap();
        assert!(evaluate(&expr, &task_on, &ctx));

        // Task before boundary
        let task_before =
            Task::new("Test").with_due_date(NaiveDate::from_ymd_opt(2025, 6, 10).unwrap());
        assert!(evaluate(&expr, &task_before, &ctx));

        // Task after boundary
        let task_after =
            Task::new("Test").with_due_date(NaiveDate::from_ymd_opt(2025, 6, 25).unwrap());
        assert!(!evaluate(&expr, &task_after, &ctx));
    }

    #[test]
    fn test_eval_created_date_range() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let ctx = ctx_with_date(today);

        let start = NaiveDate::from_ymd_opt(2025, 6, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 6, 30).unwrap();

        // Task within range
        let task_in = task_created_on(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());
        let expr = crate::domain::filter_dsl::parse("created:2025-06-01..2025-06-30").unwrap();
        assert!(evaluate(&expr, &task_in, &ctx));

        // Task before range
        let task_before = task_created_on(NaiveDate::from_ymd_opt(2025, 5, 15).unwrap());
        assert!(!evaluate(&expr, &task_before, &ctx));

        // Task on boundaries (inclusive)
        let task_start = task_created_on(start);
        let task_end = task_created_on(end);
        assert!(evaluate(&expr, &task_start, &ctx));
        assert!(evaluate(&expr, &task_end, &ctx));
    }

    #[test]
    fn test_eval_created_on_or_after() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let ctx = ctx_with_date(today);

        let boundary = NaiveDate::from_ymd_opt(2025, 6, 10).unwrap();

        let task_on = task_created_on(boundary);
        let task_after = task_created_on(NaiveDate::from_ymd_opt(2025, 6, 20).unwrap());
        let task_before = task_created_on(NaiveDate::from_ymd_opt(2025, 6, 5).unwrap());

        let expr = crate::domain::filter_dsl::parse("created:2025-06-10..").unwrap();
        assert!(evaluate(&expr, &task_on, &ctx)); // Inclusive
        assert!(evaluate(&expr, &task_after, &ctx));
        assert!(!evaluate(&expr, &task_before, &ctx));
    }

    #[test]
    fn test_eval_created_on_or_before() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let ctx = ctx_with_date(today);

        let boundary = NaiveDate::from_ymd_opt(2025, 6, 10).unwrap();

        let task_on = task_created_on(boundary);
        let task_after = task_created_on(NaiveDate::from_ymd_opt(2025, 6, 20).unwrap());
        let task_before = task_created_on(NaiveDate::from_ymd_opt(2025, 6, 5).unwrap());

        let expr = crate::domain::filter_dsl::parse("created:..2025-06-10").unwrap();
        assert!(evaluate(&expr, &task_on, &ctx)); // Inclusive
        assert!(!evaluate(&expr, &task_after, &ctx));
        assert!(evaluate(&expr, &task_before, &ctx));
    }

    #[test]
    fn test_eval_scheduled_date_range() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let ctx = ctx_with_date(today);

        let start = NaiveDate::from_ymd_opt(2025, 6, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 6, 30).unwrap();

        // Task within range
        let mut task_in = Task::new("Test");
        task_in.scheduled_date = Some(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());
        let expr = crate::domain::filter_dsl::parse("scheduled:2025-06-01..2025-06-30").unwrap();
        assert!(evaluate(&expr, &task_in, &ctx));

        // Task outside range
        let mut task_before = Task::new("Test");
        task_before.scheduled_date = Some(NaiveDate::from_ymd_opt(2025, 5, 15).unwrap());
        assert!(!evaluate(&expr, &task_before, &ctx));

        // Task on boundaries (inclusive)
        let mut task_start = Task::new("Test");
        task_start.scheduled_date = Some(start);
        let mut task_end = Task::new("Test");
        task_end.scheduled_date = Some(end);
        assert!(evaluate(&expr, &task_start, &ctx));
        assert!(evaluate(&expr, &task_end, &ctx));
    }

    #[test]
    fn test_eval_estimate_numeric_range() {
        let ctx = empty_ctx();

        // Estimate within range
        let mut task = Task::new("Test");
        task.estimated_minutes = Some(60);

        let expr = crate::domain::filter_dsl::parse("estimate:30..120").unwrap();
        assert!(evaluate(&expr, &task, &ctx));

        // Estimate below range
        task.estimated_minutes = Some(15);
        assert!(!evaluate(&expr, &task, &ctx));

        // Estimate above range
        task.estimated_minutes = Some(180);
        assert!(!evaluate(&expr, &task, &ctx));

        // Boundary values (inclusive)
        task.estimated_minutes = Some(30);
        assert!(evaluate(&expr, &task, &ctx));
        task.estimated_minutes = Some(120);
        assert!(evaluate(&expr, &task, &ctx));
    }

    #[test]
    fn test_eval_estimate_open_start() {
        let ctx = empty_ctx();

        let mut task = Task::new("Test");

        // Open-ended: >= 60
        let expr = crate::domain::filter_dsl::parse("estimate:60..").unwrap();

        task.estimated_minutes = Some(60);
        assert!(evaluate(&expr, &task, &ctx)); // Inclusive

        task.estimated_minutes = Some(120);
        assert!(evaluate(&expr, &task, &ctx));

        task.estimated_minutes = Some(30);
        assert!(!evaluate(&expr, &task, &ctx));
    }

    #[test]
    fn test_eval_estimate_open_end() {
        let ctx = empty_ctx();

        let mut task = Task::new("Test");

        // Open-ended: <= 60
        let expr = crate::domain::filter_dsl::parse("estimate:..60").unwrap();

        task.estimated_minutes = Some(60);
        assert!(evaluate(&expr, &task, &ctx)); // Inclusive

        task.estimated_minutes = Some(30);
        assert!(evaluate(&expr, &task, &ctx));

        task.estimated_minutes = Some(120);
        assert!(!evaluate(&expr, &task, &ctx));
    }

    #[test]
    fn test_eval_actual_numeric_range() {
        let ctx = empty_ctx();

        let mut task = Task::new("Test");

        let expr = crate::domain::filter_dsl::parse("actual:30..90").unwrap();

        // Within range
        task.actual_minutes = 60;
        assert!(evaluate(&expr, &task, &ctx));

        // Below range
        task.actual_minutes = 15;
        assert!(!evaluate(&expr, &task, &ctx));

        // Above range
        task.actual_minutes = 120;
        assert!(!evaluate(&expr, &task, &ctx));

        // Boundaries (inclusive)
        task.actual_minutes = 30;
        assert!(evaluate(&expr, &task, &ctx));
        task.actual_minutes = 90;
        assert!(evaluate(&expr, &task, &ctx));
    }

    #[test]
    fn test_eval_range_vs_comparison_semantics() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let ctx = ctx_with_date(today);

        let boundary = NaiveDate::from_ymd_opt(2025, 6, 10).unwrap();
        let task_on_boundary = Task::new("Test").with_due_date(boundary);

        // Strict comparison: > is exclusive
        let expr_after = crate::domain::filter_dsl::parse("due:>2025-06-10").unwrap();
        assert!(!evaluate(&expr_after, &task_on_boundary, &ctx));

        // Range syntax: start.. is inclusive (OnOrAfter)
        let expr_from = crate::domain::filter_dsl::parse("due:2025-06-10..").unwrap();
        assert!(evaluate(&expr_from, &task_on_boundary, &ctx));

        // Strict comparison: < is exclusive
        let expr_before = crate::domain::filter_dsl::parse("due:<2025-06-10").unwrap();
        assert!(!evaluate(&expr_before, &task_on_boundary, &ctx));

        // Range syntax: ..end is inclusive (OnOrBefore)
        let expr_until = crate::domain::filter_dsl::parse("due:..2025-06-10").unwrap();
        assert!(evaluate(&expr_until, &task_on_boundary, &ctx));
    }

    // ========================================================================
    // Cached evaluation tests (ensure parity with non-cached)
    // ========================================================================

    #[test]
    fn test_cached_eval_produces_same_results() {
        let task = Task::new("Fix Login Bug")
            .with_priority(Priority::High)
            .with_tags(vec!["bug".to_string(), "urgent".to_string()])
            .with_description("Users cannot log in".to_string());

        let ctx = empty_ctx();

        // Test various filter expressions
        let test_cases = [
            "priority:high",
            "tags:bug",
            "tags:BUG",
            "title:login",
            r#"search:"login""#,
            r#"search:"users""#,
            "priority:high AND tags:bug",
            "tags:urgent OR tags:feature",
            "!status:done",
            "(priority:high OR priority:urgent) AND tags:bug",
        ];

        for query in test_cases {
            let expr = crate::domain::filter_dsl::parse(query).unwrap();
            let cache = TaskLowerCache::new(&task);

            let result_uncached = evaluate(&expr, &task, &ctx);
            let result_cached = evaluate_with_cache(&expr, &cache, &ctx);

            assert_eq!(
                result_uncached, result_cached,
                "Results differ for query: {query}"
            );
        }
    }

    #[test]
    fn test_cached_eval_text_fields() {
        let task = Task::new("Refactor Authentication Module")
            .with_tags(vec!["backend".to_string(), "Security".to_string()])
            .with_description("Improve the auth system".to_string());

        let ctx = empty_ctx();
        let cache = TaskLowerCache::new(&task);

        // Tag matching (case-insensitive)
        let expr = crate::domain::filter_dsl::parse("tags:BACKEND").unwrap();
        assert!(evaluate_with_cache(&expr, &cache, &ctx));

        let expr = crate::domain::filter_dsl::parse("tags:security").unwrap();
        assert!(evaluate_with_cache(&expr, &cache, &ctx));

        // Title matching (case-insensitive substring)
        let expr = crate::domain::filter_dsl::parse("title:refactor").unwrap();
        assert!(evaluate_with_cache(&expr, &cache, &ctx));

        let expr = crate::domain::filter_dsl::parse("title:AUTH").unwrap();
        assert!(evaluate_with_cache(&expr, &cache, &ctx));

        // Search (title OR description, case-insensitive)
        let expr = crate::domain::filter_dsl::parse(r#"search:"improve""#).unwrap();
        assert!(evaluate_with_cache(&expr, &cache, &ctx));

        let expr = crate::domain::filter_dsl::parse(r#"search:"MODULE""#).unwrap();
        assert!(evaluate_with_cache(&expr, &cache, &ctx));
    }

    #[test]
    fn test_cached_eval_complex_boolean() {
        let task = Task::new("Urgent Bug Fix")
            .with_priority(Priority::Urgent)
            .with_tags(vec!["bug".to_string(), "production".to_string()]);

        let ctx = empty_ctx();
        let cache = TaskLowerCache::new(&task);

        // Complex nested expression
        let expr = crate::domain::filter_dsl::parse(
            "((priority:high OR priority:urgent) AND tags:bug) AND !status:done",
        )
        .unwrap();
        assert!(evaluate_with_cache(&expr, &cache, &ctx));

        // Mix of text and non-text conditions
        let expr = crate::domain::filter_dsl::parse(
            "title:urgent AND tags:production AND priority:urgent",
        )
        .unwrap();
        assert!(evaluate_with_cache(&expr, &cache, &ctx));
    }
}
