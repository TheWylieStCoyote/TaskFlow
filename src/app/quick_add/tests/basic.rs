//! Basic quick add parsing tests.

use chrono::Utc;

use crate::domain::Priority;

use super::super::parse_quick_add;

#[test]
fn test_parse_quick_add_simple_title() {
    let parsed = parse_quick_add("Buy groceries");
    assert_eq!(parsed.title, "Buy groceries");
    assert!(parsed.tags.is_empty());
    assert!(parsed.priority.is_none());
    assert!(parsed.due_date.is_none());
    assert!(parsed.project_name.is_none());
}

#[test]
fn test_parse_quick_add_with_tag() {
    let parsed = parse_quick_add("Fix bug #backend");
    assert_eq!(parsed.title, "Fix bug");
    assert_eq!(parsed.tags, vec!["backend"]);
}

#[test]
fn test_parse_quick_add_multiple_tags() {
    let parsed = parse_quick_add("Fix bug #backend #urgent #v2");
    assert_eq!(parsed.title, "Fix bug");
    assert_eq!(parsed.tags, vec!["backend", "urgent", "v2"]);
}

#[test]
fn test_parse_quick_add_priority_high() {
    let parsed = parse_quick_add("Important task !high");
    assert_eq!(parsed.title, "Important task");
    assert_eq!(parsed.priority, Some(Priority::High));
}

#[test]
fn test_parse_quick_add_priority_urgent() {
    let parsed = parse_quick_add("Critical issue !urgent");
    assert_eq!(parsed.priority, Some(Priority::Urgent));
}

#[test]
fn test_parse_quick_add_priority_medium() {
    let parsed = parse_quick_add("Normal task !med");
    assert_eq!(parsed.priority, Some(Priority::Medium));
}

#[test]
fn test_parse_quick_add_priority_low() {
    let parsed = parse_quick_add("Low priority task !low");
    assert_eq!(parsed.priority, Some(Priority::Low));
}

#[test]
fn test_parse_quick_add_project() {
    let parsed = parse_quick_add("Task @work");
    assert_eq!(parsed.title, "Task");
    assert_eq!(parsed.project_name, Some("work".to_string()));
}

#[test]
fn test_parse_quick_add_complex() {
    let parsed = parse_quick_add("Fix login bug #backend #auth !high due:friday @work");
    assert_eq!(parsed.title, "Fix login bug");
    assert_eq!(parsed.tags, vec!["backend", "auth"]);
    assert_eq!(parsed.priority, Some(Priority::High));
    assert!(parsed.due_date.is_some());
    assert_eq!(parsed.project_name, Some("work".to_string()));
}

#[test]
fn test_parse_quick_add_empty() {
    let parsed = parse_quick_add("");
    assert_eq!(parsed.title, "");
    assert!(parsed.tags.is_empty());
}

#[test]
fn test_parse_quick_add_only_metadata() {
    let parsed = parse_quick_add("#tag !high");
    assert_eq!(parsed.title, "");
    assert_eq!(parsed.tags, vec!["tag"]);
    assert_eq!(parsed.priority, Some(Priority::High));
}

#[test]
fn test_parse_priority_aliases() {
    use super::super::parse_priority;
    assert_eq!(parse_priority("u"), Some(Priority::Urgent));
    assert_eq!(parse_priority("h"), Some(Priority::High));
    assert_eq!(parse_priority("m"), Some(Priority::Medium));
    assert_eq!(parse_priority("l"), Some(Priority::Low));
    assert_eq!(parse_priority("n"), Some(Priority::None));
}

#[test]
fn test_parse_quick_add_preserves_title_words() {
    let parsed = parse_quick_add("This is a long task title with many words");
    assert_eq!(parsed.title, "This is a long task title with many words");
}

#[test]
fn test_parse_quick_add_metadata_in_middle() {
    let parsed = parse_quick_add("Fix #bug in the code !high today");
    assert_eq!(parsed.title, "Fix in the code today");
    assert_eq!(parsed.tags, vec!["bug"]);
    assert_eq!(parsed.priority, Some(Priority::High));
}

#[test]
fn test_parse_priority_case_insensitive() {
    let parsed1 = parse_quick_add("Task !HIGH");
    let parsed2 = parse_quick_add("Task !High");
    let parsed3 = parse_quick_add("Task !high");
    assert_eq!(parsed1.priority, Some(Priority::High));
    assert_eq!(parsed2.priority, Some(Priority::High));
    assert_eq!(parsed3.priority, Some(Priority::High));
}

#[test]
fn test_parse_unrecognized_priority() {
    let parsed = parse_quick_add("Task !invalid");
    // Unrecognized priority string should result in None
    assert!(parsed.priority.is_none());
}

#[test]
fn test_parse_quick_add_scheduled() {
    let parsed = parse_quick_add("Task sched:tomorrow");
    let expected = Utc::now().date_naive() + chrono::Duration::days(1);
    assert_eq!(parsed.scheduled_date, Some(expected));
}
