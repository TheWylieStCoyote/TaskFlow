//! TagRepository implementation for JSON backend.

use crate::domain::Tag;
use crate::storage::{StorageError, StorageResult, TagRepository};

use super::JsonBackend;

impl TagRepository for JsonBackend {
    fn save_tag(&mut self, tag: &Tag) -> StorageResult<()> {
        if let Some(existing) = self.data.tags.iter_mut().find(|t| t.name == tag.name) {
            *existing = tag.clone();
        } else {
            self.data.tags.push(tag.clone());
        }
        self.mark_dirty();
        Ok(())
    }

    fn get_tag(&self, name: &str) -> StorageResult<Option<Tag>> {
        Ok(self.data.tags.iter().find(|t| t.name == name).cloned())
    }

    fn delete_tag(&mut self, name: &str) -> StorageResult<()> {
        let len_before = self.data.tags.len();
        self.data.tags.retain(|t| t.name != name);
        if self.data.tags.len() == len_before {
            return Err(StorageError::not_found("Tag", name));
        }
        self.mark_dirty();
        Ok(())
    }

    fn list_tags(&self) -> StorageResult<Vec<Tag>> {
        Ok(self.data.tags.clone())
    }
}
