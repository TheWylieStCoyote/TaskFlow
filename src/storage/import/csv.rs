//! CSV import functionality.

use std::collections::{HashMap, HashSet};
use std::io::BufRead;

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use uuid::Uuid;

use crate::domain::{Priority, Task, TaskId, TaskStatus};
use crate::storage::{StorageError, StorageResult};

use super::types::{ImportError, ImportOptions, ImportResult};

/// Import tasks from CSV format
///
/// Expected CSV columns (header required):
/// - ID (optional) - UUID, generates new if missing/invalid
/// - Title (required) - Task title
/// - Status (optional) - todo, in_progress, blocked, done, cancelled
/// - Priority (optional) - none, low, medium, high, urgent
/// - Due Date (optional) - YYYY-MM-DD format
/// - Tags (optional) - Semicolon-separated
/// - Project ID (optional) - UUID
/// - Description (optional)
/// - Created (optional) - YYYY-MM-DD HH:MM:SS
/// - Completed (optional) - YYYY-MM-DD HH:MM:SS
///
/// # Errors
///
/// Returns a [`StorageError`] if the file cannot be read.
pub fn import_from_csv<R: BufRead>(
    reader: R,
    options: &ImportOptions,
) -> StorageResult<ImportResult> {
    let mut result = ImportResult::default();
    let mut lines = reader.lines();

    // Parse header
    let header_line = lines
        .next()
        .ok_or_else(|| StorageError::Deserialization {
            message: "Empty CSV file".to_string(),
        })?
        .map_err(|e| StorageError::Deserialization {
            message: format!("Failed to read header: {e}"),
        })?;

    let headers: Vec<&str> = parse_csv_row(&header_line);

    // Validate headers before processing
    validate_csv_headers(&headers)?;

    let column_map = build_column_map(&headers);

    // Process data rows
    for (line_num, line_result) in lines.enumerate() {
        let line_num = line_num + 2; // 1-based, after header

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

        if line.trim().is_empty() {
            continue;
        }

        let fields: Vec<&str> = parse_csv_row(&line);

        match parse_csv_task(&fields, &column_map, options.validate) {
            Ok(task) => result.imported.push(task),
            Err(e) => {
                result.errors.push(ImportError {
                    line: line_num,
                    message: e,
                    content: Some(line),
                });
            }
        }
    }

    Ok(result)
}

/// Build a map of column names to indices
fn build_column_map(headers: &[&str]) -> HashMap<String, usize> {
    headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h.to_lowercase().trim().to_string(), i))
        .collect()
}

/// Validate CSV headers for common issues
fn validate_csv_headers(headers: &[&str]) -> StorageResult<()> {
    // Check not empty
    if headers.is_empty() {
        return Err(StorageError::Deserialization {
            message: "CSV file has no columns".to_string(),
        });
    }

    // Check for required 'title' column (or common aliases)
    let has_title = headers.iter().any(|h| {
        let lower = h.to_lowercase();
        let trimmed = lower.trim();
        trimmed == "title" || trimmed == "name" || trimmed == "task" || trimmed == "summary"
    });
    if !has_title {
        return Err(StorageError::Deserialization {
            message: "CSV must have a 'title' column (or 'name', 'task', 'summary')".to_string(),
        });
    }

    // Check for duplicate column names
    let mut seen: HashSet<String> = HashSet::new();
    for header in headers {
        let normalized = header.to_lowercase().trim().to_string();
        if !normalized.is_empty() && !seen.insert(normalized.clone()) {
            return Err(StorageError::Deserialization {
                message: format!("Duplicate column name: '{header}'"),
            });
        }
    }

    Ok(())
}

/// Parse a single CSV row, handling quoted fields
fn parse_csv_row(line: &str) -> Vec<&str> {
    let mut fields = Vec::new();
    let mut current_start = 0;
    let mut in_quotes = false;
    let bytes = line.as_bytes();

    for (i, &byte) in bytes.iter().enumerate() {
        match byte {
            b'"' => {
                // Check for escaped quote
                if in_quotes && i + 1 < bytes.len() && bytes[i + 1] == b'"' {
                    continue;
                }
                in_quotes = !in_quotes;
            }
            b',' if !in_quotes => {
                fields.push(unescape_csv_field(&line[current_start..i]));
                current_start = i + 1;
            }
            _ => {}
        }
    }

    // Add final field
    fields.push(unescape_csv_field(&line[current_start..]));

    fields
}

/// Unescape a CSV field (remove surrounding quotes, unescape internal quotes)
fn unescape_csv_field(s: &str) -> &str {
    let trimmed = s.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    }
}

/// Parse a CSV row into a Task
fn parse_csv_task(
    fields: &[&str],
    column_map: &HashMap<String, usize>,
    validate: bool,
) -> Result<Task, String> {
    // Get field by column name
    let get_field =
        |name: &str| -> Option<&str> { column_map.get(name).and_then(|&i| fields.get(i).copied()) };

    // Title is required
    let title = get_field("title")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Missing or empty title".to_string())?;

    if validate && title.trim().is_empty() {
        return Err("Title cannot be whitespace only".to_string());
    }

    // Parse ID or generate new
    let id = get_field("id")
        .and_then(|s| Uuid::parse_str(s).ok())
        .map_or_else(TaskId::new, TaskId);

    // Parse status
    let status = get_field("status")
        .and_then(|s| match s.to_lowercase().as_str() {
            "todo" => Some(TaskStatus::Todo),
            "in_progress" | "inprogress" | "in progress" => Some(TaskStatus::InProgress),
            "blocked" => Some(TaskStatus::Blocked),
            "done" | "completed" => Some(TaskStatus::Done),
            "cancelled" | "canceled" => Some(TaskStatus::Cancelled),
            _ => None,
        })
        .unwrap_or(TaskStatus::Todo);

    // Parse priority
    let priority = get_field("priority")
        .and_then(|s| match s.to_lowercase().as_str() {
            "none" | "0" => Some(Priority::None),
            "low" | "1" => Some(Priority::Low),
            "medium" | "med" | "2" => Some(Priority::Medium),
            "high" | "3" => Some(Priority::High),
            "urgent" | "4" => Some(Priority::Urgent),
            _ => None,
        })
        .unwrap_or(Priority::None);

    // Parse due date
    let due_date = get_field("due date")
        .or_else(|| get_field("duedate"))
        .or_else(|| get_field("due"))
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    // Parse tags
    let tags: Vec<String> = get_field("tags")
        .map(|s| {
            s.split(';')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // Parse project ID
    let project_id = get_field("project id")
        .or_else(|| get_field("projectid"))
        .or_else(|| get_field("project"))
        .and_then(|s| Uuid::parse_str(s).ok())
        .map(crate::domain::ProjectId);

    // Parse description
    let description = get_field("description")
        .filter(|s| !s.is_empty())
        .map(|s| s.replace("\"\"", "\""));

    // Parse created timestamp
    let created_at = get_field("created")
        .and_then(parse_datetime)
        .unwrap_or_else(Utc::now);

    // Parse completed timestamp
    let completed_at = get_field("completed").and_then(parse_datetime);

    // Build task
    let mut task = Task {
        id,
        title: title.to_string(),
        description,
        status,
        priority,
        due_date,
        scheduled_date: None,
        tags,
        project_id,
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
        git_ref: None,
    };

    // Set completed_at if status is Done
    if task.status == TaskStatus::Done && task.completed_at.is_none() {
        task.completed_at = Some(Utc::now());
    }

    Ok(task)
}

/// Parse a datetime string in various formats
fn parse_datetime(s: &str) -> Option<DateTime<Utc>> {
    // Try YYYY-MM-DD HH:MM:SS format
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(Utc.from_utc_datetime(&dt));
    }

    // Try ISO 8601 format
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }

    // Try just date
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0)?));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_import_csv_basic() {
        let csv = "ID,Title,Status,Priority\n,Test Task,todo,none\n";
        let reader = Cursor::new(csv);
        let options = ImportOptions::default();

        let result = import_from_csv(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 1);
        assert_eq!(result.imported[0].title, "Test Task");
        assert_eq!(result.imported[0].status, TaskStatus::Todo);
        assert_eq!(result.imported[0].priority, Priority::None);
    }

    #[test]
    fn test_import_csv_with_due_date() {
        let csv = "Title,Due Date\nTask with date,2025-12-25\n";
        let reader = Cursor::new(csv);
        let options = ImportOptions::default();

        let result = import_from_csv(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 1);
        assert_eq!(
            result.imported[0].due_date,
            Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap())
        );
    }

    #[test]
    fn test_import_csv_with_tags() {
        let csv = "Title,Tags\nTagged task,rust;tui;cli\n";
        let reader = Cursor::new(csv);
        let options = ImportOptions::default();

        let result = import_from_csv(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 1);
        assert_eq!(result.imported[0].tags, vec!["rust", "tui", "cli"]);
    }

    #[test]
    fn test_import_csv_quoted_fields() {
        let csv = "Title,Description\n\"Task with, comma\",\"Description with \"\"quotes\"\"\"\n";
        let reader = Cursor::new(csv);
        let options = ImportOptions::default();

        let result = import_from_csv(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 1);
        assert_eq!(result.imported[0].title, "Task with, comma");
        assert_eq!(
            result.imported[0].description,
            Some("Description with \"quotes\"".to_string())
        );
    }

    #[test]
    fn test_import_csv_missing_title() {
        let csv = "Title,Status\n,todo\n";
        let reader = Cursor::new(csv);
        let options = ImportOptions::default();

        let result = import_from_csv(reader, &options).unwrap();

        assert!(result.imported.is_empty());
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("title"));
    }

    #[test]
    fn test_import_csv_empty_file() {
        let csv = "";
        let reader = Cursor::new(csv);
        let options = ImportOptions::default();

        let result = import_from_csv(reader, &options);

        assert!(result.is_err());
    }

    #[test]
    fn test_import_csv_all_statuses() {
        let csv = "Title,Status\nTodo,todo\nIn Progress,in_progress\nBlocked,blocked\nDone,done\nCancelled,cancelled\n";
        let reader = Cursor::new(csv);
        let options = ImportOptions::default();

        let result = import_from_csv(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 5);
        assert_eq!(result.imported[0].status, TaskStatus::Todo);
        assert_eq!(result.imported[1].status, TaskStatus::InProgress);
        assert_eq!(result.imported[2].status, TaskStatus::Blocked);
        assert_eq!(result.imported[3].status, TaskStatus::Done);
        assert_eq!(result.imported[4].status, TaskStatus::Cancelled);
    }

    #[test]
    fn test_import_csv_all_priorities() {
        let csv = "Title,Priority\nNone,none\nLow,low\nMed,medium\nHigh,high\nUrgent,urgent\n";
        let reader = Cursor::new(csv);
        let options = ImportOptions::default();

        let result = import_from_csv(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 5);
        assert_eq!(result.imported[0].priority, Priority::None);
        assert_eq!(result.imported[1].priority, Priority::Low);
        assert_eq!(result.imported[2].priority, Priority::Medium);
        assert_eq!(result.imported[3].priority, Priority::High);
        assert_eq!(result.imported[4].priority, Priority::Urgent);
    }

    #[test]
    fn test_parse_csv_row_simple() {
        let row = "a,b,c";
        let fields: Vec<&str> = parse_csv_row(row);
        assert_eq!(fields, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_csv_row_quoted() {
        let row = "\"a,b\",c,\"d\"";
        let fields: Vec<&str> = parse_csv_row(row);
        assert_eq!(fields, vec!["a,b", "c", "d"]);
    }

    // Header validation tests
    #[test]
    fn test_validate_headers_valid() {
        // Valid headers with title
        assert!(validate_csv_headers(&["title", "status", "priority"]).is_ok());
        assert!(validate_csv_headers(&["name", "description"]).is_ok());
        assert!(validate_csv_headers(&["task", "due"]).is_ok());
        assert!(validate_csv_headers(&["summary", "tags"]).is_ok());
    }

    #[test]
    fn test_validate_headers_missing_title() {
        let result = validate_csv_headers(&["status", "priority", "description"]);
        assert!(result.is_err());
        if let Err(StorageError::Deserialization { message }) = result {
            assert!(message.contains("title"));
        }
    }

    #[test]
    fn test_validate_headers_empty() {
        let result = validate_csv_headers(&[]);
        assert!(result.is_err());
        if let Err(StorageError::Deserialization { message }) = result {
            assert!(message.contains("no columns"));
        }
    }

    #[test]
    fn test_validate_headers_duplicate() {
        let result = validate_csv_headers(&["title", "status", "Title"]);
        assert!(result.is_err());
        if let Err(StorageError::Deserialization { message }) = result {
            assert!(message.contains("Duplicate"));
        }
    }

    #[test]
    fn test_import_csv_no_title_column() {
        let csv = "status,priority,description\ntodo,high,A task\n";
        let reader = Cursor::new(csv);
        let options = ImportOptions::default();

        let result = import_from_csv(reader, &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_import_csv_duplicate_columns() {
        let csv = "title,status,Title\nTest,todo,Duplicate\n";
        let reader = Cursor::new(csv);
        let options = ImportOptions::default();

        let result = import_from_csv(reader, &options);
        assert!(result.is_err());
    }
}
