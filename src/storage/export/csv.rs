//! CSV export functionality.

use std::io::Write;

use crate::domain::Task;

/// Exports tasks to CSV format.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if writing fails.
pub fn export_to_csv<W: Write>(tasks: &[Task], writer: &mut W) -> std::io::Result<()> {
    // Write header
    writeln!(
        writer,
        "ID,Title,Status,Priority,Due Date,Tags,Project ID,Description,Created,Completed"
    )?;

    for task in tasks {
        let id = task.id.0.to_string();
        let title = escape_csv(&task.title);
        let status = task.status.as_str();
        let priority = task.priority.as_str();
        let due_date = task.due_date.map(|d| d.to_string()).unwrap_or_default();
        let tags = task.tags.join(";");
        let project_id = task
            .project_id
            .as_ref()
            .map(|p| p.0.to_string())
            .unwrap_or_default();
        let description = task
            .description
            .as_ref()
            .map(|d| escape_csv(d))
            .unwrap_or_default();
        let created = task.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
        let completed = task
            .completed_at
            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();

        writeln!(
            writer,
            "{id},{title},{status},{priority},{due_date},{tags},{project_id},{description},{created},{completed}"
        )?;
    }

    Ok(())
}

/// Escape a string for CSV (wrap in quotes if needed, escape internal quotes)
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Priority;
    use chrono::NaiveDate;

    fn create_test_task(title: &str) -> Task {
        Task::new(title)
    }

    #[test]
    fn test_escape_csv_simple() {
        assert_eq!(escape_csv("hello"), "hello");
    }

    #[test]
    fn test_escape_csv_with_comma() {
        assert_eq!(escape_csv("hello, world"), "\"hello, world\"");
    }

    #[test]
    fn test_escape_csv_with_quotes() {
        assert_eq!(escape_csv("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn test_escape_csv_with_newline() {
        assert_eq!(escape_csv("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn test_export_csv_basic() {
        let tasks = vec![create_test_task("Test Task")];
        let mut buffer = Vec::new();
        export_to_csv(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.starts_with("ID,Title,Status,Priority"));
        assert!(result.contains("Test Task"));
        assert!(result.contains("todo"));
        assert!(result.contains("none"));
    }

    #[test]
    fn test_export_csv_with_due_date() {
        let mut task = create_test_task("Task with date");
        task.due_date = Some(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());

        let tasks = vec![task];
        let mut buffer = Vec::new();
        export_to_csv(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.contains("2025-06-15"));
    }

    #[test]
    fn test_export_csv_with_tags() {
        let mut task = create_test_task("Tagged task");
        task.tags = vec!["rust".to_string(), "tui".to_string()];

        let tasks = vec![task];
        let mut buffer = Vec::new();
        export_to_csv(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.contains("rust;tui"));
    }

    #[test]
    fn test_export_csv_with_priority() {
        let task = create_test_task("Priority task").with_priority(Priority::High);

        let tasks = vec![task];
        let mut buffer = Vec::new();
        export_to_csv(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.contains("high"));
    }

    #[test]
    fn test_export_csv_empty() {
        let tasks: Vec<Task> = vec![];
        let mut buffer = Vec::new();
        export_to_csv(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        // Should only have header
        assert!(result.starts_with("ID,Title,Status,Priority"));
        assert_eq!(result.lines().count(), 1);
    }

    // ========================================================================
    // Round-trip and Special Character Edge Cases
    // ========================================================================

    #[test]
    fn test_export_csv_combined_special_chars() {
        // Task with comma, quotes, and newline all together
        let mut task = create_test_task("Task with, \"quotes\" and\nnewline");
        task.description = Some("Description with,\n\"special\" chars".to_string());

        let tasks = vec![task];
        let mut buffer = Vec::new();
        export_to_csv(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        // The title should be properly quoted and escaped
        assert!(result.contains("\"Task with, \"\"quotes\"\" and"));
        assert!(result.contains("newline\""));
    }

    #[test]
    fn test_export_csv_unicode_characters() {
        let mut task = create_test_task("会议 📅 Meeting");
        task.description = Some("日本語の説明".to_string());
        task.tags = vec!["工作".to_string(), "日程".to_string()];

        let tasks = vec![task];
        let mut buffer = Vec::new();
        export_to_csv(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.contains("会议"));
        assert!(result.contains("📅"));
        assert!(result.contains("日本語の説明"));
        assert!(result.contains("工作;日程"));
    }

    #[test]
    fn test_escape_csv_combined() {
        // All special chars together
        assert_eq!(escape_csv("a,b\"c\nd"), "\"a,b\"\"c\nd\"");
    }

    #[test]
    fn test_export_csv_empty_title() {
        // Edge case: empty title
        let task = create_test_task("");
        let tasks = vec![task];
        let mut buffer = Vec::new();
        export_to_csv(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        // Should have two commas together where title is
        let data_line = result.lines().nth(1).unwrap();
        // Format: ID,Title,Status... - empty title means ,, after ID
        assert!(data_line.contains(",,") || data_line.contains(",todo,")); // Empty title followed by status
    }
}
