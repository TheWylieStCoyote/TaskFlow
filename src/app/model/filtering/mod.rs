//! Task filtering and sorting methods for the Model.
//!
//! This module provides functionality for:
//! - Filtering tasks based on views, search, tags, and other criteria
//! - Sorting tasks by various fields
//! - Managing task selection state
//! - Rebuilding performance caches

mod matching;
mod selection;
mod visibility;

#[cfg(test)]
mod tests;

use super::Model;

/// Pre-computed filter values to avoid repeated allocations during filtering.
pub(super) struct FilterCache {
    /// Lowercased search text (if any)
    pub search_lower: Option<String>,
    /// Lowercased filter tags (if any)
    pub filter_tags_lower: Option<Vec<String>>,
}

impl FilterCache {
    /// Build cache from current filter settings.
    pub fn new(model: &Model) -> Self {
        Self {
            search_lower: model
                .filtering
                .filter
                .search_text
                .as_ref()
                .map(|s| s.to_lowercase()),
            filter_tags_lower: model
                .filtering
                .filter
                .tags
                .as_ref()
                .map(|tags| tags.iter().map(|t| t.to_lowercase()).collect()),
        }
    }
}
