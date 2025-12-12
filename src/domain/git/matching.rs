//! Branch-to-task matching for auto-detection.
//!
//! This module provides functionality to automatically match git branch names
//! to tasks based on naming conventions or title similarity.
//!
//! # Matching Strategies
//!
//! 1. **Task ID prefix**: Match branches like `feature/TASK-abc12345` to tasks
//!    with matching ID prefixes.
//!
//! 2. **Title similarity**: Match branches like `fix/login-authentication`
//!    to tasks with similar titles (word matching).
//!
//! # Examples
//!
//! ```
//! use taskflow::domain::git::matching::BranchMatcher;
//! use taskflow::domain::Task;
//!
//! let matcher = BranchMatcher::new();
//! let task = Task::new("Fix login authentication");
//!
//! // This branch would match the task due to title similarity
//! let tasks = vec![task.clone()];
//! let matched = matcher.match_branch_to_task("fix/login-auth", tasks.iter());
//! ```

use crate::domain::{Task, TaskId};

/// Matcher for linking branches to tasks.
///
/// Supports both exact ID matching and fuzzy title matching.
pub struct BranchMatcher {
    /// Minimum word match ratio for title matching (0.0 - 1.0)
    title_match_threshold: f64,
}

impl Default for BranchMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl BranchMatcher {
    /// Create a new branch matcher with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            title_match_threshold: 0.6, // 60% word match threshold
        }
    }

    /// Create a branch matcher with a custom title match threshold.
    #[must_use]
    pub const fn with_threshold(mut self, threshold: f64) -> Self {
        self.title_match_threshold = threshold;
        self
    }

    /// Try to match a branch name to a task.
    ///
    /// Returns the `TaskId` of the best matching task, or `None` if no match found.
    ///
    /// Matching priority:
    /// 1. Exact task ID prefix match (e.g., `feature/TASK-abc12345`)
    /// 2. Best title similarity match above threshold
    pub fn match_branch_to_task<'a>(
        &self,
        branch: &str,
        tasks: impl Iterator<Item = &'a Task>,
    ) -> Option<TaskId> {
        let branch_lower = branch.to_lowercase();
        let tasks: Vec<_> = tasks.collect();

        // Strategy 1: Try exact task ID prefix match
        // Look for patterns like TASK-abc12345 or task_abc12345
        if let Some(id) = Self::try_match_by_id(&branch_lower, &tasks) {
            return Some(id);
        }

        // Strategy 2: Try title similarity match
        self.try_match_by_title(&branch_lower, &tasks)
    }

    /// Try to match by task ID embedded in branch name.
    fn try_match_by_id(branch: &str, tasks: &[&Task]) -> Option<TaskId> {
        // Extract potential ID patterns from branch name
        // Patterns: TASK-<id>, task-<id>, task_<id>, or just the UUID prefix
        let patterns = ["task-", "task_", "task/"];

        for pattern in patterns {
            if let Some(pos) = branch.find(pattern) {
                let after_pattern = &branch[pos + pattern.len()..];
                // Take the next 8 characters (UUID prefix)
                let id_prefix: String = after_pattern
                    .chars()
                    .take_while(char::is_ascii_alphanumeric)
                    .take(8)
                    .collect();

                if id_prefix.len() >= 6 {
                    // Find task with matching ID prefix
                    for task in tasks {
                        if task.id.to_string().starts_with(&id_prefix) {
                            return Some(task.id);
                        }
                    }
                }
            }
        }

        None
    }

    /// Try to match by title similarity.
    fn try_match_by_title(&self, branch: &str, tasks: &[&Task]) -> Option<TaskId> {
        let branch_words = Self::extract_words(branch);

        if branch_words.is_empty() {
            return None;
        }

        let mut best_match: Option<(TaskId, f64)> = None;

        for task in tasks {
            let title_words = Self::extract_words(&task.title.to_lowercase());
            if title_words.is_empty() {
                continue;
            }

            let score = Self::calculate_word_similarity(&branch_words, &title_words);

            if score >= self.title_match_threshold
                && (best_match.is_none() || score > best_match.as_ref().unwrap().1)
            {
                best_match = Some((task.id, score));
            }
        }

        best_match.map(|(id, _)| id)
    }

    /// Extract meaningful words from a string.
    ///
    /// Strips common branch prefixes and splits on delimiters.
    fn extract_words(s: &str) -> Vec<String> {
        // Strip common branch prefixes
        let stripped = s
            .trim_start_matches("feature/")
            .trim_start_matches("fix/")
            .trim_start_matches("bugfix/")
            .trim_start_matches("hotfix/")
            .trim_start_matches("chore/")
            .trim_start_matches("docs/")
            .trim_start_matches("refactor/")
            .trim_start_matches("test/")
            .trim_start_matches("ci/");

        // Split on common delimiters and filter empty/short words
        stripped
            .split(|c: char| c == '-' || c == '_' || c == '/' || c.is_whitespace())
            .filter(|s| s.len() >= 2) // Ignore very short words
            .map(String::from)
            .collect()
    }

    /// Calculate word similarity between two word lists.
    ///
    /// Returns a score from 0.0 to 1.0 based on word overlap.
    fn calculate_word_similarity(words1: &[String], words2: &[String]) -> f64 {
        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }

        let mut matches: usize = 0;

        for w1 in words1 {
            for w2 in words2 {
                // Check for substring match (either direction)
                if w1.contains(w2.as_str()) || w2.contains(w1.as_str()) {
                    matches += 1;
                    break;
                }
            }
        }

        // Score based on how many of the smaller set matched
        let min_len = words1.len().min(words2.len());
        #[allow(clippy::cast_precision_loss)]
        let score = matches as f64 / min_len as f64;
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_words_simple() {
        let words = BranchMatcher::extract_words("fix-login-bug");
        assert_eq!(words, vec!["fix", "login", "bug"]);
    }

    #[test]
    fn test_extract_words_with_prefix() {
        let words = BranchMatcher::extract_words("feature/add-user-auth");
        assert_eq!(words, vec!["add", "user", "auth"]);
    }

    #[test]
    fn test_extract_words_underscore() {
        let words = BranchMatcher::extract_words("fix_login_authentication");
        assert_eq!(words, vec!["fix", "login", "authentication"]);
    }

    #[test]
    fn test_word_similarity_exact() {
        let words1 = vec!["login".to_string(), "fix".to_string()];
        let words2 = vec!["fix".to_string(), "login".to_string(), "bug".to_string()];
        let score = BranchMatcher::calculate_word_similarity(&words1, &words2);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_word_similarity_partial() {
        let words1 = vec!["login".to_string(), "auth".to_string()];
        let words2 = vec![
            "fix".to_string(),
            "authentication".to_string(),
            "bug".to_string(),
        ];
        let score = BranchMatcher::calculate_word_similarity(&words1, &words2);
        // "auth" should match "authentication"
        assert!(score >= 0.5);
    }

    #[test]
    fn test_word_similarity_none() {
        let words1 = vec!["apple".to_string(), "banana".to_string()];
        let words2 = vec!["car".to_string(), "plane".to_string()];
        let score = BranchMatcher::calculate_word_similarity(&words1, &words2);
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_match_by_title() {
        let matcher = BranchMatcher::new();
        let task = Task::new("Fix login authentication bug");
        let tasks = vec![&task];

        let result = matcher.try_match_by_title("fix/login-auth", &tasks);
        assert_eq!(result, Some(task.id));
    }

    #[test]
    fn test_match_by_title_no_match() {
        let matcher = BranchMatcher::new();
        let task = Task::new("Update documentation");
        let tasks = vec![&task];

        let result = matcher.try_match_by_title("feature/new-api-endpoint", &tasks);
        assert!(result.is_none());
    }

    #[test]
    fn test_match_by_id() {
        let task = Task::new("Some task");
        let task_id_prefix = &task.id.to_string()[..8];
        let tasks = vec![&task];

        let branch = format!("feature/task-{task_id_prefix}");
        let result = BranchMatcher::try_match_by_id(&branch.to_lowercase(), &tasks);
        assert_eq!(result, Some(task.id));
    }

    #[test]
    fn test_match_branch_to_task_prefers_id() {
        let matcher = BranchMatcher::new();

        // Create a task that would match by title
        let task1 = Task::new("Fix login bug");

        // Create another task that matches by ID
        let task2 = Task::new("Different task");
        let task2_id_prefix = &task2.id.to_string()[..8];

        let tasks = [task1.clone(), task2.clone()];

        // Branch matches task1 by title but task2 by ID - ID should win
        let branch = format!("fix/login-task-{task2_id_prefix}");
        let result = matcher.match_branch_to_task(&branch, tasks.iter());
        assert_eq!(result, Some(task2.id));
    }

    #[test]
    fn test_match_branch_complete_flow() {
        let matcher = BranchMatcher::new();

        let task1 = Task::new("Implement user registration");
        let task2 = Task::new("Fix payment processing");
        let task3 = Task::new("Update documentation");

        let tasks = [task1.clone(), task2.clone(), task3.clone()];

        // Should match task1
        let result = matcher.match_branch_to_task("feature/user-registration", tasks.iter());
        assert_eq!(result, Some(task1.id));

        // Should match task2
        let result = matcher.match_branch_to_task("fix/payment-processing-bug", tasks.iter());
        assert_eq!(result, Some(task2.id));

        // Should not match (low similarity)
        let result = matcher.match_branch_to_task("chore/cleanup-old-files", tasks.iter());
        assert!(result.is_none());
    }
}
