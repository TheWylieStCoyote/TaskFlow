//! Task priority levels.

use serde::{Deserialize, Serialize};

/// Task priority levels from lowest to highest urgency.
///
/// Priority helps organize tasks by importance. Each level has an
/// associated symbol displayed in the UI.
///
/// # Examples
///
/// ```
/// use taskflow::domain::Priority;
///
/// let priority = Priority::High;
/// assert_eq!(priority.symbol(), "!!!");
/// assert_eq!(priority.as_str(), "high");
///
/// // Parse from string (case-insensitive)
/// assert_eq!(Priority::parse("HIGH"), Some(Priority::High));
/// assert_eq!(Priority::parse("med"), Some(Priority::Medium));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    /// No priority assigned (default)
    #[default]
    None,
    /// Low priority - nice to have, backlog items
    Low,
    /// Medium priority - standard work items
    Medium,
    /// High priority - important features, upcoming deadlines
    High,
    /// Urgent priority - critical issues, production bugs
    Urgent,
}

impl Priority {
    /// Returns the priority as a lowercase string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }

    /// Parses a priority from a string (case-insensitive).
    ///
    /// Accepts "med" as a shorthand for "medium".
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "none" => Some(Self::None),
            "low" => Some(Self::Low),
            "medium" | "med" => Some(Self::Medium),
            "high" => Some(Self::High),
            "urgent" => Some(Self::Urgent),
            _ => None,
        }
    }

    /// Returns the visual symbol for this priority level.
    ///
    /// Used in the UI to show priority at a glance.
    #[must_use]
    pub const fn symbol(&self) -> &'static str {
        match self {
            Self::None => " ",
            Self::Low => "!",
            Self::Medium => "!!",
            Self::High => "!!!",
            Self::Urgent => "!!!!",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_default() {
        assert_eq!(Priority::default(), Priority::None);
    }

    #[test]
    fn test_priority_as_str() {
        assert_eq!(Priority::None.as_str(), "none");
        assert_eq!(Priority::Low.as_str(), "low");
        assert_eq!(Priority::Medium.as_str(), "medium");
        assert_eq!(Priority::High.as_str(), "high");
        assert_eq!(Priority::Urgent.as_str(), "urgent");
    }

    #[test]
    fn test_priority_symbol() {
        assert_eq!(Priority::None.symbol(), " ");
        assert_eq!(Priority::Low.symbol(), "!");
        assert_eq!(Priority::Medium.symbol(), "!!");
        assert_eq!(Priority::High.symbol(), "!!!");
        assert_eq!(Priority::Urgent.symbol(), "!!!!");
    }

    #[test]
    fn test_priority_parse_lowercase() {
        assert_eq!(Priority::parse("none"), Some(Priority::None));
        assert_eq!(Priority::parse("low"), Some(Priority::Low));
        assert_eq!(Priority::parse("medium"), Some(Priority::Medium));
        assert_eq!(Priority::parse("high"), Some(Priority::High));
        assert_eq!(Priority::parse("urgent"), Some(Priority::Urgent));
    }

    #[test]
    fn test_priority_parse_uppercase() {
        assert_eq!(Priority::parse("NONE"), Some(Priority::None));
        assert_eq!(Priority::parse("LOW"), Some(Priority::Low));
        assert_eq!(Priority::parse("MEDIUM"), Some(Priority::Medium));
        assert_eq!(Priority::parse("HIGH"), Some(Priority::High));
        assert_eq!(Priority::parse("URGENT"), Some(Priority::Urgent));
    }

    #[test]
    fn test_priority_parse_mixed_case() {
        assert_eq!(Priority::parse("None"), Some(Priority::None));
        assert_eq!(Priority::parse("Low"), Some(Priority::Low));
        assert_eq!(Priority::parse("MeDiUm"), Some(Priority::Medium));
    }

    #[test]
    fn test_priority_parse_shorthand() {
        assert_eq!(Priority::parse("med"), Some(Priority::Medium));
        assert_eq!(Priority::parse("MED"), Some(Priority::Medium));
    }

    #[test]
    fn test_priority_parse_invalid() {
        assert_eq!(Priority::parse(""), None);
        assert_eq!(Priority::parse("invalid"), None);
        assert_eq!(Priority::parse("hi"), None);
        assert_eq!(Priority::parse("lo"), None);
    }

    #[test]
    fn test_priority_serialization() {
        for priority in [
            Priority::None,
            Priority::Low,
            Priority::Medium,
            Priority::High,
            Priority::Urgent,
        ] {
            let json = serde_json::to_string(&priority).expect("serialize");
            let restored: Priority = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(priority, restored);
        }
    }
}
