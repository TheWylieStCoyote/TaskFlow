use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// Storage-specific errors
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Entity not found: {entity_type} with id '{id}'")]
    NotFound { entity_type: String, id: String },

    #[error("Entity already exists: {entity_type} with id '{id}'")]
    AlreadyExists { entity_type: String, id: String },

    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    #[error("Invalid reference: {0}")]
    InvalidReference(String),

    #[error("IO error at path '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Serialization error: {message}")]
    Serialization { message: String },

    #[error("Deserialization error: {message}")]
    Deserialization { message: String },

    #[error("Database error: {0}")]
    Database(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Storage backend not initialized")]
    NotInitialized,

    #[error("Invalid storage path: {0}")]
    InvalidPath(PathBuf),

    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),
}

impl StorageError {
    pub fn not_found(entity_type: impl Into<String>, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity_type: entity_type.into(),
            id: id.into(),
        }
    }

    pub fn already_exists(entity_type: impl Into<String>, id: impl Into<String>) -> Self {
        Self::AlreadyExists {
            entity_type: entity_type.into(),
            id: id.into(),
        }
    }

    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
        }
    }

    pub fn deserialization(message: impl Into<String>) -> Self {
        Self::Deserialization {
            message: message.into(),
        }
    }
}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            path: PathBuf::new(),
            source: err,
        }
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        Self::Deserialization {
            message: err.to_string(),
        }
    }
}

impl From<serde_yaml::Error> for StorageError {
    fn from(err: serde_yaml::Error) -> Self {
        Self::Deserialization {
            message: err.to_string(),
        }
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Database(err.to_string())
    }
}
