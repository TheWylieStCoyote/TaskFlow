//! Advanced filter DSL for boolean task filtering.
//!
//! This module provides a domain-specific language for filtering tasks
//! using boolean expressions. It supports complex queries like:
//!
//! - `priority:high AND tags:bug`
//! - `status:todo OR status:in_progress`
//! - `!status:done` (negation)
//! - `(priority:high OR priority:urgent) AND tags:work`
//!
//! # Syntax Overview
//!
//! ## Basic Fields
//!
//! | Field | Aliases | Description | Values |
//! |-------|---------|-------------|--------|
//! | `priority:` | | Task priority | `none`, `low`, `medium`/`med`, `high`, `urgent` |
//! | `status:` | | Task status | `todo`, `in_progress`/`in-progress`, `blocked`, `done`/`completed`, `cancelled`/`canceled` |
//! | `tags:` | `tag:` | Tag name | Any string, case-insensitive |
//! | `project:` | | Project name | Partial match, case-insensitive |
//! | `title:` | | Title text | Partial match, case-insensitive |
//! | `search:` | | Full-text search | Searches title and description |
//!
//! ## Date Fields
//!
//! All date fields support relative keywords and specific dates with comparison operators.
//!
//! | Field | Aliases | Keywords | Date Formats |
//! |-------|---------|----------|--------------|
//! | `due:` | | `today`, `tomorrow`, `thisweek`, `nextweek`, `overdue`, `none` | `YYYY-MM-DD`, `<YYYY-MM-DD`, `>YYYY-MM-DD` |
//! | `created:` | | `today`, `yesterday`, `thisweek`, `lastweek` | `YYYY-MM-DD`, `<YYYY-MM-DD`, `>YYYY-MM-DD` |
//! | `scheduled:` | | `today`, `tomorrow`, `thisweek`, `nextweek`, `none` | `YYYY-MM-DD`, `<YYYY-MM-DD`, `>YYYY-MM-DD` |
//! | `completed:` | | `today`, `yesterday`, `thisweek`, `lastweek` | `YYYY-MM-DD`, `<YYYY-MM-DD`, `>YYYY-MM-DD` |
//! | `modified:` | `updated:` | `today`, `yesterday`, `thisweek`, `lastweek` | `YYYY-MM-DD`, `<YYYY-MM-DD`, `>YYYY-MM-DD` |
//!
//! ## Numeric Fields
//!
//! Time-based fields support comparison operators for filtering by duration (in minutes).
//!
//! | Field | Aliases | Description | Examples |
//! |-------|---------|-------------|----------|
//! | `estimate:` | `est:` | Time estimate | `>60`, `<30`, `>=60`, `<=30`, `60`, `none` |
//! | `actual:` | `tracked:` | Tracked time | `>0`, `<120`, `>=30`, `<=60`, `45`, `none` |
//!
//! ## Field Presence (`has:`)
//!
//! Check whether a task has a value set for a specific field.
//!
//! | Value | Description |
//! |-------|-------------|
//! | `has:due` | Task has a due date |
//! | `has:project` | Task is assigned to a project |
//! | `has:tags` (or `tag`) | Task has at least one tag |
//! | `has:estimate` (or `est`) | Task has a time estimate |
//! | `has:description` (or `desc`) | Task has a description |
//! | `has:recurrence` (or `recurring`) | Task has a recurrence pattern |
//! | `has:scheduled` | Task has a scheduled date |
//! | `has:dependencies` (or `deps`, `blocked`) | Task has dependencies/blockers |
//! | `has:parent` (or `subtask`) | Task is a subtask (has a parent) |
//! | `has:tracked` (or `time`) | Task has tracked time (actual > 0) |
//!
//! ## Operators
//!
//! | Operator | Precedence | Description |
//! |----------|------------|-------------|
//! | `!` or `NOT` | Highest | Negation |
//! | `AND` | Medium | Both must match |
//! | `OR` | Lowest | Either must match |
//! | `()` | Override | Grouping for precedence |
//!
//! # Examples
//!
//! ```
//! use std::collections::HashMap;
//! use taskflow::domain::{Task, Priority, TaskStatus};
//! use taskflow::domain::filter_dsl::{parse, evaluate, EvalContext};
//!
//! // Parse a filter expression
//! let expr = parse("priority:high AND !status:done").unwrap();
//!
//! // Create a task to filter
//! let task = Task::new("Important bug fix")
//!     .with_priority(Priority::High);
//!
//! // Evaluate the filter
//! let projects = HashMap::new();
//! let ctx = EvalContext::new(&projects);
//! assert!(evaluate(&expr, &task, &ctx));
//!
//! // More complex expression
//! let expr = parse("(tags:bug OR tags:urgent) AND project:backend").unwrap();
//! ```
//!
//! # Common Filter Patterns
//!
//! ## Priority and Status
//!
//! ```text
//! # High-priority incomplete tasks
//! priority:high AND !status:done
//!
//! # Urgent tasks that are blocked
//! priority:urgent AND status:blocked
//!
//! # All active tasks (not done or cancelled)
//! !status:done AND !status:cancelled
//! ```
//!
//! ## Due Date Queries
//!
//! ```text
//! # Tasks due this week with high priority
//! due:thisweek AND (priority:urgent OR priority:high)
//!
//! # Overdue tasks in a specific project
//! due:overdue AND project:frontend
//!
//! # Tasks due in January 2025
//! due:>2024-12-31 AND due:<2025-02-01
//!
//! # Tasks with no due date
//! due:none AND status:todo
//! ```
//!
//! ## Scheduling and Planning
//!
//! ```text
//! # Tasks scheduled for today
//! scheduled:today
//!
//! # Tasks scheduled but not yet started
//! has:scheduled AND status:todo
//!
//! # Tasks without estimates (need planning)
//! !has:estimate AND status:todo
//!
//! # Large tasks (over 2 hours estimated)
//! estimate:>120
//! ```
//!
//! ## Time Tracking
//!
//! ```text
//! # Tasks with tracked time
//! has:tracked
//!
//! # Tasks with over an hour tracked
//! actual:>60
//!
//! # Completed tasks with no time tracked
//! status:done AND actual:0
//! ```
//!
//! ## Recently Modified
//!
//! ```text
//! # Tasks modified today
//! modified:today
//!
//! # Tasks created this week that are still pending
//! created:thisweek AND status:todo
//!
//! # Tasks completed yesterday
//! completed:yesterday
//! ```
//!
//! ## Task Relationships
//!
//! ```text
//! # Tasks with dependencies (blocked by others)
//! has:dependencies
//!
//! # Subtasks only
//! has:parent
//!
//! # Recurring tasks
//! has:recurrence
//! ```
//!
//! ## Tags and Projects
//!
//! ```text
//! # Tasks with a specific tag
//! tags:bug AND !status:done
//!
//! # Tasks with multiple tags (must have both)
//! tags:bug AND tags:urgent
//!
//! # Tasks with either tag
//! tags:bug OR tags:feature
//!
//! # Search within a project
//! project:backend AND search:"authentication"
//! ```
//!
//! ## Complex Queries
//!
//! ```text
//! # High priority bugs due this week, not done
//! (priority:high OR priority:urgent) AND tags:bug AND due:thisweek AND !status:done
//!
//! # Tasks needing attention: overdue or blocked
//! due:overdue OR status:blocked
//!
//! # Unplanned work: no estimate, no due date, still todo
//! !has:estimate AND !has:due AND status:todo
//! ```
//!
//! # Error Handling
//!
//! ```
//! use taskflow::domain::filter_dsl::{parse, ParseError};
//!
//! // Unknown field
//! let result = parse("unknown:value");
//! assert!(result.is_err());
//!
//! // Invalid value
//! let result = parse("priority:extreme");
//! assert!(result.is_err());
//!
//! // Empty expression
//! let result = parse("");
//! assert!(result.is_err());
//! ```

mod ast;
mod error;
mod eval;
mod lexer;
mod parser;

// Re-export public types
pub use ast::{
    Condition, CreatedFilter, DueFilter, FilterExpr, FilterField, FilterValue, HasField,
    NumericFilter, ScheduledFilter,
};
pub use error::{ParseError, ParseResult};
pub use eval::{evaluate, EvalContext};
pub use parser::parse;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Priority, Task, TaskStatus};
    use std::collections::HashMap;

    /// Integration test for the full DSL pipeline.
    #[test]
    fn test_end_to_end() {
        let task1 = Task::new("High priority bug")
            .with_priority(Priority::High)
            .with_tags(vec!["bug".to_string()]);

        let task2 = Task::new("Low priority feature")
            .with_priority(Priority::Low)
            .with_tags(vec!["feature".to_string()]);

        let mut task3 = Task::new("Done task").with_priority(Priority::High);
        task3.status = TaskStatus::Done;

        let projects = HashMap::new();
        let ctx = EvalContext::new(&projects);

        // Filter: high priority AND not done
        let expr = parse("priority:high AND !status:done").unwrap();

        assert!(evaluate(&expr, &task1, &ctx)); // High, not done - matches
        assert!(!evaluate(&expr, &task2, &ctx)); // Low priority - doesn't match
        assert!(!evaluate(&expr, &task3, &ctx)); // Done - doesn't match
    }

    #[test]
    fn test_filter_multiple_tags() {
        let task = Task::new("Multi-tag task").with_tags(vec![
            "bug".to_string(),
            "urgent".to_string(),
            "backend".to_string(),
        ]);

        let projects = HashMap::new();
        let ctx = EvalContext::new(&projects);

        // OR: has any of these tags
        let expr = parse("tags:bug OR tags:feature").unwrap();
        assert!(evaluate(&expr, &task, &ctx));

        // AND: has all these tags (chained)
        let expr = parse("tags:bug AND tags:urgent").unwrap();
        assert!(evaluate(&expr, &task, &ctx));

        let expr = parse("tags:bug AND tags:frontend").unwrap();
        assert!(!evaluate(&expr, &task, &ctx));
    }

    #[test]
    fn test_complex_nested_expression() {
        let task = Task::new("Complex task")
            .with_priority(Priority::Urgent)
            .with_tags(vec!["production".to_string()]);

        let projects = HashMap::new();
        let ctx = EvalContext::new(&projects);

        // ((priority:high OR priority:urgent) AND tags:production) AND !status:done
        let expr =
            parse("((priority:high OR priority:urgent) AND tags:production) AND !status:done")
                .unwrap();

        assert!(evaluate(&expr, &task, &ctx));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let expr = parse("priority:high AND !status:done").unwrap();

        // Serialize to JSON
        let json = serde_json::to_string(&expr).unwrap();

        // Deserialize back
        let restored: FilterExpr = serde_json::from_str(&json).unwrap();

        // Should be equal
        assert_eq!(expr, restored);
    }
}
