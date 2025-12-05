use std::io::Write;

use crate::domain::{Priority, Task, TaskStatus};

/// Export format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Ics,
}

impl ExportFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "ics" | "ical" | "icalendar" => Some(Self::Ics),
            _ => None,
        }
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Ics => "ics",
        }
    }
}

/// Export tasks to CSV format
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
        let due_date = task
            .due_date
            .map(|d| d.to_string())
            .unwrap_or_default();
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
            "{},{},{},{},{},{},{},{},{},{}",
            id, title, status, priority, due_date, tags, project_id, description, created, completed
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

/// Export tasks to ICS (iCalendar) format
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
        writeln!(writer, "UID:{}", task.id.0)?;

        // DTSTAMP (timestamp)
        let dtstamp = task.created_at.format("%Y%m%dT%H%M%SZ");
        writeln!(writer, "DTSTAMP:{}", dtstamp)?;

        // CREATED
        writeln!(writer, "CREATED:{}", dtstamp)?;

        // LAST-MODIFIED
        let last_modified = task.updated_at.format("%Y%m%dT%H%M%SZ");
        writeln!(writer, "LAST-MODIFIED:{}", last_modified)?;

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
        writeln!(writer, "STATUS:{}", ics_status)?;

        // PRIORITY (1-9 in ICS, 1 is highest)
        let ics_priority = match task.priority {
            Priority::Urgent => 1,
            Priority::High => 3,
            Priority::Medium => 5,
            Priority::Low => 7,
            Priority::None => 9,
        };
        writeln!(writer, "PRIORITY:{}", ics_priority)?;

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
        writeln!(writer, "PERCENT-COMPLETE:{}", percent)?;

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

/// Export tasks to a string in the specified format
pub fn export_to_string(tasks: &[Task], format: ExportFormat) -> std::io::Result<String> {
    let mut buffer = Vec::new();
    match format {
        ExportFormat::Csv => export_to_csv(tasks, &mut buffer)?,
        ExportFormat::Ics => export_to_ics(tasks, &mut buffer)?,
    }
    String::from_utf8(buffer).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Utc};

    fn create_test_task(title: &str) -> Task {
        Task::new(title)
    }

    #[test]
    fn test_export_format_parse() {
        assert_eq!(ExportFormat::parse("csv"), Some(ExportFormat::Csv));
        assert_eq!(ExportFormat::parse("CSV"), Some(ExportFormat::Csv));
        assert_eq!(ExportFormat::parse("ics"), Some(ExportFormat::Ics));
        assert_eq!(ExportFormat::parse("ical"), Some(ExportFormat::Ics));
        assert_eq!(ExportFormat::parse("unknown"), None);
    }

    #[test]
    fn test_export_format_extension() {
        assert_eq!(ExportFormat::Csv.file_extension(), "csv");
        assert_eq!(ExportFormat::Ics.file_extension(), "ics");
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
    fn test_escape_ics() {
        assert_eq!(escape_ics("hello"), "hello");
        assert_eq!(escape_ics("a;b"), "a\\;b");
        assert_eq!(escape_ics("a,b"), "a\\,b");
        assert_eq!(escape_ics("a\nb"), "a\\nb");
    }

    #[test]
    fn test_export_csv_basic() {
        let tasks = vec![create_test_task("Test Task")];
        let result = export_to_string(&tasks, ExportFormat::Csv).unwrap();

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
        let result = export_to_string(&tasks, ExportFormat::Csv).unwrap();

        assert!(result.contains("2025-06-15"));
    }

    #[test]
    fn test_export_csv_with_tags() {
        let mut task = create_test_task("Tagged task");
        task.tags = vec!["rust".to_string(), "tui".to_string()];

        let tasks = vec![task];
        let result = export_to_string(&tasks, ExportFormat::Csv).unwrap();

        assert!(result.contains("rust;tui"));
    }

    #[test]
    fn test_export_ics_basic() {
        let tasks = vec![create_test_task("ICS Test")];
        let result = export_to_string(&tasks, ExportFormat::Ics).unwrap();

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
        let result = export_to_string(&tasks, ExportFormat::Ics).unwrap();

        assert!(result.contains("PRIORITY:1"));
    }

    #[test]
    fn test_export_ics_completed() {
        let mut task = create_test_task("Completed task");
        task.status = TaskStatus::Done;
        task.completed_at = Some(Utc::now());

        let tasks = vec![task];
        let result = export_to_string(&tasks, ExportFormat::Ics).unwrap();

        assert!(result.contains("STATUS:COMPLETED"));
        assert!(result.contains("PERCENT-COMPLETE:100"));
        assert!(result.contains("COMPLETED:"));
    }

    #[test]
    fn test_export_ics_with_due_date() {
        let mut task = create_test_task("Due task");
        task.due_date = Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap());

        let tasks = vec![task];
        let result = export_to_string(&tasks, ExportFormat::Ics).unwrap();

        assert!(result.contains("DUE;VALUE=DATE:20251225"));
    }

    #[test]
    fn test_export_ics_with_tags() {
        let mut task = create_test_task("Tagged task");
        task.tags = vec!["work".to_string(), "urgent".to_string()];

        let tasks = vec![task];
        let result = export_to_string(&tasks, ExportFormat::Ics).unwrap();

        assert!(result.contains("CATEGORIES:work,urgent"));
    }

    #[test]
    fn test_export_empty_tasks() {
        let tasks: Vec<Task> = vec![];

        let csv_result = export_to_string(&tasks, ExportFormat::Csv).unwrap();
        assert!(csv_result.starts_with("ID,Title,Status")); // Header only

        let ics_result = export_to_string(&tasks, ExportFormat::Ics).unwrap();
        assert!(ics_result.contains("BEGIN:VCALENDAR"));
        assert!(ics_result.contains("END:VCALENDAR"));
        assert!(!ics_result.contains("BEGIN:VTODO")); // No tasks
    }
}
