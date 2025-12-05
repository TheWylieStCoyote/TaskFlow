//! Storage backends and persistence layer.
//!
//! This module provides pluggable storage backends for persisting
//! application data. All backends implement the [`StorageBackend`] trait
//! which provides a unified interface for CRUD operations.
//!
//! ## Available Backends
//!
//! - [`backends::JsonBackend`] - JSON file storage (default)
//! - [`backends::YamlBackend`] - YAML file storage
//! - [`backends::SqliteBackend`] - SQLite database storage
//! - [`backends::MarkdownBackend`] - Markdown files with YAML frontmatter
//!
//! ## Usage
//!
//! Use [`create_backend`] to instantiate a backend:
//!
//! ```no_run
//! use taskflow::storage::{create_backend, BackendType};
//! use std::path::Path;
//!
//! let backend = create_backend(BackendType::Json, Path::new("tasks.json")).unwrap();
//! ```
//!
//! ## Export Formats
//!
//! Tasks can be exported to external formats via the [`export`] module:
//!
//! - CSV - Spreadsheet-compatible format
//! - ICS - iCalendar format for calendar applications

pub mod backends;
mod error;
pub mod export;
mod repository;

pub use error::*;
pub use export::{export_to_csv, export_to_ics, export_to_string, ExportFormat};
pub use repository::*;

use std::path::Path;

use clap::ValueEnum;

/// Storage backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum BackendType {
    #[default]
    Json,
    Yaml,
    Sqlite,
    Markdown,
}

impl BackendType {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "yaml" | "yml" => Some(Self::Yaml),
            "sqlite" | "db" => Some(Self::Sqlite),
            "markdown" | "md" => Some(Self::Markdown),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Sqlite => "sqlite",
            Self::Markdown => "markdown",
        }
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Sqlite => "db",
            Self::Markdown => "md",
        }
    }
}

/// Create a storage backend from configuration
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
