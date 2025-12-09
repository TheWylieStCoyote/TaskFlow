//! HabitRepository implementation for markdown backend.

use crate::domain::{Habit, HabitId};
use crate::storage::{HabitRepository, StorageError, StorageResult};

use super::MarkdownBackend;

impl HabitRepository for MarkdownBackend {
    fn create_habit(&mut self, habit: &Habit) -> StorageResult<()> {
        if self.habits.iter().any(|h| h.id == habit.id) {
            return Err(StorageError::already_exists("Habit", habit.id.to_string()));
        }
        self.habits.push(habit.clone());
        self.save_habits()?;
        Ok(())
    }

    fn get_habit(&self, id: &HabitId) -> StorageResult<Option<Habit>> {
        Ok(self.habits.iter().find(|h| &h.id == id).cloned())
    }

    fn update_habit(&mut self, habit: &Habit) -> StorageResult<()> {
        if let Some(existing) = self.habits.iter_mut().find(|h| h.id == habit.id) {
            *existing = habit.clone();
            self.save_habits()?;
            Ok(())
        } else {
            Err(StorageError::not_found("Habit", habit.id.to_string()))
        }
    }

    fn delete_habit(&mut self, id: &HabitId) -> StorageResult<()> {
        let len_before = self.habits.len();
        self.habits.retain(|h| &h.id != id);
        if self.habits.len() == len_before {
            return Err(StorageError::not_found("Habit", id.to_string()));
        }
        self.save_habits()?;
        Ok(())
    }

    fn list_habits(&self) -> StorageResult<Vec<Habit>> {
        Ok(self.habits.clone())
    }

    fn list_active_habits(&self) -> StorageResult<Vec<Habit>> {
        Ok(self
            .habits
            .iter()
            .filter(|h| !h.archived)
            .cloned()
            .collect())
    }
}
