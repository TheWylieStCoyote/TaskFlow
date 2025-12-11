//! CLI argument definitions.

use std::path::PathBuf;

use chrono::NaiveDate;
use clap::{Parser, Subcommand, ValueHint};
use clap_complete::Shell;

use taskflow::domain::{Priority, TaskStatus};
use taskflow::storage::BackendType;

/// CLI filter options for the list command
#[derive(Default)]
pub struct ListFilters {
    pub project: Option<String>,
    pub tags: Option<Vec<String>>,
    pub tags_any: bool,
    pub priority: Option<Vec<Priority>>,
    pub status: Option<Vec<TaskStatus>>,
    pub search: Option<String>,
    pub sort: String,
    pub reverse: bool,
    pub due_before: Option<NaiveDate>,
    pub due_after: Option<NaiveDate>,
    pub estimate_min: Option<u32>,
    pub estimate_max: Option<u32>,
}

/// `TaskFlow` - A TUI project management application
#[derive(Parser, Debug)]
#[command(name = "taskflow")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to data file or directory
    #[arg(short, long, global = true, value_hint = ValueHint::AnyPath)]
    pub data: Option<PathBuf>,

    /// Storage backend type
    #[arg(short, long, default_value = "json", global = true, value_enum)]
    pub backend: BackendType,

    /// Use sample data instead of loading from storage
    #[arg(long, global = true)]
    pub demo: bool,

    /// Enable debug logging (writes to taskflow.log)
    #[arg(long, global = true)]
    pub debug: bool,

    /// Set log level (trace, debug, info, warn, error)
    #[arg(long, global = true, default_value = "info")]
    pub log_level: String,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate shell completion scripts
    Completion {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Quick add a task from the command line
    #[command(alias = "a")]
    Add {
        /// Task description with optional quick-add syntax
        /// Examples:
        ///   "Buy milk #shopping !high due:tomorrow"
        ///   "Review PR @work #code due:friday"
        #[arg(trailing_var_arg = true, num_args = 1..)]
        task: Vec<String>,
    },
    /// List tasks (without launching TUI)
    #[command(alias = "ls")]
    List {
        /// Filter by view (today, overdue, upcoming, all, blocked, untagged, no-project, scheduled)
        #[arg(short, long, default_value = "all")]
        view: String,
        /// Show completed tasks
        #[arg(short, long)]
        completed: bool,
        /// Limit number of tasks shown
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
        /// Filter by project name (case-insensitive substring match)
        #[arg(short, long)]
        project: Option<String>,
        /// Filter by tags (comma-separated, requires ALL by default)
        #[arg(short, long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
        /// Match ANY tag instead of ALL tags
        #[arg(long)]
        tags_any: bool,
        /// Filter by priority (comma-separated: none, low, medium, high, urgent)
        #[arg(long, value_delimiter = ',')]
        priority: Option<Vec<String>>,
        /// Filter by status (comma-separated: todo, in-progress, blocked, done, cancelled)
        #[arg(long, value_delimiter = ',')]
        status: Option<Vec<String>>,
        /// Search in title and tags (case-insensitive)
        #[arg(short, long)]
        search: Option<String>,
        /// Sort by field (due-date, priority, title, created)
        #[arg(long, default_value = "due-date")]
        sort: String,
        /// Reverse sort order
        #[arg(long)]
        reverse: bool,
        /// Only show tasks due before this date (YYYY-MM-DD, or: today, tomorrow, +Nd)
        #[arg(long)]
        due_before: Option<String>,
        /// Only show tasks due after this date (YYYY-MM-DD, or: today, tomorrow, -Nd)
        #[arg(long)]
        due_after: Option<String>,
        /// Only show tasks with estimate >= this many minutes
        #[arg(long)]
        estimate_min: Option<u32>,
        /// Only show tasks with estimate <= this many minutes
        #[arg(long)]
        estimate_max: Option<u32>,
    },
    /// Mark a task as done by searching for it
    #[command(alias = "d")]
    Done {
        /// Search query to find the task (matches title)
        #[arg(trailing_var_arg = true, num_args = 1..)]
        query: Vec<String>,
        /// Only search in tasks from this project (case-insensitive substring match)
        #[arg(short, long)]
        project: Option<String>,
        /// Only search in tasks with these tags (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
    },
}

/// Parse priority strings into Priority enum values
pub fn parse_priorities(strings: &[String]) -> Vec<Priority> {
    strings
        .iter()
        .filter_map(|s| match s.to_lowercase().as_str() {
            "none" => Some(Priority::None),
            "low" => Some(Priority::Low),
            "medium" | "med" => Some(Priority::Medium),
            "high" => Some(Priority::High),
            "urgent" => Some(Priority::Urgent),
            _ => None,
        })
        .collect()
}

/// Parse status strings into TaskStatus enum values
pub fn parse_statuses(strings: &[String]) -> Vec<TaskStatus> {
    strings
        .iter()
        .filter_map(|s| match s.to_lowercase().replace('-', "").as_str() {
            "todo" => Some(TaskStatus::Todo),
            "inprogress" | "in_progress" | "progress" => Some(TaskStatus::InProgress),
            "blocked" => Some(TaskStatus::Blocked),
            "done" | "completed" => Some(TaskStatus::Done),
            "cancelled" | "canceled" => Some(TaskStatus::Cancelled),
            _ => None,
        })
        .collect()
}

/// Parse a date string into a NaiveDate.
///
/// Supports formats:
/// - `YYYY-MM-DD` - ISO date format
/// - `today` - Current date
/// - `tomorrow` - Tomorrow's date
/// - `yesterday` - Yesterday's date
/// - `+Nd` or `+N` - N days from today (e.g., `+7d`, `+7`)
/// - `-Nd` or `-N` - N days ago (e.g., `-3d`, `-3`)
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    use chrono::{Duration, Utc};

    let s = s.trim().to_lowercase();
    let today = Utc::now().date_naive();

    match s.as_str() {
        "today" => Some(today),
        "tomorrow" => Some(today + Duration::days(1)),
        "yesterday" => Some(today - Duration::days(1)),
        _ => {
            // Try +Nd or -Nd format
            if s.starts_with('+') || s.starts_with('-') {
                let sign = if s.starts_with('+') { 1 } else { -1 };
                let num_str = s
                    .trim_start_matches('+')
                    .trim_start_matches('-')
                    .trim_end_matches('d');
                if let Ok(days) = num_str.parse::<i64>() {
                    return Some(today + Duration::days(days * sign));
                }
            }

            // Try ISO format YYYY-MM-DD
            NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    // ========================================================================
    // parse_priorities tests
    // ========================================================================

    #[test]
    fn test_parse_priorities_all_levels() {
        let input = vec![
            "none".to_string(),
            "low".to_string(),
            "medium".to_string(),
            "high".to_string(),
            "urgent".to_string(),
        ];
        let result = parse_priorities(&input);
        assert_eq!(result.len(), 5);
        assert_eq!(result[0], Priority::None);
        assert_eq!(result[1], Priority::Low);
        assert_eq!(result[2], Priority::Medium);
        assert_eq!(result[3], Priority::High);
        assert_eq!(result[4], Priority::Urgent);
    }

    #[test]
    fn test_parse_priorities_case_insensitive() {
        let input = vec!["HIGH".to_string(), "Low".to_string(), "URGENT".to_string()];
        let result = parse_priorities(&input);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], Priority::High);
        assert_eq!(result[1], Priority::Low);
        assert_eq!(result[2], Priority::Urgent);
    }

    #[test]
    fn test_parse_priorities_med_alias() {
        let input = vec!["med".to_string()];
        let result = parse_priorities(&input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], Priority::Medium);
    }

    #[test]
    fn test_parse_priorities_invalid() {
        let input = vec![
            "invalid".to_string(),
            "high".to_string(),
            "unknown".to_string(),
        ];
        let result = parse_priorities(&input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], Priority::High);
    }

    #[test]
    fn test_parse_priorities_empty() {
        let input: Vec<String> = vec![];
        let result = parse_priorities(&input);
        assert!(result.is_empty());
    }

    // ========================================================================
    // parse_statuses tests
    // ========================================================================

    #[test]
    fn test_parse_statuses_all_statuses() {
        let input = vec![
            "todo".to_string(),
            "in-progress".to_string(),
            "blocked".to_string(),
            "done".to_string(),
            "cancelled".to_string(),
        ];
        let result = parse_statuses(&input);
        assert_eq!(result.len(), 5);
        assert_eq!(result[0], TaskStatus::Todo);
        assert_eq!(result[1], TaskStatus::InProgress);
        assert_eq!(result[2], TaskStatus::Blocked);
        assert_eq!(result[3], TaskStatus::Done);
        assert_eq!(result[4], TaskStatus::Cancelled);
    }

    #[test]
    fn test_parse_statuses_case_insensitive() {
        let input = vec!["TODO".to_string(), "Done".to_string()];
        let result = parse_statuses(&input);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], TaskStatus::Todo);
        assert_eq!(result[1], TaskStatus::Done);
    }

    #[test]
    fn test_parse_statuses_aliases() {
        let input = vec![
            "in_progress".to_string(),
            "inprogress".to_string(),
            "progress".to_string(),
            "completed".to_string(),
            "canceled".to_string(),
        ];
        let result = parse_statuses(&input);
        assert_eq!(result.len(), 5);
        assert_eq!(result[0], TaskStatus::InProgress);
        assert_eq!(result[1], TaskStatus::InProgress);
        assert_eq!(result[2], TaskStatus::InProgress);
        assert_eq!(result[3], TaskStatus::Done);
        assert_eq!(result[4], TaskStatus::Cancelled);
    }

    #[test]
    fn test_parse_statuses_invalid() {
        let input = vec!["invalid".to_string(), "todo".to_string()];
        let result = parse_statuses(&input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], TaskStatus::Todo);
    }

    #[test]
    fn test_parse_statuses_empty() {
        let input: Vec<String> = vec![];
        let result = parse_statuses(&input);
        assert!(result.is_empty());
    }

    // ========================================================================
    // parse_date tests
    // ========================================================================

    #[test]
    fn test_parse_date_today() {
        let today = Utc::now().date_naive();
        assert_eq!(parse_date("today"), Some(today));
        assert_eq!(parse_date("TODAY"), Some(today));
        assert_eq!(parse_date("  today  "), Some(today));
    }

    #[test]
    fn test_parse_date_tomorrow() {
        let tomorrow = Utc::now().date_naive() + Duration::days(1);
        assert_eq!(parse_date("tomorrow"), Some(tomorrow));
        assert_eq!(parse_date("TOMORROW"), Some(tomorrow));
    }

    #[test]
    fn test_parse_date_yesterday() {
        let yesterday = Utc::now().date_naive() - Duration::days(1);
        assert_eq!(parse_date("yesterday"), Some(yesterday));
    }

    #[test]
    fn test_parse_date_plus_days() {
        let today = Utc::now().date_naive();
        assert_eq!(parse_date("+7d"), Some(today + Duration::days(7)));
        assert_eq!(parse_date("+7"), Some(today + Duration::days(7)));
        assert_eq!(parse_date("+30d"), Some(today + Duration::days(30)));
    }

    #[test]
    fn test_parse_date_minus_days() {
        let today = Utc::now().date_naive();
        assert_eq!(parse_date("-3d"), Some(today - Duration::days(3)));
        assert_eq!(parse_date("-3"), Some(today - Duration::days(3)));
        assert_eq!(parse_date("-14d"), Some(today - Duration::days(14)));
    }

    #[test]
    fn test_parse_date_iso_format() {
        assert_eq!(
            parse_date("2025-12-25"),
            NaiveDate::from_ymd_opt(2025, 12, 25)
        );
        assert_eq!(
            parse_date("2024-01-01"),
            NaiveDate::from_ymd_opt(2024, 1, 1)
        );
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date("invalid").is_none());
        assert!(parse_date("not-a-date").is_none());
        assert!(parse_date("2025/12/25").is_none()); // Wrong separator
        assert!(parse_date("25-12-2025").is_none()); // Wrong order
    }

    #[test]
    fn test_parse_date_edge_cases() {
        let today = Utc::now().date_naive();
        assert_eq!(parse_date("+0"), Some(today));
        assert_eq!(parse_date("-0"), Some(today));
        assert_eq!(parse_date("+1"), Some(today + Duration::days(1)));
    }
}
