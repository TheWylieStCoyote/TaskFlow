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

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::domain::{KeyResultStatus, ProjectId, TaskId};
    use crate::storage::StorageBackend;

    fn create_test_backend() -> (tempfile::TempDir, MarkdownBackend) {
        let dir = tempdir().unwrap();
        let mut backend = MarkdownBackend::new(dir.path()).unwrap();
        backend.initialize().unwrap();
        (dir, backend)
    }

    fn full_key_result(goal_id: GoalId) -> KeyResult {
        let mut kr = KeyResult::new(goal_id, "Revenue")
            .with_description("Quarterly bookings")
            .with_status(KeyResultStatus::AtRisk)
            .with_target(250.5, Some("k$"))
            .with_current_value(99.25)
            .with_manual_progress(40);
        kr.linked_project_ids = vec![ProjectId::new()];
        kr.linked_task_ids = vec![TaskId::new(), TaskId::new()];
        kr
    }

    #[test]
    fn test_markdown_key_result_crud() {
        let (dir, mut backend) = create_test_backend();
        let kr = full_key_result(GoalId::new());

        backend.create_key_result(&kr).unwrap();
        assert!(dir.path().join("key_results.yaml").exists());
        assert_eq!(backend.get_key_result(&kr.id).unwrap(), Some(kr.clone()));

        let mut changed = kr.clone();
        changed.name = "ARR".to_string();
        changed.status = KeyResultStatus::Completed;
        changed.linked_task_ids.clear();
        backend.update_key_result(&changed).unwrap();
        assert_eq!(backend.get_key_result(&kr.id).unwrap(), Some(changed));
        assert_eq!(backend.list_key_results().unwrap().len(), 1);

        backend.delete_key_result(&kr.id).unwrap();
        assert_eq!(backend.get_key_result(&kr.id).unwrap(), None);
        assert!(backend.list_key_results().unwrap().is_empty());
    }

    #[test]
    fn test_markdown_key_result_persists_across_reload() {
        let dir = tempdir().unwrap();
        let kr = full_key_result(GoalId::new());
        {
            let mut backend = MarkdownBackend::new(dir.path()).unwrap();
            backend.initialize().unwrap();
            backend.create_key_result(&kr).unwrap();
        }

        let mut reloaded = MarkdownBackend::new(dir.path()).unwrap();
        reloaded.initialize().unwrap();
        assert_eq!(reloaded.list_key_results().unwrap(), vec![kr.clone()]);
        assert_eq!(reloaded.get_key_result(&kr.id).unwrap(), Some(kr));
    }

    #[test]
    fn test_markdown_key_result_duplicate_id_fails() {
        let (_dir, mut backend) = create_test_backend();
        let kr = KeyResult::new(GoalId::new(), "Once");
        backend.create_key_result(&kr).unwrap();

        let err = backend.create_key_result(&kr).unwrap_err();
        assert!(
            matches!(&err, StorageError::AlreadyExists { entity_type, id } if entity_type == "KeyResult" && *id == kr.id.to_string()),
            "unexpected error: {err:?}"
        );
        assert_eq!(backend.list_key_results().unwrap().len(), 1);
    }

    #[test]
    fn test_markdown_key_result_update_not_found() {
        let (_dir, mut backend) = create_test_backend();
        let kr = KeyResult::new(GoalId::new(), "Ghost");
        let err = backend.update_key_result(&kr).unwrap_err();
        assert!(
            matches!(&err, StorageError::NotFound { entity_type, id } if entity_type == "KeyResult" && *id == kr.id.to_string()),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn test_markdown_key_result_delete_not_found() {
        let (_dir, mut backend) = create_test_backend();
        let id = KeyResultId::new();
        let err = backend.delete_key_result(&id).unwrap_err();
        assert!(
            matches!(&err, StorageError::NotFound { entity_type, id: eid } if entity_type == "KeyResult" && *eid == id.to_string()),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn test_markdown_get_key_results_for_goal() {
        let (_dir, mut backend) = create_test_backend();
        let goal_a = GoalId::new();
        let goal_b = GoalId::new();
        let a1 = KeyResult::new(goal_a, "a1");
        let b1 = KeyResult::new(goal_b, "b1");
        let a2 = KeyResult::new(goal_a, "a2");
        for kr in [&a1, &b1, &a2] {
            backend.create_key_result(kr).unwrap();
        }

        assert_eq!(
            backend.get_key_results_for_goal(&goal_a).unwrap(),
            vec![a1, a2]
        );
        assert_eq!(backend.get_key_results_for_goal(&goal_b).unwrap(), vec![b1]);
        assert!(backend
            .get_key_results_for_goal(&GoalId::new())
            .unwrap()
            .is_empty());
        assert_eq!(backend.list_key_results().unwrap().len(), 3);
    }
}
