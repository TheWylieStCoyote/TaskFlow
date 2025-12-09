//! Duplicate detection and merge strategies for import operations.

use std::collections::HashMap;

use chrono::NaiveDate;

use crate::domain::{Task, TaskId};

use super::types::{ImportResult, ImportSkipReason, MergeStrategy};

/// Duplicate detector for import operations
pub struct DuplicateDetector<'a> {
    /// Existing tasks by ID
    existing_by_id: &'a HashMap<TaskId, Task>,
    /// Index of (lowercase title, due_date) -> TaskId for duplicate detection
    title_date_index: HashMap<(String, Option<NaiveDate>), TaskId>,
}

impl<'a> DuplicateDetector<'a> {
    /// Create a new duplicate detector from existing tasks
    #[must_use]
    pub fn new(existing_tasks: &'a HashMap<TaskId, Task>) -> Self {
        let mut title_date_index = HashMap::new();
        for task in existing_tasks.values() {
            let key = (task.title.to_lowercase(), task.due_date);
            title_date_index.insert(key, task.id);
        }
        Self {
            existing_by_id: existing_tasks,
            title_date_index,
        }
    }

    /// Check if a task is a duplicate
    #[must_use]
    pub fn check(&self, task: &Task) -> Option<ImportSkipReason> {
        // Check by ID first
        if self.existing_by_id.contains_key(&task.id) {
            return Some(ImportSkipReason::DuplicateId(task.id));
        }

        // Check by title + due date
        let key = (task.title.to_lowercase(), task.due_date);
        if self.title_date_index.contains_key(&key) {
            return Some(ImportSkipReason::DuplicateTitleDate {
                title: task.title.clone(),
                due_date: task.due_date,
            });
        }

        None
    }
}

/// Apply duplicate detection and merge strategy to import results
pub fn apply_merge_strategy(
    result: &mut ImportResult,
    existing_tasks: &HashMap<TaskId, Task>,
    strategy: MergeStrategy,
) {
    if strategy == MergeStrategy::CreateNew {
        // Generate new IDs for all imported tasks
        for task in &mut result.imported {
            task.id = TaskId::new();
        }
        return;
    }

    let detector = DuplicateDetector::new(existing_tasks);
    let mut new_imported = Vec::new();

    // At this point, strategy is either Skip or Overwrite (CreateNew returned early)
    for task in result.imported.drain(..) {
        if let Some(reason) = detector.check(&task) {
            if strategy == MergeStrategy::Skip {
                result.skipped.push((task, reason));
            } else {
                // Overwrite - task will replace existing
                new_imported.push(task);
            }
        } else {
            new_imported.push(task);
        }
    }

    result.imported = new_imported;
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_duplicate_detector_by_id() {
        let mut existing = HashMap::new();
        let task = Task::new("Existing task");
        let task_id = task.id;
        existing.insert(task.id, task);

        let detector = DuplicateDetector::new(&existing);

        let mut import_task = Task::new("New task");
        import_task.id = task_id;

        let result = detector.check(&import_task);
        assert!(matches!(result, Some(ImportSkipReason::DuplicateId(_))));
    }

    #[test]
    fn test_duplicate_detector_by_title_date() {
        let mut existing = HashMap::new();
        let mut task = Task::new("Existing task");
        task.due_date = Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap());
        existing.insert(task.id, task);

        let detector = DuplicateDetector::new(&existing);

        let mut import_task = Task::new("EXISTING TASK"); // Different case
        import_task.due_date = Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap());

        let result = detector.check(&import_task);
        assert!(matches!(
            result,
            Some(ImportSkipReason::DuplicateTitleDate { .. })
        ));
    }

    #[test]
    fn test_merge_strategy_skip() {
        let mut existing = HashMap::new();
        let task = Task::new("Existing task");
        existing.insert(task.id, task.clone());

        let mut result = ImportResult::default();
        let mut import_task = Task::new("Different task");
        import_task.id = task.id; // Same ID
        result.imported.push(import_task);

        apply_merge_strategy(&mut result, &existing, MergeStrategy::Skip);

        assert!(result.imported.is_empty());
        assert_eq!(result.skipped.len(), 1);
    }

    #[test]
    fn test_merge_strategy_create_new() {
        let existing = HashMap::new();

        let mut result = ImportResult::default();
        let task = Task::new("New task");
        let original_id = task.id;
        result.imported.push(task);

        apply_merge_strategy(&mut result, &existing, MergeStrategy::CreateNew);

        assert_eq!(result.imported.len(), 1);
        assert_ne!(result.imported[0].id, original_id); // ID should be different
    }
}
