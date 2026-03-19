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
///
/// A minimum of 50ms is enforced so that small task counts (e.g. 100 tasks)
/// don't produce unrealistically tight thresholds (e.g. 14ms) that flake on
/// loaded CI runners.
fn threshold_for_count(base_ms: u128, count: usize) -> u128 {
    const MIN_THRESHOLD_MS: u128 = 50;
    let base_count = 1000_f64;
    let ratio = count as f64 / base_count;
    // Use n*log(n) scaling for realistic complexity
    let log_factor = if count > 1 {
        (count as f64).ln() / (base_count).ln()
    } else {
        1.0
    };
    let computed = (base_ms as f64 * ratio * log_factor).ceil() as u128;
    computed.max(MIN_THRESHOLD_MS)
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

// ============================================================================
// Deep Hierarchy Tests
// ============================================================================
//
// Tests for deeply nested task hierarchies (parent-child relationships)

mod deep_hierarchy {
    use super::*;
    use taskflow::domain::TaskId;

    /// Create a deep hierarchy with specified depth and width.
    /// Returns (model, root_task_id).
    fn create_deep_hierarchy(depth: usize, width: usize) -> (Model, TaskId) {
        let mut model = Model::new();

        // Create root task
        let root = Task::new("Root task");
        let root_id = root.id;
        model.tasks.insert(root.id, root);

        // Create hierarchy level by level
        let mut current_level_ids = vec![root_id];

        for level in 1..=depth {
            let mut next_level_ids = Vec::new();

            for parent_id in &current_level_ids {
                for child_num in 0..width {
                    let mut child = Task::new(format!("Task L{level} C{child_num}"));
                    child.parent_task_id = Some(*parent_id);
                    let child_id = child.id;
                    model.tasks.insert(child.id, child);
                    next_level_ids.push(child_id);
                }
            }

            current_level_ids = next_level_ids;
        }

        model.refresh_visible_tasks();
        (model, root_id)
    }

    /// Create a linear chain (depth N, width 1) - worst case for tree traversal.
    fn create_linear_chain(length: usize) -> (Model, TaskId) {
        let mut model = Model::new();

        let root = Task::new("Chain root");
        let root_id = root.id;
        model.tasks.insert(root.id, root);

        let mut parent_id = root_id;
        for i in 1..length {
            let mut child = Task::new(format!("Chain task {i}"));
            child.parent_task_id = Some(parent_id);
            let child_id = child.id;
            model.tasks.insert(child.id, child);
            parent_id = child_id;
        }

        model.refresh_visible_tasks();
        (model, root_id)
    }

    #[test]
    fn test_moderate_hierarchy_depth_5_width_3() {
        // 5 levels deep, 3 children per node
        // Total tasks: 1 + 3 + 9 + 27 + 81 + 243 = 364
        let (mut model, _root_id) = create_deep_hierarchy(5, 3);

        let expected_total = 1 + 3 + 9 + 27 + 81 + 243;
        assert_eq!(
            model.tasks.len(),
            expected_total,
            "Expected {expected_total} tasks"
        );

        // Measure refresh performance
        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        println!(
            "Depth 5, Width 3 hierarchy ({} tasks): {elapsed}ms",
            model.tasks.len()
        );
        assert!(
            elapsed < 100,
            "Hierarchy refresh took {elapsed}ms, expected < 100ms"
        );
    }

    #[test]
    fn test_deep_hierarchy_depth_10_width_2() {
        // 10 levels deep, 2 children per node
        // Total tasks: 2^11 - 1 = 2047
        let (mut model, _root_id) = create_deep_hierarchy(10, 2);

        let expected_total = (1 << 11) - 1; // 2^11 - 1
        assert_eq!(
            model.tasks.len(),
            expected_total,
            "Expected {expected_total} tasks"
        );

        // Measure refresh performance
        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        println!(
            "Depth 10, Width 2 hierarchy ({} tasks): {elapsed}ms",
            model.tasks.len()
        );
        assert!(
            elapsed < 200,
            "Deep hierarchy refresh took {elapsed}ms, expected < 200ms"
        );
    }

    #[test]
    fn test_linear_chain_100() {
        // Linear chain: 100 tasks deep (worst case for tree traversal)
        let (mut model, _root_id) = create_linear_chain(100);

        assert_eq!(model.tasks.len(), 100);

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        println!("Linear chain (100 tasks): {elapsed}ms");
        assert!(
            elapsed < 50,
            "Linear chain refresh took {elapsed}ms, expected < 50ms"
        );
    }

    #[test]
    fn test_linear_chain_500() {
        // Linear chain: 500 tasks deep
        let (mut model, _root_id) = create_linear_chain(500);

        assert_eq!(model.tasks.len(), 500);

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        println!("Linear chain (500 tasks): {elapsed}ms");
        // Allow generous threshold for debug builds (release is much faster)
        assert!(
            elapsed < 500,
            "Linear chain refresh took {elapsed}ms, expected < 500ms"
        );
    }

    #[test]
    #[ignore = "slow test - run with: cargo test --test stress deep_hierarchy -- --ignored"]
    fn test_linear_chain_1000() {
        // Linear chain: 1000 tasks deep
        let (mut model, _root_id) = create_linear_chain(1000);

        assert_eq!(model.tasks.len(), 1000);

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        println!("Linear chain (1000 tasks): {elapsed}ms");
        // Allow generous threshold for debug builds (release is much faster)
        assert!(
            elapsed < 1000,
            "Linear chain refresh took {elapsed}ms, expected < 1000ms"
        );
    }

    #[test]
    fn test_wide_hierarchy_depth_2_width_100() {
        // Very wide: 2 levels, 100 children each
        // Total: 1 + 100 + 10000 = 10101
        let (mut model, _root_id) = create_deep_hierarchy(2, 100);

        let expected_total = 1 + 100 + 10000;
        assert_eq!(
            model.tasks.len(),
            expected_total,
            "Expected {expected_total} tasks"
        );

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        println!(
            "Wide hierarchy (depth 2, width 100, {} tasks): {elapsed}ms",
            model.tasks.len()
        );
        assert!(
            elapsed < 500,
            "Wide hierarchy refresh took {elapsed}ms, expected < 500ms"
        );
    }

    #[test]
    fn test_hierarchy_with_filtering() {
        let (mut model, _root_id) = create_deep_hierarchy(5, 3);

        // Set some tasks as completed
        let task_ids: Vec<_> = model.tasks.keys().copied().collect();
        for (i, id) in task_ids.iter().enumerate() {
            if i % 3 == 0 {
                if let Some(task) = model.tasks.get_mut(id) {
                    task.status = TaskStatus::Done;
                }
            }
        }

        // Filter out completed
        model.filtering.show_completed = false;

        let start = Instant::now();
        model.refresh_visible_tasks();
        let elapsed = start.elapsed().as_millis();

        println!(
            "Hierarchy with filter ({} total, {} visible): {elapsed}ms",
            model.tasks.len(),
            model.visible_tasks.len()
        );
        assert!(
            elapsed < 100,
            "Filtered hierarchy refresh took {elapsed}ms, expected < 100ms"
        );

        // About 2/3 of tasks should be visible
        let expected_visible = model.tasks.len() * 2 / 3;
        assert!(
            model.visible_tasks.len() > expected_visible - 50
                && model.visible_tasks.len() < expected_visible + 50,
            "Expected ~{expected_visible} visible, got {}",
            model.visible_tasks.len()
        );
    }
}

// ============================================================================
// Storage Backend Stress Tests
// ============================================================================
//
// Tests for storage operations with large datasets

mod storage_stress {
    use super::*;
    use taskflow::storage::{create_backend, BackendType};
    use tempfile::tempdir;

    #[test]
    fn test_json_backend_1000_tasks() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create 1000 tasks
        let start = Instant::now();
        for i in 0..1000 {
            let task = Task::new(format!("Task {i}"));
            backend.create_task(&task).unwrap();
        }
        let create_elapsed = start.elapsed().as_millis();

        // List all tasks
        let start = Instant::now();
        let tasks = backend.list_tasks().unwrap();
        let list_elapsed = start.elapsed().as_millis();

        println!("JSON backend 1000 tasks - create: {create_elapsed}ms, list: {list_elapsed}ms");
        assert_eq!(tasks.len(), 1000);
        assert!(
            create_elapsed < 2000,
            "Create took {create_elapsed}ms, expected < 2000ms"
        );
        assert!(
            list_elapsed < 500,
            "List took {list_elapsed}ms, expected < 500ms"
        );
    }

    #[test]
    fn test_sqlite_backend_1000_tasks() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let mut backend = create_backend(BackendType::Sqlite, &path).unwrap();

        // Create 1000 tasks
        let start = Instant::now();
        for i in 0..1000 {
            let task = Task::new(format!("Task {i}"));
            backend.create_task(&task).unwrap();
        }
        let create_elapsed = start.elapsed().as_millis();

        // List all tasks
        let start = Instant::now();
        let tasks = backend.list_tasks().unwrap();
        let list_elapsed = start.elapsed().as_millis();

        println!("SQLite backend 1000 tasks - create: {create_elapsed}ms, list: {list_elapsed}ms");
        assert_eq!(tasks.len(), 1000);
        assert!(
            create_elapsed < 5000,
            "Create took {create_elapsed}ms, expected < 5000ms"
        );
        assert!(
            list_elapsed < 500,
            "List took {list_elapsed}ms, expected < 500ms"
        );
    }

    #[test]
    #[ignore = "slow test - run with: cargo test --test stress storage_stress -- --ignored"]
    fn test_json_backend_10000_tasks() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create 10000 tasks
        let start = Instant::now();
        for i in 0..10_000 {
            let task = Task::new(format!("Task {i}"));
            backend.create_task(&task).unwrap();
        }
        let create_elapsed = start.elapsed().as_millis();

        // List all tasks
        let start = Instant::now();
        let tasks = backend.list_tasks().unwrap();
        let list_elapsed = start.elapsed().as_millis();

        println!("JSON backend 10000 tasks - create: {create_elapsed}ms, list: {list_elapsed}ms");
        assert_eq!(tasks.len(), 10_000);
    }

    #[test]
    #[ignore = "slow test - run with: cargo test --test stress storage_stress -- --ignored"]
    fn test_sqlite_backend_10000_tasks() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let mut backend = create_backend(BackendType::Sqlite, &path).unwrap();

        // Create 10000 tasks
        let start = Instant::now();
        for i in 0..10_000 {
            let task = Task::new(format!("Task {i}"));
            backend.create_task(&task).unwrap();
        }
        let create_elapsed = start.elapsed().as_millis();

        // List all tasks
        let start = Instant::now();
        let tasks = backend.list_tasks().unwrap();
        let list_elapsed = start.elapsed().as_millis();

        println!("SQLite backend 10000 tasks - create: {create_elapsed}ms, list: {list_elapsed}ms");
        assert_eq!(tasks.len(), 10_000);
    }

    #[test]
    fn test_backend_filtered_query_performance() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create 1000 tasks with varied properties
        for i in 0..1000 {
            let priority = match i % 5 {
                0 => Priority::None,
                1 => Priority::Low,
                2 => Priority::Medium,
                3 => Priority::High,
                _ => Priority::Urgent,
            };
            let mut task = Task::new(format!("Task {i}"));
            task.priority = priority;
            task.tags = vec![format!("tag{}", i % 10)];
            backend.create_task(&task).unwrap();
        }

        // Filter by priority
        let filter = taskflow::domain::Filter {
            priority: Some(vec![Priority::High, Priority::Urgent]),
            include_completed: true,
            ..Default::default()
        };

        let start = Instant::now();
        let filtered = backend.list_tasks_filtered(&filter).unwrap();
        let elapsed = start.elapsed().as_millis();

        println!(
            "Filtered query (1000 tasks): {elapsed}ms, found {} tasks",
            filtered.len()
        );
        assert!(
            elapsed < 200,
            "Filtered query took {elapsed}ms, expected < 200ms"
        );
        // High + Urgent = 2/5 of tasks = 400
        assert!(
            filtered.len() > 350 && filtered.len() < 450,
            "Expected ~400 tasks, got {}",
            filtered.len()
        );
    }
}
