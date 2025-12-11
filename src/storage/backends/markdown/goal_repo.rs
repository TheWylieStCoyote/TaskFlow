//! GoalRepository implementation for markdown backend.

use crate::domain::{Goal, GoalId, GoalStatus};
use crate::storage::{GoalRepository, StorageError, StorageResult};

use super::MarkdownBackend;

impl GoalRepository for MarkdownBackend {
    fn create_goal(&mut self, goal: &Goal) -> StorageResult<()> {
        if self.goals.iter().any(|g| g.id == goal.id) {
            return Err(StorageError::already_exists("Goal", goal.id.to_string()));
        }
        self.goals.push(goal.clone());
        self.save_goals()?;
        Ok(())
    }

    fn get_goal(&self, id: &GoalId) -> StorageResult<Option<Goal>> {
        Ok(self.goals.iter().find(|g| &g.id == id).cloned())
    }

    fn update_goal(&mut self, goal: &Goal) -> StorageResult<()> {
        if let Some(existing) = self.goals.iter_mut().find(|g| g.id == goal.id) {
            *existing = goal.clone();
            self.save_goals()?;
            Ok(())
        } else {
            Err(StorageError::not_found("Goal", goal.id.to_string()))
        }
    }

    fn delete_goal(&mut self, id: &GoalId) -> StorageResult<()> {
        let len_before = self.goals.len();
        self.goals.retain(|g| &g.id != id);
        if self.goals.len() == len_before {
            return Err(StorageError::not_found("Goal", id.to_string()));
        }
        // Also delete associated key results
        self.key_results.retain(|kr| &kr.goal_id != id);
        self.save_goals()?;
        self.save_key_results()?;
        Ok(())
    }

    fn list_goals(&self) -> StorageResult<Vec<Goal>> {
        Ok(self.goals.clone())
    }

    fn list_active_goals(&self) -> StorageResult<Vec<Goal>> {
        Ok(self
            .goals
            .iter()
            .filter(|g| g.status == GoalStatus::Active)
            .cloned()
            .collect())
    }
}
