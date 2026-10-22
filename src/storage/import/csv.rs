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
/// - Estimated Minutes (optional) - whole minutes; `estimated_minutes` and
///   `estimate` are accepted too
/// - Actual Minutes (optional) - whole minutes tracked; `actual_minutes` and
///   `actual` are accepted too
///
/// Columns are matched by header name, so unknown columns are ignored.
///
/// Records follow RFC 4180: a field wrapped in double quotes may contain
/// commas, line breaks and `""` for a literal quote; records end with LF or
/// CRLF. A leading UTF-8 byte order mark is ignored.
///
/// # Errors
///
/// Returns a [`StorageError`] if the file cannot be read, is empty, or has
/// an unusable header row.
pub fn import_from_csv<R: BufRead>(
    mut reader: R,
    options: &ImportOptions,
) -> StorageResult<ImportResult> {
    let mut result = ImportResult::default();

    let mut input = String::new();
    reader
        .read_to_string(&mut input)
        .map_err(|e| StorageError::Deserialization {
            message: format!("Failed to read CSV: {e}"),
        })?;
    let input = input.strip_prefix('\u{feff}').unwrap_or(&input);

    let mut records = read_csv_records(input).into_iter();

    // Parse header
    let header = records
        .next()
        .ok_or_else(|| StorageError::Deserialization {
            message: "Empty CSV file".to_string(),
        })?;
    let headers: Vec<&str> = header.fields.iter().map(String::as_str).collect();

    // Validate headers before processing
    validate_csv_headers(&headers)?;

    let column_map = build_column_map(&headers);

    // Process data records (a record may span several lines)
    for record in records {
        if record.raw.trim().is_empty() {
            continue;
        }

        match parse_csv_task(&record.fields, &column_map, options.validate) {
            Ok(task) => result.imported.push(task),
            Err(e) => {
                result.errors.push(ImportError {
                    line: record.line,
                    message: e,
                    content: Some(record.raw),
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

/// One CSV record: the 1-based line it starts on, its raw text and its fields.
struct CsvRecord {
    line: usize,
    raw: String,
    fields: Vec<String>,
}

/// Split `input` into RFC 4180 records.
///
/// Fields are separated by commas and records by LF or CRLF. A field wrapped
/// in double quotes may contain commas, line breaks and `""` for a literal
/// quote; its content is kept verbatim. Unquoted fields are trimmed.
fn read_csv_records(input: &str) -> Vec<CsvRecord> {
    let mut records = Vec::new();
    let mut chars = input.chars().peekable();
    let mut line = 1;

    while chars.peek().is_some() {
        let start_line = line;
        let mut raw = String::new();
        let mut fields = Vec::new();
        let mut field = String::new();
        // The current field was opened with a quote (so it is not trimmed).
        let mut quoted = false;
        let mut in_quotes = false;

        loop {
            let Some(c) = chars.next() else {
                // End of input ends the last record.
                fields.push(finish_field(field, quoted));
                break;
            };

            if in_quotes {
                raw.push(c);
                match c {
                    '"' if chars.peek() == Some(&'"') => {
                        chars.next();
                        raw.push('"');
                        field.push('"');
                    }
                    '"' => in_quotes = false,
                    '\n' => {
                        line += 1;
                        field.push(c);
                    }
                    _ => field.push(c),
                }
                continue;
            }

            match c {
                '"' if !quoted && field.trim().is_empty() => {
                    raw.push(c);
                    field.clear();
                    quoted = true;
                    in_quotes = true;
                }
                ',' => {
                    raw.push(c);
                    fields.push(finish_field(std::mem::take(&mut field), quoted));
                    quoted = false;
                }
                '\r' | '\n' => {
                    if c == '\r' && chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    line += 1;
                    fields.push(finish_field(field, quoted));
                    break;
                }
                _ => {
                    raw.push(c);
                    // Whitespace after a closing quote is not part of the field.
                    if !(quoted && c.is_whitespace()) {
                        field.push(c);
                    }
                }
            }
        }

        records.push(CsvRecord {
            line: start_line,
            raw,
            fields,
        });
    }

    records
}

/// Finish a field: quoted content is kept as written, unquoted is trimmed.
fn finish_field(field: String, quoted: bool) -> String {
    if quoted {
        field
    } else {
        field.trim().to_string()
    }
}

/// Parse a CSV row into a Task
fn parse_csv_task(
    fields: &[String],
    column_map: &HashMap<String, usize>,
    validate: bool,
) -> Result<Task, String> {
    // Get field by column name
    let get_field = |name: &str| -> Option<&str> {
        column_map
            .get(name)
            .and_then(|&i| fields.get(i))
            .map(String::as_str)
    };

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

    // Parse description (the record reader has already unescaped quotes)
    let description = get_field("description")
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    // Parse created timestamp
    let created_at = get_field("created")
        .and_then(parse_datetime)
        .unwrap_or_else(Utc::now);

    // Parse completed timestamp
    let completed_at = get_field("completed").and_then(parse_datetime);

    // Parse time columns (whole minutes; blank or unparsable means unset)
    let estimated_minutes = get_field("estimated minutes")
        .or_else(|| get_field("estimated_minutes"))
        .or_else(|| get_field("estimate"))
        .and_then(|s| s.trim().parse::<u32>().ok())
        .filter(|&m| m > 0);
    let actual_minutes = get_field("actual minutes")
        .or_else(|| get_field("actual_minutes"))
        .or_else(|| get_field("actual"))
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0);

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
        recurrence_options: crate::domain::RecurrenceOptions::default(),
        estimated_minutes,
        actual_minutes,
        sort_order: None,
        custom_fields: std::collections::HashMap::new(),
        created_at,
        updated_at: Utc::now(),
        completed_at,
        snooze_until: None,
        scheduled_start_time: None,
        scheduled_end_time: None,
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

    /// Fields of a single-record input.
    fn fields_of(input: &str) -> Vec<String> {
        let mut records = read_csv_records(input);
        assert_eq!(records.len(), 1, "expected one record in {input:?}");
        records.remove(0).fields
    }

    #[test]
    fn test_parse_csv_row_simple() {
        assert_eq!(fields_of("a,b,c"), ["a", "b", "c"]);
    }

    #[test]
    fn test_parse_csv_row_quoted() {
        assert_eq!(fields_of("\"a,b\",c,\"d\""), ["a,b", "c", "d"]);
    }

    #[test]
    fn test_read_csv_records_quoted_newlines_and_doubled_quotes() {
        let input = "Title,Description\n\"Line 1\nLine 2\",\"He said \"\"hi\"\"\"\nNext,plain\n";
        let records = read_csv_records(input);
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].fields, ["Title", "Description"]);
        assert_eq!(records[1].fields, ["Line 1\nLine 2", "He said \"hi\""]);
        assert_eq!(records[1].line, 2);
        assert_eq!(records[2].fields, ["Next", "plain"]);
        assert_eq!(
            records[2].line, 4,
            "line numbers count the embedded newline"
        );
        assert_eq!(
            records[1].raw, "\"Line 1\nLine 2\",\"He said \"\"hi\"\"\"",
            "raw keeps the record as written"
        );
    }

    #[test]
    fn test_read_csv_records_line_endings_and_edges() {
        // CRLF and LF both end a record; the last record needs no newline.
        let records = read_csv_records("a,b\r\nc,d\ne,f");
        assert_eq!(records.len(), 3);
        assert_eq!(records[1].fields, ["c", "d"]);
        assert_eq!(records[2].fields, ["e", "f"]);
        assert_eq!(records[2].line, 3);

        // Empty fields, trailing comma and whitespace handling.
        assert_eq!(fields_of("a,,c,"), ["a", "", "c", ""]);
        assert_eq!(fields_of("  a , \" b \" ,c "), ["a", " b ", "c"]);
        assert_eq!(fields_of("\"\""), [""]);
        assert_eq!(fields_of("\"\"\"\""), ["\""]);

        // A blank line is a record with one empty field.
        let records = read_csv_records("a\n\nb\n");
        assert_eq!(records.len(), 3);
        assert_eq!(records[1].fields, [""]);
        assert!(records[1].raw.is_empty());
    }

    #[test]
    fn test_import_csv_crlf_line_endings() {
        let csv = "Title,Status\r\nA,todo\r\nB,done\r\n";
        let result = import_from_csv(Cursor::new(csv), &ImportOptions::default()).unwrap();
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.imported.len(), 2);
        assert_eq!(result.imported[0].title, "A");
        assert_eq!(result.imported[1].status, TaskStatus::Done);
    }

    #[test]
    fn test_import_csv_quoted_newline_and_quotes_in_fields() {
        let csv = "Title,Description\n\"Say \"\"hi\"\"\",\"First line\nSecond, line\"\n";
        let result = import_from_csv(Cursor::new(csv), &ImportOptions::default()).unwrap();
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.imported.len(), 1);
        assert_eq!(result.imported[0].title, "Say \"hi\"");
        assert_eq!(
            result.imported[0].description.as_deref(),
            Some("First line\nSecond, line")
        );
    }

    #[test]
    fn test_import_csv_error_line_numbers_span_multiline_records() {
        let csv = "Title,Description\n\"A\nspans\ntwo lines\",x\n,missing title\n";
        let result = import_from_csv(Cursor::new(csv), &ImportOptions::default()).unwrap();
        assert_eq!(result.imported.len(), 1);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].line, 5);
        assert_eq!(result.errors[0].content.as_deref(), Some(",missing title"));
    }

    #[test]
    fn test_import_csv_skips_blank_lines_and_bom() {
        let csv = "\u{feff}Title,Status\nA,todo\n\n   \nB,done\n";
        let result = import_from_csv(Cursor::new(csv), &ImportOptions::default()).unwrap();
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.imported.len(), 2);
        assert_eq!(result.imported[1].title, "B");
    }

    #[test]
    fn test_import_csv_quoted_header_names() {
        let csv = "\"Title\",\"Due Date\"\nA,2025-01-02\n";
        let result = import_from_csv(Cursor::new(csv), &ImportOptions::default()).unwrap();
        assert_eq!(result.imported.len(), 1);
        assert_eq!(
            result.imported[0].due_date,
            NaiveDate::from_ymd_opt(2025, 1, 2)
        );
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
    fn test_import_csv_time_columns_by_header_name() {
        let csv = "Title,Estimated Minutes,Actual Minutes\nA,90,75\nB,,\nC,abc,-5\nD,0,0\n";
        let result = import_from_csv(Cursor::new(csv), &ImportOptions::default()).unwrap();

        assert_eq!(result.imported.len(), 4);
        assert_eq!(result.imported[0].estimated_minutes, Some(90));
        assert_eq!(result.imported[0].actual_minutes, 75);
        assert_eq!(result.imported[1].estimated_minutes, None);
        assert_eq!(result.imported[1].actual_minutes, 0);
        assert_eq!(result.imported[2].estimated_minutes, None);
        assert_eq!(result.imported[2].actual_minutes, 0);
        // A zero estimate means "no estimate".
        assert_eq!(result.imported[3].estimated_minutes, None);

        // Snake-case and short aliases are accepted too.
        let csv = "Title,estimated_minutes,actual_minutes\nA,30,10\n";
        let result = import_from_csv(Cursor::new(csv), &ImportOptions::default()).unwrap();
        assert_eq!(result.imported[0].estimated_minutes, Some(30));
        assert_eq!(result.imported[0].actual_minutes, 10);
        let csv = "Title,Estimate,Actual\nA,45,20\n";
        let result = import_from_csv(Cursor::new(csv), &ImportOptions::default()).unwrap();
        assert_eq!(result.imported[0].estimated_minutes, Some(45));
        assert_eq!(result.imported[0].actual_minutes, 20);
    }

    #[test]
    fn test_import_csv_time_columns_round_trip() {
        use crate::storage::export_to_csv;

        let mut timed = Task::new("Timed").with_estimated_minutes(120);
        timed.actual_minutes = 95;
        let untimed = Task::new("Untimed");

        let mut buffer = Vec::new();
        export_to_csv(&[timed.clone(), untimed.clone()], &mut buffer).unwrap();
        let result = import_from_csv(Cursor::new(buffer), &ImportOptions::default()).unwrap();

        assert_eq!(result.imported.len(), 2);
        assert!(result.errors.is_empty());
        assert_eq!(result.imported[0].id, timed.id);
        assert_eq!(result.imported[0].estimated_minutes, Some(120));
        assert_eq!(result.imported[0].actual_minutes, 95);
        assert_eq!(result.imported[1].id, untimed.id);
        assert_eq!(result.imported[1].estimated_minutes, None);
        assert_eq!(result.imported[1].actual_minutes, 0);
    }

    #[test]
    fn test_import_csv_ignores_unknown_and_trailing_columns() {
        let csv = "Title,Status,Extra\nA,todo,whatever\nB,done,x,y,z\n";
        let result = import_from_csv(Cursor::new(csv), &ImportOptions::default()).unwrap();
        assert_eq!(result.imported.len(), 2);
        assert!(result.errors.is_empty());
        assert_eq!(result.imported[1].status, TaskStatus::Done);
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
