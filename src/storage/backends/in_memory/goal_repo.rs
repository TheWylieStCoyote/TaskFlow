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

#[cfg(test)]
mod tests {
    use super::*;
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
    fn create_and_get_goal_roundtrips_and_marks_dirty() {
        let mut backend = backend();
        let goal = Goal::new("Launch")
            .with_description("Ship it")
            .with_quarter(2025, crate::domain::Quarter::Q2)
            .with_manual_progress(30);

        backend.create_goal(&goal).unwrap();

        assert!(backend.dirty);
        assert_eq!(backend.get_goal(&goal.id).unwrap(), Some(goal.clone()));
        assert_eq!(backend.list_goals().unwrap(), vec![goal]);
    }

    #[test]
    fn get_goal_missing_returns_none() {
        let backend = backend();
        assert_eq!(backend.get_goal(&GoalId::new()).unwrap(), None);
        assert!(backend.list_goals().unwrap().is_empty());
    }

    #[test]
    fn create_goal_duplicate_id_is_rejected() {
        let mut backend = backend();
        let goal = Goal::new("Once");
        backend.create_goal(&goal).unwrap();
        backend.dirty = false;

        let mut again = goal.clone();
        again.name = "Twice".to_string();
        let err = backend.create_goal(&again).unwrap_err();

        assert!(
            matches!(&err, StorageError::AlreadyExists { entity_type, id } if entity_type == "Goal" && *id == goal.id.to_string()),
            "unexpected error: {err:?}"
        );
        assert!(!backend.dirty, "failed create must not mark dirty");
        assert_eq!(backend.list_goals().unwrap().len(), 1);
        assert_eq!(backend.get_goal(&goal.id).unwrap().unwrap().name, "Once");
    }

    #[test]
    fn update_goal_replaces_stored_value() {
        let mut backend = backend();
        let goal = Goal::new("Draft");
        backend.create_goal(&goal).unwrap();
        backend.dirty = false;

        let mut changed = goal.clone();
        changed.name = "Final".to_string();
        changed.status = GoalStatus::Completed;
        changed.color = Some("#123456".to_string());
        backend.update_goal(&changed).unwrap();

        assert!(backend.dirty);
        assert_eq!(backend.get_goal(&goal.id).unwrap(), Some(changed));
        assert_eq!(backend.list_goals().unwrap().len(), 1);
    }

    #[test]
    fn update_goal_not_found() {
        let mut backend = backend();
        let goal = Goal::new("Ghost");
        let err = backend.update_goal(&goal).unwrap_err();
        assert!(
            matches!(&err, StorageError::NotFound { entity_type, id } if entity_type == "Goal" && *id == goal.id.to_string()),
            "unexpected error: {err:?}"
        );
        assert!(!backend.dirty);
    }

    #[test]
    fn delete_goal_removes_only_that_goal() {
        let mut backend = backend();
        let keep = Goal::new("Keep");
        let drop = Goal::new("Drop");
        backend.create_goal(&keep).unwrap();
        backend.create_goal(&drop).unwrap();
        backend.dirty = false;

        backend.delete_goal(&drop.id).unwrap();

        assert!(backend.dirty);
        assert_eq!(backend.get_goal(&drop.id).unwrap(), None);
        assert_eq!(backend.list_goals().unwrap(), vec![keep]);
    }

    #[test]
    fn delete_goal_not_found() {
        let mut backend = backend();
        backend.create_goal(&Goal::new("Present")).unwrap();
        backend.dirty = false;

        let id = GoalId::new();
        let err = backend.delete_goal(&id).unwrap_err();
        assert!(
            matches!(&err, StorageError::NotFound { entity_type, id: eid } if entity_type == "Goal" && *eid == id.to_string()),
            "unexpected error: {err:?}"
        );
        assert!(!backend.dirty);
        assert_eq!(backend.list_goals().unwrap().len(), 1);
    }

    #[test]
    fn list_goals_preserves_insertion_order() {
        let mut backend = backend();
        for name in ["c", "a", "b"] {
            backend.create_goal(&Goal::new(name)).unwrap();
        }
        let names: Vec<String> = backend
            .list_goals()
            .unwrap()
            .into_iter()
            .map(|g| g.name)
            .collect();
        assert_eq!(names, vec!["c", "a", "b"]);
    }

    #[test]
    fn list_active_goals_filters_out_every_other_status() {
        let mut backend = backend();
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

        // Completing the last active goal empties the active list.
        let mut done = active[0].clone();
        done.complete();
        backend.update_goal(&done).unwrap();
        assert!(backend.list_active_goals().unwrap().is_empty());
    }
}
