//! Fuzzy duplicate detection for tasks.
//!
//! This module provides functions for detecting potential duplicate tasks
//! using string similarity algorithms. Tasks are compared within the same
//! project scope using the Jaro-Winkler distance metric.

use std::cmp::Ordering;
use std::collections::HashMap;

use strsim::jaro_winkler;

use crate::domain::{ProjectId, Task, TaskId};

/// Default similarity threshold for duplicate detection (85%).
pub const DEFAULT_SIMILARITY_THRESHOLD: f64 = 0.85;

/// A pair of tasks that are potential duplicates.
#[derive(Debug, Clone)]
pub struct DuplicatePair {
    /// ID of the first task.
    pub task1_id: TaskId,
    /// ID of the second task.
    pub task2_id: TaskId,
    /// Similarity score between 0.0 and 1.0.
    pub similarity: f64,
}

/// Find the most similar task to a given title within the same project.
///
/// Returns the most similar task and its similarity score if one exceeds
/// the threshold. Useful for warning on task creation.
///
/// # Arguments
///
/// * `title` - The title to search for similar tasks
/// * `project_id` - The project to scope the search (None for inbox)
/// * `tasks` - All tasks to search through
/// * `threshold` - Minimum similarity score (0.0 to 1.0)
/// * `exclude_id` - Optional task ID to exclude from results (for editing)
#[must_use]
pub fn find_similar_task<'a>(
    title: &str,
    project_id: Option<ProjectId>,
    tasks: &'a HashMap<TaskId, Task>,
    threshold: f64,
    exclude_id: Option<TaskId>,
) -> Option<(&'a Task, f64)> {
    let title_lower = title.to_lowercase();

    tasks
        .values()
        .filter(|t| t.project_id == project_id)
        .filter(|t| exclude_id.is_none_or(|id| t.id != id))
        .filter_map(|t| {
            let similarity = jaro_winkler(&title_lower, &t.title.to_lowercase());
            if similarity >= threshold {
                Some((t, similarity))
            } else {
                None
            }
        })
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
}

/// Find all duplicate pairs across all tasks.
///
/// Returns a list of task pairs that exceed the similarity threshold,
/// sorted by similarity score (highest first). Only compares tasks
/// within the same project.
///
/// # Arguments
///
/// * `tasks` - All tasks to check for duplicates
/// * `threshold` - Minimum similarity score (0.0 to 1.0)
///
/// # Performance
///
/// This function pre-groups tasks by project and caches lowercase titles
/// to avoid repeated allocations. Complexity is O(Σ(p²)) where p is the
/// number of tasks per project, rather than O(n²) for all tasks.
#[must_use]
pub fn find_all_duplicates(tasks: &HashMap<TaskId, Task>, threshold: f64) -> Vec<DuplicatePair> {
    let mut duplicates = Vec::new();

    // Pre-compute lowercase titles once per task
    let tasks_with_lower: Vec<_> = tasks
        .values()
        .map(|t| (t, t.title.to_lowercase()))
        .collect();

    // Group by project_id for efficient comparison (avoids wasted cross-project comparisons)
    let mut by_project: HashMap<Option<ProjectId>, Vec<(&Task, &str)>> = HashMap::new();
    for (task, title_lower) in &tasks_with_lower {
        by_project
            .entry(task.project_id)
            .or_default()
            .push((*task, title_lower.as_str()));
    }

    // Only compare within same project
    for project_tasks in by_project.values() {
        for (i, (task1, title1)) in project_tasks.iter().enumerate() {
            for (task2, title2) in project_tasks.iter().skip(i + 1) {
                let similarity = jaro_winkler(title1, title2);

                if similarity >= threshold {
                    duplicates.push(DuplicatePair {
                        task1_id: task1.id,
                        task2_id: task2.id,
                        similarity,
                    });
                }
            }
        }
    }

    // Sort by similarity descending
    duplicates.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(Ordering::Equal)
    });
    duplicates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Task;

    fn create_task(title: &str, project_id: Option<ProjectId>) -> Task {
        let mut task = Task::new(title);
        task.project_id = project_id;
        task
    }

    #[test]
    fn test_find_similar_task_exact_match() {
        let mut tasks = HashMap::new();
        let task = create_task("Buy groceries", None);
        tasks.insert(task.id, task);

        let result = find_similar_task("Buy groceries", None, &tasks, 0.85, None);
        assert!(result.is_some());
        let (_, similarity) = result.unwrap();
        assert!((similarity - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_find_similar_task_fuzzy_match() {
        let mut tasks = HashMap::new();
        let task = create_task("Buy groceries from store", None);
        tasks.insert(task.id, task);

        let result = find_similar_task("Buy groceries at store", None, &tasks, 0.85, None);
        assert!(result.is_some());
        let (_, similarity) = result.unwrap();
        assert!(similarity > 0.85);
    }

    #[test]
    fn test_find_similar_task_case_insensitive() {
        let mut tasks = HashMap::new();
        let task = create_task("Buy GROCERIES", None);
        tasks.insert(task.id, task);

        let result = find_similar_task("buy groceries", None, &tasks, 0.85, None);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_similar_task_respects_project_scope() {
        let mut tasks = HashMap::new();
        let project_id = ProjectId::new();

        let task1 = create_task("Buy groceries", Some(project_id));
        let task2 = create_task("Buy groceries", None);
        tasks.insert(task1.id, task1);
        tasks.insert(task2.id, task2);

        // Should only find the task in the same project
        let result = find_similar_task("Buy groceries", Some(project_id), &tasks, 0.85, None);
        assert!(result.is_some());
        let (found, _) = result.unwrap();
        assert_eq!(found.project_id, Some(project_id));

        // Should only find the task in inbox
        let result = find_similar_task("Buy groceries", None, &tasks, 0.85, None);
        assert!(result.is_some());
        let (found, _) = result.unwrap();
        assert_eq!(found.project_id, None);
    }

    #[test]
    fn test_find_similar_task_no_match_below_threshold() {
        let mut tasks = HashMap::new();
        let task = create_task("Buy groceries", None);
        tasks.insert(task.id, task);

        let result = find_similar_task("Write documentation", None, &tasks, 0.85, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_similar_task_excludes_id() {
        let mut tasks = HashMap::new();
        let task = create_task("Buy groceries", None);
        let task_id = task.id;
        tasks.insert(task.id, task);

        // Should not find itself when excluded
        let result = find_similar_task("Buy groceries", None, &tasks, 0.85, Some(task_id));
        assert!(result.is_none());
    }

    #[test]
    fn test_find_all_duplicates_empty() {
        let tasks = HashMap::new();
        let duplicates = find_all_duplicates(&tasks, 0.85);
        assert!(duplicates.is_empty());
    }

    #[test]
    fn test_find_all_duplicates_finds_pairs() {
        let mut tasks = HashMap::new();
        let task1 = create_task("Buy groceries", None);
        let task2 = create_task("Buy groceries from store", None);
        let task1_id = task1.id;
        let task2_id = task2.id;
        tasks.insert(task1.id, task1);
        tasks.insert(task2.id, task2);

        let duplicates = find_all_duplicates(&tasks, 0.8);
        assert_eq!(duplicates.len(), 1);
        assert!(
            (duplicates[0].task1_id == task1_id && duplicates[0].task2_id == task2_id)
                || (duplicates[0].task1_id == task2_id && duplicates[0].task2_id == task1_id)
        );
    }

    #[test]
    fn test_find_all_duplicates_sorted_by_similarity() {
        let mut tasks = HashMap::new();
        let task1 = create_task("Buy groceries", None);
        let task2 = create_task("Buy groceries now", None);
        let task3 = create_task("Buy groceries from the store today", None);
        tasks.insert(task1.id, task1);
        tasks.insert(task2.id, task2);
        tasks.insert(task3.id, task3);

        let duplicates = find_all_duplicates(&tasks, 0.7);
        assert!(!duplicates.is_empty());

        // Verify sorted by similarity descending
        for i in 1..duplicates.len() {
            assert!(duplicates[i - 1].similarity >= duplicates[i].similarity);
        }
    }

    #[test]
    fn test_find_all_duplicates_respects_project_scope() {
        let mut tasks = HashMap::new();
        let project1 = ProjectId::new();
        let project2 = ProjectId::new();

        let task1 = create_task("Buy groceries", Some(project1));
        let task2 = create_task("Buy groceries", Some(project2));
        tasks.insert(task1.id, task1);
        tasks.insert(task2.id, task2);

        // Should not find duplicates across different projects
        let duplicates = find_all_duplicates(&tasks, 0.85);
        assert!(duplicates.is_empty());
    }

    #[test]
    fn test_default_threshold() {
        assert!((DEFAULT_SIMILARITY_THRESHOLD - 0.85).abs() < f64::EPSILON);
    }

    // === Edge case tests for Phase 5 improvements ===

    #[test]
    fn test_unicode_emoji_titles() {
        let mut tasks = HashMap::new();
        let task1 = create_task("🚀 Launch feature", None);
        let task2 = create_task("🚀 Launch feature update", None);
        let task1_id = task1.id;
        let task2_id = task2.id;
        tasks.insert(task1.id, task1);
        tasks.insert(task2.id, task2);

        let result = find_all_duplicates(&tasks, 0.8);
        assert_eq!(result.len(), 1);
        assert!(
            (result[0].task1_id == task1_id && result[0].task2_id == task2_id)
                || (result[0].task1_id == task2_id && result[0].task2_id == task1_id)
        );
    }

    #[test]
    fn test_very_long_titles() {
        let mut tasks = HashMap::new();
        let long_title1 = "A".repeat(1000);
        let long_title2 = format!("{}B", "A".repeat(999));
        let task1 = create_task(&long_title1, None);
        let task2 = create_task(&long_title2, None);
        tasks.insert(task1.id, task1);
        tasks.insert(task2.id, task2);

        // Should not panic and should find similarity
        let result = find_all_duplicates(&tasks, 0.9);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_special_characters_in_titles() {
        let mut tasks = HashMap::new();
        let task1 = create_task("Fix bug #123 in @module", None);
        let task2 = create_task("Fix bug #124 in @module", None);
        tasks.insert(task1.id, task1);
        tasks.insert(task2.id, task2);

        let result = find_all_duplicates(&tasks, 0.9);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_single_task_no_duplicates() {
        let mut tasks = HashMap::new();
        let task = create_task("Only task", None);
        tasks.insert(task.id, task);

        let result = find_all_duplicates(&tasks, 0.85);
        assert!(result.is_empty());
    }

    #[test]
    fn test_cross_project_isolation() {
        // Ensure tasks in different projects are never compared
        let mut tasks = HashMap::new();
        let project1 = Some(ProjectId::new());
        let project2 = Some(ProjectId::new());

        // Add identical titles in different projects
        let task1 = create_task("Exact same title", project1);
        let task2 = create_task("Exact same title", project2);
        tasks.insert(task1.id, task1);
        tasks.insert(task2.id, task2);

        // Should find no duplicates since they're in different projects
        let result = find_all_duplicates(&tasks, 0.85);
        assert!(result.is_empty());
    }
}
