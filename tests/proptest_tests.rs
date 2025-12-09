//! Property-based tests using proptest.
//!
//! These tests verify invariants that should hold for all inputs,
//! using randomized test data to find edge cases.

use proptest::prelude::*;
use taskflow::domain::{Priority, Project, Task, TaskId, TaskStatus};
use taskflow::storage::{create_backend, BackendType};
use tempfile::tempdir;

// ============================================================================
// Strategies for generating test data
// ============================================================================

/// Strategy for generating valid task titles
fn task_title_strategy() -> impl Strategy<Value = String> {
    // Non-empty strings up to 200 chars
    "[a-zA-Z0-9 .,!?()-]{1,200}".prop_map(|s| s.trim().to_string())
}

/// Strategy for generating optional descriptions
fn description_strategy() -> impl Strategy<Value = Option<String>> {
    prop_oneof![Just(None), "[a-zA-Z0-9 .,!?\\n()-]{0,1000}".prop_map(Some),]
}

/// Strategy for generating task tags
fn tags_strategy() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec("[a-z0-9-]{1,20}", 0..10)
}

/// Strategy for generating priorities
fn priority_strategy() -> impl Strategy<Value = Priority> {
    prop_oneof![
        Just(Priority::None),
        Just(Priority::Low),
        Just(Priority::Medium),
        Just(Priority::High),
        Just(Priority::Urgent),
    ]
}

/// Strategy for generating task statuses
fn status_strategy() -> impl Strategy<Value = TaskStatus> {
    prop_oneof![
        Just(TaskStatus::Todo),
        Just(TaskStatus::InProgress),
        Just(TaskStatus::Blocked),
        Just(TaskStatus::Done),
        Just(TaskStatus::Cancelled),
    ]
}

/// Strategy for generating estimated minutes
fn estimated_minutes_strategy() -> impl Strategy<Value = Option<u32>> {
    prop_oneof![Just(None), (1u32..1000).prop_map(Some),]
}

/// Strategy for generating a complete Task
fn task_strategy() -> impl Strategy<Value = Task> {
    (
        task_title_strategy(),
        description_strategy(),
        priority_strategy(),
        status_strategy(),
        tags_strategy(),
        estimated_minutes_strategy(),
    )
        .prop_map(|(title, desc, priority, status, tags, estimated)| {
            let mut task = Task::new(&title);
            task.description = desc;
            task.priority = priority;
            task.status = status;
            task.tags = tags;
            task.estimated_minutes = estimated;
            task
        })
}

/// Strategy for generating project names
fn project_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 -]{1,100}".prop_map(|s| s.trim().to_string())
}

// ============================================================================
// Property tests for Task
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Task IDs are always unique
    #[test]
    fn prop_task_ids_are_unique(count in 1usize..100) {
        let mut ids: Vec<TaskId> = Vec::new();
        for _ in 0..count {
            ids.push(TaskId::new());
        }

        // All IDs should be unique
        let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len();
        prop_assert_eq!(unique_count, count);
    }

    /// Property: Task creation preserves all fields
    #[test]
    fn prop_task_preserves_fields(
        title in task_title_strategy(),
        desc in description_strategy(),
        priority in priority_strategy(),
        tags in tags_strategy(),
    ) {
        let mut task = Task::new(&title);
        task.description = desc.clone();
        task.priority = priority;
        task.tags = tags.clone();

        prop_assert_eq!(&task.title, &title);
        prop_assert_eq!(&task.description, &desc);
        prop_assert_eq!(task.priority, priority);
        prop_assert_eq!(&task.tags, &tags);
    }

    /// Property: Priority parsing roundtrips correctly
    #[test]
    fn prop_priority_roundtrip(priority in priority_strategy()) {
        let as_str = priority.as_str();
        let parsed = Priority::parse(as_str);
        prop_assert_eq!(parsed, Some(priority));
    }

    /// Property: TaskStatus as_str returns valid strings
    #[test]
    fn prop_status_as_str_valid(status in status_strategy()) {
        let as_str = status.as_str();
        // as_str should return a non-empty string
        prop_assert!(!as_str.is_empty());
        // All strings should be lowercase
        prop_assert_eq!(as_str, as_str.to_lowercase());
    }

    /// Property: is_complete matches Done and Cancelled statuses
    #[test]
    fn prop_is_complete_consistency(status in status_strategy()) {
        let is_complete = status.is_complete();
        let expected = matches!(status, TaskStatus::Done | TaskStatus::Cancelled);
        prop_assert_eq!(is_complete, expected);
    }
}

// ============================================================================
// Property tests for Storage Backend
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: JSON backend CRUD operations are consistent
    #[test]
    fn prop_json_backend_crud_consistent(task in task_strategy()) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create
        backend.create_task(&task).unwrap();

        // Read
        let retrieved = backend.get_task(&task.id).unwrap().unwrap();

        // Core fields should match
        prop_assert_eq!(&retrieved.title, &task.title);
        prop_assert_eq!(retrieved.priority, task.priority);
        prop_assert_eq!(retrieved.status, task.status);
        prop_assert_eq!(&retrieved.tags, &task.tags);
        prop_assert_eq!(retrieved.estimated_minutes, task.estimated_minutes);

        // Delete
        backend.delete_task(&task.id).unwrap();
        prop_assert!(backend.get_task(&task.id).unwrap().is_none());
    }

    /// Property: SQLite backend CRUD operations are consistent
    #[test]
    fn prop_sqlite_backend_crud_consistent(task in task_strategy()) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let mut backend = create_backend(BackendType::Sqlite, &path).unwrap();

        // Create
        backend.create_task(&task).unwrap();

        // Read
        let retrieved = backend.get_task(&task.id).unwrap().unwrap();

        // Core fields should match
        prop_assert_eq!(&retrieved.title, &task.title);
        prop_assert_eq!(retrieved.priority, task.priority);
        prop_assert_eq!(retrieved.status, task.status);
        prop_assert_eq!(&retrieved.tags, &task.tags);
        prop_assert_eq!(retrieved.estimated_minutes, task.estimated_minutes);

        // Delete
        backend.delete_task(&task.id).unwrap();
        prop_assert!(backend.get_task(&task.id).unwrap().is_none());
    }

    /// Property: Update preserves non-modified fields
    #[test]
    fn prop_update_preserves_unmodified_fields(
        task in task_strategy(),
        new_title in task_title_strategy(),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create
        backend.create_task(&task).unwrap();

        // Update only title
        let mut updated = task.clone();
        updated.title = new_title.clone();
        backend.update_task(&updated).unwrap();

        // Retrieve and check
        let retrieved = backend.get_task(&task.id).unwrap().unwrap();
        prop_assert_eq!(&retrieved.title, &new_title);
        // Other fields should be unchanged
        prop_assert_eq!(retrieved.priority, task.priority);
        prop_assert_eq!(&retrieved.tags, &task.tags);
    }

    /// Property: List tasks returns all created tasks
    #[test]
    fn prop_list_returns_all_tasks(tasks in prop::collection::vec(task_strategy(), 1..20)) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create all tasks
        for task in &tasks {
            backend.create_task(task).unwrap();
        }

        // List should return same count
        let listed = backend.list_tasks().unwrap();
        prop_assert_eq!(listed.len(), tasks.len());

        // All IDs should be present
        let listed_ids: std::collections::HashSet<_> = listed.iter().map(|t| t.id).collect();
        for task in &tasks {
            prop_assert!(listed_ids.contains(&task.id));
        }
    }
}

// ============================================================================
// Property tests for filtering
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    /// Property: Filtering by status returns correct subset
    #[test]
    fn prop_filter_by_status_correct(
        tasks in prop::collection::vec(task_strategy(), 1..20),
        filter_status in status_strategy(),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create all tasks
        for task in &tasks {
            backend.create_task(task).unwrap();
        }

        // Filter by status
        let filter = taskflow::domain::Filter {
            status: Some(vec![filter_status]),
            include_completed: true,
            ..Default::default()
        };
        let filtered = backend.list_tasks_filtered(&filter).unwrap();

        // Count expected
        let expected_count = tasks.iter().filter(|t| t.status == filter_status).count();
        prop_assert_eq!(filtered.len(), expected_count);

        // All returned tasks should have the filter status
        for task in &filtered {
            prop_assert_eq!(task.status, filter_status);
        }
    }

    /// Property: Filtering by priority returns correct subset
    #[test]
    fn prop_filter_by_priority_correct(
        tasks in prop::collection::vec(task_strategy(), 1..20),
        filter_priority in priority_strategy(),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create all tasks
        for task in &tasks {
            backend.create_task(task).unwrap();
        }

        // Filter by priority
        let filter = taskflow::domain::Filter {
            priority: Some(vec![filter_priority]),
            include_completed: true,
            ..Default::default()
        };
        let filtered = backend.list_tasks_filtered(&filter).unwrap();

        // Count expected
        let expected_count = tasks.iter().filter(|t| t.priority == filter_priority).count();
        prop_assert_eq!(filtered.len(), expected_count);

        // All returned tasks should have the filter priority
        for task in &filtered {
            prop_assert_eq!(task.priority, filter_priority);
        }
    }
}

// ============================================================================
// Property tests for projects
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: Project creation preserves name
    #[test]
    fn prop_project_preserves_name(name in project_name_strategy()) {
        let project = Project::new(&name);
        prop_assert_eq!(&project.name, &name);
    }

    /// Property: Project backend CRUD is consistent
    #[test]
    fn prop_project_crud_consistent(name in project_name_strategy()) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        let project = Project::new(&name);

        // Create
        backend.create_project(&project).unwrap();

        // Read
        let retrieved = backend.get_project(&project.id).unwrap().unwrap();
        prop_assert_eq!(&retrieved.name, &name);

        // Delete
        backend.delete_project(&project.id).unwrap();
        prop_assert!(backend.get_project(&project.id).unwrap().is_none());
    }
}

// ============================================================================
// Property tests for task-project relationships
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    /// Property: Tasks with project ID can be retrieved by project
    #[test]
    fn prop_tasks_by_project_correct(
        project_name in project_name_strategy(),
        tasks_in_project in prop::collection::vec(task_strategy(), 1..10),
        tasks_without_project in prop::collection::vec(task_strategy(), 0..10),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create project
        let project = Project::new(&project_name);
        backend.create_project(&project).unwrap();

        // Create tasks in project
        for mut task in tasks_in_project.clone() {
            task.project_id = Some(project.id);
            backend.create_task(&task).unwrap();
        }

        // Create tasks without project
        for task in &tasks_without_project {
            backend.create_task(task).unwrap();
        }

        // Get tasks by project
        let project_tasks = backend.get_tasks_by_project(&project.id).unwrap();

        prop_assert_eq!(project_tasks.len(), tasks_in_project.len());
        for task in &project_tasks {
            prop_assert_eq!(task.project_id, Some(project.id));
        }
    }
}

// ============================================================================
// Property tests for tag operations
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    /// Property: Tasks can be retrieved by tag
    #[test]
    fn prop_tasks_by_tag_correct(
        tag_to_find in "[a-z]{3,10}",
        tasks_with_tag in prop::collection::vec(task_strategy(), 1..10),
        tasks_without_tag in prop::collection::vec(task_strategy(), 0..10),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create tasks with the target tag
        for mut task in tasks_with_tag.clone() {
            if !task.tags.contains(&tag_to_find) {
                task.tags.push(tag_to_find.clone());
            }
            backend.create_task(&task).unwrap();
        }

        // Create tasks without the target tag
        for mut task in tasks_without_tag.clone() {
            task.tags.retain(|t| t != &tag_to_find);
            backend.create_task(&task).unwrap();
        }

        // Get tasks by tag
        let tagged_tasks = backend.get_tasks_by_tag(&tag_to_find).unwrap();

        prop_assert_eq!(tagged_tasks.len(), tasks_with_tag.len());
        for task in &tagged_tasks {
            prop_assert!(task.tags.contains(&tag_to_find));
        }
    }
}

// ============================================================================
// Property tests for combined filter operations
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    /// Property: Combining status and priority filters returns intersection
    #[test]
    fn prop_combined_status_priority_filter(
        tasks in prop::collection::vec(task_strategy(), 1..20),
        filter_status in status_strategy(),
        filter_priority in priority_strategy(),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create all tasks
        for task in &tasks {
            backend.create_task(task).unwrap();
        }

        // Filter by both status and priority
        let filter = taskflow::domain::Filter {
            status: Some(vec![filter_status]),
            priority: Some(vec![filter_priority]),
            include_completed: true,
            ..Default::default()
        };
        let filtered = backend.list_tasks_filtered(&filter).unwrap();

        // Count expected (intersection)
        let expected_count = tasks
            .iter()
            .filter(|t| t.status == filter_status && t.priority == filter_priority)
            .count();
        prop_assert_eq!(filtered.len(), expected_count);

        // All returned tasks should match both criteria
        for task in &filtered {
            prop_assert_eq!(task.status, filter_status);
            prop_assert_eq!(task.priority, filter_priority);
        }
    }

    /// Property: Tag filter combined with status filter returns intersection
    #[test]
    fn prop_combined_tag_status_filter(
        tag_to_filter in "[a-z]{3,10}",
        filter_status in status_strategy(),
        base_tasks in prop::collection::vec(task_strategy(), 1..15),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create tasks with varied tags and statuses
        let mut tasks_with_tag_and_status = 0;
        for mut task in base_tasks {
            // Randomly add the target tag to some tasks
            if task.title.len() % 2 == 0 && !task.tags.contains(&tag_to_filter) {
                task.tags.push(tag_to_filter.clone());
            }
            if task.tags.contains(&tag_to_filter) && task.status == filter_status {
                tasks_with_tag_and_status += 1;
            }
            backend.create_task(&task).unwrap();
        }

        // Filter by tag and status
        let filter = taskflow::domain::Filter {
            tags: Some(vec![tag_to_filter.clone()]),
            status: Some(vec![filter_status]),
            include_completed: true,
            ..Default::default()
        };
        let filtered = backend.list_tasks_filtered(&filter).unwrap();

        prop_assert_eq!(filtered.len(), tasks_with_tag_and_status);

        // All returned tasks should match both criteria
        for task in &filtered {
            prop_assert!(task.tags.contains(&tag_to_filter));
            prop_assert_eq!(task.status, filter_status);
        }
    }

    /// Property: Empty filter returns all tasks (respecting include_completed)
    #[test]
    fn prop_empty_filter_returns_all(
        tasks in prop::collection::vec(task_strategy(), 1..20),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        for task in &tasks {
            backend.create_task(task).unwrap();
        }

        // Empty filter with include_completed = true should return all
        let filter = taskflow::domain::Filter {
            include_completed: true,
            ..Default::default()
        };
        let filtered = backend.list_tasks_filtered(&filter).unwrap();

        prop_assert_eq!(filtered.len(), tasks.len());
    }

    /// Property: Filter excluding completed removes done/cancelled tasks
    #[test]
    fn prop_exclude_completed_filter(
        tasks in prop::collection::vec(task_strategy(), 1..20),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        for task in &tasks {
            backend.create_task(task).unwrap();
        }

        // Filter excluding completed
        let filter = taskflow::domain::Filter {
            include_completed: false,
            ..Default::default()
        };
        let filtered = backend.list_tasks_filtered(&filter).unwrap();

        // Count incomplete tasks
        let incomplete_count = tasks.iter().filter(|t| !t.status.is_complete()).count();
        prop_assert_eq!(filtered.len(), incomplete_count);

        // All returned tasks should be incomplete
        for task in &filtered {
            prop_assert!(!task.status.is_complete());
        }
    }
}

// ============================================================================
// Property tests for recurrence patterns
// ============================================================================

use taskflow::domain::Recurrence;

/// Strategy for generating recurrence patterns
fn recurrence_strategy() -> impl Strategy<Value = Recurrence> {
    prop_oneof![
        Just(Recurrence::Daily),
        prop::collection::vec(weekday_strategy(), 1..7).prop_map(|days| Recurrence::Weekly {
            days: days
                .into_iter()
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect()
        }),
        (1u32..29).prop_map(|day| Recurrence::Monthly { day }),
        (1u32..13, 1u32..29).prop_map(|(month, day)| Recurrence::Yearly { month, day }),
    ]
}

/// Strategy for generating weekdays
fn weekday_strategy() -> impl Strategy<Value = chrono::Weekday> {
    prop_oneof![
        Just(chrono::Weekday::Mon),
        Just(chrono::Weekday::Tue),
        Just(chrono::Weekday::Wed),
        Just(chrono::Weekday::Thu),
        Just(chrono::Weekday::Fri),
        Just(chrono::Weekday::Sat),
        Just(chrono::Weekday::Sun),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: Recurrence Display is non-empty and consistent
    #[test]
    fn prop_recurrence_display_non_empty(recurrence in recurrence_strategy()) {
        let display = format!("{}", recurrence);
        prop_assert!(!display.is_empty());
        // Display should contain recognizable pattern name
        prop_assert!(
            display.contains("Daily")
                || display.contains("Weekly")
                || display.contains("Monthly")
                || display.contains("Yearly")
        );
    }

    /// Property: Monthly recurrence day is always valid (1-31)
    #[test]
    fn prop_monthly_recurrence_valid_day(day in 1u32..32) {
        let recurrence = Recurrence::Monthly { day };
        let display = format!("{}", recurrence);
        prop_assert!(display.contains(&day.to_string()));
    }

    /// Property: Yearly recurrence serialization roundtrip
    #[test]
    fn prop_recurrence_serde_roundtrip(recurrence in recurrence_strategy()) {
        let json = serde_json::to_string(&recurrence).unwrap();
        let parsed: Recurrence = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(recurrence, parsed);
    }

    /// Property: Task with recurrence preserves pattern through storage
    #[test]
    fn prop_task_recurrence_storage_roundtrip(
        task in task_strategy(),
        recurrence in recurrence_strategy(),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        let mut task_with_recurrence = task;
        task_with_recurrence.recurrence = Some(recurrence.clone());

        backend.create_task(&task_with_recurrence).unwrap();
        let retrieved = backend.get_task(&task_with_recurrence.id).unwrap().unwrap();

        prop_assert_eq!(retrieved.recurrence, Some(recurrence));
    }
}

// ============================================================================
// Property tests for task chains (dependencies)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    /// Property: Task chain links are preserved through storage
    #[test]
    fn prop_task_chain_storage_roundtrip(
        task1 in task_strategy(),
        task2 in task_strategy(),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create task2 first (the next task)
        backend.create_task(&task2).unwrap();

        // Create task1 with next_task_id pointing to task2
        let mut linked_task = task1;
        linked_task.next_task_id = Some(task2.id);
        backend.create_task(&linked_task).unwrap();

        // Retrieve and verify link is preserved
        let retrieved = backend.get_task(&linked_task.id).unwrap().unwrap();
        prop_assert_eq!(retrieved.next_task_id, Some(task2.id));
    }

    /// Property: Task dependencies are preserved through storage
    #[test]
    fn prop_task_dependencies_storage_roundtrip(
        tasks in prop::collection::vec(task_strategy(), 2..5),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create all tasks first
        for task in &tasks {
            backend.create_task(task).unwrap();
        }

        // Make first task depend on all others
        let mut dependent_task = tasks[0].clone();
        dependent_task.dependencies = tasks[1..].iter().map(|t| t.id).collect();
        backend.update_task(&dependent_task).unwrap();

        // Retrieve and verify dependencies
        let retrieved = backend.get_task(&dependent_task.id).unwrap().unwrap();
        prop_assert_eq!(retrieved.dependencies.len(), tasks.len() - 1);
        for dep_id in &dependent_task.dependencies {
            prop_assert!(retrieved.dependencies.contains(dep_id));
        }
    }
}
