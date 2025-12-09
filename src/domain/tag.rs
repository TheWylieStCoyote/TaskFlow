//! Tag entity for task categorization.
//!
//! Tags provide a flexible way to categorize and filter tasks across projects.
//! Unlike projects which are hierarchical, tags are flat labels that can be
//! applied to any task.
//!
//! # Examples
//!
//! ```
//! use taskflow::domain::Tag;
//!
//! let tag = Tag::new("urgent").with_color("#e74c3c");
//! assert_eq!(tag.name, "urgent");
//! ```

use serde::{Deserialize, Serialize};

/// Tag entity for categorizing tasks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
}

impl Tag {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            color: None,
            description: None,
        }
    }

    #[must_use]
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
