//! ICS (iCalendar) export functionality.

use std::io::Write;

use crate::domain::{Priority, Task, TaskStatus};

/// Exports tasks to ICS (iCalendar) format.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if writing fails.
pub fn export_to_ics<W: Write>(tasks: &[Task], writer: &mut W) -> std::io::Result<()> {
    // Write calendar header
    writeln!(writer, "BEGIN:VCALENDAR")?;
    writeln!(writer, "VERSION:2.0")?;
    writeln!(writer, "PRODID:-//TaskFlow//TaskFlow TUI//EN")?;
    writeln!(writer, "CALSCALE:GREGORIAN")?;
    writeln!(writer, "METHOD:PUBLISH")?;

    for task in tasks {
        // Export as VTODO (task) component
        writeln!(writer, "BEGIN:VTODO")?;

        // UID (unique identifier)
        let uid = task.id.0;
        writeln!(writer, "UID:{uid}")?;

        // DTSTAMP (timestamp)
        let dtstamp = task.created_at.format("%Y%m%dT%H%M%SZ");
        writeln!(writer, "DTSTAMP:{dtstamp}")?;

        // CREATED
        writeln!(writer, "CREATED:{dtstamp}")?;

        // LAST-MODIFIED
        let last_modified = task.updated_at.format("%Y%m%dT%H%M%SZ");
        writeln!(writer, "LAST-MODIFIED:{last_modified}")?;

        // SUMMARY (title)
        writeln!(writer, "SUMMARY:{}", escape_ics(&task.title))?;

        // DESCRIPTION
        if let Some(ref desc) = task.description {
            writeln!(writer, "DESCRIPTION:{}", escape_ics(desc))?;
        }

        // DUE date
        if let Some(due) = task.due_date {
            writeln!(writer, "DUE;VALUE=DATE:{}", due.format("%Y%m%d"))?;
        }

        // STATUS
        let ics_status = match task.status {
            TaskStatus::Todo => "NEEDS-ACTION",
            TaskStatus::InProgress => "IN-PROCESS",
            TaskStatus::Blocked => "NEEDS-ACTION",
            TaskStatus::Done => "COMPLETED",
            TaskStatus::Cancelled => "CANCELLED",
        };
        writeln!(writer, "STATUS:{ics_status}")?;

        // PRIORITY (1-9 in ICS, 1 is highest)
        let ics_priority = match task.priority {
            Priority::Urgent => 1,
            Priority::High => 3,
            Priority::Medium => 5,
            Priority::Low => 7,
            Priority::None => 9,
        };
        writeln!(writer, "PRIORITY:{ics_priority}")?;

        // COMPLETED timestamp
        if let Some(completed) = task.completed_at {
            writeln!(writer, "COMPLETED:{}", completed.format("%Y%m%dT%H%M%SZ"))?;
        }

        // PERCENT-COMPLETE
        let percent = match task.status {
            TaskStatus::Todo => 0,
            TaskStatus::InProgress => 50,
            TaskStatus::Blocked => 25,
            TaskStatus::Done => 100,
            TaskStatus::Cancelled => 100,
        };
        writeln!(writer, "PERCENT-COMPLETE:{percent}")?;

        // CATEGORIES (tags)
        if !task.tags.is_empty() {
            writeln!(writer, "CATEGORIES:{}", task.tags.join(","))?;
        }

        writeln!(writer, "END:VTODO")?;
    }

    writeln!(writer, "END:VCALENDAR")?;
    Ok(())
}

/// Escape special characters for ICS format
fn escape_ics(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_task(title: &str) -> Task {
        Task::new(title)
    }

    #[test]
    fn test_escape_ics() {
        assert_eq!(escape_ics("hello"), "hello");
        assert_eq!(escape_ics("a;b"), "a\\;b");
        assert_eq!(escape_ics("a,b"), "a\\,b");
        assert_eq!(escape_ics("a\nb"), "a\\nb");
        assert_eq!(escape_ics("a\\b"), "a\\\\b");
    }

    #[test]
    fn test_export_ics_basic() {
        let tasks = vec![create_test_task("ICS Test")];
        let mut buffer = Vec::new();
        export_to_ics(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.starts_with("BEGIN:VCALENDAR"));
        assert!(result.contains("VERSION:2.0"));
        assert!(result.contains("BEGIN:VTODO"));
        assert!(result.contains("SUMMARY:ICS Test"));
        assert!(result.contains("STATUS:NEEDS-ACTION"));
        assert!(result.contains("END:VTODO"));
        assert!(result.ends_with("END:VCALENDAR\n"));
    }

    #[test]
    fn test_export_ics_with_priority() {
        let task = create_test_task("Urgent task").with_priority(Priority::Urgent);
        let tasks = vec![task];
        let mut buffer = Vec::new();
        export_to_ics(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.contains("PRIORITY:1"));
    }

    #[test]
    fn test_export_ics_with_high_priority() {
        let task = create_test_task("High task").with_priority(Priority::High);
        let tasks = vec![task];
        let mut buffer = Vec::new();
        export_to_ics(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.contains("PRIORITY:3"));
    }

    #[test]
    fn test_export_ics_completed() {
        let mut task = create_test_task("Completed task");
        task.status = TaskStatus::Done;
        task.completed_at = Some(Utc::now());

        let tasks = vec![task];
        let mut buffer = Vec::new();
        export_to_ics(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.contains("STATUS:COMPLETED"));
        assert!(result.contains("PERCENT-COMPLETE:100"));
        assert!(result.contains("COMPLETED:"));
    }

    #[test]
    fn test_export_ics_in_progress() {
        let mut task = create_test_task("Working task");
        task.status = TaskStatus::InProgress;

        let tasks = vec![task];
        let mut buffer = Vec::new();
        export_to_ics(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.contains("STATUS:IN-PROCESS"));
        assert!(result.contains("PERCENT-COMPLETE:50"));
    }

    #[test]
    fn test_export_ics_with_tags() {
        let mut task = create_test_task("Tagged task");
        task.tags = vec!["rust".to_string(), "tui".to_string()];

        let tasks = vec![task];
        let mut buffer = Vec::new();
        export_to_ics(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.contains("CATEGORIES:rust,tui"));
    }

    #[test]
    fn test_export_ics_with_description() {
        let task = create_test_task("Task").with_description("A detailed description");

        let tasks = vec![task];
        let mut buffer = Vec::new();
        export_to_ics(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.contains("DESCRIPTION:A detailed description"));
    }

    #[test]
    fn test_export_ics_empty() {
        let tasks: Vec<Task> = vec![];
        let mut buffer = Vec::new();
        export_to_ics(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.starts_with("BEGIN:VCALENDAR"));
        assert!(result.ends_with("END:VCALENDAR\n"));
        assert!(!result.contains("BEGIN:VTODO"));
    }

    // ========================================================================
    // Special Character Edge Cases
    // ========================================================================

    #[test]
    fn test_export_ics_special_characters() {
        // Task with semicolons, commas, backslashes, and newlines
        // Note: Colons don't need escaping in ICS property values
        let mut task = create_test_task("Meeting; with, special\\chars");
        task.description = Some("Line1\nLine2\nLine3".to_string());

        let tasks = vec![task];
        let mut buffer = Vec::new();
        export_to_ics(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        // ICS escaping: semicolon, comma, backslash, newlines
        // Colons are NOT escaped in ICS as they're valid in property values
        assert!(result.contains("Meeting\\; with\\, special\\\\chars"));
        assert!(result.contains("Line1\\nLine2\\nLine3"));
    }

    #[test]
    fn test_export_ics_unicode() {
        let mut task = create_test_task("会议 📅 Meeting");
        task.description = Some("日本語の説明".to_string());
        task.tags = vec!["工作".to_string(), "日程".to_string()];

        let tasks = vec![task];
        let mut buffer = Vec::new();
        export_to_ics(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.contains("会议"));
        assert!(result.contains("📅"));
        assert!(result.contains("日本語の説明"));
        // Tags with commas should be escaped
        assert!(
            result.contains("CATEGORIES:工作\\,日程") || result.contains("CATEGORIES:工作,日程")
        );
    }

    #[test]
    fn test_export_ics_combined_escaping() {
        // Test all escape characters combined
        assert_eq!(escape_ics("a\\b;c,d\ne"), "a\\\\b\\;c\\,d\\ne");
    }

    #[test]
    fn test_export_ics_structure_validity() {
        let mut task = create_test_task("Test");
        task.due_date = Some(chrono::NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());
        task.tags = vec!["tag1".to_string()];
        task.description = Some("Description".to_string());

        let tasks = vec![task];
        let mut buffer = Vec::new();
        export_to_ics(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        // Verify ICS structure
        assert!(result.starts_with("BEGIN:VCALENDAR"));
        assert!(result.contains("VERSION:2.0"));
        assert!(result.contains("BEGIN:VTODO"));
        assert!(result.contains("UID:"));
        assert!(result.contains("DTSTAMP:"));
        assert!(result.contains("SUMMARY:"));
        assert!(result.contains("STATUS:"));
        assert!(result.contains("PRIORITY:"));
        assert!(result.contains("PERCENT-COMPLETE:"));
        assert!(result.contains("DUE;VALUE=DATE:20250615"));
        assert!(result.contains("DESCRIPTION:"));
        assert!(result.contains("CATEGORIES:"));
        assert!(result.contains("END:VTODO"));
        assert!(result.ends_with("END:VCALENDAR\n"));

        // Every BEGIN should have matching END
        let begin_count = result.matches("BEGIN:").count();
        let end_count = result.matches("END:").count();
        assert_eq!(begin_count, end_count);
    }
}
