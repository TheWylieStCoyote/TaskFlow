//! KeyResultRepository implementation for markdown backend.

use crate::domain::{GoalId, KeyResult, KeyResultId};
use crate::storage::{KeyResultRepository, StorageError, StorageResult};

use super::MarkdownBackend;

impl KeyResultRepository for MarkdownBackend {
    fn create_key_result(&mut self, kr: &KeyResult) -> StorageResult<()> {
        if self.key_results.iter().any(|k| k.id == kr.id) {
            return Err(StorageError::already_exists("KeyResult", kr.id.to_string()));
        }
        self.key_results.push(kr.clone());
        self.save_key_results()?;
        Ok(())
    }

    fn get_key_result(&self, id: &KeyResultId) -> StorageResult<Option<KeyResult>> {
        Ok(self.key_results.iter().find(|k| &k.id == id).cloned())
    }

    fn update_key_result(&mut self, kr: &KeyResult) -> StorageResult<()> {
        if let Some(existing) = self.key_results.iter_mut().find(|k| k.id == kr.id) {
            *existing = kr.clone();
            self.save_key_results()?;
            Ok(())
        } else {
            Err(StorageError::not_found("KeyResult", kr.id.to_string()))
        }
    }

    fn delete_key_result(&mut self, id: &KeyResultId) -> StorageResult<()> {
        let len_before = self.key_results.len();
        self.key_results.retain(|k| &k.id != id);
        if self.key_results.len() == len_before {
            return Err(StorageError::not_found("KeyResult", id.to_string()));
        }
        self.save_key_results()?;
        Ok(())
    }

    fn list_key_results(&self) -> StorageResult<Vec<KeyResult>> {
        Ok(self.key_results.clone())
    }

    fn get_key_results_for_goal(&self, goal_id: &GoalId) -> StorageResult<Vec<KeyResult>> {
        Ok(self
            .key_results
            .iter()
            .filter(|k| &k.goal_id == goal_id)
            .cloned()
            .collect())
    }
}
