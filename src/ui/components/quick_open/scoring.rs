//! Fuzzy scoring for the quick-open task picker.

use strsim::jaro_winkler;

use crate::domain::{Task, TaskId};

/// A scored task entry for quick-open display.
#[derive(Debug, Clone)]
pub struct QuickOpenEntry {
    /// Task ID for navigation on confirm
    pub task_id: TaskId,
    /// Task title for display
    pub title: String,
    /// Project name for right-aligned annotation (if any)
    pub project_name: Option<String>,
    /// Score in [0.0, 1.0] — higher is a better match
    pub score: f64,
}

/// Minimum score threshold below which tasks are excluded from results.
pub const QUICK_OPEN_THRESHOLD: f64 = 0.5;

/// Score a single task against a query string.
///
/// Returns a score in `[0.0, 1.0]`. An empty query always returns `1.0`
/// so that all tasks appear when the picker first opens.
#[must_use]
pub fn score_task(query: &str, task: &Task, project_name: Option<&str>) -> f64 {
    if query.is_empty() {
        return 1.0;
    }
    let query_lower = query.to_lowercase();
    let title_lower = task.title.to_lowercase();

    // Exact substring match wins outright; prefix match scores highest.
    if title_lower.contains(&query_lower) {
        if title_lower.starts_with(&query_lower) {
            return 1.0;
        }
        return 0.95;
    }

    // Jaro-Winkler fuzzy fallback for typo tolerance.
    let jw = jaro_winkler(&query_lower, &title_lower);

    // Also consider the project name — useful when the user types the project.
    let proj_score = project_name.map_or(0.0, |p| {
        let p_lower = p.to_lowercase();
        if p_lower.contains(&query_lower) {
            0.7
        } else {
            jaro_winkler(&query_lower, &p_lower) * 0.5
        }
    });

    jw.max(proj_score)
}

/// Filter and rank all tasks against a query.
///
/// Returns entries sorted by score descending, then title ascending.
/// Tasks scoring below `threshold` are excluded.
///
/// When the query is empty pass `0.0` as the threshold so all tasks appear.
pub fn filter_tasks<'a>(
    query: &str,
    tasks: impl Iterator<Item = &'a Task>,
    project_name_fn: impl Fn(&Task) -> Option<String>,
    threshold: f64,
) -> Vec<QuickOpenEntry> {
    let mut entries: Vec<QuickOpenEntry> = tasks
        .filter_map(|task| {
            let proj = project_name_fn(task);
            let score = score_task(query, task, proj.as_deref());
            if score >= threshold {
                Some(QuickOpenEntry {
                    task_id: task.id,
                    title: task.title.clone(),
                    project_name: proj,
                    score,
                })
            } else {
                None
            }
        })
        .collect();

    entries.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.title.cmp(&b.title))
    });
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Task;

    fn make_task(title: &str) -> Task {
        Task::new(title)
    }

    #[test]
    fn test_empty_query_scores_one() {
        let task = make_task("Buy groceries");
        assert!((score_task("", &task, None) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_exact_prefix_scores_one() {
        let task = make_task("Buy groceries");
        assert!((score_task("buy", &task, None) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_mid_substring_scores_high() {
        let task = make_task("Buy groceries");
        let score = score_task("grocer", &task, None);
        assert!(score >= 0.9, "score was {score}");
    }

    #[test]
    fn test_no_match_scores_low() {
        let task = make_task("Buy groceries");
        let score = score_task("zzzzz", &task, None);
        assert!(score < QUICK_OPEN_THRESHOLD, "score was {score}");
    }

    #[test]
    fn test_filter_tasks_empty_query_returns_all() {
        let tasks = [make_task("Alpha"), make_task("Beta"), make_task("Gamma")];
        let results = filter_tasks("", tasks.iter(), |_| None, 0.0);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_filter_tasks_sorted_by_score_then_title() {
        let tasks = [make_task("Beta"), make_task("Alpha"), make_task("Gamma")];
        let results = filter_tasks("", tasks.iter(), |_| None, 0.0);
        // With empty query all scores are 1.0, so secondary sort by title applies.
        assert_eq!(results[0].title, "Alpha");
        assert_eq!(results[1].title, "Beta");
        assert_eq!(results[2].title, "Gamma");
    }
}
