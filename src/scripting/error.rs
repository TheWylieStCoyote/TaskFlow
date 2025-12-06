//! Error types for the scripting system.

use thiserror::Error;

/// Result type for scripting operations.
pub type ScriptResult<T> = Result<T, ScriptError>;

/// Errors that can occur during script execution.
#[derive(Debug, Error)]
pub enum ScriptError {
    /// Script execution timed out.
    #[error("script execution timed out after {0} seconds")]
    Timeout(u64),

    /// Script compilation failed.
    #[error("script compilation error: {0}")]
    Compilation(String),

    /// Script runtime error.
    #[error("script runtime error: {0}")]
    Runtime(String),

    /// Configuration file error.
    #[error("configuration error: {0}")]
    Config(String),

    /// IO error reading config.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// TOML parsing error.
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// Hook not found.
    #[error("hook not found: {0}")]
    HookNotFound(String),

    /// Command not found.
    #[error("command not found: {0}")]
    CommandNotFound(String),

    /// Invalid task ID in script.
    #[error("invalid task ID: {0}")]
    InvalidTaskId(String),
}

impl From<rhai::ParseError> for ScriptError {
    fn from(err: rhai::ParseError) -> Self {
        Self::Compilation(err.to_string())
    }
}

impl From<Box<rhai::EvalAltResult>> for ScriptError {
    fn from(err: Box<rhai::EvalAltResult>) -> Self {
        Self::Runtime(err.to_string())
    }
}
