//! TagRepository implementation for SQLite backend.

use rusqlite::{params, OptionalExtension};

use crate::domain::Tag;
use crate::storage::{StorageError, StorageResult, TagRepository};

use super::SqliteBackend;

impl TagRepository for SqliteBackend {
    fn save_tag(&mut self, tag: &Tag) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO tags (name, color, description) VALUES (?1, ?2, ?3)",
            params![tag.name, tag.color, tag.description],
        )?;
        Ok(())
    }

    fn get_tag(&self, name: &str) -> StorageResult<Option<Tag>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM tags WHERE name = ?1")?;
        let tag = stmt
            .query_row(params![name], |row| {
                Ok(Tag {
                    name: row.get("name")?,
                    color: row.get("color")?,
                    description: row.get("description")?,
                })
            })
            .optional()?;
        Ok(tag)
    }

    fn delete_tag(&mut self, name: &str) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        let rows = conn.execute("DELETE FROM tags WHERE name = ?1", params![name])?;
        if rows == 0 {
            return Err(StorageError::not_found("Tag", name));
        }
        Ok(())
    }

    fn list_tags(&self) -> StorageResult<Vec<Tag>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM tags")?;
        let tags = stmt
            .query_map([], |row| {
                Ok(Tag {
                    name: row.get("name")?,
                    color: row.get("color")?,
                    description: row.get("description")?,
                })
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(tags)
    }
}
