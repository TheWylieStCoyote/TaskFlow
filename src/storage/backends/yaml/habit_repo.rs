//! HabitRepository implementation for YAML backend.

use crate::domain::{Habit, HabitId};
use crate::storage::{HabitRepository, StorageError, StorageResult};

use super::YamlBackend;

impl HabitRepository for YamlBackend {
    fn create_habit(&mut self, habit: &Habit) -> StorageResult<()> {
        if self.data.habits.iter().any(|h| h.id == habit.id) {
            return Err(StorageError::already_exists("Habit", habit.id.to_string()));
        }
        self.data.habits.push(habit.clone());
        self.mark_dirty();
        Ok(())
    }

    fn get_habit(&self, id: &HabitId) -> StorageResult<Option<Habit>> {
        Ok(self.data.habits.iter().find(|h| &h.id == id).cloned())
    }

    fn update_habit(&mut self, habit: &Habit) -> StorageResult<()> {
        if let Some(existing) = self.data.habits.iter_mut().find(|h| h.id == habit.id) {
            *existing = habit.clone();
            self.mark_dirty();
            Ok(())
        } else {
            Err(StorageError::not_found("Habit", habit.id.to_string()))
        }
    }

    fn delete_habit(&mut self, id: &HabitId) -> StorageResult<()> {
        let len_before = self.data.habits.len();
        self.data.habits.retain(|h| &h.id != id);
        if self.data.habits.len() == len_before {
            return Err(StorageError::not_found("Habit", id.to_string()));
        }
        self.mark_dirty();
        Ok(())
    }

    fn list_habits(&self) -> StorageResult<Vec<Habit>> {
        Ok(self.data.habits.clone())
    }

    fn list_active_habits(&self) -> StorageResult<Vec<Habit>> {
        Ok(self
            .data
            .habits
            .iter()
            .filter(|h| !h.archived)
            .cloned()
            .collect())
    }
}
