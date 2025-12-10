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

    /// Property: List tasks returns all created tasks (including empty case)
    #[test]
    fn prop_list_returns_all_tasks(tasks in prop::collection::vec(task_strategy(), 0..20)) {
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

    /// Property: Filtering by status returns correct subset (including empty case)
    #[test]
    fn prop_filter_by_status_correct(
        tasks in prop::collection::vec(task_strategy(), 0..20),
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
        tasks in prop::collection::vec(task_strategy(), 0..20),
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
        tasks in prop::collection::vec(task_strategy(), 0..20),
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

/// Strategy for generating recurrence patterns.
/// Uses full day range (1-31) to test boundary cases like month-end dates.
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
        (1u32..32).prop_map(|day| Recurrence::Monthly { day }),
        (1u32..13, 1u32..32).prop_map(|(month, day)| Recurrence::Yearly { month, day }),
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

// ============================================================================
// Property tests for YAML backend
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    /// Property: YAML backend CRUD operations are consistent
    #[test]
    fn prop_yaml_backend_crud_consistent(task in task_strategy()) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.yaml");
        let mut backend = create_backend(BackendType::Yaml, &path).unwrap();

        // Create
        backend.create_task(&task).unwrap();

        // Read
        let retrieved = backend.get_task(&task.id).unwrap().unwrap();

        // Core fields should match
        prop_assert_eq!(&retrieved.title, &task.title);
        prop_assert_eq!(retrieved.priority, task.priority);
        prop_assert_eq!(retrieved.status, task.status);
        prop_assert_eq!(&retrieved.tags, &task.tags);

        // Delete
        backend.delete_task(&task.id).unwrap();
        prop_assert!(backend.get_task(&task.id).unwrap().is_none());
    }

    /// Property: YAML backend handles special characters in titles
    #[test]
    fn prop_yaml_special_chars(title in "[a-zA-Z0-9 :'\"-]{1,100}") {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.yaml");
        let mut backend = create_backend(BackendType::Yaml, &path).unwrap();

        let task = Task::new(&title);
        backend.create_task(&task).unwrap();

        let retrieved = backend.get_task(&task.id).unwrap().unwrap();
        prop_assert_eq!(&retrieved.title, &title);
    }
}

// ============================================================================
// Property tests for Markdown backend
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// Property: Markdown backend CRUD operations are consistent
    #[test]
    fn prop_markdown_backend_crud_consistent(task in task_strategy()) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks");
        let mut backend = create_backend(BackendType::Markdown, &path).unwrap();

        // Create
        backend.create_task(&task).unwrap();

        // Read
        let retrieved = backend.get_task(&task.id).unwrap().unwrap();

        // Core fields should match
        prop_assert_eq!(&retrieved.title, &task.title);
        prop_assert_eq!(retrieved.priority, task.priority);
        prop_assert_eq!(retrieved.status, task.status);

        // Delete
        backend.delete_task(&task.id).unwrap();
        prop_assert!(backend.get_task(&task.id).unwrap().is_none());
    }
}

// ============================================================================
// Property tests for search text filtering
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    /// Property: Search filter returns tasks containing search text
    #[test]
    fn prop_search_text_filter(
        search_term in "[a-z]{3,8}",
        tasks_with_term in prop::collection::vec(task_strategy(), 1..10),
        tasks_without_term in prop::collection::vec(task_strategy(), 0..10),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create tasks with search term in title
        for mut task in tasks_with_term.clone() {
            task.title = format!("{} {}", task.title, search_term);
            backend.create_task(&task).unwrap();
        }

        // Create tasks without search term
        for mut task in tasks_without_term.clone() {
            task.title = task.title.replace(&search_term, "");
            if !task.title.to_lowercase().contains(&search_term.to_lowercase()) {
                backend.create_task(&task).unwrap();
            }
        }

        // Filter by search text
        let filter = taskflow::domain::Filter {
            search_text: Some(search_term.clone()),
            include_completed: true,
            ..Default::default()
        };
        let filtered = backend.list_tasks_filtered(&filter).unwrap();

        // All returned tasks should contain search term
        for task in &filtered {
            prop_assert!(
                task.title.to_lowercase().contains(&search_term.to_lowercase())
                    || task.description.as_ref().is_some_and(|d| d.to_lowercase().contains(&search_term.to_lowercase()))
            );
        }
    }

    /// Property: Search is case-insensitive
    #[test]
    fn prop_search_case_insensitive(
        search_term in "[a-z]{3,8}",
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create task with uppercase version of search term
        let task = Task::new(search_term.to_uppercase());
        backend.create_task(&task).unwrap();

        // Search with lowercase should find it
        let filter = taskflow::domain::Filter {
            search_text: Some(search_term.to_lowercase()),
            include_completed: true,
            ..Default::default()
        };
        let filtered = backend.list_tasks_filtered(&filter).unwrap();

        prop_assert_eq!(filtered.len(), 1);
    }
}

// ============================================================================
// Property tests for due date handling
// ============================================================================

use chrono::NaiveDate;

/// Strategy for generating dates within a reasonable range.
/// Generates all possible days (1-31) and clamps invalid dates to the last valid day
/// of the month, ensuring we test boundary cases like month-end and leap years.
fn date_strategy() -> impl Strategy<Value = NaiveDate> {
    (2020i32..2030, 1u32..13, 1u32..32).prop_map(|(year, month, day)| {
        // Try the exact date first, then fall back to the last valid day of the month
        NaiveDate::from_ymd_opt(year, month, day).unwrap_or_else(|| {
            // Find the last valid day of this month by trying backwards
            (1..=31)
                .rev()
                .find_map(|d| NaiveDate::from_ymd_opt(year, month, d))
                .expect("every month has at least one valid day")
        })
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    /// Property: Due date is preserved through storage
    #[test]
    fn prop_due_date_storage_roundtrip(
        task in task_strategy(),
        due_date in date_strategy(),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        let mut task_with_due = task;
        task_with_due.due_date = Some(due_date);
        backend.create_task(&task_with_due).unwrap();

        let retrieved = backend.get_task(&task_with_due.id).unwrap().unwrap();
        prop_assert_eq!(retrieved.due_date, Some(due_date));
    }

    /// Property: Scheduled date is preserved through storage
    #[test]
    fn prop_scheduled_date_storage_roundtrip(
        task in task_strategy(),
        scheduled_date in date_strategy(),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        let mut task_with_scheduled = task;
        task_with_scheduled.scheduled_date = Some(scheduled_date);
        backend.create_task(&task_with_scheduled).unwrap();

        let retrieved = backend.get_task(&task_with_scheduled.id).unwrap().unwrap();
        prop_assert_eq!(retrieved.scheduled_date, Some(scheduled_date));
    }

    /// Property: Filter by due_before only returns tasks with due dates on or before cutoff
    #[test]
    fn prop_filter_due_before(
        tasks in prop::collection::vec(task_strategy(), 1..10),
        cutoff_date in date_strategy(),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Assign due dates to tasks
        for (i, mut task) in tasks.into_iter().enumerate() {
            // Set due dates: some before, some after cutoff
            let days_offset = if i % 2 == 0 { -10i64 } else { 10i64 };
            if let Some(due) = cutoff_date.checked_add_signed(chrono::Duration::days(days_offset)) {
                task.due_date = Some(due);
                backend.create_task(&task).unwrap();
            }
        }

        let filter = taskflow::domain::Filter {
            due_before: Some(cutoff_date),
            include_completed: true,
            ..Default::default()
        };
        let filtered = backend.list_tasks_filtered(&filter).unwrap();

        // All returned tasks should have due dates on or before cutoff
        for task in &filtered {
            if let Some(due) = task.due_date {
                prop_assert!(due <= cutoff_date, "Task due date {} should be <= cutoff {}", due, cutoff_date);
            }
        }
    }
}

// ============================================================================
// Property tests for time entries
// ============================================================================

use taskflow::domain::TimeEntry;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    /// Property: Time entry can be created and has correct task_id
    #[test]
    fn prop_time_entry_creation(task in task_strategy()) {
        let entry = TimeEntry::start(task.id);
        prop_assert_eq!(entry.task_id, task.id);
        prop_assert!(entry.ended_at.is_none());
    }

    /// Property: Time entries are preserved through storage
    #[test]
    fn prop_time_entry_storage_roundtrip(task in task_strategy()) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create task
        backend.create_task(&task).unwrap();

        // Create time entry for task
        let mut entry = TimeEntry::start(task.id);
        entry.stop(); // End it so it has a duration
        backend.create_time_entry(&entry).unwrap();

        // Retrieve time entries
        let entries = backend.get_entries_for_task(&task.id).unwrap();
        prop_assert_eq!(entries.len(), 1);
        prop_assert_eq!(entries[0].task_id, task.id);
    }
}

// ============================================================================
// Property tests for subtask relationships
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// Property: Subtask parent_task_id is preserved through storage
    #[test]
    fn prop_subtask_parent_storage_roundtrip(
        parent_task in task_strategy(),
        child_task in task_strategy(),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create parent
        backend.create_task(&parent_task).unwrap();

        // Create child with parent_task_id
        let mut child = child_task;
        child.parent_task_id = Some(parent_task.id);
        backend.create_task(&child).unwrap();

        // Verify parent_task_id is preserved
        let retrieved = backend.get_task(&child.id).unwrap().unwrap();
        prop_assert_eq!(retrieved.parent_task_id, Some(parent_task.id));
    }

    /// Property: Multiple subtasks can share same parent
    #[test]
    fn prop_multiple_subtasks_same_parent(
        parent_task in task_strategy(),
        child_tasks in prop::collection::vec(task_strategy(), 2..5),
    ) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let mut backend = create_backend(BackendType::Json, &path).unwrap();

        // Create parent
        backend.create_task(&parent_task).unwrap();

        // Create children
        for mut child in child_tasks.clone() {
            child.parent_task_id = Some(parent_task.id);
            backend.create_task(&child).unwrap();
        }

        // Verify all children have correct parent
        for child in &child_tasks {
            let retrieved = backend.get_task(&child.id).unwrap().unwrap();
            prop_assert_eq!(retrieved.parent_task_id, Some(parent_task.id));
        }
    }
}

// ============================================================================
// Property tests for Model operations
// ============================================================================

use taskflow::app::{update, Message, Model, NavigationMessage, SystemMessage, TaskMessage};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    /// Property: Creating tasks maintains model consistency
    #[test]
    fn prop_model_create_task_consistent(
        titles in prop::collection::vec(task_title_strategy(), 1..20),
    ) {
        let mut model = Model::new();

        for title in &titles {
            let msg = Message::Task(TaskMessage::Create(title.clone()));
            update(&mut model, msg);
        }

        // Task count should match
        prop_assert_eq!(model.tasks.len(), titles.len());

        // visible_tasks should be updated
        model.refresh_visible_tasks();
        prop_assert_eq!(model.visible_tasks.len(), titles.len());
    }

    /// Property: Undo after create removes the task
    #[test]
    fn prop_undo_create_removes_task(title in task_title_strategy()) {
        let mut model = Model::new();

        // Create a task
        let msg = Message::Task(TaskMessage::Create(title));
        update(&mut model, msg);
        prop_assert_eq!(model.tasks.len(), 1);

        // Undo should remove it
        let msg = Message::System(SystemMessage::Undo);
        update(&mut model, msg);
        prop_assert_eq!(model.tasks.len(), 0);
    }

    /// Property: Redo after undo restores the task
    #[test]
    fn prop_redo_restores_task(title in task_title_strategy()) {
        let mut model = Model::new();

        // Create a task
        let msg = Message::Task(TaskMessage::Create(title.clone()));
        update(&mut model, msg);
        let task_id = *model.tasks.keys().next().unwrap();

        // Undo
        update(&mut model, Message::System(SystemMessage::Undo));
        prop_assert_eq!(model.tasks.len(), 0);

        // Redo should restore it
        update(&mut model, Message::System(SystemMessage::Redo));
        prop_assert_eq!(model.tasks.len(), 1);
        prop_assert!(model.tasks.contains_key(&task_id));
    }

    /// Property: Navigation stays within bounds
    #[test]
    fn prop_navigation_within_bounds(
        task_count in 1usize..50,
        nav_count in 1usize..100,
    ) {
        let mut model = Model::new();

        // Create tasks
        for i in 0..task_count {
            let msg = Message::Task(TaskMessage::Create(format!("Task {}", i)));
            update(&mut model, msg);
        }
        model.refresh_visible_tasks();

        // Navigate many times
        for _ in 0..nav_count {
            let msg = Message::Navigation(NavigationMessage::Down);
            update(&mut model, msg);
        }

        // selected_index should never exceed visible_tasks length
        prop_assert!(model.selected_index < model.visible_tasks.len() || model.visible_tasks.is_empty());
    }

    /// Property: Toggle complete changes status between Todo and Done
    #[test]
    fn prop_toggle_complete_changes_status(title in task_title_strategy()) {
        let mut model = Model::new();
        model.filtering.show_completed = true; // Ensure we can see completed tasks

        // Create a task
        update(&mut model, Message::Task(TaskMessage::Create(title)));
        model.refresh_visible_tasks();
        model.selected_index = 0;

        let task_id = model.visible_tasks[0];
        prop_assert_eq!(model.tasks[&task_id].status, TaskStatus::Todo);

        // Toggle once - should become Done
        update(&mut model, Message::Task(TaskMessage::ToggleComplete));
        prop_assert_eq!(model.tasks[&task_id].status, TaskStatus::Done);

        // Refresh visible tasks (completed task might be hidden)
        model.refresh_visible_tasks();
        model.selected_index = 0;

        // Toggle again - should go back to Todo
        update(&mut model, Message::Task(TaskMessage::ToggleComplete));
        prop_assert_eq!(model.tasks[&task_id].status, TaskStatus::Todo);
    }

    /// Property: Multiple undos don't crash with empty history
    #[test]
    fn prop_multiple_undos_safe(undo_count in 1usize..20) {
        let mut model = Model::new();

        // Create one task
        update(&mut model, Message::Task(TaskMessage::Create("Test".to_string())));

        // Try many undos (more than history)
        for _ in 0..undo_count {
            update(&mut model, Message::System(SystemMessage::Undo));
        }

        // Should not crash, task count should be 0 or 1
        prop_assert!(model.tasks.len() <= 1);
    }

    /// Property: Empty model handles navigation safely
    #[test]
    fn prop_empty_model_navigation_safe(nav_count in 0usize..50) {
        let mut model = Model::new();
        model.refresh_visible_tasks();

        // Navigate on empty task list
        for _ in 0..nav_count {
            update(&mut model, Message::Navigation(NavigationMessage::Down));
            update(&mut model, Message::Navigation(NavigationMessage::Up));
        }

        // Should not crash, selected_index should be 0
        prop_assert_eq!(model.selected_index, 0);
        prop_assert!(model.visible_tasks.is_empty());
    }

    /// Property: Empty model handles toggle complete safely
    #[test]
    fn prop_empty_model_toggle_safe(toggle_count in 1usize..10) {
        let mut model = Model::new();
        model.refresh_visible_tasks();

        // Toggle complete on empty model multiple times should not crash
        for _ in 0..toggle_count {
            update(&mut model, Message::Task(TaskMessage::ToggleComplete));
        }

        prop_assert!(model.visible_tasks.is_empty());
    }
}
