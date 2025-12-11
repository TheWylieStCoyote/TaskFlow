//! Error types for the filter DSL parser.

use std::fmt;

/// Errors that can occur during filter DSL parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// Unexpected character encountered during lexing.
    UnexpectedChar { position: usize, char: char },

    /// Unexpected token during parsing.
    UnexpectedToken {
        expected: String,
        found: String,
        position: usize,
    },

    /// Unterminated quoted string.
    UnterminatedString { position: usize },

    /// Unknown field name in condition.
    UnknownField { name: String, position: usize },

    /// Invalid value for a field.
    InvalidValue {
        field: String,
        value: String,
        hint: Option<String>,
        position: usize,
    },

    /// Empty expression provided.
    EmptyExpression,

    /// Unexpected end of input.
    UnexpectedEof { expected: String },

    /// Missing value after field and colon.
    MissingValue { field: String, position: usize },
}

impl ParseError {
    /// Create an UnexpectedChar error.
    #[must_use]
    pub fn unexpected_char(position: usize, char: char) -> Self {
        Self::UnexpectedChar { position, char }
    }

    /// Create an UnexpectedToken error.
    #[must_use]
    pub fn unexpected_token(
        expected: impl Into<String>,
        found: impl Into<String>,
        position: usize,
    ) -> Self {
        Self::UnexpectedToken {
            expected: expected.into(),
            found: found.into(),
            position,
        }
    }

    /// Create an UnterminatedString error.
    #[must_use]
    pub fn unterminated_string(position: usize) -> Self {
        Self::UnterminatedString { position }
    }

    /// Create an UnknownField error.
    #[must_use]
    pub fn unknown_field(name: impl Into<String>, position: usize) -> Self {
        Self::UnknownField {
            name: name.into(),
            position,
        }
    }

    /// Create an InvalidValue error.
    #[must_use]
    pub fn invalid_value(
        field: impl Into<String>,
        value: impl Into<String>,
        hint: Option<String>,
        position: usize,
    ) -> Self {
        Self::InvalidValue {
            field: field.into(),
            value: value.into(),
            hint,
            position,
        }
    }

    /// Create an UnexpectedEof error.
    #[must_use]
    pub fn unexpected_eof(expected: impl Into<String>) -> Self {
        Self::UnexpectedEof {
            expected: expected.into(),
        }
    }

    /// Create a MissingValue error.
    #[must_use]
    pub fn missing_value(field: impl Into<String>, position: usize) -> Self {
        Self::MissingValue {
            field: field.into(),
            position,
        }
    }

    /// Get the position in the input where the error occurred, if available.
    #[must_use]
    pub fn position(&self) -> Option<usize> {
        match self {
            Self::UnexpectedChar { position, .. }
            | Self::UnexpectedToken { position, .. }
            | Self::UnterminatedString { position }
            | Self::UnknownField { position, .. }
            | Self::InvalidValue { position, .. }
            | Self::MissingValue { position, .. } => Some(*position),
            Self::EmptyExpression | Self::UnexpectedEof { .. } => None,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedChar { position, char } => {
                write!(f, "Unexpected character '{char}' at position {position}")
            }
            Self::UnexpectedToken {
                expected,
                found,
                position,
            } => {
                write!(
                    f,
                    "Expected {expected} but found '{found}' at position {position}"
                )
            }
            Self::UnterminatedString { position } => {
                write!(f, "Unterminated string starting at position {position}")
            }
            Self::UnknownField { name, position } => {
                write!(
                    f,
                    "Unknown field '{name}' at position {position}. Valid fields: priority, status, tags, project, due, created, scheduled, completed, modified, estimate, actual, search, has, title"
                )
            }
            Self::InvalidValue {
                field,
                value,
                hint,
                position,
            } => {
                write!(
                    f,
                    "Invalid value '{value}' for field '{field}' at position {position}"
                )?;
                if let Some(h) = hint {
                    write!(f, ". {h}")?;
                }
                Ok(())
            }
            Self::EmptyExpression => {
                write!(f, "Empty filter expression")
            }
            Self::UnexpectedEof { expected } => {
                write!(f, "Unexpected end of input, expected {expected}")
            }
            Self::MissingValue { field, position } => {
                write!(
                    f,
                    "Missing value for field '{field}' at position {position}"
                )
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Result type for filter DSL operations.
pub type ParseResult<T> = Result<T, ParseError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unexpected_char_display() {
        let err = ParseError::unexpected_char(5, '@');
        assert_eq!(err.to_string(), "Unexpected character '@' at position 5");
        assert_eq!(err.position(), Some(5));
    }

    #[test]
    fn test_unexpected_token_display() {
        let err = ParseError::unexpected_token("identifier", "AND", 10);
        assert_eq!(
            err.to_string(),
            "Expected identifier but found 'AND' at position 10"
        );
    }

    #[test]
    fn test_unterminated_string_display() {
        let err = ParseError::unterminated_string(3);
        assert_eq!(
            err.to_string(),
            "Unterminated string starting at position 3"
        );
    }

    #[test]
    fn test_unknown_field_display() {
        let err = ParseError::unknown_field("foo", 0);
        assert!(err.to_string().contains("Unknown field 'foo'"));
        assert!(err.to_string().contains("Valid fields:"));
    }

    #[test]
    fn test_invalid_value_display() {
        let err = ParseError::invalid_value(
            "priority",
            "extreme",
            Some("Try: none, low, medium, high, urgent".to_string()),
            9,
        );
        let msg = err.to_string();
        assert!(msg.contains("Invalid value 'extreme'"));
        assert!(msg.contains("field 'priority'"));
        assert!(msg.contains("Try:"));
    }

    #[test]
    fn test_invalid_value_without_hint() {
        let err = ParseError::invalid_value("status", "broken", None, 7);
        let msg = err.to_string();
        assert!(msg.contains("Invalid value 'broken'"));
        assert!(!msg.contains("Try:"));
    }

    #[test]
    fn test_empty_expression_display() {
        let err = ParseError::EmptyExpression;
        assert_eq!(err.to_string(), "Empty filter expression");
        assert_eq!(err.position(), None);
    }

    #[test]
    fn test_unexpected_eof_display() {
        let err = ParseError::unexpected_eof("value");
        assert_eq!(err.to_string(), "Unexpected end of input, expected value");
        assert_eq!(err.position(), None);
    }

    #[test]
    fn test_missing_value_display() {
        let err = ParseError::missing_value("priority", 8);
        assert_eq!(
            err.to_string(),
            "Missing value for field 'priority' at position 8"
        );
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ParseError>();
    }
}
