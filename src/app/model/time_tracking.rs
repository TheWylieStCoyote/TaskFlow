//! Time tracking methods for the Model.

use crate::domain::{time_entry_display_order, TaskId, TimeEntry, TimeEntryId};

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
        self.recompute_task_time(&task_id);

        (entry, stopped_entry)
    }

    /// Recompute a task's cached `actual_minutes` from its time entries and
    /// refresh every derived cache.
    ///
    /// This is the single place that keeps `Task::actual_minutes`, the task
    /// list's time sums and the report cache consistent with `time_entries`.
    /// Call it after any insert, edit or delete of a time entry.
    pub fn recompute_task_time(&mut self, task_id: &TaskId) {
        let total = self.total_time_for_task(task_id);
        let changed = match self.tasks.get_mut(task_id) {
            Some(task) if task.actual_minutes != total => {
                task.actual_minutes = total;
                true
            }
            _ => false,
        };
        if changed {
            self.sync_task_by_id(task_id);
        }
        self.task_cache.rebuild_time_sums(&self.time_entries);
        self.invalidate_report_cache();
        self.storage.dirty = true;
    }

    /// Add a new (or restored) time entry, persist it and recompute its task.
    ///
    /// A running entry becomes the active timer when its task exists and no
    /// other timer is running.
    pub fn add_time_entry(&mut self, entry: TimeEntry) {
        self.restore_time_entry(entry);
    }

    /// Replace an existing entry with an edited copy, persist it and
    /// recompute its task. Falls back to an insert if the id is unknown.
    pub fn upsert_time_entry(&mut self, entry: TimeEntry) {
        let task_id = entry.task_id;
        let previous_task = self
            .time_entries
            .insert(entry.id, entry.clone())
            .map(|old| old.task_id);
        self.sync_time_entry(&entry);
        if let Some(old_task) = previous_task.filter(|t| *t != task_id) {
            self.recompute_task_time(&old_task);
        }
        self.recompute_task_time(&task_id);
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

            self.active_time_entry = None;
            self.storage.dirty = true;

            if let Some(ref task_id) = task_id {
                self.recompute_task_time(task_id);
            }

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

    /// Returns all time entries for a task in display order: days newest
    /// first, and within a day in chronological order (see
    /// [`time_entry_display_order`]).
    ///
    /// Uses cached task→entry index for O(k) lookup where k = entries for task.
    #[must_use]
    pub fn time_entries_for_task(&self, task_id: &TaskId) -> Vec<&TimeEntry> {
        let mut entries: Vec<_> = self
            .task_cache
            .time_entries_by_task
            .get(task_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.time_entries.get(id))
                    .collect()
            })
            .unwrap_or_default();
        entries.sort_by(|a, b| time_entry_display_order(a, b));
        entries
    }

    /// Deletes a time entry by ID, persists the removal and recomputes the
    /// owning task's total (also used by undo).
    pub fn delete_time_entry(&mut self, entry_id: &TimeEntryId) {
        // If this is the active entry, clear it
        if self.active_time_entry.as_ref() == Some(entry_id) {
            self.active_time_entry = None;
        }
        let task_id = self.time_entries.remove(entry_id).map(|e| e.task_id);
        // Also delete from storage
        if let Some(ref mut backend) = self.storage.backend {
            let _ = backend.delete_time_entry(entry_id);
        }
        self.storage.dirty = true;
        if let Some(task_id) = task_id {
            self.recompute_task_time(&task_id);
        }
    }

    /// Restores a time entry (used for undo and redo).
    ///
    /// If the entry is still running (no ended_at), it becomes the active entry,
    /// but only if:
    /// - The associated task still exists
    /// - There's no current active entry (to avoid stealing the timer)
    ///
    /// If the entry is stopped and the active timer points at this very entry
    /// (redo of a stop, or an edit that ended the running entry), the active
    /// timer is cleared so the model never reports a finished entry as running.
    pub fn restore_time_entry(&mut self, entry: TimeEntry) {
        let is_running = entry.ended_at.is_none();
        let task_exists = self.tasks.contains_key(&entry.task_id);
        let entry_id = entry.id;

        self.time_entries.insert(entry_id, entry.clone());

        if is_running {
            // Only make active if the task exists and no other timer is running.
            if task_exists && self.active_time_entry.is_none() {
                self.active_time_entry = Some(entry_id);
            }
        } else if self.active_time_entry == Some(entry_id) {
            self.active_time_entry = None;
        }

        self.sync_time_entry(&entry);
        self.recompute_task_time(&entry.task_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Task;

    fn make_model() -> Model {
        Model::new()
    }

    #[test]
    fn test_start_time_tracking() {
        let mut model = make_model();
        let task = Task::new("Track me");
        let task_id = task.id;
        model.tasks.insert(task_id, task);

        let (entry, stopped) = model.start_time_tracking(task_id);

        assert_eq!(entry.task_id, task_id);
        assert!(entry.ended_at.is_none()); // Still running
        assert!(stopped.is_none()); // No previous timer
        assert!(model.active_time_entry.is_some());
    }

    #[test]
    fn test_start_time_tracking_switches_task() {
        let mut model = make_model();
        let task_a = Task::new("Task A");
        let task_b = Task::new("Task B");
        let a_id = task_a.id;
        let b_id = task_b.id;
        model.tasks.insert(a_id, task_a);
        model.tasks.insert(b_id, task_b);

        // Start tracking task A
        model.start_time_tracking(a_id);
        assert!(model.is_tracking_task(&a_id));

        // Start tracking task B - should stop A
        let (entry_b, stopped) = model.start_time_tracking(b_id);

        assert_eq!(entry_b.task_id, b_id);
        assert!(stopped.is_some()); // Previous A entry was stopped
        assert!(model.is_tracking_task(&b_id));
        assert!(!model.is_tracking_task(&a_id));
    }

    #[test]
    fn test_stop_time_tracking() {
        let mut model = make_model();
        let task = Task::new("Track me");
        let task_id = task.id;
        model.tasks.insert(task_id, task);

        model.start_time_tracking(task_id);
        assert!(model.active_time_entry.is_some());

        let result = model.stop_time_tracking();

        assert!(result.is_some());
        assert!(model.active_time_entry.is_none());
    }

    #[test]
    fn test_stop_time_tracking_when_none() {
        let mut model = make_model();
        let result = model.stop_time_tracking();
        assert!(result.is_none());
    }

    #[test]
    fn test_active_time_entry() {
        let mut model = make_model();
        assert!(model.active_time_entry().is_none());

        let task = Task::new("Task");
        let task_id = task.id;
        model.tasks.insert(task_id, task);
        model.start_time_tracking(task_id);

        assert!(model.active_time_entry().is_some());
        assert_eq!(model.active_time_entry().unwrap().task_id, task_id);
    }

    #[test]
    fn test_is_tracking_task() {
        let mut model = make_model();
        let task_a = Task::new("Task A");
        let task_b = Task::new("Task B");
        let a_id = task_a.id;
        let b_id = task_b.id;
        model.tasks.insert(a_id, task_a);
        model.tasks.insert(b_id, task_b);

        assert!(!model.is_tracking_task(&a_id));
        assert!(!model.is_tracking_task(&b_id));

        model.start_time_tracking(a_id);

        assert!(model.is_tracking_task(&a_id));
        assert!(!model.is_tracking_task(&b_id));
    }

    #[test]
    fn test_total_time_for_task_no_entries() {
        let model = make_model();
        let task = Task::new("Task");
        let task_id = task.id;
        assert_eq!(model.total_time_for_task(&task_id), 0);
    }

    #[test]
    fn test_time_entries_for_task() {
        let mut model = make_model();
        let task = Task::new("Task");
        let task_id = task.id;
        model.tasks.insert(task_id, task);

        model.start_time_tracking(task_id);
        model.stop_time_tracking();
        // Rebuild cache so time_entries_by_task is populated
        model.rebuild_caches();

        let entries = model.time_entries_for_task(&task_id);
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_delete_time_entry() {
        let mut model = make_model();
        let task = Task::new("Task");
        let task_id = task.id;
        model.tasks.insert(task_id, task);

        let (entry, _) = model.start_time_tracking(task_id);
        let entry_id = entry.id;

        assert!(model.time_entries.contains_key(&entry_id));
        assert!(model.active_time_entry.is_some());

        model.delete_time_entry(&entry_id);

        assert!(!model.time_entries.contains_key(&entry_id));
        assert!(model.active_time_entry.is_none()); // Active was cleared
    }

    #[test]
    fn test_restore_time_entry_running_with_task() {
        let mut model = make_model();
        let task = Task::new("Task");
        let task_id = task.id;
        model.tasks.insert(task_id, task);

        // Create a running entry
        let entry = TimeEntry::start(task_id);
        let entry_id = entry.id;

        model.restore_time_entry(entry);

        // Should become active since task exists and no current active entry
        assert_eq!(model.active_time_entry, Some(entry_id));
    }

    #[test]
    fn test_restore_time_entry_running_without_task() {
        let mut model = make_model();
        let task_id = TaskId::new(); // Not in model

        let entry = TimeEntry::start(task_id);
        model.restore_time_entry(entry);

        // Should NOT become active since task doesn't exist
        assert!(model.active_time_entry.is_none());
    }

    #[test]
    fn test_restore_time_entry_with_active_entry() {
        let mut model = make_model();
        let task = Task::new("Task");
        let task_id = task.id;
        model.tasks.insert(task_id, task);

        // Start one timer
        model.start_time_tracking(task_id);

        // Try to restore another running entry
        let task2 = Task::new("Task 2");
        let task2_id = task2.id;
        model.tasks.insert(task2_id, task2);
        let entry2 = TimeEntry::start(task2_id);
        let entry2_id = entry2.id;
        model.restore_time_entry(entry2);

        // entry2 should NOT become active (active entry already exists)
        assert_ne!(model.active_time_entry, Some(entry2_id));
    }

    #[test]
    fn restore_stopped_copy_of_active_entry_clears_active_timer() {
        let mut model = make_model();
        let task = Task::new("Track me");
        let task_id = task.id;
        model.tasks.insert(task_id, task);

        let (entry, _) = model.start_time_tracking(task_id);
        model.time_entries.get_mut(&entry.id).unwrap().started_at -= chrono::Duration::minutes(25);
        let (before, after) = model.stop_time_tracking().expect("timer was running");
        assert!(model.active_time_entry.is_none());

        // Undo the stop: the running copy comes back and is active again.
        model.restore_time_entry(before);
        assert_eq!(model.active_time_entry, Some(entry.id));
        assert!(model.is_tracking_task(&task_id));

        // Redo the stop: restoring the stopped copy must clear the active timer.
        model.restore_time_entry(after);
        assert_eq!(model.active_time_entry, None, "stopped entry is not active");
        assert!(!model.is_tracking_task(&task_id));
        assert_eq!(model.tasks[&task_id].actual_minutes, 25);
        assert_eq!(model.task_cache.time_sums.get(&task_id), Some(&25));
    }

    #[test]
    fn restore_stopped_entry_leaves_another_active_timer_alone() {
        let mut model = make_model();
        let a = Task::new("A");
        let b = Task::new("B");
        let (a_id, b_id) = (a.id, b.id);
        model.tasks.insert(a_id, a);
        model.tasks.insert(b_id, b);

        let mut finished = TimeEntry::start(a_id);
        finished.ended_at = Some(finished.started_at);
        finished.duration_minutes = Some(10);

        let (running_b, _) = model.start_time_tracking(b_id);
        model.restore_time_entry(finished);

        assert_eq!(model.active_time_entry, Some(running_b.id));
    }

    #[test]
    fn stop_recomputes_actual_minutes_and_cache() {
        let mut model = make_model();
        let task = Task::new("Track me");
        let task_id = task.id;
        model.tasks.insert(task_id, task);

        let (entry, _) = model.start_time_tracking(task_id);
        // Pretend the timer has been running for 40 minutes.
        model.time_entries.get_mut(&entry.id).unwrap().started_at -= chrono::Duration::minutes(40);
        model.stop_time_tracking();

        assert_eq!(model.tasks[&task_id].actual_minutes, 40);
        assert_eq!(model.task_cache.time_sums.get(&task_id), Some(&40));
        assert_eq!(model.time_entries_for_task(&task_id).len(), 1);
    }

    #[test]
    fn add_upsert_delete_keep_task_total_in_sync() {
        let mut model = make_model();
        let task = Task::new("Manual entries");
        let task_id = task.id;
        model.tasks.insert(task_id, task);

        let mut entry = TimeEntry::start(task_id);
        entry.started_at -= chrono::Duration::hours(2);
        entry.ended_at = Some(entry.started_at + chrono::Duration::minutes(30));
        entry.duration_minutes = Some(30);
        let entry_id = entry.id;
        model.add_time_entry(entry.clone());
        assert_eq!(model.tasks[&task_id].actual_minutes, 30);
        assert_eq!(model.task_cache.time_sums.get(&task_id), Some(&30));
        assert!(
            model.active_time_entry.is_none(),
            "a stopped entry never becomes active"
        );

        entry.duration_minutes = Some(45);
        model.upsert_time_entry(entry);
        assert_eq!(model.tasks[&task_id].actual_minutes, 45);

        model.delete_time_entry(&entry_id);
        assert_eq!(model.tasks[&task_id].actual_minutes, 0);
        assert!(model.time_entries_for_task(&task_id).is_empty());
    }

    #[test]
    fn upsert_moving_entry_between_tasks_recomputes_both() {
        let mut model = make_model();
        let a = Task::new("A");
        let b = Task::new("B");
        let (a_id, b_id) = (a.id, b.id);
        model.tasks.insert(a_id, a);
        model.tasks.insert(b_id, b);

        let mut entry = TimeEntry::start(a_id);
        entry.ended_at = Some(entry.started_at);
        entry.duration_minutes = Some(20);
        model.add_time_entry(entry.clone());
        assert_eq!(model.tasks[&a_id].actual_minutes, 20);

        entry.task_id = b_id;
        model.upsert_time_entry(entry);
        assert_eq!(model.tasks[&a_id].actual_minutes, 0);
        assert_eq!(model.tasks[&b_id].actual_minutes, 20);
    }
}
