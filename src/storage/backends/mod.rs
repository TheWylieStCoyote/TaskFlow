//! Storage backend implementations.
//!
//! This module provides concrete implementations of the [`StorageBackend`] trait
//! for different file formats and databases. Each backend supports the full
//! CRUD operations for tasks, projects, time entries, and habits.
//!
//! # Available Backends
//!
//! | Backend | Format | Use Case |
//! |---------|--------|----------|
//! | [`JsonBackend`] | JSON | Human-readable, easy debugging |
//! | [`YamlBackend`] | YAML | Human-readable, good for config-like data |
//! | [`SqliteBackend`] | SQLite | Fast queries, large datasets |
//! | [`MarkdownBackend`] | Markdown | Portable, works with other tools |
//!
//! # Choosing a Backend
//!
//! - **JSON**: Best for development and small task lists (<1000 tasks)
//! - **YAML**: Similar to JSON, preferred if you edit files manually
//! - **SQLite**: Best for large task lists, fast filtering and search
//! - **Markdown**: Best for interoperability with other note-taking tools
//!
//! # Example
//!
//! ```ignore
//! use taskflow::storage::{create_backend, BackendType};
//!
//! // Create a JSON backend
//! let backend = create_backend(BackendType::Json, "tasks.json")?;
//!
//! // Create a SQLite backend for better performance
//! let backend = create_backend(BackendType::Sqlite, "tasks.db")?;
//! ```
//!
//! [`StorageBackend`]: crate::storage::StorageBackend

mod filter_utils;
mod json;
mod markdown;
mod sqlite;
mod yaml;

pub use json::JsonBackend;
pub use markdown::MarkdownBackend;
pub use sqlite::SqliteBackend;
pub use yaml::YamlBackend;
