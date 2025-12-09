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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_new() {
        let tag = Tag::new("urgent");
        assert_eq!(tag.name, "urgent");
        assert!(tag.color.is_none());
        assert!(tag.description.is_none());
    }

    #[test]
    fn test_tag_with_color() {
        let tag = Tag::new("work").with_color("#3498db");
        assert_eq!(tag.name, "work");
        assert_eq!(tag.color, Some("#3498db".to_string()));
    }

    #[test]
    fn test_tag_display() {
        let tag = Tag::new("important");
        assert_eq!(tag.to_string(), "important");
    }

    #[test]
    fn test_tag_equality() {
        let tag1 = Tag::new("bug");
        let tag2 = Tag::new("bug");
        let tag3 = Tag::new("feature");

        assert_eq!(tag1, tag2);
        assert_ne!(tag1, tag3);
    }

    #[test]
    fn test_tag_equality_with_color() {
        let tag1 = Tag::new("bug").with_color("red");
        let tag2 = Tag::new("bug").with_color("red");
        let tag3 = Tag::new("bug").with_color("blue");

        assert_eq!(tag1, tag2);
        assert_ne!(tag1, tag3);
    }

    #[test]
    fn test_tag_serialization() {
        let tag = Tag::new("feature").with_color("#2ecc71");
        let json = serde_json::to_string(&tag).expect("serialize");
        let restored: Tag = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(tag, restored);
    }

    #[test]
    fn test_tag_serialization_without_color() {
        let tag = Tag::new("bug");
        let json = serde_json::to_string(&tag).expect("serialize");
        let restored: Tag = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(tag, restored);
        assert!(restored.color.is_none());
    }

    #[test]
    fn test_tag_from_string_types() {
        // Test that Tag::new accepts various string types
        let tag1 = Tag::new("literal");
        let tag2 = Tag::new(String::from("owned"));
        let s = "borrowed";
        let tag3 = Tag::new(s);

        assert_eq!(tag1.name, "literal");
        assert_eq!(tag2.name, "owned");
        assert_eq!(tag3.name, "borrowed");
    }
}
