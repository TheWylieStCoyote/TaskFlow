//! Shared filtering utilities for storage backends.
//!
//! This module provides a common `task_matches_filter` function
//! that can be used by in-memory storage backends (JSON, YAML, Markdown)
//! to filter tasks according to [`Filter`] criteria.

use crate::domain::{Filter, TagFilterMode, Task};

/// Check if a task matches all the criteria in the given filter.
///
/// This function is used by JSON, YAML, and Markdown backends to
/// perform in-memory filtering. The SQLite backend uses SQL queries
/// directly for efficiency.
///
/// # Filter Criteria
///
/// - `status`: Task status must be in the allowed set
/// - `priority`: Task priority must be in the allowed set
/// - `project_id`: Task must belong to the specified project
/// - `tags`: Task must have matching tags (Any or All mode)
/// - `due_before`/`due_after`: Task due date must be in range
/// - `search_text`: Task title or description must contain text
/// - `include_completed`: Whether to include completed tasks
#[must_use]
pub fn task_matches_filter(task: &Task, filter: &Filter) -> bool {
    // Filter by status
    if let Some(ref statuses) = filter.status {
        if !statuses.contains(&task.status) {
            return false;
        }
    }

    // Filter by priority
    if let Some(ref priorities) = filter.priority {
        if !priorities.contains(&task.priority) {
            return false;
        }
    }

    // Filter by project
    if let Some(ref project_id) = filter.project_id {
        if task.project_id.as_ref() != Some(project_id) {
            return false;
        }
    }

    // Filter by tags
    if let Some(ref tags) = filter.tags {
        if !tags.is_empty() {
            let has_tags = match filter.tags_mode {
                TagFilterMode::Any => tags.iter().any(|t| task.tags.contains(t)),
                TagFilterMode::All => tags.iter().all(|t| task.tags.contains(t)),
            };
            if !has_tags {
                return false;
            }
        }
    }

    // Filter by due date range
    if let Some(due_before) = filter.due_before {
        match task.due_date {
            Some(due) if due < due_before => {}
            _ => return false,
        }
    }

    if let Some(due_after) = filter.due_after {
        match task.due_date {
            Some(due) if due > due_after => {}
            _ => return false,
        }
    }

    // Filter by search text (case-insensitive)
    if let Some(ref search) = filter.search_text {
        let search_lower = search.to_lowercase();
        let title_matches = task.title.to_lowercase().contains(&search_lower);
        let desc_matches = task
            .description
            .as_ref()
            .is_some_and(|d| d.to_lowercase().contains(&search_lower));
        if !title_matches && !desc_matches {
            return false;
        }
    }

    // Filter completed tasks
    if !filter.include_completed && task.status.is_complete() {
        return false;
    }

    true
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::domain::{Priority, ProjectId, TaskStatus};

    #[test]
    fn test_empty_filter_matches_all() {
        let filter = Filter::default();
        let task = Task::new("Test task");
        assert!(task_matches_filter(&task, &filter));
    }

    #[test]
    fn test_status_filter() {
        let mut filter = Filter::default();
        filter.status = Some(vec![TaskStatus::Done]);
        filter.include_completed = true; // Need this to allow Done status

        let pending = Task::new("Pending");
        let done = Task::new("Done").with_status(TaskStatus::Done);

        assert!(!task_matches_filter(&pending, &filter));
        assert!(task_matches_filter(&done, &filter));
    }

    #[test]
    fn test_priority_filter() {
        let mut filter = Filter::default();
        filter.priority = Some(vec![Priority::High, Priority::Urgent]);

        let low = Task::new("Low").with_priority(Priority::Low);
        let high = Task::new("High").with_priority(Priority::High);

        assert!(!task_matches_filter(&low, &filter));
        assert!(task_matches_filter(&high, &filter));
    }

    #[test]
    fn test_project_filter() {
        let project_id = ProjectId::new();
        let mut filter = Filter::default();
        filter.project_id = Some(project_id);

        let no_project = Task::new("No project");
        let in_project = Task::new("In project").with_project(project_id);

        assert!(!task_matches_filter(&no_project, &filter));
        assert!(task_matches_filter(&in_project, &filter));
    }

    #[test]
    fn test_tag_filter_any_mode() {
        let mut filter = Filter::default();
        filter.tags = Some(vec!["rust".to_string(), "python".to_string()]);
        filter.tags_mode = TagFilterMode::Any;

        let rust_only = Task::new("Rust").with_tags(vec!["rust".into()]);
        let no_tags = Task::new("No tags");

        assert!(task_matches_filter(&rust_only, &filter));
        assert!(!task_matches_filter(&no_tags, &filter));
    }

    #[test]
    fn test_tag_filter_all_mode() {
        let mut filter = Filter::default();
        filter.tags = Some(vec!["rust".to_string(), "async".to_string()]);
        filter.tags_mode = TagFilterMode::All;

        let rust_only = Task::new("Rust").with_tags(vec!["rust".into()]);
        let both = Task::new("Both").with_tags(vec!["rust".into(), "async".into()]);

        assert!(!task_matches_filter(&rust_only, &filter));
        assert!(task_matches_filter(&both, &filter));
    }

    #[test]
    fn test_search_filter_title() {
        let mut filter = Filter::default();
        filter.search_text = Some("rust".to_string());

        let matches = Task::new("Learn Rust programming");
        let no_match = Task::new("Learn Python programming");

        assert!(task_matches_filter(&matches, &filter));
        assert!(!task_matches_filter(&no_match, &filter));
    }

    #[test]
    fn test_search_filter_description() {
        let mut filter = Filter::default();
        filter.search_text = Some("important".to_string());

        let mut task = Task::new("Task");
        task.description = Some("This is important work".to_string());
        let no_match = Task::new("Task");

        assert!(task_matches_filter(&task, &filter));
        assert!(!task_matches_filter(&no_match, &filter));
    }

    #[test]
    fn test_include_completed() {
        let mut filter = Filter::default();
        filter.include_completed = false;

        let done = Task::new("Done").with_status(TaskStatus::Done);
        let pending = Task::new("Pending");

        assert!(!task_matches_filter(&done, &filter));
        assert!(task_matches_filter(&pending, &filter));

        filter.include_completed = true;
        assert!(task_matches_filter(&done, &filter));
    }

    #[test]
    fn test_due_date_filter() {
        use chrono::NaiveDate;

        let mut filter = Filter::default();
        let dec_15 = NaiveDate::from_ymd_opt(2024, 12, 15).unwrap();
        let dec_10 = NaiveDate::from_ymd_opt(2024, 12, 10).unwrap();
        let dec_20 = NaiveDate::from_ymd_opt(2024, 12, 20).unwrap();

        // Due before Dec 15
        filter.due_before = Some(dec_15);
        let task_early = Task::new("Early").with_due_date(dec_10);
        let task_late = Task::new("Late").with_due_date(dec_20);
        let no_due = Task::new("No due");

        assert!(task_matches_filter(&task_early, &filter));
        assert!(!task_matches_filter(&task_late, &filter));
        assert!(!task_matches_filter(&no_due, &filter));
    }
}
