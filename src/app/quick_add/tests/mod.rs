//! Tests for quick add parsing.
//!
//! Tests are organized into submodules by category:
//! - `basic` - Basic parsing tests (title, tags, priority, project)
//! - `date_formats` - Date format parsing tests
//! - `smart_dates` - Smart/relative date parsing tests
//! - `edge_cases` - Edge cases and validation tests

mod basic;
mod date_formats;
mod edge_cases;
mod smart_dates;
