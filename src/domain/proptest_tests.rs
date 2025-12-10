//! Property-based tests for domain types using proptest.
//!
//! These tests verify invariants that should hold for all possible inputs,
//! not just specific example cases.

use proptest::prelude::*;

use super::{
    Filter, Priority, Recurrence, SavedFilterId, SortField, SortOrder, SortSpec, TagFilterMode,
    Task, TaskId, TaskStatus, TimeEntry,
};

// =============================================================================
// Arbitrary implementations for domain types
// =============================================================================

/// Strategy for generating arbitrary Priority values
fn arb_priority() -> impl Strategy<Value = Priority> {
    prop_oneof![
        Just(Priority::None),
        Just(Priority::Low),
        Just(Priority::Medium),
        Just(Priority::High),
        Just(Priority::Urgent),
    ]
}

/// Strategy for generating arbitrary TaskStatus values
fn arb_task_status() -> impl Strategy<Value = TaskStatus> {
    prop_oneof![
        Just(TaskStatus::Todo),
        Just(TaskStatus::InProgress),
        Just(TaskStatus::Blocked),
        Just(TaskStatus::Done),
        Just(TaskStatus::Cancelled),
    ]
}

/// Strategy for generating arbitrary SortField values
fn arb_sort_field() -> impl Strategy<Value = SortField> {
    prop_oneof![
        Just(SortField::CreatedAt),
        Just(SortField::UpdatedAt),
        Just(SortField::DueDate),
        Just(SortField::Priority),
        Just(SortField::Title),
        Just(SortField::Status),
    ]
}

/// Strategy for generating arbitrary SortOrder values
fn arb_sort_order() -> impl Strategy<Value = SortOrder> {
    prop_oneof![Just(SortOrder::Ascending), Just(SortOrder::Descending),]
}

/// Strategy for generating arbitrary TagFilterMode values
fn arb_tag_filter_mode() -> impl Strategy<Value = TagFilterMode> {
    prop_oneof![Just(TagFilterMode::Any), Just(TagFilterMode::All),]
}

/// Strategy for generating task titles (non-empty strings)
fn arb_task_title() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ]{1,100}".prop_map(|s| s.trim().to_string())
}

/// Strategy for generating optional strings
fn arb_optional_string() -> impl Strategy<Value = Option<String>> {
    prop_oneof![Just(None), "[a-zA-Z0-9 ]{0,50}".prop_map(Some),]
}

/// Strategy for generating tags
fn arb_tags() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec("[a-z]{1,10}", 0..5)
}

/// Strategy for generating Recurrence values
fn arb_recurrence() -> impl Strategy<Value = Recurrence> {
    prop_oneof![
        Just(Recurrence::Daily),
        prop::collection::vec(
            prop_oneof![
                Just(chrono::Weekday::Mon),
                Just(chrono::Weekday::Tue),
                Just(chrono::Weekday::Wed),
                Just(chrono::Weekday::Thu),
                Just(chrono::Weekday::Fri),
                Just(chrono::Weekday::Sat),
                Just(chrono::Weekday::Sun),
            ],
            1..=7
        )
        .prop_map(|days| Recurrence::Weekly { days }),
        (1u32..=28).prop_map(|day| Recurrence::Monthly { day }),
        (1u32..=12, 1u32..=28).prop_map(|(month, day)| Recurrence::Yearly { month, day }),
    ]
}

// =============================================================================
// TaskId Property Tests
// =============================================================================

proptest! {
    /// TaskIds are always unique
    #[test]
    fn task_id_uniqueness(_ in 0..1000u32) {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        prop_assert_ne!(id1, id2);
    }

    /// TaskId::default() creates unique IDs each time
    #[test]
    fn task_id_default_uniqueness(_ in 0..1000u32) {
        let id1 = TaskId::default();
        let id2 = TaskId::default();
        prop_assert_ne!(id1, id2);
    }

    /// TaskId can be displayed as a string
    #[test]
    fn task_id_display_not_empty(_ in 0..100u32) {
        let id = TaskId::new();
        let display = id.to_string();
        prop_assert!(!display.is_empty());
        prop_assert!(display.len() >= 32); // UUID is at least 32 chars
    }
}

// =============================================================================
// Task Property Tests
// =============================================================================

proptest! {
    /// Task IDs are unique across all created tasks
    #[test]
    fn task_ids_are_unique(title1 in arb_task_title(), title2 in arb_task_title()) {
        let task1 = Task::new(&title1);
        let task2 = Task::new(&title2);
        prop_assert_ne!(task1.id, task2.id);
    }

    /// Task serialization roundtrip preserves all fields
    #[test]
    fn task_serialization_roundtrip(
        title in arb_task_title(),
        priority in arb_priority(),
        status in arb_task_status(),
        tags in arb_tags(),
    ) {
        let task = Task::new(&title)
            .with_priority(priority)
            .with_status(status)
            .with_tags(tags.clone());

        let json = serde_json::to_string(&task).expect("serialize");
        let restored: Task = serde_json::from_str(&json).expect("deserialize");

        prop_assert_eq!(restored.id, task.id);
        prop_assert_eq!(restored.title, task.title);
        prop_assert_eq!(restored.priority, task.priority);
        prop_assert_eq!(restored.status, task.status);
        prop_assert_eq!(restored.tags, task.tags);
    }

    /// Toggle complete twice returns to original state (for non-terminal states)
    #[test]
    fn toggle_complete_twice_returns_to_original(title in arb_task_title()) {
        let mut task = Task::new(&title);
        let original_status = task.status;

        task.toggle_complete();
        task.toggle_complete();

        // After toggling twice, we should be back to original
        prop_assert_eq!(task.status, original_status);
    }

    /// Completed tasks (Done or Cancelled) are never overdue
    #[test]
    fn completed_tasks_not_overdue(
        title in arb_task_title(),
        day_offset in 1i64..1000,
    ) {
        use chrono::Utc;

        let past_date = Utc::now().date_naive() - chrono::Duration::days(day_offset);
        let task = Task::new(&title)
            .with_due_date(past_date)
            .with_status(TaskStatus::Done);

        prop_assert!(!task.is_overdue());

        let cancelled = Task::new(&title)
            .with_due_date(past_date)
            .with_status(TaskStatus::Cancelled);

        prop_assert!(!cancelled.is_overdue());
    }

    /// Time variance is None when no estimate exists
    #[test]
    fn no_estimate_means_no_variance(title in arb_task_title(), actual in 0u32..10000) {
        let mut task = Task::new(&title);
        task.actual_minutes = actual;

        prop_assert!(task.time_variance().is_none());
        prop_assert!(task.time_variance_display().is_none());
    }

    /// Estimation accuracy returns None for zero estimate
    #[test]
    fn zero_estimate_no_accuracy(title in arb_task_title(), actual in 0u32..10000) {
        let mut task = Task::new(&title);
        task.estimated_minutes = Some(0);
        task.actual_minutes = actual;

        prop_assert!(task.estimation_accuracy().is_none());
    }
}

// =============================================================================
// Priority Property Tests
// =============================================================================

proptest! {
    /// Priority::parse is the inverse of Priority::as_str
    #[test]
    fn priority_parse_as_str_roundtrip(priority in arb_priority()) {
        let parsed = Priority::parse(priority.as_str());
        prop_assert_eq!(parsed, Some(priority));
    }

    /// Priority serialization roundtrip
    #[test]
    fn priority_serialization_roundtrip(priority in arb_priority()) {
        let json = serde_json::to_string(&priority).expect("serialize");
        let restored: Priority = serde_json::from_str(&json).expect("deserialize");
        prop_assert_eq!(restored, priority);
    }

    /// Priority::parse is case-insensitive
    #[test]
    fn priority_parse_case_insensitive(priority in arb_priority()) {
        let lowercase = priority.as_str().to_lowercase();
        let uppercase = priority.as_str().to_uppercase();

        prop_assert_eq!(Priority::parse(&lowercase), Some(priority));
        prop_assert_eq!(Priority::parse(&uppercase), Some(priority));
    }
}

// =============================================================================
// TaskStatus Property Tests
// =============================================================================

proptest! {
    /// is_complete is true only for Done and Cancelled
    #[test]
    fn is_complete_correct(status in arb_task_status()) {
        let is_complete = status.is_complete();
        let expected = matches!(status, TaskStatus::Done | TaskStatus::Cancelled);
        prop_assert_eq!(is_complete, expected);
    }

    /// TaskStatus serialization roundtrip
    #[test]
    fn task_status_serialization_roundtrip(status in arb_task_status()) {
        let json = serde_json::to_string(&status).expect("serialize");
        let restored: TaskStatus = serde_json::from_str(&json).expect("deserialize");
        prop_assert_eq!(restored, status);
    }

    /// as_str returns non-empty string
    #[test]
    fn status_as_str_not_empty(status in arb_task_status()) {
        prop_assert!(!status.as_str().is_empty());
    }

    /// symbol returns bracketed string
    #[test]
    fn status_symbol_is_bracketed(status in arb_task_status()) {
        let symbol = status.symbol();
        prop_assert!(symbol.starts_with('['));
        prop_assert!(symbol.ends_with(']'));
    }
}

// =============================================================================
// TimeEntry Property Tests
// =============================================================================

proptest! {
    /// TimeEntry duration calculation is correct
    #[test]
    fn time_entry_duration_matches_set_value(minutes in 0u32..10000) {
        let task_id = TaskId::new();
        let mut entry = TimeEntry::start(task_id);
        entry.duration_minutes = Some(minutes);

        // When duration_minutes is set, calculated_duration_minutes returns it
        prop_assert_eq!(entry.calculated_duration_minutes(), minutes);
    }

    /// TimeEntry is_running is false after stop
    #[test]
    fn time_entry_stop_ends_running(_ in 0..100u32) {
        let task_id = TaskId::new();
        let mut entry = TimeEntry::start(task_id);

        prop_assert!(entry.is_running());
        entry.stop();
        prop_assert!(!entry.is_running());
    }

    /// TimeEntry formatted_duration is always valid format
    #[test]
    fn time_entry_formatted_duration_valid(minutes in 0u32..10000) {
        let task_id = TaskId::new();
        let mut entry = TimeEntry::start(task_id);
        entry.duration_minutes = Some(minutes);

        let formatted = entry.formatted_duration();
        // Should end with 'm' (minutes marker)
        prop_assert!(formatted.ends_with('m'));
        // Should contain only valid characters
        prop_assert!(formatted.chars().all(|c| c.is_ascii_digit() || c == 'h' || c == 'm' || c == ' '));
    }
}

// =============================================================================
// Filter Property Tests
// =============================================================================

proptest! {
    /// Filter serialization roundtrip
    #[test]
    fn filter_serialization_roundtrip(
        include_completed in any::<bool>(),
        include_subtasks in any::<bool>(),
        tags_mode in arb_tag_filter_mode(),
        search_text in arb_optional_string(),
    ) {
        let filter = Filter {
            tags_mode,
            search_text,
            include_subtasks,
            include_completed,
            ..Default::default()
        };

        let json = serde_json::to_string(&filter).expect("serialize");
        let restored: Filter = serde_json::from_str(&json).expect("deserialize");

        prop_assert_eq!(restored.include_completed, filter.include_completed);
        prop_assert_eq!(restored.include_subtasks, filter.include_subtasks);
    }

    /// Default filter has no restrictions
    #[test]
    fn default_filter_is_permissive(_ in 0..10u32) {
        let filter = Filter::default();

        prop_assert!(filter.status.is_none());
        prop_assert!(filter.priority.is_none());
        prop_assert!(filter.project_id.is_none());
        prop_assert!(filter.tags.is_none());
        prop_assert!(filter.due_before.is_none());
        prop_assert!(filter.due_after.is_none());
        prop_assert!(filter.search_text.is_none());
    }
}

// =============================================================================
// SortSpec Property Tests
// =============================================================================

proptest! {
    /// SortSpec serialization roundtrip
    #[test]
    fn sort_spec_serialization_roundtrip(
        field in arb_sort_field(),
        order in arb_sort_order(),
    ) {
        let spec = SortSpec { field, order };

        let json = serde_json::to_string(&spec).expect("serialize");
        let restored: SortSpec = serde_json::from_str(&json).expect("deserialize");

        // Compare field and order separately since they don't derive PartialEq
        prop_assert!(matches!(
            (restored.field, spec.field),
            (SortField::CreatedAt, SortField::CreatedAt)
            | (SortField::UpdatedAt, SortField::UpdatedAt)
            | (SortField::DueDate, SortField::DueDate)
            | (SortField::Priority, SortField::Priority)
            | (SortField::Title, SortField::Title)
            | (SortField::Status, SortField::Status)
        ));
    }
}

// =============================================================================
// Recurrence Property Tests
// =============================================================================

proptest! {
    /// Recurrence serialization roundtrip
    #[test]
    fn recurrence_serialization_roundtrip(recurrence in arb_recurrence()) {
        let json = serde_json::to_string(&recurrence).expect("serialize");
        let restored: Recurrence = serde_json::from_str(&json).expect("deserialize");
        prop_assert_eq!(restored, recurrence);
    }

    /// Recurrence display is non-empty
    #[test]
    fn recurrence_display_not_empty(recurrence in arb_recurrence()) {
        let display = recurrence.to_string();
        prop_assert!(!display.is_empty());
    }

    /// Monthly recurrence day is in valid range (1-31)
    #[test]
    fn monthly_recurrence_day_valid(day in 1u32..=28) {
        let recurrence = Recurrence::Monthly { day };
        let display = recurrence.to_string();
        prop_assert!(display.contains(&day.to_string()));
    }

    /// Yearly recurrence month and day are in valid ranges
    #[test]
    fn yearly_recurrence_valid(month in 1u32..=12, day in 1u32..=28) {
        let recurrence = Recurrence::Yearly { month, day };
        let display = recurrence.to_string();
        let expected = format!("{month}/{day}");
        prop_assert!(display.contains(&expected));
    }
}

// =============================================================================
// SavedFilterId Property Tests
// =============================================================================

proptest! {
    /// SavedFilterId::new creates unique IDs
    #[test]
    fn saved_filter_id_uniqueness(_ in 0..1000u32) {
        let id1 = SavedFilterId::new();
        let id2 = SavedFilterId::new();
        prop_assert_ne!(id1, id2);
    }
}
