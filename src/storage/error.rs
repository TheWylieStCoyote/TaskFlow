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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, ErrorKind};

    #[test]
    fn test_not_found_error_display() {
        let err = StorageError::not_found("Task", "task-123");
        assert_eq!(err.to_string(), "Entity not found: Task with id 'task-123'");
    }

    #[test]
    fn test_already_exists_error_display() {
        let err = StorageError::already_exists("Project", "proj-456");
        assert_eq!(
            err.to_string(),
            "Entity already exists: Project with id 'proj-456'"
        );
    }

    #[test]
    fn test_circular_dependency_error_display() {
        let err = StorageError::CircularDependency("task1 -> task2 -> task1".to_string());
        assert_eq!(
            err.to_string(),
            "Circular dependency detected: task1 -> task2 -> task1"
        );
    }

    #[test]
    fn test_invalid_reference_error_display() {
        let err = StorageError::InvalidReference("Parent project not found".to_string());
        assert_eq!(
            err.to_string(),
            "Invalid reference: Parent project not found"
        );
    }

    #[test]
    fn test_io_error_display() {
        let io_err = io::Error::new(ErrorKind::NotFound, "file not found");
        let err = StorageError::io("/path/to/file.json", io_err);
        assert!(err
            .to_string()
            .contains("IO error at path '/path/to/file.json'"));
    }

    #[test]
    fn test_io_error_from_conversion() {
        let io_err = io::Error::new(ErrorKind::PermissionDenied, "permission denied");
        let err: StorageError = io_err.into();
        assert!(matches!(err, StorageError::Io { .. }));
    }

    #[test]
    fn test_serialization_error_display() {
        let err = StorageError::serialization("invalid UTF-8 sequence");
        assert_eq!(
            err.to_string(),
            "Serialization error: invalid UTF-8 sequence"
        );
    }

    #[test]
    fn test_deserialization_error_display() {
        let err = StorageError::deserialization("unexpected end of input");
        assert_eq!(
            err.to_string(),
            "Deserialization error: unexpected end of input"
        );
    }

    #[test]
    fn test_database_error_display() {
        let err = StorageError::Database("UNIQUE constraint failed".to_string());
        assert_eq!(err.to_string(), "Database error: UNIQUE constraint failed");
    }

    #[test]
    fn test_migration_error_display() {
        let err = StorageError::Migration("schema version mismatch".to_string());
        assert_eq!(err.to_string(), "Migration error: schema version mismatch");
    }

    #[test]
    fn test_validation_error_display() {
        let err = StorageError::Validation("task title cannot be empty".to_string());
        assert_eq!(
            err.to_string(),
            "Validation error: task title cannot be empty"
        );
    }

    #[test]
    fn test_not_initialized_error_display() {
        let err = StorageError::NotInitialized;
        assert_eq!(err.to_string(), "Storage backend not initialized");
    }

    #[test]
    fn test_invalid_path_error_display() {
        let err = StorageError::InvalidPath(PathBuf::from("/invalid\0path"));
        assert!(err.to_string().contains("Invalid storage path"));
    }

    #[test]
    fn test_permission_denied_error_display() {
        let err = StorageError::PermissionDenied(PathBuf::from("/root/secret"));
        assert_eq!(err.to_string(), "Permission denied: /root/secret");
    }

    #[test]
    fn test_json_error_conversion() {
        let json_str = "{ invalid json }";
        let result: Result<serde_json::Value, _> = serde_json::from_str(json_str);
        let json_err = result.unwrap_err();
        let storage_err: StorageError = json_err.into();
        assert!(matches!(storage_err, StorageError::Deserialization { .. }));
    }

    #[test]
    fn test_yaml_error_conversion() {
        let yaml_str = "key: [invalid";
        let result: Result<serde_yaml::Value, _> = serde_yaml::from_str(yaml_str);
        let yaml_err = result.unwrap_err();
        let storage_err: StorageError = yaml_err.into();
        assert!(matches!(storage_err, StorageError::Deserialization { .. }));
    }

    #[test]
    fn test_error_debug_impl() {
        let err = StorageError::not_found("Task", "abc");
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("NotFound"));
        assert!(debug_str.contains("Task"));
        assert!(debug_str.contains("abc"));
    }

    #[test]
    fn test_helper_methods_accept_string_types() {
        // Test with &str
        let err1 = StorageError::not_found("Task", "id1");
        assert!(matches!(err1, StorageError::NotFound { .. }));

        // Test with String
        let err2 = StorageError::already_exists(String::from("Project"), String::from("id2"));
        assert!(matches!(err2, StorageError::AlreadyExists { .. }));

        // Test io with PathBuf
        let io_err = io::Error::new(ErrorKind::NotFound, "not found");
        let err3 = StorageError::io(PathBuf::from("/test"), io_err);
        assert!(matches!(err3, StorageError::Io { .. }));

        // Test io with &str path
        let io_err2 = io::Error::new(ErrorKind::NotFound, "not found");
        let err4 = StorageError::io("/test/path", io_err2);
        assert!(matches!(err4, StorageError::Io { .. }));
    }
}
