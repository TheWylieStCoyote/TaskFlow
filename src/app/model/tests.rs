//! Tests for the Model.

use super::*;
use crate::domain::{Priority, TaskStatus};

#[test]
fn test_model_new_defaults() {
    let model = Model::new();

    assert_eq!(model.running, RunningState::Running);
    assert!(model.tasks.is_empty());
    assert!(model.projects.is_empty());
    assert!(model.time_entries.is_empty());
    assert!(model.active_time_entry.is_none());
    assert_eq!(model.selected_index, 0);
    assert!(model.visible_tasks.is_empty());
    assert!(!model.show_completed);
    assert!(model.show_sidebar);
    assert!(!model.show_help);
    assert_eq!(model.input_mode, InputMode::Normal);
    assert!(model.input_buffer.is_empty());
    assert!(!model.dirty);
}

#[test]
fn test_model_with_sample_data() {
    let model = Model::new().with_sample_data();

    // Sample data creates 15 tasks across 3 projects
    assert_eq!(model.tasks.len(), 15);
    assert_eq!(model.projects.len(), 3);
    // Some are completed, so visible should be less
    assert!(model.visible_tasks.len() < 15);
}

#[test]
fn test_model_refresh_visible_tasks_sorts_by_priority() {
    use crate::domain::{SortField, SortOrder};

    let mut model = Model::new();

    // Add tasks with different priorities
    let urgent = Task::new("Urgent").with_priority(Priority::Urgent);
    let low = Task::new("Low").with_priority(Priority::Low);
    let high = Task::new("High").with_priority(Priority::High);

    model.tasks.insert(low.id.clone(), low.clone());
    model.tasks.insert(urgent.id.clone(), urgent.clone());
    model.tasks.insert(high.id.clone(), high.clone());

    // Set sort to priority (default is CreatedAt)
    model.sort = SortSpec {
        field: SortField::Priority,
        order: SortOrder::Ascending,
    };
    model.refresh_visible_tasks();

    // Order should be: Urgent, High, Low
    assert_eq!(model.visible_tasks.len(), 3);
    assert_eq!(model.visible_tasks[0], urgent.id);
    assert_eq!(model.visible_tasks[1], high.id);
    assert_eq!(model.visible_tasks[2], low.id);
}

#[test]
fn test_model_refresh_visible_tasks_hides_completed() {
    let mut model = Model::new();
    model.show_completed = false;

    let todo = Task::new("Todo");
    let done = Task::new("Done").with_status(TaskStatus::Done);
    let cancelled = Task::new("Cancelled").with_status(TaskStatus::Cancelled);

    model.tasks.insert(todo.id.clone(), todo);
    model.tasks.insert(done.id.clone(), done);
    model.tasks.insert(cancelled.id.clone(), cancelled);

    model.refresh_visible_tasks();

    // Only non-completed tasks should be visible
    assert_eq!(model.visible_tasks.len(), 1);
}

#[test]
fn test_model_refresh_visible_tasks_shows_completed() {
    let mut model = Model::new();
    model.show_completed = true;

    let todo = Task::new("Todo");
    let done = Task::new("Done").with_status(TaskStatus::Done);
    let cancelled = Task::new("Cancelled").with_status(TaskStatus::Cancelled);

    model.tasks.insert(todo.id.clone(), todo);
    model.tasks.insert(done.id.clone(), done);
    model.tasks.insert(cancelled.id.clone(), cancelled);

    model.refresh_visible_tasks();

    // All tasks should be visible
    assert_eq!(model.visible_tasks.len(), 3);
}

#[test]
fn test_model_refresh_visible_tasks_subtasks_follow_parent() {
    use crate::domain::{SortField, SortOrder};

    let mut model = Model::new();

    // Create a parent task and two subtasks
    let parent1 = Task::new("Parent 1").with_priority(Priority::High);
    let subtask1a = Task::new("Subtask 1a").with_parent(parent1.id.clone());
    let subtask1b = Task::new("Subtask 1b").with_parent(parent1.id.clone());

    let parent2 = Task::new("Parent 2").with_priority(Priority::Low);
    let subtask2a = Task::new("Subtask 2a").with_parent(parent2.id.clone());

    // Insert in random order
    model.tasks.insert(subtask1b.id.clone(), subtask1b.clone());
    model.tasks.insert(parent2.id.clone(), parent2.clone());
    model.tasks.insert(subtask2a.id.clone(), subtask2a.clone());
    model.tasks.insert(parent1.id.clone(), parent1.clone());
    model.tasks.insert(subtask1a.id.clone(), subtask1a.clone());

    // Sort by priority so parent order is deterministic
    model.sort = SortSpec {
        field: SortField::Priority,
        order: SortOrder::Ascending,
    };
    model.refresh_visible_tasks();

    // Should be: Parent 1 (High), Subtask 1a, Subtask 1b, Parent 2 (Low), Subtask 2a
    assert_eq!(model.visible_tasks.len(), 5);
    assert_eq!(model.visible_tasks[0], parent1.id);
    // Subtasks of parent1 should immediately follow
    assert!(
        model.visible_tasks[1] == subtask1a.id || model.visible_tasks[1] == subtask1b.id,
        "Subtask 1 should follow parent 1"
    );
    assert!(
        model.visible_tasks[2] == subtask1a.id || model.visible_tasks[2] == subtask1b.id,
        "Subtask 2 should follow parent 1"
    );
    assert_eq!(model.visible_tasks[3], parent2.id);
    assert_eq!(model.visible_tasks[4], subtask2a.id);
}

#[test]
fn test_model_selected_task_returns_correct() {
    let mut model = Model::new();

    let task1 = Task::new("Task 1");
    let task2 = Task::new("Task 2");

    model.tasks.insert(task1.id.clone(), task1.clone());
    model.tasks.insert(task2.id.clone(), task2.clone());
    model.refresh_visible_tasks();

    // Select first task
    model.selected_index = 0;
    let selected = model.selected_task().unwrap();
    assert_eq!(selected.id, model.visible_tasks[0]);

    // Select second task
    model.selected_index = 1;
    let selected = model.selected_task().unwrap();
    assert_eq!(selected.id, model.visible_tasks[1]);
}

#[test]
fn test_model_selected_task_empty_list() {
    let model = Model::new();

    assert!(model.selected_task().is_none());
}

#[test]
fn test_model_selected_index_adjustment() {
    let mut model = Model::new();

    // Add 3 tasks
    for i in 0..3 {
        let task = Task::new(format!("Task {}", i));
        model.tasks.insert(task.id.clone(), task);
    }
    model.refresh_visible_tasks();

    // Select last item
    model.selected_index = 2;

    // Remove all tasks except one
    let ids: Vec<_> = model.tasks.keys().skip(1).cloned().collect();
    for id in ids {
        model.tasks.remove(&id);
    }

    model.refresh_visible_tasks();

    // Selection should be adjusted to valid range
    assert!(model.selected_index < model.visible_tasks.len());
}

#[test]
fn test_model_start_time_tracking() {
    let mut model = Model::new();

    let task = Task::new("Task");
    model.tasks.insert(task.id.clone(), task.clone());

    model.start_time_tracking(task.id.clone());

    assert!(model.active_time_entry.is_some());
    assert!(model.time_entries.len() == 1);
    assert!(model.dirty);

    let entry = model.active_time_entry().unwrap();
    assert_eq!(entry.task_id, task.id);
    assert!(entry.is_running());
}

#[test]
fn test_model_stop_time_tracking() {
    let mut model = Model::new();

    let task = Task::new("Task");
    model.tasks.insert(task.id.clone(), task.clone());

    model.start_time_tracking(task.id.clone());
    model.stop_time_tracking();

    assert!(model.active_time_entry.is_none());

    // Entry should still exist but be stopped
    let entry = model.time_entries.values().next().unwrap();
    assert!(!entry.is_running());
}

#[test]
fn test_model_start_stops_previous() {
    let mut model = Model::new();

    let task1 = Task::new("Task 1");
    let task2 = Task::new("Task 2");
    model.tasks.insert(task1.id.clone(), task1.clone());
    model.tasks.insert(task2.id.clone(), task2.clone());

    // Start tracking task1
    model.start_time_tracking(task1.id.clone());
    let first_entry_id = model.active_time_entry.clone().unwrap();

    // Start tracking task2 (should stop task1)
    model.start_time_tracking(task2.id.clone());

    // Two entries total
    assert_eq!(model.time_entries.len(), 2);

    // First entry should be stopped
    let first_entry = model.time_entries.get(&first_entry_id).unwrap();
    assert!(!first_entry.is_running());

    // Active entry should be for task2
    let active = model.active_time_entry().unwrap();
    assert_eq!(active.task_id, task2.id);
}

#[test]
fn test_model_is_tracking_task() {
    let mut model = Model::new();

    let task1 = Task::new("Task 1");
    let task2 = Task::new("Task 2");
    model.tasks.insert(task1.id.clone(), task1.clone());
    model.tasks.insert(task2.id.clone(), task2.clone());

    // Not tracking anything initially
    assert!(!model.is_tracking_task(&task1.id));
    assert!(!model.is_tracking_task(&task2.id));

    // Start tracking task1
    model.start_time_tracking(task1.id.clone());

    assert!(model.is_tracking_task(&task1.id));
    assert!(!model.is_tracking_task(&task2.id));
}

#[test]
fn test_model_total_time_for_task() {
    let mut model = Model::new();

    let task = Task::new("Task");
    model.tasks.insert(task.id.clone(), task.clone());

    // Add multiple completed time entries
    let mut entry1 = TimeEntry::start(task.id.clone());
    entry1.duration_minutes = Some(30);
    entry1.ended_at = Some(chrono::Utc::now());

    let mut entry2 = TimeEntry::start(task.id.clone());
    entry2.duration_minutes = Some(45);
    entry2.ended_at = Some(chrono::Utc::now());

    model.time_entries.insert(entry1.id.clone(), entry1);
    model.time_entries.insert(entry2.id.clone(), entry2);

    let total = model.total_time_for_task(&task.id);
    assert_eq!(total, 75); // 30 + 45
}

#[test]
fn test_model_dirty_flag() {
    let mut model = Model::new();
    assert!(!model.dirty);

    let task = Task::new("Task");
    model.tasks.insert(task.id.clone(), task.clone());

    model.start_time_tracking(task.id.clone());
    assert!(model.dirty);
}

#[test]
fn test_model_has_storage() {
    let model = Model::new();
    assert!(!model.has_storage());
}

#[test]
fn test_running_state_default() {
    let state = RunningState::default();
    assert_eq!(state, RunningState::Running);
}

#[test]
fn test_view_tasklist_shows_all() {
    let mut model = Model::new();
    model.current_view = ViewId::TaskList;

    // Create tasks with various due dates and project associations
    let task_no_date = Task::new("No due date");
    let task_with_date =
        Task::new("Has date").with_due_date(chrono::NaiveDate::from_ymd_opt(2025, 12, 15).unwrap());
    let task_with_project = Task::new("Has project").with_project(crate::domain::ProjectId::new());

    model.tasks.insert(task_no_date.id.clone(), task_no_date);
    model
        .tasks
        .insert(task_with_date.id.clone(), task_with_date);
    model
        .tasks
        .insert(task_with_project.id.clone(), task_with_project);

    model.refresh_visible_tasks();

    // TaskList view should show all tasks
    assert_eq!(model.visible_tasks.len(), 3);
}

#[test]
fn test_view_today_filters_due_today() {
    let mut model = Model::new();
    model.current_view = ViewId::Today;

    let today = chrono::Utc::now().date_naive();
    let tomorrow = today + chrono::Duration::days(1);

    let task_today = Task::new("Due today").with_due_date(today);
    let task_tomorrow = Task::new("Due tomorrow").with_due_date(tomorrow);
    let task_no_date = Task::new("No due date");

    model
        .tasks
        .insert(task_today.id.clone(), task_today.clone());
    model.tasks.insert(task_tomorrow.id.clone(), task_tomorrow);
    model.tasks.insert(task_no_date.id.clone(), task_no_date);

    model.refresh_visible_tasks();

    // Only today's task should be visible
    assert_eq!(model.visible_tasks.len(), 1);
    assert_eq!(model.visible_tasks[0], task_today.id);
}

#[test]
fn test_view_upcoming_filters_future() {
    let mut model = Model::new();
    model.current_view = ViewId::Upcoming;

    let today = chrono::Utc::now().date_naive();
    let tomorrow = today + chrono::Duration::days(1);
    let next_week = today + chrono::Duration::days(7);

    let task_today = Task::new("Due today").with_due_date(today);
    let task_tomorrow = Task::new("Due tomorrow").with_due_date(tomorrow);
    let task_next_week = Task::new("Due next week").with_due_date(next_week);
    let task_no_date = Task::new("No due date");

    model.tasks.insert(task_today.id.clone(), task_today);
    model
        .tasks
        .insert(task_tomorrow.id.clone(), task_tomorrow.clone());
    model
        .tasks
        .insert(task_next_week.id.clone(), task_next_week.clone());
    model.tasks.insert(task_no_date.id.clone(), task_no_date);

    model.refresh_visible_tasks();

    // Only future tasks should be visible (not today, not tasks without dates)
    assert_eq!(model.visible_tasks.len(), 2);
    assert!(model.visible_tasks.contains(&task_tomorrow.id));
    assert!(model.visible_tasks.contains(&task_next_week.id));
}

#[test]
fn test_view_projects_filters_with_project() {
    let mut model = Model::new();
    model.current_view = ViewId::Projects;

    let project_id = crate::domain::ProjectId::new();

    let task_with_project = Task::new("Has project").with_project(project_id);
    let task_no_project = Task::new("No project");

    model
        .tasks
        .insert(task_with_project.id.clone(), task_with_project.clone());
    model
        .tasks
        .insert(task_no_project.id.clone(), task_no_project);

    model.refresh_visible_tasks();

    // Only tasks with projects should be visible
    assert_eq!(model.visible_tasks.len(), 1);
    assert_eq!(model.visible_tasks[0], task_with_project.id);
}

#[test]
fn test_view_overdue_filters_past_due() {
    let mut model = Model::new();
    model.current_view = ViewId::Overdue;

    let today = chrono::Utc::now().date_naive();
    let yesterday = today - chrono::Duration::days(1);
    let last_week = today - chrono::Duration::days(7);
    let tomorrow = today + chrono::Duration::days(1);

    let task_yesterday = Task::new("Due yesterday").with_due_date(yesterday);
    let task_last_week = Task::new("Due last week").with_due_date(last_week);
    let task_today = Task::new("Due today").with_due_date(today);
    let task_tomorrow = Task::new("Due tomorrow").with_due_date(tomorrow);
    let task_no_date = Task::new("No due date");

    model
        .tasks
        .insert(task_yesterday.id.clone(), task_yesterday.clone());
    model
        .tasks
        .insert(task_last_week.id.clone(), task_last_week.clone());
    model.tasks.insert(task_today.id.clone(), task_today);
    model.tasks.insert(task_tomorrow.id.clone(), task_tomorrow);
    model.tasks.insert(task_no_date.id.clone(), task_no_date);

    model.refresh_visible_tasks();

    // Only overdue tasks (past due dates) should be visible
    assert_eq!(model.visible_tasks.len(), 2);
    assert!(model.visible_tasks.contains(&task_yesterday.id));
    assert!(model.visible_tasks.contains(&task_last_week.id));
}

#[test]
fn test_view_overdue_excludes_today() {
    let mut model = Model::new();
    model.current_view = ViewId::Overdue;

    let today = chrono::Utc::now().date_naive();
    let task_today = Task::new("Due today").with_due_date(today);

    model.tasks.insert(task_today.id.clone(), task_today);

    model.refresh_visible_tasks();

    // Today's tasks are not overdue
    assert!(model.visible_tasks.is_empty());
}

#[test]
fn test_view_overdue_excludes_no_due_date() {
    let mut model = Model::new();
    model.current_view = ViewId::Overdue;

    let task_no_date = Task::new("No due date");
    model.tasks.insert(task_no_date.id.clone(), task_no_date);

    model.refresh_visible_tasks();

    // Tasks without due dates are not overdue
    assert!(model.visible_tasks.is_empty());
}

#[test]
fn test_search_filter_matches_title() {
    let mut model = Model::new();

    let task_match = Task::new("Build the feature");
    let task_no_match = Task::new("Fix the bug");

    model
        .tasks
        .insert(task_match.id.clone(), task_match.clone());
    model.tasks.insert(task_no_match.id.clone(), task_no_match);

    model.filter.search_text = Some("build".to_string());
    model.refresh_visible_tasks();

    assert_eq!(model.visible_tasks.len(), 1);
    assert_eq!(model.visible_tasks[0], task_match.id);
}

#[test]
fn test_search_filter_case_insensitive() {
    let mut model = Model::new();

    let task = Task::new("Build Feature");
    model.tasks.insert(task.id.clone(), task.clone());

    // Search with different cases
    model.filter.search_text = Some("BUILD".to_string());
    model.refresh_visible_tasks();
    assert_eq!(model.visible_tasks.len(), 1);

    model.filter.search_text = Some("feature".to_string());
    model.refresh_visible_tasks();
    assert_eq!(model.visible_tasks.len(), 1);
}

#[test]
fn test_search_filter_matches_tags() {
    let mut model = Model::new();

    let task_with_tag = Task::new("Some task").with_tags(vec!["urgent".to_string()]);
    let task_no_tag = Task::new("Other task");

    model
        .tasks
        .insert(task_with_tag.id.clone(), task_with_tag.clone());
    model.tasks.insert(task_no_tag.id.clone(), task_no_tag);

    model.filter.search_text = Some("urgent".to_string());
    model.refresh_visible_tasks();

    assert_eq!(model.visible_tasks.len(), 1);
    assert_eq!(model.visible_tasks[0], task_with_tag.id);
}

#[test]
fn test_search_filter_partial_match() {
    let mut model = Model::new();

    let task = Task::new("Implement authentication");
    model.tasks.insert(task.id.clone(), task.clone());

    model.filter.search_text = Some("auth".to_string());
    model.refresh_visible_tasks();

    assert_eq!(model.visible_tasks.len(), 1);
}

#[test]
fn test_search_filter_empty_clears() {
    let mut model = Model::new();

    let task1 = Task::new("Task one");
    let task2 = Task::new("Task two");

    model.tasks.insert(task1.id.clone(), task1);
    model.tasks.insert(task2.id.clone(), task2);

    // With filter
    model.filter.search_text = Some("one".to_string());
    model.refresh_visible_tasks();
    assert_eq!(model.visible_tasks.len(), 1);

    // Without filter
    model.filter.search_text = None;
    model.refresh_visible_tasks();
    assert_eq!(model.visible_tasks.len(), 2);
}

#[test]
fn test_tag_filter_any_mode() {
    use crate::domain::TagFilterMode;

    let mut model = Model::new();

    let task_rust = Task::new("Task Rust").with_tags(vec!["rust".to_string()]);
    let task_python = Task::new("Task Python").with_tags(vec!["python".to_string()]);
    let task_both =
        Task::new("Task Both").with_tags(vec!["rust".to_string(), "python".to_string()]);
    let task_none = Task::new("Task None");

    model.tasks.insert(task_rust.id.clone(), task_rust.clone());
    model
        .tasks
        .insert(task_python.id.clone(), task_python.clone());
    model.tasks.insert(task_both.id.clone(), task_both.clone());
    model.tasks.insert(task_none.id.clone(), task_none);

    // Filter by "rust" tag (Any mode - default)
    model.filter.tags = Some(vec!["rust".to_string()]);
    model.filter.tags_mode = TagFilterMode::Any;
    model.refresh_visible_tasks();

    assert_eq!(model.visible_tasks.len(), 2);
    assert!(model.visible_tasks.contains(&task_rust.id));
    assert!(model.visible_tasks.contains(&task_both.id));
}

#[test]
fn test_tag_filter_all_mode() {
    use crate::domain::TagFilterMode;

    let mut model = Model::new();

    let task_rust = Task::new("Task Rust").with_tags(vec!["rust".to_string()]);
    let task_both =
        Task::new("Task Both").with_tags(vec!["rust".to_string(), "python".to_string()]);
    let task_none = Task::new("Task None");

    model.tasks.insert(task_rust.id.clone(), task_rust.clone());
    model.tasks.insert(task_both.id.clone(), task_both.clone());
    model.tasks.insert(task_none.id.clone(), task_none);

    // Filter by "rust" AND "python" tags (All mode)
    model.filter.tags = Some(vec!["rust".to_string(), "python".to_string()]);
    model.filter.tags_mode = TagFilterMode::All;
    model.refresh_visible_tasks();

    // Only task_both has both tags
    assert_eq!(model.visible_tasks.len(), 1);
    assert!(model.visible_tasks.contains(&task_both.id));
}

#[test]
fn test_tag_filter_case_insensitive() {
    let mut model = Model::new();

    let task = Task::new("Task").with_tags(vec!["Rust".to_string()]);
    model.tasks.insert(task.id.clone(), task.clone());

    // Filter with different case
    model.filter.tags = Some(vec!["rust".to_string()]);
    model.refresh_visible_tasks();

    assert_eq!(model.visible_tasks.len(), 1);
    assert!(model.visible_tasks.contains(&task.id));
}

#[test]
fn test_tag_filter_clear() {
    let mut model = Model::new();

    let task_tagged = Task::new("Tagged").with_tags(vec!["work".to_string()]);
    let task_untagged = Task::new("Untagged");

    model
        .tasks
        .insert(task_tagged.id.clone(), task_tagged.clone());
    model.tasks.insert(task_untagged.id.clone(), task_untagged);

    // With filter
    model.filter.tags = Some(vec!["work".to_string()]);
    model.refresh_visible_tasks();
    assert_eq!(model.visible_tasks.len(), 1);

    // Clear filter
    model.filter.tags = None;
    model.refresh_visible_tasks();
    assert_eq!(model.visible_tasks.len(), 2);
}

#[test]
fn test_tag_filter_with_search() {
    let mut model = Model::new();

    let task_match =
        Task::new("Important Task").with_tags(vec!["work".to_string(), "urgent".to_string()]);
    let task_wrong_tag = Task::new("Important Other").with_tags(vec!["home".to_string()]);
    let task_wrong_title = Task::new("Regular Task").with_tags(vec!["work".to_string()]);

    model
        .tasks
        .insert(task_match.id.clone(), task_match.clone());
    model
        .tasks
        .insert(task_wrong_tag.id.clone(), task_wrong_tag);
    model
        .tasks
        .insert(task_wrong_title.id.clone(), task_wrong_title);

    // Both search and tag filter
    model.filter.search_text = Some("Important".to_string());
    model.filter.tags = Some(vec!["work".to_string()]);
    model.refresh_visible_tasks();

    // Only task_match matches both criteria
    assert_eq!(model.visible_tasks.len(), 1);
    assert!(model.visible_tasks.contains(&task_match.id));
}

#[test]
fn test_sort_by_title() {
    use crate::domain::{SortField, SortOrder};

    let mut model = Model::new();

    let task_b = Task::new("Banana");
    let task_a = Task::new("Apple");
    let task_c = Task::new("Cherry");

    model.tasks.insert(task_b.id.clone(), task_b.clone());
    model.tasks.insert(task_a.id.clone(), task_a.clone());
    model.tasks.insert(task_c.id.clone(), task_c.clone());

    model.sort = SortSpec {
        field: SortField::Title,
        order: SortOrder::Ascending,
    };
    model.refresh_visible_tasks();

    assert_eq!(model.visible_tasks[0], task_a.id);
    assert_eq!(model.visible_tasks[1], task_b.id);
    assert_eq!(model.visible_tasks[2], task_c.id);
}

#[test]
fn test_sort_by_title_descending() {
    use crate::domain::{SortField, SortOrder};

    let mut model = Model::new();

    let task_b = Task::new("Banana");
    let task_a = Task::new("Apple");
    let task_c = Task::new("Cherry");

    model.tasks.insert(task_b.id.clone(), task_b.clone());
    model.tasks.insert(task_a.id.clone(), task_a.clone());
    model.tasks.insert(task_c.id.clone(), task_c.clone());

    model.sort = SortSpec {
        field: SortField::Title,
        order: SortOrder::Descending,
    };
    model.refresh_visible_tasks();

    assert_eq!(model.visible_tasks[0], task_c.id);
    assert_eq!(model.visible_tasks[1], task_b.id);
    assert_eq!(model.visible_tasks[2], task_a.id);
}

#[test]
fn test_sort_by_due_date() {
    use crate::domain::{SortField, SortOrder};

    let mut model = Model::new();

    let today = chrono::Utc::now().date_naive();
    let tomorrow = today + chrono::Duration::days(1);
    let next_week = today + chrono::Duration::days(7);

    let task_soon = Task::new("Soon").with_due_date(tomorrow);
    let task_later = Task::new("Later").with_due_date(next_week);
    let task_no_date = Task::new("No date");

    model
        .tasks
        .insert(task_later.id.clone(), task_later.clone());
    model.tasks.insert(task_soon.id.clone(), task_soon.clone());
    model
        .tasks
        .insert(task_no_date.id.clone(), task_no_date.clone());

    model.sort = SortSpec {
        field: SortField::DueDate,
        order: SortOrder::Ascending,
    };
    model.refresh_visible_tasks();

    // Tasks with dates come first, then tasks without dates
    assert_eq!(model.visible_tasks[0], task_soon.id);
    assert_eq!(model.visible_tasks[1], task_later.id);
    assert_eq!(model.visible_tasks[2], task_no_date.id);
}

#[test]
fn test_sort_by_status() {
    use crate::domain::{SortField, SortOrder};

    let mut model = Model::new();
    model.show_completed = true; // Show completed for this test

    let task_todo = Task::new("Todo").with_status(TaskStatus::Todo);
    let task_in_progress = Task::new("In Progress").with_status(TaskStatus::InProgress);
    let task_done = Task::new("Done").with_status(TaskStatus::Done);

    model.tasks.insert(task_done.id.clone(), task_done.clone());
    model.tasks.insert(task_todo.id.clone(), task_todo.clone());
    model
        .tasks
        .insert(task_in_progress.id.clone(), task_in_progress.clone());

    model.sort = SortSpec {
        field: SortField::Status,
        order: SortOrder::Ascending,
    };
    model.refresh_visible_tasks();

    // Order: InProgress, Todo, Blocked, Done, Cancelled
    assert_eq!(model.visible_tasks[0], task_in_progress.id);
    assert_eq!(model.visible_tasks[1], task_todo.id);
    assert_eq!(model.visible_tasks[2], task_done.id);
}

#[test]
fn test_sort_order_toggle() {
    use crate::domain::{SortField, SortOrder};

    let mut model = Model::new();

    let task_high = Task::new("High").with_priority(Priority::High);
    let task_low = Task::new("Low").with_priority(Priority::Low);

    model.tasks.insert(task_high.id.clone(), task_high.clone());
    model.tasks.insert(task_low.id.clone(), task_low.clone());

    // Ascending: High first (lower priority number)
    model.sort = SortSpec {
        field: SortField::Priority,
        order: SortOrder::Ascending,
    };
    model.refresh_visible_tasks();
    assert_eq!(model.visible_tasks[0], task_high.id);
    assert_eq!(model.visible_tasks[1], task_low.id);

    // Descending: Low first
    model.sort.order = SortOrder::Descending;
    model.refresh_visible_tasks();
    assert_eq!(model.visible_tasks[0], task_low.id);
    assert_eq!(model.visible_tasks[1], task_high.id);
}

#[test]
fn test_get_tasks_grouped_by_project_basic() {
    use crate::domain::Project;

    let mut model = Model::new();
    model.current_view = ViewId::Projects;

    // Create two projects
    let project_a = Project::new("Alpha Project");
    let project_b = Project::new("Beta Project");
    let project_a_id = project_a.id.clone();
    let project_b_id = project_b.id.clone();

    model.projects.insert(project_a_id.clone(), project_a);
    model.projects.insert(project_b_id.clone(), project_b);

    // Create tasks for each project
    let task_a1 = Task::new("Alpha Task 1").with_project(project_a_id.clone());
    let task_a2 = Task::new("Alpha Task 2").with_project(project_a_id.clone());
    let task_b1 = Task::new("Beta Task 1").with_project(project_b_id.clone());

    model.tasks.insert(task_a1.id.clone(), task_a1);
    model.tasks.insert(task_a2.id.clone(), task_a2);
    model.tasks.insert(task_b1.id.clone(), task_b1);

    model.refresh_visible_tasks();

    let grouped = model.get_tasks_grouped_by_project();

    // Should have 2 groups (Alpha and Beta, sorted alphabetically)
    assert_eq!(grouped.len(), 2);
    assert_eq!(grouped[0].1, "Alpha Project");
    assert_eq!(grouped[0].2.len(), 2); // 2 tasks in Alpha
    assert_eq!(grouped[1].1, "Beta Project");
    assert_eq!(grouped[1].2.len(), 1); // 1 task in Beta
}

#[test]
fn test_get_tasks_grouped_by_project_alphabetical_order() {
    use crate::domain::Project;

    let mut model = Model::new();
    model.current_view = ViewId::Projects;

    // Create projects out of alphabetical order
    let project_z = Project::new("Zebra");
    let project_a = Project::new("Apple");
    let project_m = Project::new("Mango");

    let z_id = project_z.id.clone();
    let a_id = project_a.id.clone();
    let m_id = project_m.id.clone();

    model.projects.insert(z_id.clone(), project_z);
    model.projects.insert(a_id.clone(), project_a);
    model.projects.insert(m_id.clone(), project_m);

    // Create one task per project
    let task_z = Task::new("Z task").with_project(z_id);
    let task_a = Task::new("A task").with_project(a_id);
    let task_m = Task::new("M task").with_project(m_id);

    model.tasks.insert(task_z.id.clone(), task_z);
    model.tasks.insert(task_a.id.clone(), task_a);
    model.tasks.insert(task_m.id.clone(), task_m);

    model.refresh_visible_tasks();

    let grouped = model.get_tasks_grouped_by_project();

    // Should be sorted alphabetically: Apple, Mango, Zebra
    assert_eq!(grouped.len(), 3);
    assert_eq!(grouped[0].1, "Apple");
    assert_eq!(grouped[1].1, "Mango");
    assert_eq!(grouped[2].1, "Zebra");
}

#[test]
fn test_get_tasks_grouped_no_project_goes_last() {
    use crate::domain::Project;

    let mut model = Model::new();
    model.current_view = ViewId::Projects;

    // Create one project
    let project = Project::new("My Project");
    let project_id = project.id.clone();
    model.projects.insert(project_id.clone(), project);

    // Task with project
    let task_with = Task::new("With project").with_project(project_id);
    // Task without project (shouldn't appear in Projects view normally,
    // but test the grouping logic)
    let task_without = Task::new("Without project");

    model.tasks.insert(task_with.id.clone(), task_with);
    model.tasks.insert(task_without.id.clone(), task_without);

    // For this test, we need to make both visible
    // Override the view filtering by using TaskList view
    model.current_view = ViewId::TaskList;
    model.refresh_visible_tasks();

    // Now get grouped (the function doesn't filter, just groups visible tasks)
    let grouped = model.get_tasks_grouped_by_project();

    // Should have 2 groups: My Project first, No Project last
    assert_eq!(grouped.len(), 2);
    assert_eq!(grouped[0].1, "My Project");
    assert_eq!(grouped[1].1, "No Project");
}

#[test]
fn test_get_tasks_grouped_empty() {
    let mut model = Model::new();
    model.current_view = ViewId::Projects;
    model.refresh_visible_tasks();

    let grouped = model.get_tasks_grouped_by_project();

    // No tasks, no groups
    assert!(grouped.is_empty());
}

#[test]
fn test_get_tasks_grouped_preserves_task_order_within_group() {
    use crate::domain::{Project, SortField, SortOrder};

    let mut model = Model::new();
    model.current_view = ViewId::Projects;

    // Sort by title ascending
    model.sort.field = SortField::Title;
    model.sort.order = SortOrder::Ascending;

    let project = Project::new("Test Project");
    let project_id = project.id.clone();
    model.projects.insert(project_id.clone(), project);

    // Create tasks with different titles (will be sorted alphabetically)
    let task_c = Task::new("Charlie").with_project(project_id.clone());
    let task_a = Task::new("Alpha").with_project(project_id.clone());
    let task_b = Task::new("Bravo").with_project(project_id.clone());

    let task_a_id = task_a.id.clone();
    let task_b_id = task_b.id.clone();
    let task_c_id = task_c.id.clone();

    model.tasks.insert(task_c.id.clone(), task_c);
    model.tasks.insert(task_a.id.clone(), task_a);
    model.tasks.insert(task_b.id.clone(), task_b);

    model.refresh_visible_tasks();

    let grouped = model.get_tasks_grouped_by_project();

    assert_eq!(grouped.len(), 1);
    let task_ids = &grouped[0].2;
    assert_eq!(task_ids.len(), 3);

    // Tasks should be in order based on visible_tasks order (sorted by title)
    // Alpha, Bravo, Charlie
    assert_eq!(task_ids[0], task_a_id);
    assert_eq!(task_ids[1], task_b_id);
    assert_eq!(task_ids[2], task_c_id);
}

#[test]
fn test_view_blocked_shows_tasks_with_unmet_dependencies() {
    let mut model = Model::new();
    model.current_view = ViewId::Blocked;

    // Task A is a prerequisite (incomplete)
    let task_a = Task::new("Prerequisite task");
    let task_a_id = task_a.id.clone();

    // Task B depends on task A (blocked because A is not done)
    let mut task_b = Task::new("Blocked task");
    task_b.dependencies.push(task_a_id.clone());

    // Task C has no dependencies
    let task_c = Task::new("Independent task");

    let task_b_id = task_b.id.clone();
    model.tasks.insert(task_a.id.clone(), task_a);
    model.tasks.insert(task_b.id.clone(), task_b);
    model.tasks.insert(task_c.id.clone(), task_c);

    model.refresh_visible_tasks();

    // Only task B should be visible (blocked because task A is not done)
    assert_eq!(model.visible_tasks.len(), 1);
    assert_eq!(model.visible_tasks[0], task_b_id);
}

#[test]
fn test_view_blocked_excludes_tasks_with_completed_dependencies() {
    let mut model = Model::new();
    model.current_view = ViewId::Blocked;

    // Task A is a completed prerequisite
    let task_a = Task::new("Done prerequisite").with_status(TaskStatus::Done);
    let task_a_id = task_a.id.clone();

    // Task B depends on task A (NOT blocked because A is done)
    let mut task_b = Task::new("Unblocked task");
    task_b.dependencies.push(task_a_id.clone());

    model.tasks.insert(task_a.id.clone(), task_a);
    model.tasks.insert(task_b.id.clone(), task_b);

    model.show_completed = true; // Include completed tasks
    model.refresh_visible_tasks();

    // Task B should NOT be visible in Blocked view since its dependency is complete
    assert!(!model.visible_tasks.iter().any(|id| {
        model
            .tasks
            .get(id)
            .is_some_and(|t| t.title == "Unblocked task")
    }));
}

#[test]
fn test_view_untagged_shows_tasks_without_tags() {
    let mut model = Model::new();
    model.current_view = ViewId::Untagged;

    let task_with_tags = Task::new("Has tags").with_tags(vec!["work".to_string()]);
    let task_no_tags = Task::new("No tags");

    model
        .tasks
        .insert(task_with_tags.id.clone(), task_with_tags);
    model
        .tasks
        .insert(task_no_tags.id.clone(), task_no_tags.clone());

    model.refresh_visible_tasks();

    // Only the task without tags should be visible
    assert_eq!(model.visible_tasks.len(), 1);
    assert_eq!(model.visible_tasks[0], task_no_tags.id);
}

#[test]
fn test_view_no_project_shows_tasks_without_project() {
    let mut model = Model::new();
    model.current_view = ViewId::NoProject;

    let project_id = crate::domain::ProjectId::new();
    let task_with_project = Task::new("Has project").with_project(project_id);
    let task_no_project = Task::new("No project");

    model
        .tasks
        .insert(task_with_project.id.clone(), task_with_project);
    model
        .tasks
        .insert(task_no_project.id.clone(), task_no_project.clone());

    model.refresh_visible_tasks();

    // Only the task without a project should be visible
    assert_eq!(model.visible_tasks.len(), 1);
    assert_eq!(model.visible_tasks[0], task_no_project.id);
}

#[test]
fn test_view_recently_modified_filters_by_date() {
    let mut model = Model::new();
    model.current_view = ViewId::RecentlyModified;

    // Create a task modified now (recent)
    let recent_task = Task::new("Recent task");

    // Create a task and modify its updated_at to be old
    let mut old_task = Task::new("Old task");
    old_task.updated_at = chrono::Utc::now() - chrono::Duration::days(14);

    model
        .tasks
        .insert(recent_task.id.clone(), recent_task.clone());
    model.tasks.insert(old_task.id.clone(), old_task);

    model.refresh_visible_tasks();

    // Only the recent task should be visible (modified within 7 days)
    assert_eq!(model.visible_tasks.len(), 1);
    assert_eq!(model.visible_tasks[0], recent_task.id);
}

// ==================== Hierarchy Helper Method Tests ====================

#[test]
fn test_task_depth_root_task() {
    let mut model = Model::new();
    let task = Task::new("Root");
    model.tasks.insert(task.id.clone(), task.clone());
    assert_eq!(model.task_depth(&task.id), 0);
}

#[test]
fn test_task_depth_nested() {
    let mut model = Model::new();
    let root = Task::new("Root");
    let child = Task::new("Child").with_parent(root.id.clone());
    let grandchild = Task::new("Grandchild").with_parent(child.id.clone());

    model.tasks.insert(root.id.clone(), root.clone());
    model.tasks.insert(child.id.clone(), child.clone());
    model
        .tasks
        .insert(grandchild.id.clone(), grandchild.clone());

    assert_eq!(model.task_depth(&root.id), 0);
    assert_eq!(model.task_depth(&child.id), 1);
    assert_eq!(model.task_depth(&grandchild.id), 2);
}

#[test]
fn test_task_depth_missing_parent() {
    let mut model = Model::new();
    // Create a task with a parent_task_id that doesn't exist
    let orphan_parent_id = TaskId::new();
    let orphan = Task::new("Orphan").with_parent(orphan_parent_id);
    model.tasks.insert(orphan.id.clone(), orphan.clone());

    // Returns 1 because the function counts parent hops: orphan → missing parent (1 hop).
    // Note that orphaned tasks will display indented even though their parent doesn't exist.
    assert_eq!(model.task_depth(&orphan.id), 1);
}

#[test]
fn test_get_all_descendants_empty() {
    let mut model = Model::new();
    let task = Task::new("Standalone");
    model.tasks.insert(task.id.clone(), task.clone());

    let descendants = model.get_all_descendants(&task.id);
    assert!(descendants.is_empty());
}

#[test]
fn test_get_all_descendants_nested() {
    let mut model = Model::new();
    let root = Task::new("Root");
    let child1 = Task::new("Child1").with_parent(root.id.clone());
    let child2 = Task::new("Child2").with_parent(root.id.clone());
    let grandchild = Task::new("Grandchild").with_parent(child1.id.clone());

    model.tasks.insert(root.id.clone(), root.clone());
    model.tasks.insert(child1.id.clone(), child1.clone());
    model.tasks.insert(child2.id.clone(), child2.clone());
    model
        .tasks
        .insert(grandchild.id.clone(), grandchild.clone());

    let descendants = model.get_all_descendants(&root.id);
    assert_eq!(descendants.len(), 3);
    assert!(descendants.contains(&child1.id));
    assert!(descendants.contains(&child2.id));
    assert!(descendants.contains(&grandchild.id));
}

#[test]
fn test_get_all_ancestors_empty() {
    let mut model = Model::new();
    let task = Task::new("Root");
    model.tasks.insert(task.id.clone(), task.clone());

    let ancestors = model.get_all_ancestors(&task.id);
    assert!(ancestors.is_empty());
}

#[test]
fn test_get_all_ancestors_nested() {
    let mut model = Model::new();
    let root = Task::new("Root");
    let child = Task::new("Child").with_parent(root.id.clone());
    let grandchild = Task::new("Grandchild").with_parent(child.id.clone());

    model.tasks.insert(root.id.clone(), root.clone());
    model.tasks.insert(child.id.clone(), child.clone());
    model
        .tasks
        .insert(grandchild.id.clone(), grandchild.clone());

    let ancestors = model.get_all_ancestors(&grandchild.id);
    assert_eq!(ancestors.len(), 2);
    assert_eq!(ancestors[0], child.id); // Direct parent first
    assert_eq!(ancestors[1], root.id); // Then grandparent
}

#[test]
fn test_would_create_cycle_self_reference() {
    let mut model = Model::new();
    let task = Task::new("Task");
    model.tasks.insert(task.id.clone(), task.clone());

    assert!(model.would_create_cycle(&task.id, &task.id));
}

#[test]
fn test_would_create_cycle_descendant() {
    let mut model = Model::new();
    let root = Task::new("Root");
    let child = Task::new("Child").with_parent(root.id.clone());
    let grandchild = Task::new("Grandchild").with_parent(child.id.clone());

    model.tasks.insert(root.id.clone(), root.clone());
    model.tasks.insert(child.id.clone(), child.clone());
    model
        .tasks
        .insert(grandchild.id.clone(), grandchild.clone());

    // Setting root's parent to grandchild would create a cycle
    assert!(model.would_create_cycle(&root.id, &grandchild.id));
    assert!(model.would_create_cycle(&root.id, &child.id));

    // Setting grandchild's parent to a new task is fine
    let new_task = Task::new("New");
    model.tasks.insert(new_task.id.clone(), new_task.clone());
    assert!(!model.would_create_cycle(&grandchild.id, &new_task.id));
}

#[test]
fn test_has_subtasks() {
    let mut model = Model::new();
    let parent = Task::new("Parent");
    let child = Task::new("Child").with_parent(parent.id.clone());
    let standalone = Task::new("Standalone");

    model.tasks.insert(parent.id.clone(), parent.clone());
    model.tasks.insert(child.id.clone(), child);
    model
        .tasks
        .insert(standalone.id.clone(), standalone.clone());

    assert!(model.has_subtasks(&parent.id));
    assert!(!model.has_subtasks(&standalone.id));
}

#[test]
fn test_subtask_progress_recursive() {
    let mut model = Model::new();
    let root = Task::new("Root");
    let child1 = Task::new("Child1")
        .with_parent(root.id.clone())
        .with_status(TaskStatus::Done);
    let child2 = Task::new("Child2").with_parent(root.id.clone());
    let grandchild = Task::new("Grandchild")
        .with_parent(child2.id.clone())
        .with_status(TaskStatus::Done);

    model.tasks.insert(root.id.clone(), root.clone());
    model.tasks.insert(child1.id.clone(), child1);
    model.tasks.insert(child2.id.clone(), child2);
    model.tasks.insert(grandchild.id.clone(), grandchild);

    let (completed, total) = model.subtask_progress(&root.id);
    assert_eq!(total, 3); // child1, child2, grandchild
    assert_eq!(completed, 2); // child1, grandchild
}

#[test]
fn test_subtask_percentage() {
    let mut model = Model::new();
    let root = Task::new("Root");
    let child1 = Task::new("Child1")
        .with_parent(root.id.clone())
        .with_status(TaskStatus::Done);
    let child2 = Task::new("Child2").with_parent(root.id.clone());

    model.tasks.insert(root.id.clone(), root.clone());
    model.tasks.insert(child1.id.clone(), child1);
    model.tasks.insert(child2.id.clone(), child2);

    // 1 of 2 completed = 50%
    assert_eq!(model.subtask_percentage(&root.id), Some(50));
}

#[test]
fn test_subtask_percentage_no_subtasks() {
    let mut model = Model::new();
    let task = Task::new("Standalone");
    model.tasks.insert(task.id.clone(), task.clone());

    assert_eq!(model.subtask_percentage(&task.id), None);
}

#[test]
fn test_refresh_visible_tasks_deep_nesting_order() {
    // Test that visible_tasks orders: Root -> Child -> Grandchild -> Root2
    let mut model = Model::new();

    let root1 = Task::new("Root1");
    let child1 = Task::new("Child1").with_parent(root1.id.clone());
    let grandchild = Task::new("Grandchild").with_parent(child1.id.clone());
    let root2 = Task::new("Root2");

    let root1_id = root1.id.clone();
    let child1_id = child1.id.clone();
    let grandchild_id = grandchild.id.clone();
    let root2_id = root2.id.clone();

    // Insert in random order
    model.tasks.insert(grandchild.id.clone(), grandchild);
    model.tasks.insert(root2.id.clone(), root2);
    model.tasks.insert(child1.id.clone(), child1);
    model.tasks.insert(root1.id.clone(), root1);

    model.refresh_visible_tasks();

    // Check ordering: should be Root1 -> Child1 -> Grandchild -> Root2
    // (roots sorted by created_at, subtasks inserted after their parents)
    let root1_pos = model
        .visible_tasks
        .iter()
        .position(|id| id == &root1_id)
        .unwrap();
    let child1_pos = model
        .visible_tasks
        .iter()
        .position(|id| id == &child1_id)
        .unwrap();
    let grandchild_pos = model
        .visible_tasks
        .iter()
        .position(|id| id == &grandchild_id)
        .unwrap();
    let root2_pos = model
        .visible_tasks
        .iter()
        .position(|id| id == &root2_id)
        .unwrap();

    // Child1 should come after Root1
    assert!(child1_pos > root1_pos, "Child1 should appear after Root1");

    // Grandchild should come after Child1
    assert!(
        grandchild_pos > child1_pos,
        "Grandchild should appear after Child1"
    );

    // Grandchild should come before Root2 (if Root2 comes after Root1)
    // This ensures the hierarchy is kept together
    if root2_pos > root1_pos {
        assert!(
            grandchild_pos < root2_pos,
            "Grandchild should appear before Root2"
        );
    }
}

#[test]
fn test_model_refresh_storage_without_backend() {
    let mut model = Model::new();
    // No storage backend attached
    assert!(model.storage.is_none());

    // refresh_storage should return 0 when no backend
    let changes = model.refresh_storage();
    assert_eq!(changes, 0);
}

#[test]
fn test_model_refresh_storage_with_markdown_backend() {
    use crate::storage::{backends::MarkdownBackend, BackendType, StorageBackend, TaskRepository};
    use std::fs;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();

    // First, create a task directly in the markdown directory structure
    let mut backend = MarkdownBackend::new(dir.path()).unwrap();
    backend.initialize().unwrap();
    let task = Task::new("Original task");
    backend.create_task(&task).unwrap();
    backend.flush().unwrap();
    drop(backend); // Close the backend

    // Create model using the proper API
    let mut model = Model::new()
        .with_storage(BackendType::Markdown, dir.path().to_path_buf())
        .unwrap();
    assert_eq!(model.tasks.len(), 1);
    assert_eq!(model.tasks.get(&task.id).unwrap().title, "Original task");

    // Externally modify the file
    let file_path = dir.path().join("tasks").join(format!("{}.md", task.id.0));
    let content = fs::read_to_string(&file_path).unwrap();
    let modified = content.replace("Original task", "Externally modified");
    std::thread::sleep(std::time::Duration::from_millis(10));
    fs::write(&file_path, modified).unwrap();

    // Refresh should detect the change
    let changes = model.refresh_storage();
    assert!(changes > 0, "Should detect external modification");

    // Model should have updated task
    assert_eq!(
        model.tasks.get(&task.id).unwrap().title,
        "Externally modified"
    );
}
