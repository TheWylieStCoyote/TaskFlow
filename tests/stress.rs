//! Stress tests for TaskFlow with large task volumes.
//!
//! Tests verify performance of key operations at multiple scale levels:
//! - Level 1: 100 tasks (quick sanity check)
//! - Level 2: 1,000 tasks (standard stress test)
//! - Level 3: 10,000 tasks (heavy load)
//! - Level 4: 100,000 tasks (extreme load)
//! - Level 5: 1,000,000 tasks (maximum stress)
//!
//! Run specific levels with:
//! ```bash
//! cargo test --test stress level_1  # 100 tasks
//! cargo test --test stress level_2  # 1,000 tasks
//! cargo test --test stress level_3  # 10,000 tasks
//! cargo test --test stress level_4  # 100,000 tasks (slow)
//! cargo test --test stress level_5  # 1,000,000 tasks (very slow)
//! ```

use std::time::Instant;

use chrono::{Duration, Utc};
use taskflow::app::Model;
use taskflow::domain::{Priority, SortField, SortOrder, SortSpec, Task, TaskStatus};

// Task counts for each level
const LEVEL_1_COUNT: usize = 100;
const LEVEL_2_COUNT: usize = 1_000;
const LEVEL_3_COUNT: usize = 10_000;
const LEVEL_4_COUNT: usize = 100_000;
const LEVEL_5_COUNT: usize = 1_000_000;

// Time thresholds scale with task count
// Base thresholds (for 1,000 tasks)
const BASE_REFRESH_MS: u128 = 200;
const BASE_FILTER_MS: u128 = 100;
const BASE_SORT_MS: u128 = 100;
const BASE_SEARCH_MS: u128 = 150;

/// Calculate acceptable time threshold based on task count.
/// Uses O(n log n) scaling assumption for sort operations.
fn threshold_for_count(base_ms: u128, count: usize) -> u128 {
    let base_count = 1000_f64;
    let ratio = count as f64 / base_count;
    // Use n*log(n) scaling for realistic complexity
    let log_factor = if count > 1 {
        (count as f64).ln() / (base_count).ln()
    } else {
        1.0
    };
    (base_ms as f64 * ratio * log_factor).ceil() as u128
}

/// Create a model with n tasks having varied properties.
fn create_model_with_n_tasks(n: usize) -> Model {
    let mut model = Model::new();
    let today = Utc::now().date_naive();

    for i in 0..n {
        let priority = match i % 5 {
            0 => Priority::None,
            1 => Priority::Low,
            2 => Priority::Medium,
            3 => Priority::High,
            _ => Priority::Urgent,
        };

        let status = match i % 4 {
            0 => TaskStatus::Todo,
            1 => TaskStatus::InProgress,
            2 => TaskStatus::Done,
            _ => TaskStatus::Blocked,
        };

        let due_date = if i % 3 == 0 {
            Some(today + Duration::days((i as i64 % 30) - 15))
        } else {
            None
        };

        let mut task = Task::new(format!("Stress test task {i}"))
            .with_priority(priority)
            .with_status(status)
            .with_tags(vec![format!("tag{}", i % 10), format!("category{}", i % 5)]);

        if let Some(date) = due_date {
            task = task.with_due_date(date);
        }

        model.tasks.insert(task.id, task);
    }

    model.refresh_visible_tasks();
    model
}

// ============================================================================
// Level 1: 100 tasks (quick sanity check)
// ============================================================================

mod level_1 {
    use super::*;

    const COUNT: usize = LEVEL_1_COUNT;

    #[test]
    fn test_refresh_visible_tasks() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_REFRESH_MS, COUNT);
        println!("Level 1 refresh: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "refresh took {elapsed}ms, expected < {threshold}ms"
        );
        assert_eq!(model.visible_tasks.len(), COUNT);
    }

    #[test]
    fn test_filter_by_priority() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;
        model.filtering.filter.priority = Some(vec![Priority::Urgent, Priority::High]);

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_FILTER_MS, COUNT);
        println!("Level 1 priority filter: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "filtering took {elapsed}ms, expected < {threshold}ms"
        );
    }

    #[test]
    fn test_sort_by_due_date() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;
        model.filtering.sort = SortSpec {
            field: SortField::DueDate,
            order: SortOrder::Ascending,
        };

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_SORT_MS, COUNT);
        println!("Level 1 due date sort: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "sorting took {elapsed}ms, expected < {threshold}ms"
        );
    }

    #[test]
    fn test_search() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;
        model.filtering.filter.search_text = Some("task 5".to_string());

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_SEARCH_MS, COUNT);
        println!("Level 1 search: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "search took {elapsed}ms, expected < {threshold}ms"
        );
    }

    #[test]
    fn test_combined_operations() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = false;
        model.filtering.filter.priority = Some(vec![Priority::High, Priority::Urgent]);
        model.filtering.filter.search_text = Some("task".to_string());
        model.filtering.sort = SortSpec {
            field: SortField::DueDate,
            order: SortOrder::Ascending,
        };

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_REFRESH_MS, COUNT);
        println!("Level 1 combined: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "combined ops took {elapsed}ms, expected < {threshold}ms"
        );
    }
}

// ============================================================================
// Level 2: 1,000 tasks (standard stress test)
// ============================================================================

mod level_2 {
    use super::*;

    const COUNT: usize = LEVEL_2_COUNT;

    #[test]
    fn test_refresh_visible_tasks() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_REFRESH_MS, COUNT);
        println!("Level 2 refresh: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "refresh took {elapsed}ms, expected < {threshold}ms"
        );
        assert_eq!(model.visible_tasks.len(), COUNT);
    }

    #[test]
    fn test_filter_by_priority() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;
        model.filtering.filter.priority = Some(vec![Priority::Urgent, Priority::High]);

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_FILTER_MS, COUNT);
        println!("Level 2 priority filter: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "filtering took {elapsed}ms, expected < {threshold}ms"
        );

        // High + Urgent = 2/5 of tasks = 400
        let expected = COUNT * 2 / 5;
        assert!(
            model.visible_tasks.len() > expected - 50 && model.visible_tasks.len() < expected + 50,
            "Expected ~{expected} tasks, got {}",
            model.visible_tasks.len()
        );
    }

    #[test]
    fn test_sort_by_due_date() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;
        model.filtering.sort = SortSpec {
            field: SortField::DueDate,
            order: SortOrder::Ascending,
        };

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_SORT_MS, COUNT);
        println!("Level 2 due date sort: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "sorting took {elapsed}ms, expected < {threshold}ms"
        );
    }

    #[test]
    fn test_sort_by_priority() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;
        model.filtering.sort = SortSpec {
            field: SortField::Priority,
            order: SortOrder::Descending,
        };

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_SORT_MS, COUNT);
        println!("Level 2 priority sort: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "sorting took {elapsed}ms, expected < {threshold}ms"
        );
    }

    #[test]
    fn test_search() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;
        model.filtering.filter.search_text = Some("task 5".to_string());

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_SEARCH_MS, COUNT);
        println!("Level 2 search: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "search took {elapsed}ms, expected < {threshold}ms"
        );
        assert!(
            !model.visible_tasks.is_empty(),
            "Search should find some tasks"
        );
    }

    #[test]
    fn test_filter_by_tags() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;
        model.filtering.filter.tags = Some(vec!["tag5".to_string()]);

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_FILTER_MS, COUNT);
        println!("Level 2 tag filter: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "filtering took {elapsed}ms, expected < {threshold}ms"
        );

        // ~10% of tasks have tag5
        let expected = COUNT / 10;
        assert!(
            model.visible_tasks.len() >= expected - 10
                && model.visible_tasks.len() <= expected + 10,
            "Expected ~{expected} tasks with tag5, got {}",
            model.visible_tasks.len()
        );
    }

    #[test]
    fn test_combined_operations() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = false;
        model.filtering.filter.priority = Some(vec![Priority::High, Priority::Urgent]);
        model.filtering.filter.search_text = Some("task".to_string());
        model.filtering.sort = SortSpec {
            field: SortField::DueDate,
            order: SortOrder::Ascending,
        };

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_REFRESH_MS, COUNT);
        println!("Level 2 combined: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "combined ops took {elapsed}ms, expected < {threshold}ms"
        );
    }

    #[test]
    fn test_multiple_refresh_cycles() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;

        let start = Instant::now();
        for _ in 0..10 {
            model.refresh_visible_tasks();
        }
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_REFRESH_MS, COUNT) * 10;
        println!("Level 2 10x refresh: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "10 refresh cycles took {elapsed}ms, expected < {threshold}ms"
        );
    }
}

// ============================================================================
// Level 3: 10,000 tasks (heavy load)
// ============================================================================

mod level_3 {
    use super::*;

    const COUNT: usize = LEVEL_3_COUNT;

    #[test]
    fn test_refresh_visible_tasks() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_REFRESH_MS, COUNT);
        println!("Level 3 refresh: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "refresh took {elapsed}ms, expected < {threshold}ms"
        );
        assert_eq!(model.visible_tasks.len(), COUNT);
    }

    #[test]
    fn test_filter_by_priority() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;
        model.filtering.filter.priority = Some(vec![Priority::Urgent, Priority::High]);

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_FILTER_MS, COUNT);
        println!("Level 3 priority filter: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "filtering took {elapsed}ms, expected < {threshold}ms"
        );
    }

    #[test]
    fn test_sort_by_due_date() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;
        model.filtering.sort = SortSpec {
            field: SortField::DueDate,
            order: SortOrder::Ascending,
        };

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_SORT_MS, COUNT);
        println!("Level 3 due date sort: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "sorting took {elapsed}ms, expected < {threshold}ms"
        );
    }

    #[test]
    fn test_search() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;
        model.filtering.filter.search_text = Some("task 5".to_string());

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_SEARCH_MS, COUNT);
        println!("Level 3 search: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "search took {elapsed}ms, expected < {threshold}ms"
        );
    }

    #[test]
    fn test_combined_operations() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = false;
        model.filtering.filter.priority = Some(vec![Priority::High, Priority::Urgent]);
        model.filtering.filter.search_text = Some("task".to_string());
        model.filtering.sort = SortSpec {
            field: SortField::DueDate,
            order: SortOrder::Ascending,
        };

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_REFRESH_MS, COUNT);
        println!("Level 3 combined: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "combined ops took {elapsed}ms, expected < {threshold}ms"
        );
    }
}

// ============================================================================
// Level 4: 100,000 tasks (extreme load)
// ============================================================================

mod level_4 {
    use super::*;

    const COUNT: usize = LEVEL_4_COUNT;

    #[test]
    #[ignore = "slow test - run with: cargo test --test stress level_4 -- --ignored"]
    fn test_refresh_visible_tasks() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_REFRESH_MS, COUNT);
        println!("Level 4 refresh: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "refresh took {elapsed}ms, expected < {threshold}ms"
        );
        assert_eq!(model.visible_tasks.len(), COUNT);
    }

    #[test]
    #[ignore = "slow test - run with: cargo test --test stress level_4 -- --ignored"]
    fn test_filter_by_priority() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;
        model.filtering.filter.priority = Some(vec![Priority::Urgent, Priority::High]);

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_FILTER_MS, COUNT);
        println!("Level 4 priority filter: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "filtering took {elapsed}ms, expected < {threshold}ms"
        );
    }

    #[test]
    #[ignore = "slow test - run with: cargo test --test stress level_4 -- --ignored"]
    fn test_sort_by_due_date() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;
        model.filtering.sort = SortSpec {
            field: SortField::DueDate,
            order: SortOrder::Ascending,
        };

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_SORT_MS, COUNT);
        println!("Level 4 due date sort: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "sorting took {elapsed}ms, expected < {threshold}ms"
        );
    }

    #[test]
    #[ignore = "slow test - run with: cargo test --test stress level_4 -- --ignored"]
    fn test_search() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;
        model.filtering.filter.search_text = Some("task 5".to_string());

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_SEARCH_MS, COUNT);
        println!("Level 4 search: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "search took {elapsed}ms, expected < {threshold}ms"
        );
    }

    #[test]
    #[ignore = "slow test - run with: cargo test --test stress level_4 -- --ignored"]
    fn test_combined_operations() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = false;
        model.filtering.filter.priority = Some(vec![Priority::High, Priority::Urgent]);
        model.filtering.filter.search_text = Some("task".to_string());
        model.filtering.sort = SortSpec {
            field: SortField::DueDate,
            order: SortOrder::Ascending,
        };

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_REFRESH_MS, COUNT);
        println!("Level 4 combined: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "combined ops took {elapsed}ms, expected < {threshold}ms"
        );
    }
}

// ============================================================================
// Level 5: 1,000,000 tasks (maximum stress)
// ============================================================================

mod level_5 {
    use super::*;

    const COUNT: usize = LEVEL_5_COUNT;

    #[test]
    #[ignore = "very slow test - run with: cargo test --test stress level_5 -- --ignored"]
    fn test_refresh_visible_tasks() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_REFRESH_MS, COUNT);
        println!("Level 5 refresh: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "refresh took {elapsed}ms, expected < {threshold}ms"
        );
        assert_eq!(model.visible_tasks.len(), COUNT);
    }

    #[test]
    #[ignore = "very slow test - run with: cargo test --test stress level_5 -- --ignored"]
    fn test_filter_by_priority() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;
        model.filtering.filter.priority = Some(vec![Priority::Urgent, Priority::High]);

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_FILTER_MS, COUNT);
        println!("Level 5 priority filter: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "filtering took {elapsed}ms, expected < {threshold}ms"
        );
    }

    #[test]
    #[ignore = "very slow test - run with: cargo test --test stress level_5 -- --ignored"]
    fn test_sort_by_due_date() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;
        model.filtering.sort = SortSpec {
            field: SortField::DueDate,
            order: SortOrder::Ascending,
        };

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_SORT_MS, COUNT);
        println!("Level 5 due date sort: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "sorting took {elapsed}ms, expected < {threshold}ms"
        );
    }

    #[test]
    #[ignore = "very slow test - run with: cargo test --test stress level_5 -- --ignored"]
    fn test_search() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = true;
        model.filtering.filter.search_text = Some("task 5".to_string());

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_SEARCH_MS, COUNT);
        println!("Level 5 search: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "search took {elapsed}ms, expected < {threshold}ms"
        );
    }

    #[test]
    #[ignore = "very slow test - run with: cargo test --test stress level_5 -- --ignored"]
    fn test_combined_operations() {
        let mut model = create_model_with_n_tasks(COUNT);
        model.filtering.show_completed = false;
        model.filtering.filter.priority = Some(vec![Priority::High, Priority::Urgent]);
        model.filtering.filter.search_text = Some("task".to_string());
        model.filtering.sort = SortSpec {
            field: SortField::DueDate,
            order: SortOrder::Ascending,
        };

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        let threshold = threshold_for_count(BASE_REFRESH_MS, COUNT);
        println!("Level 5 combined: {elapsed}ms (threshold: {threshold}ms)");
        assert!(
            elapsed < threshold,
            "combined ops took {elapsed}ms, expected < {threshold}ms"
        );
    }
}
