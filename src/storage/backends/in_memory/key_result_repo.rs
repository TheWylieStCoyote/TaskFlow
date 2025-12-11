//! KeyResultRepository blanket implementation for in-memory backends.

use crate::domain::{GoalId, KeyResult, KeyResultId};
use crate::storage::{KeyResultRepository, StorageError, StorageResult};

use super::InMemoryBackend;

impl<B: InMemoryBackend> KeyResultRepository for B {
    fn create_key_result(&mut self, kr: &KeyResult) -> StorageResult<()> {
        if self.data().key_results.iter().any(|k| k.id == kr.id) {
            return Err(StorageError::already_exists("KeyResult", kr.id.to_string()));
        }
        self.data_mut().key_results.push(kr.clone());
        self.mark_dirty();
        Ok(())
    }

    fn get_key_result(&self, id: &KeyResultId) -> StorageResult<Option<KeyResult>> {
        Ok(self
            .data()
            .key_results
            .iter()
            .find(|k| &k.id == id)
            .cloned())
    }

    fn update_key_result(&mut self, kr: &KeyResult) -> StorageResult<()> {
        if let Some(existing) = self
            .data_mut()
            .key_results
            .iter_mut()
            .find(|k| k.id == kr.id)
        {
            *existing = kr.clone();
            self.mark_dirty();
            Ok(())
        } else {
            Err(StorageError::not_found("KeyResult", kr.id.to_string()))
        }
    }

    fn delete_key_result(&mut self, id: &KeyResultId) -> StorageResult<()> {
        let len_before = self.data().key_results.len();
        self.data_mut().key_results.retain(|k| &k.id != id);
        if self.data().key_results.len() == len_before {
            return Err(StorageError::not_found("KeyResult", id.to_string()));
        }
        self.mark_dirty();
        Ok(())
    }

    fn list_key_results(&self) -> StorageResult<Vec<KeyResult>> {
        Ok(self.data().key_results.clone())
    }

    fn get_key_results_for_goal(&self, goal_id: &GoalId) -> StorageResult<Vec<KeyResult>> {
        Ok(self
            .data()
            .key_results
            .iter()
            .filter(|k| &k.goal_id == goal_id)
            .cloned()
            .collect())
    }
}
