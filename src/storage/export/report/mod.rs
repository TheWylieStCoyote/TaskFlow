//! Analytics report export functionality (Markdown and HTML).
//!
//! This module provides export functions for generating formatted analytics
//! reports from [`AnalyticsReport`] data.
//!
//! # Supported Formats
//!
//! - **Markdown**: Human-readable text format with tables
//! - **HTML**: Styled web pages with cards and progress bars

mod html;
mod markdown;

#[cfg(test)]
mod tests;

pub use html::{export_report_to_html, export_report_to_html_string};
pub use markdown::{export_report_to_markdown, export_report_to_markdown_string};
