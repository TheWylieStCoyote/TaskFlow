//! TagRepository implementation for markdown backend.

use crate::domain::Tag;
use crate::storage::{StorageError, StorageResult, TagRepository};

use super::MarkdownBackend;

impl TagRepository for MarkdownBackend {
    fn save_tag(&mut self, tag: &Tag) -> StorageResult<()> {
        if let Some(existing) = self.tags.iter_mut().find(|t| t.name == tag.name) {
            *existing = tag.clone();
        } else {
            self.tags.push(tag.clone());
        }
        self.save_tags()?;
        Ok(())
    }

    fn get_tag(&self, name: &str) -> StorageResult<Option<Tag>> {
        Ok(self.tags.iter().find(|t| t.name == name).cloned())
    }

    fn delete_tag(&mut self, name: &str) -> StorageResult<()> {
        let len_before = self.tags.len();
        self.tags.retain(|t| t.name != name);
        if self.tags.len() == len_before {
            return Err(StorageError::not_found("Tag", name));
        }
        self.save_tags()?;
        Ok(())
    }

    fn list_tags(&self) -> StorageResult<Vec<Tag>> {
        Ok(self.tags.clone())
    }
}
