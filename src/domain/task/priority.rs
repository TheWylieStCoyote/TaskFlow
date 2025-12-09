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
