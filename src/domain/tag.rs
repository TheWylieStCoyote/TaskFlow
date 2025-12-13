//! Tag entity for task categorization.
//!
//! Tags provide a flexible way to categorize and filter tasks across projects.
//! Unlike projects which are hierarchical, tags are flat labels that can be
//! applied to any task.
//!
//! # Context Tags
//!
//! Tags starting with `@` are treated as context tags (GTD-style), representing
//! where or when a task can be done: `@home`, `@work`, `@errands`, `@phone`.
//!
//! # Tag Conventions
//!
//! ## Naming
//! - Use lowercase with hyphens: `high-priority`, `needs-review`
//! - Context tags start with `@`: `@home`, `@office`, `@errands`
//! - Project prefixes can help organization: `proj:website`, `area:finance`
//!
//! ## Common Patterns
//! - **Priority**: `urgent`, `important`, `low-priority`
//! - **Status**: `blocked`, `waiting-on`, `needs-review`
//! - **Type**: `bug`, `feature`, `docs`, `refactor`
//! - **Context**: `@home`, `@work`, `@phone`, `@computer`
//!
//! # Filtering with Tags
//!
//! Tags integrate with the filter DSL for powerful querying:
//!
//! ```
//! use taskflow::domain::filter_dsl::parse;
//!
//! // Find tasks with a specific tag
//! let filter = parse("tag:urgent").unwrap();
//!
//! // Combine with other filters
//! let filter = parse("tag:urgent AND status:todo").unwrap();
//!
//! // Multiple tags (OR logic)
//! let filter = parse("tag:bug OR tag:feature").unwrap();
//! ```
//!
//! Note: Context tags (starting with `@`) are identified using [`is_context_tag`]
//! and can be filtered via the UI's context selector.
//!
//! # Examples
//!
//! ```
//! use taskflow::domain::Tag;
//!
//! let tag = Tag::new("urgent").with_color("#e74c3c");
//! assert_eq!(tag.name, "urgent");
//! ```

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::domain::Task;

/// Tag entity for categorizing tasks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
}

impl Tag {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            color: None,
            description: None,
        }
    }

    #[must_use]
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

// ============================================================================
// Context Tag Functions
// ============================================================================

/// Check if a tag is a context tag (starts with '@').
///
/// Context tags follow GTD (Getting Things Done) convention and represent
/// where or when a task can be done.
///
/// # Examples
///
/// ```
/// use taskflow::domain::is_context_tag;
///
/// assert!(is_context_tag("@home"));
/// assert!(is_context_tag("@work"));
/// assert!(!is_context_tag("urgent"));
/// assert!(!is_context_tag("#feature"));
/// ```
#[must_use]
pub fn is_context_tag(tag: &str) -> bool {
    tag.starts_with('@')
}

/// Extract all unique context tags from a collection of tasks, sorted alphabetically.
///
/// Scans through all tasks and collects tags that start with `@`.
///
/// # Examples
///
/// ```
/// use taskflow::domain::{Task, extract_contexts};
///
/// let task1 = Task::new("Work task").with_tags(vec!["@work".into(), "urgent".into()]);
/// let task2 = Task::new("Home task").with_tags(vec!["@home".into(), "@phone".into()]);
/// let tasks = vec![task1, task2];
///
/// let contexts = extract_contexts(tasks.iter());
/// assert_eq!(contexts, vec!["@home", "@phone", "@work"]);
/// ```
#[must_use]
pub fn extract_contexts<'a>(tasks: impl Iterator<Item = &'a Task>) -> Vec<String> {
    let mut contexts: HashSet<String> = HashSet::new();
    for task in tasks {
        for tag in &task.tags {
            if is_context_tag(tag) {
                contexts.insert(tag.clone());
            }
        }
    }
    let mut result: Vec<String> = contexts.into_iter().collect();
    result.sort();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_new() {
        let tag = Tag::new("urgent");
        assert_eq!(tag.name, "urgent");
        assert!(tag.color.is_none());
        assert!(tag.description.is_none());
    }

    #[test]
    fn test_tag_with_color() {
        let tag = Tag::new("work").with_color("#3498db");
        assert_eq!(tag.name, "work");
        assert_eq!(tag.color, Some("#3498db".to_string()));
    }

    #[test]
    fn test_tag_display() {
        let tag = Tag::new("important");
        assert_eq!(tag.to_string(), "important");
    }

    #[test]
    fn test_tag_equality() {
        let tag1 = Tag::new("bug");
        let tag2 = Tag::new("bug");
        let tag3 = Tag::new("feature");

        assert_eq!(tag1, tag2);
        assert_ne!(tag1, tag3);
    }

    #[test]
    fn test_tag_equality_with_color() {
        let tag1 = Tag::new("bug").with_color("red");
        let tag2 = Tag::new("bug").with_color("red");
        let tag3 = Tag::new("bug").with_color("blue");

        assert_eq!(tag1, tag2);
        assert_ne!(tag1, tag3);
    }

    #[test]
    fn test_tag_serialization() {
        let tag = Tag::new("feature").with_color("#2ecc71");
        let json = serde_json::to_string(&tag).expect("serialize");
        let restored: Tag = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(tag, restored);
    }

    #[test]
    fn test_tag_serialization_without_color() {
        let tag = Tag::new("bug");
        let json = serde_json::to_string(&tag).expect("serialize");
        let restored: Tag = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(tag, restored);
        assert!(restored.color.is_none());
    }

    #[test]
    fn test_tag_from_string_types() {
        // Test that Tag::new accepts various string types
        let tag1 = Tag::new("literal");
        let tag2 = Tag::new(String::from("owned"));
        let s = "borrowed";
        let tag3 = Tag::new(s);

        assert_eq!(tag1.name, "literal");
        assert_eq!(tag2.name, "owned");
        assert_eq!(tag3.name, "borrowed");
    }

    // Context tag tests
    #[test]
    fn test_is_context_tag_with_at_prefix() {
        assert!(is_context_tag("@home"));
        assert!(is_context_tag("@work"));
        assert!(is_context_tag("@errands"));
        assert!(is_context_tag("@phone"));
        assert!(is_context_tag("@"));
    }

    #[test]
    fn test_is_context_tag_without_at_prefix() {
        assert!(!is_context_tag("home"));
        assert!(!is_context_tag("work"));
        assert!(!is_context_tag("#feature"));
        assert!(!is_context_tag("urgent"));
        assert!(!is_context_tag(""));
    }

    #[test]
    fn test_extract_contexts_basic() {
        let task1 = Task::new("Task 1").with_tags(vec!["@home".into(), "urgent".into()]);
        let task2 = Task::new("Task 2").with_tags(vec!["@work".into(), "@phone".into()]);
        let task3 = Task::new("Task 3").with_tags(vec!["bug".into()]);
        let tasks = [task1, task2, task3];

        let contexts = extract_contexts(tasks.iter());

        assert_eq!(contexts.len(), 3);
        assert!(contexts.contains(&"@home".to_string()));
        assert!(contexts.contains(&"@phone".to_string()));
        assert!(contexts.contains(&"@work".to_string()));
    }

    #[test]
    fn test_extract_contexts_sorted() {
        let task1 = Task::new("Task 1").with_tags(vec!["@zebra".into()]);
        let task2 = Task::new("Task 2").with_tags(vec!["@apple".into()]);
        let task3 = Task::new("Task 3").with_tags(vec!["@middle".into()]);
        let tasks = [task1, task2, task3];

        let contexts = extract_contexts(tasks.iter());

        assert_eq!(contexts, vec!["@apple", "@middle", "@zebra"]);
    }

    #[test]
    fn test_extract_contexts_deduplicates() {
        let task1 = Task::new("Task 1").with_tags(vec!["@home".into()]);
        let task2 = Task::new("Task 2").with_tags(vec!["@home".into()]);
        let task3 = Task::new("Task 3").with_tags(vec!["@home".into(), "@work".into()]);
        let tasks = [task1, task2, task3];

        let contexts = extract_contexts(tasks.iter());

        assert_eq!(contexts, vec!["@home", "@work"]);
    }

    #[test]
    fn test_extract_contexts_empty() {
        let task1 = Task::new("Task 1").with_tags(vec!["regular".into()]);
        let task2 = Task::new("Task 2");
        let tasks = [task1, task2];

        let contexts = extract_contexts(tasks.iter());

        assert!(contexts.is_empty());
    }

    #[test]
    fn test_extract_contexts_no_tasks() {
        let tasks: Vec<Task> = vec![];
        let contexts = extract_contexts(tasks.iter());
        assert!(contexts.is_empty());
    }
}
