//! Import functionality for CSV and ICS files.
//!
//! This module provides parsing and import capabilities for task data
//! from CSV (spreadsheet) and ICS (iCalendar) formats.
//!
//! ## Supported Formats
//!
//! - **CSV**: Standard comma-separated values with header row
//! - **ICS**: iCalendar VTODO components
//!
//! ## Example
//!
//! ```no_run
//! use taskflow::storage::import::{import_from_csv, ImportOptions, MergeStrategy};
//! use std::io::BufReader;
//! use std::fs::File;
//!
//! let file = File::open("tasks.csv").unwrap();
//! let reader = BufReader::new(file);
//! let options = ImportOptions::default();
//!
//! let result = import_from_csv(reader, &options).unwrap();
//! println!("Imported {} tasks", result.imported.len());
//! ```

use std::collections::{HashMap, HashSet};
use std::io::BufRead;

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use uuid::Uuid;

use crate::domain::{Priority, Task, TaskId, TaskStatus};
use crate::storage::{StorageError, StorageResult};

/// Import format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFormat {
    /// Comma-separated values
    Csv,
    /// iCalendar format (VTODO components)
    Ics,
}

impl ImportFormat {
    /// Parse an import format from a string (case-insensitive)
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "ics" | "ical" | "icalendar" => Some(Self::Ics),
            _ => None,
        }
    }

    /// Get the file extension for this format
    #[must_use]
    pub const fn file_extension(&self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Ics => "ics",
        }
    }
}

/// Strategy for handling duplicate tasks during import
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MergeStrategy {
    /// Skip duplicates, keeping existing tasks
    #[default]
    Skip,
    /// Overwrite existing tasks with imported data
    Overwrite,
    /// Always create new tasks with new IDs
    CreateNew,
}

/// Options for import operations
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// How to handle duplicates
    pub merge_strategy: MergeStrategy,
    /// Whether to validate imported data
    pub validate: bool,
    /// If true, parse but don't actually import (preview mode)
    pub dry_run: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            merge_strategy: MergeStrategy::Skip,
            validate: true,
            dry_run: false,
        }
    }
}

/// Reason a task was skipped during import
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportSkipReason {
    /// Task already exists (by ID)
    DuplicateId(TaskId),
    /// Task already exists (by title + due date)
    DuplicateTitleDate {
        title: String,
        due_date: Option<NaiveDate>,
    },
    /// Task failed validation
    ValidationFailed(String),
}

/// Error that occurred during import of a specific row/entry
#[derive(Debug, Clone)]
pub struct ImportError {
    /// Line number or entry index (1-based)
    pub line: usize,
    /// Error message
    pub message: String,
    /// Raw line/entry content (if available)
    pub content: Option<String>,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Line {}: {}", self.line, self.message)
    }
}

/// Result of an import operation
#[derive(Debug, Default)]
pub struct ImportResult {
    /// Successfully parsed tasks
    pub imported: Vec<Task>,
    /// Tasks that were skipped (with reason)
    pub skipped: Vec<(Task, ImportSkipReason)>,
    /// Errors that occurred during parsing
    pub errors: Vec<ImportError>,
}

impl ImportResult {
    /// Returns the total number of tasks processed
    #[must_use]
    pub fn total_processed(&self) -> usize {
        self.imported.len() + self.skipped.len() + self.errors.len()
    }

    /// Returns true if there were any errors
    #[must_use]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Returns true if any tasks were successfully imported
    #[must_use]
    pub fn has_imported(&self) -> bool {
        !self.imported.is_empty()
    }
}

/// Duplicate detector for import operations
pub struct DuplicateDetector<'a> {
    /// Existing tasks by ID
    existing_by_id: &'a HashMap<TaskId, Task>,
    /// Index of (lowercase title, due_date) -> TaskId for duplicate detection
    title_date_index: HashMap<(String, Option<NaiveDate>), TaskId>,
}

impl<'a> DuplicateDetector<'a> {
    /// Create a new duplicate detector from existing tasks
    #[must_use]
    pub fn new(existing_tasks: &'a HashMap<TaskId, Task>) -> Self {
        let mut title_date_index = HashMap::new();
        for task in existing_tasks.values() {
            let key = (task.title.to_lowercase(), task.due_date);
            title_date_index.insert(key, task.id.clone());
        }
        Self {
            existing_by_id: existing_tasks,
            title_date_index,
        }
    }

    /// Check if a task is a duplicate
    pub fn check(&self, task: &Task) -> Option<ImportSkipReason> {
        // Check by ID first
        if self.existing_by_id.contains_key(&task.id) {
            return Some(ImportSkipReason::DuplicateId(task.id.clone()));
        }

        // Check by title + due date
        let key = (task.title.to_lowercase(), task.due_date);
        if self.title_date_index.contains_key(&key) {
            return Some(ImportSkipReason::DuplicateTitleDate {
                title: task.title.clone(),
                due_date: task.due_date,
            });
        }

        None
    }
}

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
                message: format!("Duplicate column name: '{}'", header),
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
        .map(|s| s.replace("\"\"", "\"").to_string());

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

/// Import tasks from ICS (iCalendar) format
///
/// Parses VTODO components from an iCalendar file.
///
/// # Errors
///
/// Returns a [`StorageError`] if the file cannot be read or parsed.
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
    let status = props
        .get("STATUS")
        .map(|s| match s.to_uppercase().as_str() {
            "NEEDS-ACTION" => TaskStatus::Todo,
            "IN-PROCESS" => TaskStatus::InProgress,
            "COMPLETED" => TaskStatus::Done,
            "CANCELLED" => TaskStatus::Cancelled,
            _ => TaskStatus::Todo,
        })
        .unwrap_or(TaskStatus::Todo);

    // Parse PRIORITY (1-9 in ICS, 1 is highest)
    let priority = props
        .get("PRIORITY")
        .and_then(|s| s.parse::<u8>().ok())
        .map(|p| match p {
            1 => Priority::Urgent,
            2..=3 => Priority::High,
            4..=6 => Priority::Medium,
            7..=8 => Priority::Low,
            _ => Priority::None,
        })
        .unwrap_or(Priority::None);

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

/// Apply duplicate detection and merge strategy to import results
pub fn apply_merge_strategy(
    result: &mut ImportResult,
    existing_tasks: &HashMap<TaskId, Task>,
    strategy: MergeStrategy,
) {
    if strategy == MergeStrategy::CreateNew {
        // Generate new IDs for all imported tasks
        for task in &mut result.imported {
            task.id = TaskId::new();
        }
        return;
    }

    let detector = DuplicateDetector::new(existing_tasks);
    let mut new_imported = Vec::new();

    for task in result.imported.drain(..) {
        if let Some(reason) = detector.check(&task) {
            match strategy {
                MergeStrategy::Skip => {
                    result.skipped.push((task, reason));
                }
                MergeStrategy::Overwrite => {
                    // Allow overwrite - task will replace existing
                    new_imported.push(task);
                }
                MergeStrategy::CreateNew => unreachable!(), // Handled above
            }
        } else {
            new_imported.push(task);
        }
    }

    result.imported = new_imported;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_import_format_parse() {
        assert_eq!(ImportFormat::parse("csv"), Some(ImportFormat::Csv));
        assert_eq!(ImportFormat::parse("CSV"), Some(ImportFormat::Csv));
        assert_eq!(ImportFormat::parse("ics"), Some(ImportFormat::Ics));
        assert_eq!(ImportFormat::parse("ical"), Some(ImportFormat::Ics));
        assert_eq!(ImportFormat::parse("unknown"), None);
    }

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
    fn test_duplicate_detector_by_id() {
        let mut existing = HashMap::new();
        let task = Task::new("Existing task");
        let task_id = task.id.clone();
        existing.insert(task.id.clone(), task);

        let detector = DuplicateDetector::new(&existing);

        let mut import_task = Task::new("New task");
        import_task.id = task_id.clone();

        let result = detector.check(&import_task);
        assert!(matches!(result, Some(ImportSkipReason::DuplicateId(_))));
    }

    #[test]
    fn test_duplicate_detector_by_title_date() {
        let mut existing = HashMap::new();
        let mut task = Task::new("Existing task");
        task.due_date = Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap());
        existing.insert(task.id.clone(), task);

        let detector = DuplicateDetector::new(&existing);

        let mut import_task = Task::new("EXISTING TASK"); // Different case
        import_task.due_date = Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap());

        let result = detector.check(&import_task);
        assert!(matches!(
            result,
            Some(ImportSkipReason::DuplicateTitleDate { .. })
        ));
    }

    #[test]
    fn test_merge_strategy_skip() {
        let mut existing = HashMap::new();
        let task = Task::new("Existing task");
        existing.insert(task.id.clone(), task.clone());

        let mut result = ImportResult::default();
        let mut import_task = Task::new("Different task");
        import_task.id = task.id.clone(); // Same ID
        result.imported.push(import_task);

        apply_merge_strategy(&mut result, &existing, MergeStrategy::Skip);

        assert!(result.imported.is_empty());
        assert_eq!(result.skipped.len(), 1);
    }

    #[test]
    fn test_merge_strategy_create_new() {
        let existing = HashMap::new();

        let mut result = ImportResult::default();
        let task = Task::new("New task");
        let original_id = task.id.clone();
        result.imported.push(task);

        apply_merge_strategy(&mut result, &existing, MergeStrategy::CreateNew);

        assert_eq!(result.imported.len(), 1);
        assert_ne!(result.imported[0].id, original_id); // ID should be different
    }

    #[test]
    fn test_import_result_methods() {
        let mut result = ImportResult::default();

        assert_eq!(result.total_processed(), 0);
        assert!(!result.has_errors());
        assert!(!result.has_imported());

        result.imported.push(Task::new("Task 1"));
        assert_eq!(result.total_processed(), 1);
        assert!(result.has_imported());

        result.errors.push(ImportError {
            line: 1,
            message: "test".to_string(),
            content: None,
        });
        assert!(result.has_errors());
        assert_eq!(result.total_processed(), 2);
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
