pub mod backends;
mod error;
mod repository;

pub use error::*;
pub use repository::*;

use std::path::Path;

/// Storage backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
