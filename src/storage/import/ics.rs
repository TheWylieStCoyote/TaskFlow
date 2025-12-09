//! ICS (iCalendar) import functionality.

use std::collections::HashMap;
use std::io::BufRead;

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use uuid::Uuid;

use crate::domain::{Priority, Task, TaskId, TaskStatus};
use crate::storage::StorageResult;

use super::types::{ImportError, ImportOptions, ImportResult};

/// Import tasks from ICS (iCalendar) format
///
/// Parses VTODO components from an iCalendar file.
///
/// # Errors
///
/// Returns a [`StorageError`](crate::storage::StorageError) if the file cannot be read or parsed.
pub fn import_from_ics<R: BufRead>(
    reader: R,
    options: &ImportOptions,
) -> StorageResult<ImportResult> {
    let mut result = ImportResult::default();
    let mut current_vtodo: Option<HashMap<String, String>> = None;
    let mut line_num = 0;
    let mut unfolded_line = String::new();

    for line_result in reader.lines() {
        line_num += 1;

        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                result.errors.push(ImportError {
                    line: line_num,
                    message: format!("Failed to read line: {e}"),
                    content: None,
                });
                continue;
            }
        };

        // Handle line folding (continuation lines start with space or tab)
        if line.starts_with(' ') || line.starts_with('\t') {
            unfolded_line.push_str(line.trim_start());
            continue;
        }

        // Process the previous unfolded line
        if !unfolded_line.is_empty() {
            process_ics_line(
                &unfolded_line,
                &mut current_vtodo,
                &mut result,
                line_num,
                options,
            );
        }

        unfolded_line = line;
    }

    // Process final line
    if !unfolded_line.is_empty() {
        process_ics_line(
            &unfolded_line,
            &mut current_vtodo,
            &mut result,
            line_num,
            options,
        );
    }

    Ok(result)
}

/// Process a single ICS line
fn process_ics_line(
    line: &str,
    current_vtodo: &mut Option<HashMap<String, String>>,
    result: &mut ImportResult,
    line_num: usize,
    options: &ImportOptions,
) {
    let trimmed = line.trim();

    if trimmed == "BEGIN:VTODO" {
        *current_vtodo = Some(HashMap::new());
        return;
    }

    if trimmed == "END:VTODO" {
        if let Some(props) = current_vtodo.take() {
            match parse_ics_vtodo(&props, options.validate) {
                Ok(task) => result.imported.push(task),
                Err(e) => {
                    result.errors.push(ImportError {
                        line: line_num,
                        message: e,
                        content: None,
                    });
                }
            }
        }
        return;
    }

    // Parse property if inside VTODO
    if let Some(ref mut props) = current_vtodo {
        if let Some((key, value)) = parse_ics_property(trimmed) {
            props.insert(key, value);
        }
    }
}

/// Parse an ICS property line (KEY:VALUE or KEY;PARAM=X:VALUE)
fn parse_ics_property(line: &str) -> Option<(String, String)> {
    let colon_pos = line.find(':')?;
    let key_part = &line[..colon_pos];
    let value = &line[colon_pos + 1..];

    // Extract just the property name (before any parameters)
    let key = key_part.split(';').next()?.to_uppercase();

    Some((key, unescape_ics(value)))
}

/// Unescape ICS special characters
fn unescape_ics(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\;", ";")
        .replace("\\,", ",")
        .replace("\\\\", "\\")
}

/// Parse a VTODO component into a Task
fn parse_ics_vtodo(props: &HashMap<String, String>, validate: bool) -> Result<Task, String> {
    // SUMMARY (title) is required
    let title = props
        .get("SUMMARY")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Missing SUMMARY (title)".to_string())?;

    if validate && title.trim().is_empty() {
        return Err("SUMMARY cannot be whitespace only".to_string());
    }

    // Parse UID or generate new
    let id = props
        .get("UID")
        .and_then(|s| Uuid::parse_str(s).ok())
        .map_or_else(TaskId::new, TaskId);

    // Parse STATUS
    let status =
        props
            .get("STATUS")
            .map_or(TaskStatus::Todo, |s| match s.to_uppercase().as_str() {
                "NEEDS-ACTION" => TaskStatus::Todo,
                "IN-PROCESS" => TaskStatus::InProgress,
                "COMPLETED" => TaskStatus::Done,
                "CANCELLED" => TaskStatus::Cancelled,
                _ => TaskStatus::Todo,
            });

    // Parse PRIORITY (1-9 in ICS, 1 is highest)
    let priority = props
        .get("PRIORITY")
        .and_then(|s| s.parse::<u8>().ok())
        .map_or(Priority::None, |p| match p {
            1 => Priority::Urgent,
            2..=3 => Priority::High,
            4..=6 => Priority::Medium,
            7..=8 => Priority::Low,
            _ => Priority::None,
        });

    // Parse DUE date
    let due_date = props.get("DUE").and_then(|s| parse_ics_date(s));

    // Parse DESCRIPTION
    let description = props.get("DESCRIPTION").filter(|s| !s.is_empty()).cloned();

    // Parse CATEGORIES (tags)
    let tags: Vec<String> = props
        .get("CATEGORIES")
        .map(|s| {
            s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // Parse CREATED timestamp
    let created_at = props
        .get("CREATED")
        .or_else(|| props.get("DTSTAMP"))
        .and_then(|s| parse_ics_datetime(s))
        .unwrap_or_else(Utc::now);

    // Parse COMPLETED timestamp
    let completed_at = props.get("COMPLETED").and_then(|s| parse_ics_datetime(s));

    let task = Task {
        id,
        title: title.clone(),
        description,
        status,
        priority,
        due_date,
        scheduled_date: None,
        tags,
        project_id: None,
        parent_task_id: None,
        dependencies: Vec::new(),
        next_task_id: None,
        recurrence: None,
        estimated_minutes: None,
        actual_minutes: 0,
        sort_order: None,
        custom_fields: std::collections::HashMap::new(),
        created_at,
        updated_at: Utc::now(),
        completed_at,
        snooze_until: None,
    };

    Ok(task)
}

/// Parse an ICS date (YYYYMMDD)
fn parse_ics_date(s: &str) -> Option<NaiveDate> {
    // Handle both "YYYYMMDD" and "VALUE=DATE:YYYYMMDD" formats
    let date_str = s.trim_start_matches("VALUE=DATE:");

    if date_str.len() >= 8 {
        let date_part = &date_str[..8];
        NaiveDate::parse_from_str(date_part, "%Y%m%d").ok()
    } else {
        None
    }
}

/// Parse an ICS datetime (YYYYMMDDTHHMMSSz or YYYYMMDDTHHMMSS)
fn parse_ics_datetime(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim().to_uppercase();

    // Try with Z suffix (UTC)
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y%m%dT%H%M%SZ") {
        return Some(Utc.from_utc_datetime(&dt));
    }

    // Try without Z
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y%m%dT%H%M%S") {
        return Some(Utc.from_utc_datetime(&dt));
    }

    // Try date only
    if let Some(d) = parse_ics_date(&s) {
        return Some(Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0)?));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_import_ics_basic() {
        let ics = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VTODO
UID:test-123
SUMMARY:Test Task
STATUS:NEEDS-ACTION
PRIORITY:5
END:VTODO
END:VCALENDAR
"#;
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 1);
        assert_eq!(result.imported[0].title, "Test Task");
        assert_eq!(result.imported[0].status, TaskStatus::Todo);
        assert_eq!(result.imported[0].priority, Priority::Medium);
    }

    #[test]
    fn test_import_ics_with_due_date() {
        let ics = r#"BEGIN:VCALENDAR
BEGIN:VTODO
SUMMARY:Task with due
DUE;VALUE=DATE:20251225
END:VTODO
END:VCALENDAR
"#;
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 1);
        assert_eq!(
            result.imported[0].due_date,
            Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap())
        );
    }

    #[test]
    fn test_import_ics_with_categories() {
        let ics = r#"BEGIN:VCALENDAR
BEGIN:VTODO
SUMMARY:Tagged task
CATEGORIES:work,urgent,project
END:VTODO
END:VCALENDAR
"#;
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 1);
        assert_eq!(result.imported[0].tags, vec!["work", "urgent", "project"]);
    }

    #[test]
    fn test_import_ics_completed() {
        let ics = r#"BEGIN:VCALENDAR
BEGIN:VTODO
SUMMARY:Completed task
STATUS:COMPLETED
COMPLETED:20251201T120000Z
END:VTODO
END:VCALENDAR
"#;
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 1);
        assert_eq!(result.imported[0].status, TaskStatus::Done);
        assert!(result.imported[0].completed_at.is_some());
    }

    #[test]
    fn test_import_ics_line_folding() {
        let ics = r#"BEGIN:VCALENDAR
BEGIN:VTODO
SUMMARY:Task with a very long title that spans
  multiple lines due to line folding
END:VTODO
END:VCALENDAR
"#;
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 1);
        assert!(result.imported[0].title.contains("long title"));
        assert!(result.imported[0].title.contains("multiple lines"));
    }

    #[test]
    fn test_import_ics_escaped_chars() {
        let ics = r#"BEGIN:VCALENDAR
BEGIN:VTODO
SUMMARY:Task with\, commas
DESCRIPTION:Line 1\nLine 2
END:VTODO
END:VCALENDAR
"#;
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 1);
        assert_eq!(result.imported[0].title, "Task with, commas");
        assert_eq!(
            result.imported[0].description,
            Some("Line 1\nLine 2".to_string())
        );
    }

    #[test]
    fn test_import_ics_multiple_vtodos() {
        let ics = r#"BEGIN:VCALENDAR
BEGIN:VTODO
SUMMARY:Task 1
END:VTODO
BEGIN:VTODO
SUMMARY:Task 2
END:VTODO
BEGIN:VTODO
SUMMARY:Task 3
END:VTODO
END:VCALENDAR
"#;
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 3);
        assert_eq!(result.imported[0].title, "Task 1");
        assert_eq!(result.imported[1].title, "Task 2");
        assert_eq!(result.imported[2].title, "Task 3");
    }

    #[test]
    fn test_parse_ics_date() {
        assert_eq!(
            parse_ics_date("20251225"),
            Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap())
        );
        assert_eq!(
            parse_ics_date("VALUE=DATE:20251225"),
            Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap())
        );
    }

    #[test]
    fn test_parse_ics_datetime() {
        let result = parse_ics_datetime("20251225T120000Z");
        assert!(result.is_some());

        let result = parse_ics_datetime("20251225T120000");
        assert!(result.is_some());
    }

    #[test]
    fn test_unescape_ics() {
        assert_eq!(unescape_ics("hello"), "hello");
        assert_eq!(unescape_ics("a\\;b"), "a;b");
        assert_eq!(unescape_ics("a\\,b"), "a,b");
        assert_eq!(unescape_ics("a\\nb"), "a\nb");
        assert_eq!(unescape_ics("a\\\\b"), "a\\b");
    }
}
