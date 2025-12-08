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

mod csv;
mod duplicates;
mod ics;
mod types;

pub use csv::import_from_csv;
pub use duplicates::{apply_merge_strategy, DuplicateDetector};
pub use ics::import_from_ics;
pub use types::{
    ImportError, ImportFormat, ImportOptions, ImportResult, ImportSkipReason, MergeStrategy,
};
