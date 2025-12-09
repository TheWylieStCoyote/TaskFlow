//! Export functionality for tasks and analytics reports.
//!
//! This module provides export capabilities for various formats:
//! - CSV (task data)
//! - ICS/iCalendar (task data)
//! - DOT/Graphviz (task chains and dependencies)
//! - Mermaid (task chains and dependencies)
//! - Markdown/HTML (analytics reports)

mod csv;
mod dot;
mod ics;
mod mermaid;
mod report;

use std::collections::HashMap;

use crate::domain::{Task, TaskId};

// Re-export format-specific functions
pub use csv::export_to_csv;
pub use dot::export_to_dot;
pub use ics::export_to_ics;
pub use mermaid::export_to_mermaid;
pub use report::{
    export_report_to_html, export_report_to_html_string, export_report_to_markdown,
    export_report_to_markdown_string,
};

/// Export format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Ics,
    Dot,
    Mermaid,
}

impl ExportFormat {
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "ics" | "ical" | "icalendar" => Some(Self::Ics),
            "dot" | "graphviz" => Some(Self::Dot),
            "mermaid" | "md" => Some(Self::Mermaid),
            _ => None,
        }
    }

    #[must_use]
    pub const fn file_extension(&self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Ics => "ics",
            Self::Dot => "dot",
            Self::Mermaid => "md",
        }
    }
}

/// Exports tasks to a string in the specified format.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if formatting fails.
pub fn export_to_string(tasks: &[Task], format: ExportFormat) -> std::io::Result<String> {
    let mut buffer = Vec::new();
    match format {
        ExportFormat::Csv => export_to_csv(tasks, &mut buffer)?,
        ExportFormat::Ics => export_to_ics(tasks, &mut buffer)?,
        ExportFormat::Dot | ExportFormat::Mermaid => {
            // These formats need the full task map for chain/dependency lookups
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Use export_chains_to_string for DOT/Mermaid formats",
            ));
        }
    }
    String::from_utf8(buffer).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Exports task chains to a string in DOT or Mermaid format.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if formatting fails.
pub fn export_chains_to_string(
    tasks: &HashMap<TaskId, Task>,
    format: ExportFormat,
) -> std::io::Result<String> {
    let mut buffer = Vec::new();
    match format {
        ExportFormat::Dot => export_to_dot(tasks, &mut buffer)?,
        ExportFormat::Mermaid => export_to_mermaid(tasks, &mut buffer)?,
        ExportFormat::Csv | ExportFormat::Ics => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Use export_to_string for CSV/ICS formats",
            ));
        }
    }
    String::from_utf8(buffer).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_parse() {
        assert_eq!(ExportFormat::parse("csv"), Some(ExportFormat::Csv));
        assert_eq!(ExportFormat::parse("CSV"), Some(ExportFormat::Csv));
        assert_eq!(ExportFormat::parse("ics"), Some(ExportFormat::Ics));
        assert_eq!(ExportFormat::parse("ical"), Some(ExportFormat::Ics));
        assert_eq!(ExportFormat::parse("dot"), Some(ExportFormat::Dot));
        assert_eq!(ExportFormat::parse("graphviz"), Some(ExportFormat::Dot));
        assert_eq!(ExportFormat::parse("mermaid"), Some(ExportFormat::Mermaid));
        assert_eq!(ExportFormat::parse("unknown"), None);
    }

    #[test]
    fn test_export_format_extension() {
        assert_eq!(ExportFormat::Csv.file_extension(), "csv");
        assert_eq!(ExportFormat::Ics.file_extension(), "ics");
        assert_eq!(ExportFormat::Dot.file_extension(), "dot");
        assert_eq!(ExportFormat::Mermaid.file_extension(), "md");
    }
}
