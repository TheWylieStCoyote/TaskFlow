//! GoalRepository blanket implementation for in-memory backends.

use crate::domain::{Goal, GoalId, GoalStatus};
use crate::storage::{GoalRepository, StorageError, StorageResult};

use super::InMemoryBackend;

impl<B: InMemoryBackend> GoalRepository for B {
    fn create_goal(&mut self, goal: &Goal) -> StorageResult<()> {
        if self.data().goals.iter().any(|g| g.id == goal.id) {
            return Err(StorageError::already_exists("Goal", goal.id.to_string()));
        }
        self.data_mut().goals.push(goal.clone());
        self.mark_dirty();
        Ok(())
    }

    fn get_goal(&self, id: &GoalId) -> StorageResult<Option<Goal>> {
        Ok(self.data().goals.iter().find(|g| &g.id == id).cloned())
    }

    fn update_goal(&mut self, goal: &Goal) -> StorageResult<()> {
        if let Some(existing) = self.data_mut().goals.iter_mut().find(|g| g.id == goal.id) {
            *existing = goal.clone();
            self.mark_dirty();
            Ok(())
        } else {
            Err(StorageError::not_found("Goal", goal.id.to_string()))
        }
    }

    fn delete_goal(&mut self, id: &GoalId) -> StorageResult<()> {
        let len_before = self.data().goals.len();
        self.data_mut().goals.retain(|g| &g.id != id);
        if self.data().goals.len() == len_before {
            return Err(StorageError::not_found("Goal", id.to_string()));
        }
        self.mark_dirty();
        Ok(())
    }

    fn list_goals(&self) -> StorageResult<Vec<Goal>> {
        Ok(self.data().goals.clone())
    }

    fn list_active_goals(&self) -> StorageResult<Vec<Goal>> {
        Ok(self
            .data()
            .goals
            .iter()
            .filter(|g| g.status == GoalStatus::Active)
            .cloned()
            .collect())
    }
}
