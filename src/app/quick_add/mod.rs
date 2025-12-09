//! Quick add parsing for task creation.
//!
//! Parses task titles for embedded metadata using a simple syntax.
//!
//! # Syntax Reference
//!
//! | Syntax | Description | Examples |
//! |--------|-------------|----------|
//! | `#tag` | Add a tag | `#work`, `#urgent`, `#home` |
//! | `!priority` | Set priority | `!urgent`, `!high`, `!med`, `!low` |
//! | `due:date` | Set due date | `due:tomorrow`, `due:friday`, `due:2024-12-25` |
//! | `sched:date` | Set scheduled date | `sched:monday`, `sched:next week` |
//! | `@project` | Assign to project | `@work`, `@personal` |
//!
//! # Priority Values
//!
//! | Input | Priority |
//! |-------|----------|
//! | `urgent`, `u`, `!!!!` | Urgent |
//! | `high`, `h`, `!!!` | High |
//! | `med`, `medium`, `m`, `!!` | Medium |
//! | `low`, `l`, `!` | Low |
//! | `none`, `n`, `0` | None |
//!
//! # Date Formats
//!
//! Dates can be specified in many formats:
//!
//! ## Keywords
//! - `today`, `tod` - Today's date
//! - `tomorrow`, `tom` - Tomorrow's date
//! - `yesterday` - Yesterday's date
//!
//! ## Weekdays
//! - Short: `mon`, `tue`, `wed`, `thu`, `fri`, `sat`, `sun`
//! - Full: `monday`, `tuesday`, etc.
//! - Extended: `next monday`, `this friday`
//!
//! ## Relative Dates
//! - `in 3 days`, `in 2 weeks`, `in 1 month`
//! - `next week` - Monday of next week
//! - `next month` - 1st of next month
//! - `next year` - January 1st of next year
//!
//! ## End of Period
//! - `eow`, `end of week` - Next Sunday
//! - `eom`, `end of month` - Last day of current month
//! - `eoy`, `end of year` - December 31st
//!
//! ## Specific Days
//! - Ordinal: `1st`, `15th`, `22nd`, `3rd`
//! - `last day` - Last day of current month
//! - ISO format: `2024-12-25` (YYYY-MM-DD)
//! - Month/Day: `12/25`, `12-25` (current year assumed)
//!
//! # Edge Cases
//!
//! - **Weekday same as today**: Returns next week's occurrence (e.g., "monday" on a Monday returns next Monday)
//! - **Ordinal day passed**: If the day has passed this month, returns next month's occurrence
//! - **Month overflow**: `in 1 month` on Jan 31 returns Feb 28/29 (clamped to valid day)
//! - **Unknown format**: Returns `None`, task created without date
//!
//! # Example
//!
//! ```
//! use taskflow::app::quick_add::parse_quick_add;
//!
//! let parsed = parse_quick_add("Fix bug #backend !high due:tomorrow");
//! assert_eq!(parsed.title, "Fix bug");
//! assert_eq!(parsed.tags, vec!["backend"]);
//! assert!(parsed.priority.is_some());
//! assert!(parsed.due_date.is_some());
//! ```

mod date;

#[cfg(test)]
mod tests;

use std::sync::LazyLock;

use chrono::NaiveDate;
use regex::Regex;

use crate::domain::Priority;

pub use date::{parse_date, parse_date_with_reference};

// Pre-compiled regex patterns for quick add parsing (compiled once at startup)
// These patterns are compile-time constants and will never fail
static TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#(\w+)").expect("valid regex pattern"));
static PRIORITY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"!(\w+)").expect("valid regex pattern"));
static DUE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"due:(\S+)").expect("valid regex pattern"));
static SCHED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"sched:(\S+)").expect("valid regex pattern"));
static PROJECT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"@(\w+)").expect("valid regex pattern"));

/// Result of parsing a quick add string
#[derive(Debug, Clone, Default)]
pub struct ParsedTask {
    /// The cleaned task title (metadata tokens removed)
    pub title: String,
    /// Tags extracted from #tag syntax
    pub tags: Vec<String>,
    /// Priority extracted from !priority syntax
    pub priority: Option<Priority>,
    /// Due date extracted from due:date syntax
    pub due_date: Option<NaiveDate>,
    /// Scheduled date extracted from sched:date syntax
    pub scheduled_date: Option<NaiveDate>,
    /// Project name extracted from @project syntax
    pub project_name: Option<String>,
}

/// Parse a quick add string to extract metadata
///
/// # Arguments
///
/// * `input` - The raw task input string
///
/// # Returns
///
/// A `ParsedTask` containing the cleaned title and extracted metadata
///
/// # Example
///
/// ```
/// use taskflow::app::quick_add::parse_quick_add;
///
/// let parsed = parse_quick_add("Buy groceries #shopping !med due:saturday @home");
/// assert_eq!(parsed.title, "Buy groceries");
/// assert_eq!(parsed.tags, vec!["shopping"]);
/// assert_eq!(parsed.project_name, Some("home".to_string()));
/// ```
pub fn parse_quick_add(input: &str) -> ParsedTask {
    let mut result = ParsedTask::default();
    let mut remaining = input.to_string();

    // Extract tags (#tag) - using pre-compiled regex
    for cap in TAG_RE.captures_iter(input) {
        result.tags.push(cap[1].to_string());
    }
    remaining = TAG_RE.replace_all(&remaining, "").to_string();

    // Extract priority (!priority) - using pre-compiled regex
    if let Some(cap) = PRIORITY_RE.captures(input) {
        result.priority = parse_priority(&cap[1]);
    }
    remaining = PRIORITY_RE.replace_all(&remaining, "").to_string();

    // Extract due date (due:date) - using pre-compiled regex
    if let Some(cap) = DUE_RE.captures(input) {
        result.due_date = parse_date(&cap[1]);
    }
    remaining = DUE_RE.replace_all(&remaining, "").to_string();

    // Extract scheduled date (sched:date) - using pre-compiled regex
    if let Some(cap) = SCHED_RE.captures(input) {
        result.scheduled_date = parse_date(&cap[1]);
    }
    remaining = SCHED_RE.replace_all(&remaining, "").to_string();

    // Extract project (@project) - using pre-compiled regex
    if let Some(cap) = PROJECT_RE.captures(input) {
        result.project_name = Some(cap[1].to_string());
    }
    remaining = PROJECT_RE.replace_all(&remaining, "").to_string();

    // Clean up title: collapse multiple spaces and trim
    result.title = remaining.split_whitespace().collect::<Vec<_>>().join(" ");

    result
}

/// Parse a priority string.
pub(crate) fn parse_priority(s: &str) -> Option<Priority> {
    match s.to_lowercase().as_str() {
        "urgent" | "u" | "!!!!" => Some(Priority::Urgent),
        "high" | "h" | "!!!" => Some(Priority::High),
        "med" | "medium" | "m" | "!!" => Some(Priority::Medium),
        "low" | "l" | "!" => Some(Priority::Low),
        "none" | "n" | "0" => Some(Priority::None),
        _ => None,
    }
}
