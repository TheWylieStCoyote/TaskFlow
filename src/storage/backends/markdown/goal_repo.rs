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

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::domain::{KeyResult, Quarter};
    use crate::storage::{KeyResultRepository, StorageBackend};

    fn create_test_backend() -> (tempfile::TempDir, MarkdownBackend) {
        let dir = tempdir().unwrap();
        let mut backend = MarkdownBackend::new(dir.path()).unwrap();
        backend.initialize().unwrap();
        (dir, backend)
    }

    #[test]
    fn test_markdown_goal_crud() {
        let (dir, mut backend) = create_test_backend();
        let goal = Goal::new("Launch")
            .with_description("Ship the thing")
            .with_quarter(2025, Quarter::Q4)
            .with_manual_progress(25)
            .with_color("#abcdef")
            .with_icon("🎯");

        backend.create_goal(&goal).unwrap();
        assert!(dir.path().join("goals.yaml").exists());
        assert_eq!(backend.get_goal(&goal.id).unwrap(), Some(goal.clone()));

        let mut changed = goal.clone();
        changed.name = "Launched".to_string();
        changed.status = GoalStatus::Completed;
        backend.update_goal(&changed).unwrap();
        assert_eq!(backend.get_goal(&goal.id).unwrap(), Some(changed));
        assert_eq!(backend.list_goals().unwrap().len(), 1);

        backend.delete_goal(&goal.id).unwrap();
        assert_eq!(backend.get_goal(&goal.id).unwrap(), None);
        assert!(backend.list_goals().unwrap().is_empty());
    }

    #[test]
    fn test_markdown_goal_persists_across_reload() {
        let dir = tempdir().unwrap();
        let goal = Goal::new("Durable")
            .with_dates(
                chrono::NaiveDate::from_ymd_opt(2025, 1, 1),
                chrono::NaiveDate::from_ymd_opt(2025, 2, 28),
            )
            .with_status(GoalStatus::OnHold);
        {
            let mut backend = MarkdownBackend::new(dir.path()).unwrap();
            backend.initialize().unwrap();
            backend.create_goal(&goal).unwrap();
        }

        let mut reloaded = MarkdownBackend::new(dir.path()).unwrap();
        reloaded.initialize().unwrap();
        assert_eq!(reloaded.list_goals().unwrap(), vec![goal.clone()]);
        assert_eq!(reloaded.get_goal(&goal.id).unwrap(), Some(goal));
    }

    #[test]
    fn test_markdown_goal_duplicate_id_fails() {
        let (_dir, mut backend) = create_test_backend();
        let goal = Goal::new("Once");
        backend.create_goal(&goal).unwrap();

        let err = backend.create_goal(&goal).unwrap_err();
        assert!(
            matches!(&err, StorageError::AlreadyExists { entity_type, id } if entity_type == "Goal" && *id == goal.id.to_string()),
            "unexpected error: {err:?}"
        );
        assert_eq!(backend.list_goals().unwrap().len(), 1);
    }

    #[test]
    fn test_markdown_goal_update_not_found() {
        let (_dir, mut backend) = create_test_backend();
        let goal = Goal::new("Ghost");
        let err = backend.update_goal(&goal).unwrap_err();
        assert!(
            matches!(&err, StorageError::NotFound { entity_type, id } if entity_type == "Goal" && *id == goal.id.to_string()),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn test_markdown_goal_delete_not_found() {
        let (_dir, mut backend) = create_test_backend();
        let id = GoalId::new();
        let err = backend.delete_goal(&id).unwrap_err();
        assert!(
            matches!(&err, StorageError::NotFound { entity_type, id: eid } if entity_type == "Goal" && *eid == id.to_string()),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn test_markdown_delete_goal_removes_its_key_results() {
        let dir = tempdir().unwrap();
        let keep = Goal::new("Keep");
        let drop = Goal::new("Drop");
        let kr_keep = KeyResult::new(keep.id, "kr-keep");
        {
            let mut backend = MarkdownBackend::new(dir.path()).unwrap();
            backend.initialize().unwrap();
            backend.create_goal(&keep).unwrap();
            backend.create_goal(&drop).unwrap();
            backend.create_key_result(&kr_keep).unwrap();
            backend
                .create_key_result(&KeyResult::new(drop.id, "kr-drop-1"))
                .unwrap();
            backend
                .create_key_result(&KeyResult::new(drop.id, "kr-drop-2"))
                .unwrap();

            backend.delete_goal(&drop.id).unwrap();

            assert_eq!(backend.list_goals().unwrap(), vec![keep.clone()]);
            assert_eq!(backend.list_key_results().unwrap(), vec![kr_keep.clone()]);
            assert!(backend
                .get_key_results_for_goal(&drop.id)
                .unwrap()
                .is_empty());
        }

        // Both goals.yaml and key_results.yaml were rewritten.
        let mut reloaded = MarkdownBackend::new(dir.path()).unwrap();
        reloaded.initialize().unwrap();
        assert_eq!(reloaded.list_goals().unwrap(), vec![keep]);
        assert_eq!(reloaded.list_key_results().unwrap(), vec![kr_keep]);
    }

    #[test]
    fn test_markdown_list_active_goals() {
        let (_dir, mut backend) = create_test_backend();
        backend
            .create_goal(&Goal::new("active").with_status(GoalStatus::Active))
            .unwrap();
        backend
            .create_goal(&Goal::new("on hold").with_status(GoalStatus::OnHold))
            .unwrap();
        backend
            .create_goal(&Goal::new("completed").with_status(GoalStatus::Completed))
            .unwrap();
        backend
            .create_goal(&Goal::new("archived").with_status(GoalStatus::Archived))
            .unwrap();

        let active = backend.list_active_goals().unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name, "active");
        assert_eq!(backend.list_goals().unwrap().len(), 4);
    }
}
