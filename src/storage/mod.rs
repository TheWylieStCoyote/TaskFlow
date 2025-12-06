//! Storage backends and persistence layer.
//!
//! This module provides pluggable storage backends for persisting
//! application data. All backends implement the [`StorageBackend`] trait
//! which provides a unified interface for CRUD operations.
//!
//! ## Available Backends
//!
//! | Backend | Format | Best For |
//! |---------|--------|----------|
//! | [`backends::JsonBackend`] | JSON file | Fast loading, compact storage |
//! | [`backends::YamlBackend`] | YAML file | Human-readable, easy manual editing |
//! | [`backends::SqliteBackend`] | SQLite DB | Large datasets, complex queries |
//! | [`backends::MarkdownBackend`] | Markdown + YAML | Git-friendly, text editors |
//!
//! ## Quick Start
//!
//! ### Creating a Backend
//!
//! ```no_run
//! use taskflow::storage::{create_backend, BackendType, StorageBackend};
//! use taskflow::domain::Task;
//! use std::path::Path;
//!
//! // Create a JSON backend
//! let mut backend = create_backend(
//!     BackendType::Json,
//!     Path::new("tasks.json")
//! ).unwrap();
//!
//! // Create a task
//! let task = Task::new("My task");
//! backend.create_task(&task).unwrap();
//!
//! // Load all tasks
//! let tasks = backend.list_tasks().unwrap();
//! ```
//!
//! ### Choosing a Backend
//!
//! ```
//! use taskflow::storage::BackendType;
//!
//! // Parse from string (case-insensitive)
//! let json = BackendType::parse("json");
//! let yaml = BackendType::parse("yml"); // "yaml" also works
//! let sqlite = BackendType::parse("sqlite");
//! let markdown = BackendType::parse("md"); // "markdown" also works
//!
//! // Get file extension
//! assert_eq!(BackendType::Json.file_extension(), "json");
//! assert_eq!(BackendType::Sqlite.file_extension(), "db");
//! ```
//!
//! ## Export Formats
//!
//! Tasks can be exported to external formats:
//!
//! ```
//! use taskflow::storage::{export_to_string, ExportFormat};
//! use taskflow::domain::{Task, Priority};
//! use chrono::Utc;
//!
//! let tasks = vec![
//!     Task::new("Task 1").with_priority(Priority::High),
//!     Task::new("Task 2").with_priority(Priority::Low),
//! ];
//!
//! // Export to CSV for spreadsheets
//! let csv = export_to_string(&tasks, ExportFormat::Csv);
//!
//! // Export to ICS for calendar apps
//! let ics = export_to_string(&tasks, ExportFormat::Ics);
//! ```
//!
//! ## Backend Comparison
//!
//! ### JSON (Default)
//! - **Pros**: Fast, compact, widely supported
//! - **Cons**: Not human-readable for large files
//! - **Use when**: Default choice for most users
//!
//! ### YAML
//! - **Pros**: Human-readable, easy to edit manually
//! - **Cons**: Larger file size, slower parsing
//! - **Use when**: You want to edit tasks in a text editor
//!
//! ### SQLite
//! - **Pros**: Efficient for large datasets, ACID transactions
//! - **Cons**: Binary format, requires SQLite
//! - **Use when**: Managing hundreds/thousands of tasks
//!
//! ### Markdown
//! - **Pros**: Git-friendly, works with any text editor
//! - **Cons**: Slower for large datasets
//! - **Use when**: You want tasks in version control

pub mod backends;
mod error;
pub mod export;
mod repository;

pub use error::*;
pub use export::{export_to_csv, export_to_ics, export_to_string, ExportFormat};
pub use repository::*;

use std::path::Path;

use clap::ValueEnum;

/// Storage backend type identifier.
///
/// Used to select which storage format to use for persisting tasks
/// and projects.
///
/// # Examples
///
/// ```
/// use taskflow::storage::BackendType;
///
/// // Default is JSON
/// let default = BackendType::default();
/// assert_eq!(default, BackendType::Json);
///
/// // Parse from CLI argument or config
/// let backend = BackendType::parse("yaml").unwrap();
/// assert_eq!(backend.as_str(), "yaml");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum BackendType {
    /// JSON file storage (default) - fast and compact
    #[default]
    Json,
    /// YAML file storage - human-readable
    Yaml,
    /// SQLite database - efficient for large datasets
    Sqlite,
    /// Markdown files with YAML frontmatter - git-friendly
    Markdown,
}

impl BackendType {
    /// Parses a backend type from a string (case-insensitive).
    ///
    /// Accepts common aliases like "yml" for YAML and "md" for Markdown.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "yaml" | "yml" => Some(Self::Yaml),
            "sqlite" | "db" => Some(Self::Sqlite),
            "markdown" | "md" => Some(Self::Markdown),
            _ => None,
        }
    }

    /// Returns the backend type as a lowercase string.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Sqlite => "sqlite",
            Self::Markdown => "markdown",
        }
    }

    /// Returns the typical file extension for this backend type.
    #[must_use]
    pub fn file_extension(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Sqlite => "db",
            Self::Markdown => "md",
        }
    }
}

/// Creates a storage backend instance.
///
/// This is the main entry point for creating storage backends.
/// The backend will be initialized and ready for use.
///
/// # Arguments
///
/// * `backend_type` - The type of backend to create
/// * `path` - Path to the storage file or directory
///
/// # Errors
///
/// Returns a [`StorageError`] if the backend cannot be initialized.
///
/// # Examples
///
/// ```no_run
/// use taskflow::storage::{create_backend, BackendType};
/// use std::path::Path;
///
/// // Create a JSON backend
/// let backend = create_backend(BackendType::Json, Path::new("tasks.json"))?;
///
/// // Create a SQLite backend
/// let backend = create_backend(BackendType::Sqlite, Path::new("tasks.db"))?;
/// # Ok::<(), taskflow::storage::StorageError>(())
/// ```
pub fn create_backend(
    backend_type: BackendType,
    path: &Path,
) -> StorageResult<Box<dyn StorageBackend>> {
    match backend_type {
        BackendType::Json => {
            let mut backend = backends::JsonBackend::new(path)?;
            backend.initialize()?;
            Ok(Box::new(backend))
        }
        BackendType::Yaml => {
            let mut backend = backends::YamlBackend::new(path)?;
            backend.initialize()?;
            Ok(Box::new(backend))
        }
        BackendType::Sqlite => {
            let mut backend = backends::SqliteBackend::new(path)?;
            backend.initialize()?;
            Ok(Box::new(backend))
        }
        BackendType::Markdown => {
            let mut backend = backends::MarkdownBackend::new(path)?;
            backend.initialize()?;
            Ok(Box::new(backend))
        }
    }
}
