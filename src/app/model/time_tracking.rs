//! Time tracking methods for the Model.

use crate::domain::{TaskId, TimeEntry, TimeEntryId};

use super::Model;

impl Model {
    /// Starts time tracking for a task.
    ///
    /// If another task is currently being tracked, stops it before starting
    /// a new one. Creates a new time entry and sets it as active.
    ///
    /// Returns a tuple of:
    /// - The newly created time entry
    /// - Optionally, the stopped entry (before/after states) if one was running
    pub fn start_time_tracking(
        &mut self,
        task_id: TaskId,
    ) -> (TimeEntry, Option<(TimeEntry, TimeEntry)>) {
        // Stop any currently running timer and get before/after states
        let stopped_entry = self.stop_time_tracking_internal();

        // Start new timer
        let entry = TimeEntry::start(task_id);
        let entry_id = entry.id;
        self.time_entries.insert(entry_id, entry.clone());
        self.active_time_entry = Some(entry_id);
        self.sync_time_entry(&entry);
        self.storage.dirty = true;

        (entry, stopped_entry)
    }

    /// Stops the currently active time tracking session.
    ///
    /// Records the end time and calculates duration for the active entry.
    /// Also updates the task's actual_minutes with the total tracked time.
    ///
    /// Returns a tuple of (before, after) states for undo support, if there was
    /// an active entry to stop.
    pub fn stop_time_tracking(&mut self) -> Option<(TimeEntry, TimeEntry)> {
        self.stop_time_tracking_internal()
    }

    /// Internal method for stopping time tracking, returning before/after states
    fn stop_time_tracking_internal(&mut self) -> Option<(TimeEntry, TimeEntry)> {
        if let Some(ref entry_id) = self.active_time_entry {
            let (before, after, task_id) = {
                if let Some(entry) = self.time_entries.get_mut(entry_id) {
                    let before = entry.clone();
                    entry.stop();
                    let after = entry.clone();
                    (Some(before), Some(after), Some(entry.task_id))
                } else {
                    (None, None, None)
                }
            };

            // Sync the stopped entry to storage
            if let Some(ref entry) = after {
                self.sync_time_entry(entry);
            }

            // Update task's actual_minutes with total tracked time
            if let Some(ref task_id) = task_id {
                let total_minutes = self.total_time_for_task(task_id);
                if let Some(task) = self.tasks.get_mut(task_id) {
                    task.actual_minutes = total_minutes;
                }
                self.sync_task_by_id(task_id);
            }

            self.active_time_entry = None;
            self.storage.dirty = true;

            // Return before/after if we had an entry
            if let (Some(before), Some(after)) = (before, after) {
                return Some((before, after));
            }
        }
        None
    }

    /// Returns the currently active time entry, if any.
    #[must_use]
    pub fn active_time_entry(&self) -> Option<&TimeEntry> {
        self.active_time_entry
            .as_ref()
            .and_then(|id| self.time_entries.get(id))
    }

    /// Returns true if time is being tracked for the given task.
    #[must_use]
    pub fn is_tracking_task(&self, task_id: &TaskId) -> bool {
        self.active_time_entry()
            .is_some_and(|e| &e.task_id == task_id)
    }

    /// Returns total time tracked for a task in minutes.
    ///
    /// Sums the duration of all time entries for the given task.
    #[must_use]
    pub fn total_time_for_task(&self, task_id: &TaskId) -> u32 {
        self.time_entries
            .values()
            .filter(|e| &e.task_id == task_id)
            .map(TimeEntry::calculated_duration_minutes)
            .sum()
    }

    /// Returns all time entries for a task, sorted by start time (newest first).
    #[must_use]
    pub fn time_entries_for_task(&self, task_id: &TaskId) -> Vec<&TimeEntry> {
        let mut entries: Vec<_> = self
            .time_entries
            .values()
            .filter(|e| &e.task_id == task_id)
            .collect();
        entries.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        entries
    }

    /// Deletes a time entry by ID (used for undo).
    pub fn delete_time_entry(&mut self, entry_id: &TimeEntryId) {
        // If this is the active entry, clear it
        if self.active_time_entry.as_ref() == Some(entry_id) {
            self.active_time_entry = None;
        }
        self.time_entries.remove(entry_id);
        // Also delete from storage
        if let Some(ref mut backend) = self.storage.backend {
            let _ = backend.delete_time_entry(entry_id);
        }
        self.storage.dirty = true;
    }

    /// Restores a time entry (used for undo).
    ///
    /// If the entry is still running (no ended_at), it becomes the active entry,
    /// but only if:
    /// - The associated task still exists
    /// - There's no current active entry (to avoid stealing the timer)
    pub fn restore_time_entry(&mut self, entry: TimeEntry) {
        let is_running = entry.ended_at.is_none();
        let task_exists = self.tasks.contains_key(&entry.task_id);
        let no_active_entry = self.active_time_entry.is_none();
        let entry_id = entry.id;

        self.time_entries.insert(entry_id, entry.clone());

        // Only make active if: running, task exists, AND no current active entry
        if is_running && task_exists && no_active_entry {
            self.active_time_entry = Some(entry_id);
        }

        self.sync_time_entry(&entry);
        self.storage.dirty = true;
    }
}
