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
