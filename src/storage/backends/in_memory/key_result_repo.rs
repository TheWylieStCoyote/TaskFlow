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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{KeyResultStatus, ProjectId, TaskId};
    use crate::storage::ExportData;

    struct TestBackend {
        data: ExportData,
        dirty: bool,
    }

    impl InMemoryBackend for TestBackend {
        fn data(&self) -> &ExportData {
            &self.data
        }

        fn data_mut(&mut self) -> &mut ExportData {
            &mut self.data
        }

        fn mark_dirty(&mut self) {
            self.dirty = true;
        }
    }

    fn backend() -> TestBackend {
        TestBackend {
            data: ExportData::default(),
            dirty: false,
        }
    }

    #[test]
    fn create_and_get_key_result_roundtrips_and_marks_dirty() {
        let mut backend = backend();
        let goal_id = GoalId::new();
        let mut kr = KeyResult::new(goal_id, "Users")
            .with_description("Monthly actives")
            .with_status(KeyResultStatus::InProgress)
            .with_target(500.0, Some("users"))
            .with_current_value(120.0)
            .with_manual_progress(10);
        kr.linked_project_ids.push(ProjectId::new());
        kr.linked_task_ids.push(TaskId::new());

        backend.create_key_result(&kr).unwrap();

        assert!(backend.dirty);
        assert_eq!(backend.get_key_result(&kr.id).unwrap(), Some(kr.clone()));
        assert_eq!(backend.list_key_results().unwrap(), vec![kr]);
    }

    #[test]
    fn get_key_result_missing_returns_none() {
        let backend = backend();
        assert_eq!(backend.get_key_result(&KeyResultId::new()).unwrap(), None);
        assert!(backend.list_key_results().unwrap().is_empty());
    }

    #[test]
    fn create_key_result_duplicate_id_is_rejected() {
        let mut backend = backend();
        let kr = KeyResult::new(GoalId::new(), "Once");
        backend.create_key_result(&kr).unwrap();
        backend.dirty = false;

        let mut again = kr.clone();
        again.name = "Twice".to_string();
        let err = backend.create_key_result(&again).unwrap_err();

        assert!(
            matches!(&err, StorageError::AlreadyExists { entity_type, id } if entity_type == "KeyResult" && *id == kr.id.to_string()),
            "unexpected error: {err:?}"
        );
        assert!(!backend.dirty);
        assert_eq!(backend.list_key_results().unwrap().len(), 1);
        assert_eq!(
            backend.get_key_result(&kr.id).unwrap().unwrap().name,
            "Once"
        );
    }

    #[test]
    fn update_key_result_replaces_stored_value() {
        let mut backend = backend();
        let kr = KeyResult::new(GoalId::new(), "Draft").with_target(10.0, None);
        backend.create_key_result(&kr).unwrap();
        backend.dirty = false;

        let mut changed = kr.clone();
        changed.name = "Final".to_string();
        changed.set_value(10.0);
        changed.goal_id = GoalId::new();
        backend.update_key_result(&changed).unwrap();

        assert!(backend.dirty);
        let stored = backend.get_key_result(&kr.id).unwrap().unwrap();
        assert_eq!(stored, changed);
        assert_eq!(stored.status, KeyResultStatus::Completed);
        assert_eq!(backend.list_key_results().unwrap().len(), 1);
    }

    #[test]
    fn update_key_result_not_found() {
        let mut backend = backend();
        let kr = KeyResult::new(GoalId::new(), "Ghost");
        let err = backend.update_key_result(&kr).unwrap_err();
        assert!(
            matches!(&err, StorageError::NotFound { entity_type, id } if entity_type == "KeyResult" && *id == kr.id.to_string()),
            "unexpected error: {err:?}"
        );
        assert!(!backend.dirty);
    }

    #[test]
    fn delete_key_result_removes_only_that_entry() {
        let mut backend = backend();
        let goal_id = GoalId::new();
        let keep = KeyResult::new(goal_id, "Keep");
        let drop = KeyResult::new(goal_id, "Drop");
        backend.create_key_result(&keep).unwrap();
        backend.create_key_result(&drop).unwrap();
        backend.dirty = false;

        backend.delete_key_result(&drop.id).unwrap();

        assert!(backend.dirty);
        assert_eq!(backend.get_key_result(&drop.id).unwrap(), None);
        assert_eq!(backend.list_key_results().unwrap(), vec![keep]);
    }

    #[test]
    fn delete_key_result_not_found() {
        let mut backend = backend();
        backend
            .create_key_result(&KeyResult::new(GoalId::new(), "Present"))
            .unwrap();
        backend.dirty = false;

        let id = KeyResultId::new();
        let err = backend.delete_key_result(&id).unwrap_err();
        assert!(
            matches!(&err, StorageError::NotFound { entity_type, id: eid } if entity_type == "KeyResult" && *eid == id.to_string()),
            "unexpected error: {err:?}"
        );
        assert!(!backend.dirty);
        assert_eq!(backend.list_key_results().unwrap().len(), 1);
    }

    #[test]
    fn get_key_results_for_goal_filters_by_parent() {
        let mut backend = backend();
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
