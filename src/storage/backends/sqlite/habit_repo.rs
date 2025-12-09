//! HabitRepository implementation for SQLite backend.

use std::collections::HashMap;

use chrono::{NaiveDate, Utc};
use rusqlite::{params, OptionalExtension};

use crate::domain::{Habit, HabitCheckIn, HabitId};
use crate::storage::{HabitRepository, StorageError, StorageResult};

use super::rows::habit_from_row;
use super::SqliteBackend;

impl SqliteBackend {
    /// Load check-ins for a habit from the database.
    fn load_habit_check_ins(
        &self,
        habit_id: &HabitId,
    ) -> StorageResult<HashMap<NaiveDate, HabitCheckIn>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare(
            "SELECT date, completed, note, checked_at FROM habit_check_ins WHERE habit_id = ?1",
        )?;

        let check_ins = stmt
            .query_map(params![habit_id.0.to_string()], |row| {
                let date_str: String = row.get(0)?;
                let completed: i32 = row.get(1)?;
                let note: Option<String> = row.get(2)?;
                let checked_at_str: String = row.get(3)?;

                let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                    .unwrap_or_else(|_| Utc::now().date_naive());
                let checked_at = chrono::DateTime::parse_from_rfc3339(&checked_at_str)
                    .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));

                Ok((
                    date,
                    HabitCheckIn {
                        date,
                        completed: completed != 0,
                        note,
                        checked_at,
                    },
                ))
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(check_ins)
    }

    /// Save check-ins for a habit to the database.
    fn save_habit_check_ins(&self, habit: &Habit) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        let habit_id_str = habit.id.0.to_string();

        // Delete existing check-ins
        conn.execute(
            "DELETE FROM habit_check_ins WHERE habit_id = ?1",
            params![habit_id_str],
        )?;

        // Insert new check-ins
        for (date, check_in) in &habit.check_ins {
            conn.execute(
                r"INSERT INTO habit_check_ins (habit_id, date, completed, note, checked_at)
                VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    habit_id_str,
                    date.format("%Y-%m-%d").to_string(),
                    i32::from(check_in.completed),
                    check_in.note,
                    check_in.checked_at.to_rfc3339(),
                ],
            )?;
        }

        Ok(())
    }
}

impl HabitRepository for SqliteBackend {
    fn create_habit(&mut self, habit: &Habit) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        let frequency_json = serde_json::to_string(&habit.frequency)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        let tags_json = serde_json::to_string(&habit.tags)
            .map_err(|e| StorageError::serialization(e.to_string()))?;

        conn.execute(
            r"INSERT INTO habits (id, name, description, frequency, start_date, end_date,
                color, icon, tags, archived, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                habit.id.0.to_string(),
                habit.name,
                habit.description,
                frequency_json,
                habit.start_date.format("%Y-%m-%d").to_string(),
                habit.end_date.map(|d| d.format("%Y-%m-%d").to_string()),
                habit.color,
                habit.icon,
                tags_json,
                i32::from(habit.archived),
                habit.created_at.to_rfc3339(),
                habit.updated_at.to_rfc3339(),
            ],
        )?;

        // Save check-ins
        self.save_habit_check_ins(habit)?;

        Ok(())
    }

    fn get_habit(&self, id: &HabitId) -> StorageResult<Option<Habit>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM habits WHERE id = ?1")?;

        let habit = stmt
            .query_row(params![id.0.to_string()], habit_from_row)
            .optional()?;

        if let Some(mut habit) = habit {
            habit.check_ins = self.load_habit_check_ins(&habit.id)?;
            Ok(Some(habit))
        } else {
            Ok(None)
        }
    }

    fn update_habit(&mut self, habit: &Habit) -> StorageResult<()> {
        let conn = self.inner.conn()?;
        let frequency_json = serde_json::to_string(&habit.frequency)
            .map_err(|e| StorageError::serialization(e.to_string()))?;
        let tags_json = serde_json::to_string(&habit.tags)
            .map_err(|e| StorageError::serialization(e.to_string()))?;

        let rows = conn.execute(
            r"UPDATE habits SET name = ?2, description = ?3, frequency = ?4,
                start_date = ?5, end_date = ?6, color = ?7, icon = ?8,
                tags = ?9, archived = ?10, updated_at = ?11
            WHERE id = ?1",
            params![
                habit.id.0.to_string(),
                habit.name,
                habit.description,
                frequency_json,
                habit.start_date.format("%Y-%m-%d").to_string(),
                habit.end_date.map(|d| d.format("%Y-%m-%d").to_string()),
                habit.color,
                habit.icon,
                tags_json,
                i32::from(habit.archived),
                habit.updated_at.to_rfc3339(),
            ],
        )?;

        if rows == 0 {
            return Err(StorageError::not_found("Habit", habit.id.0.to_string()));
        }

        // Update check-ins
        self.save_habit_check_ins(habit)?;

        Ok(())
    }

    fn delete_habit(&mut self, id: &HabitId) -> StorageResult<()> {
        let conn = self.inner.conn()?;

        // Check-ins are deleted via ON DELETE CASCADE
        let rows = conn.execute(
            "DELETE FROM habits WHERE id = ?1",
            params![id.0.to_string()],
        )?;

        if rows == 0 {
            return Err(StorageError::not_found("Habit", id.0.to_string()));
        }

        Ok(())
    }

    fn list_habits(&self) -> StorageResult<Vec<Habit>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM habits ORDER BY name")?;

        let habits: Vec<Habit> = stmt
            .query_map([], habit_from_row)?
            .filter_map(Result::ok)
            .collect();

        // Load check-ins for each habit
        let mut result = Vec::with_capacity(habits.len());
        for mut habit in habits {
            habit.check_ins = self.load_habit_check_ins(&habit.id)?;
            result.push(habit);
        }

        Ok(result)
    }

    fn list_active_habits(&self) -> StorageResult<Vec<Habit>> {
        let conn = self.inner.conn()?;
        let mut stmt = conn.prepare("SELECT * FROM habits WHERE archived = 0 ORDER BY name")?;

        let habits: Vec<Habit> = stmt
            .query_map([], habit_from_row)?
            .filter_map(Result::ok)
            .collect();

        // Load check-ins for each habit
        let mut result = Vec::with_capacity(habits.len());
        for mut habit in habits {
            habit.check_ins = self.load_habit_check_ins(&habit.id)?;
            result.push(habit);
        }

        Ok(result)
    }
}
