//! Import types and options.

use chrono::NaiveDate;

use crate::domain::{CalendarEvent, Task, TaskId};

/// Import format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFormat {
    /// Comma-separated values
    Csv,
    /// iCalendar format (VTODO and VEVENT components)
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

impl std::fmt::Display for ImportSkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(f, "task with ID '{id}' already exists"),
            Self::DuplicateTitleDate { title, due_date } => {
                if let Some(date) = due_date {
                    write!(
                        f,
                        "task '{}' with due date {} already exists",
                        title,
                        date.format("%Y-%m-%d")
                    )
                } else {
                    write!(f, "task '{title}' already exists")
                }
            }
            Self::ValidationFailed(msg) => write!(f, "validation failed: {msg}"),
        }
    }
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
#[derive(Debug, Clone, Default)]
pub struct ImportResult {
    /// Successfully parsed tasks (from VTODO)
    pub imported: Vec<Task>,
    /// Successfully parsed calendar events (from VEVENT)
    pub imported_events: Vec<CalendarEvent>,
    /// Tasks that were skipped (with reason)
    pub skipped: Vec<(Task, ImportSkipReason)>,
    /// Errors that occurred during parsing
    pub errors: Vec<ImportError>,
}

impl ImportResult {
    /// Returns the total number of items processed (tasks + events)
    #[must_use]
    pub fn total_processed(&self) -> usize {
        self.imported.len() + self.imported_events.len() + self.skipped.len() + self.errors.len()
    }

    /// Returns the total number of items successfully imported
    #[must_use]
    pub fn total_imported(&self) -> usize {
        self.imported.len() + self.imported_events.len()
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

    /// Returns true if any events were successfully imported
    #[must_use]
    pub fn has_imported_events(&self) -> bool {
        !self.imported_events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_format_parse() {
        assert_eq!(ImportFormat::parse("csv"), Some(ImportFormat::Csv));
        assert_eq!(ImportFormat::parse("CSV"), Some(ImportFormat::Csv));
        assert_eq!(ImportFormat::parse("ics"), Some(ImportFormat::Ics));
        assert_eq!(ImportFormat::parse("ical"), Some(ImportFormat::Ics));
        assert_eq!(ImportFormat::parse("unknown"), None);
    }

    #[test]
    fn test_import_result_methods() {
        let mut result = ImportResult::default();

        assert_eq!(result.total_processed(), 0);
        assert_eq!(result.total_imported(), 0);
        assert!(!result.has_errors());
        assert!(!result.has_imported());
        assert!(!result.has_imported_events());

        result.imported.push(Task::new("Task 1"));
        assert_eq!(result.total_processed(), 1);
        assert_eq!(result.total_imported(), 1);
        assert!(result.has_imported());

        result.imported_events.push(CalendarEvent::new("Meeting"));
        assert_eq!(result.total_processed(), 2);
        assert_eq!(result.total_imported(), 2);
        assert!(result.has_imported_events());

        result.errors.push(ImportError {
            line: 1,
            message: "test".to_string(),
            content: None,
        });
        assert!(result.has_errors());
        assert_eq!(result.total_processed(), 3);
    }

    #[test]
    fn test_import_skip_reason_display_duplicate_id() {
        let task_id = TaskId::new();
        let reason = ImportSkipReason::DuplicateId(task_id);
        let display = reason.to_string();
        assert!(display.contains("already exists"));
        assert!(display.contains(&task_id.to_string()));
    }

    #[test]
    fn test_import_skip_reason_display_duplicate_title_with_date() {
        let reason = ImportSkipReason::DuplicateTitleDate {
            title: "Buy milk".to_string(),
            due_date: Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap()),
        };
        let display = reason.to_string();
        assert!(display.contains("Buy milk"));
        assert!(display.contains("2025-12-25"));
        assert!(display.contains("already exists"));
    }

    #[test]
    fn test_import_skip_reason_display_duplicate_title_without_date() {
        let reason = ImportSkipReason::DuplicateTitleDate {
            title: "Buy milk".to_string(),
            due_date: None,
        };
        let display = reason.to_string();
        assert!(display.contains("Buy milk"));
        assert!(display.contains("already exists"));
        assert!(!display.contains("due date"));
    }

    #[test]
    fn test_import_skip_reason_display_validation_failed() {
        let reason = ImportSkipReason::ValidationFailed("title cannot be empty".to_string());
        let display = reason.to_string();
        assert!(display.contains("validation failed"));
        assert!(display.contains("title cannot be empty"));
    }

    #[test]
    fn test_import_error_display() {
        let error = ImportError {
            line: 42,
            message: "invalid date format".to_string(),
            content: Some("bad,data,here".to_string()),
        };
        assert_eq!(error.to_string(), "Line 42: invalid date format");
    }
}
