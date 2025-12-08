//! Filtering and view tests.

use crate::app::model::{Model, SortSpec, ViewId};
use crate::domain::{Priority, SortField, SortOrder, TagFilterMode, Task, TaskStatus};

#[test]
fn test_model_refresh_visible_tasks_sorts_by_priority() {
    let mut model = Model::new();

    // Add tasks with different priorities
    let urgent = Task::new("Urgent").with_priority(Priority::Urgent);
    let low = Task::new("Low").with_priority(Priority::Low);
    let high = Task::new("High").with_priority(Priority::High);

    model.tasks.insert(low.id, low.clone());
    model.tasks.insert(urgent.id, urgent.clone());
    model.tasks.insert(high.id, high.clone());

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

    model.tasks.insert(todo.id, todo);
    model.tasks.insert(done.id, done);
    model.tasks.insert(cancelled.id, cancelled);

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

    model.tasks.insert(todo.id, todo);
    model.tasks.insert(done.id, done);
    model.tasks.insert(cancelled.id, cancelled);

    model.refresh_visible_tasks();

    // All tasks should be visible
    assert_eq!(model.visible_tasks.len(), 3);
}

#[test]
fn test_model_refresh_visible_tasks_subtasks_follow_parent() {
    let mut model = Model::new();

    // Create a parent task and two subtasks
    let parent1 = Task::new("Parent 1").with_priority(Priority::High);
    let subtask1a = Task::new("Subtask 1a").with_parent(parent1.id);
    let subtask1b = Task::new("Subtask 1b").with_parent(parent1.id);

    let parent2 = Task::new("Parent 2").with_priority(Priority::Low);
    let subtask2a = Task::new("Subtask 2a").with_parent(parent2.id);

    // Insert in random order
    model.tasks.insert(subtask1b.id, subtask1b.clone());
    model.tasks.insert(parent2.id, parent2.clone());
    model.tasks.insert(subtask2a.id, subtask2a.clone());
    model.tasks.insert(parent1.id, parent1.clone());
    model.tasks.insert(subtask1a.id, subtask1a.clone());

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
fn test_view_tasklist_shows_all() {
    let mut model = Model::new();
    model.current_view = ViewId::TaskList;

    // Create tasks with various due dates and project associations
    let task_no_date = Task::new("No due date");
    let task_with_date =
        Task::new("Has date").with_due_date(chrono::NaiveDate::from_ymd_opt(2025, 12, 15).unwrap());
    let task_with_project = Task::new("Has project").with_project(crate::domain::ProjectId::new());

    model.tasks.insert(task_no_date.id, task_no_date);
    model.tasks.insert(task_with_date.id, task_with_date);
    model.tasks.insert(task_with_project.id, task_with_project);

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

    model.tasks.insert(task_today.id, task_today.clone());
    model.tasks.insert(task_tomorrow.id, task_tomorrow);
    model.tasks.insert(task_no_date.id, task_no_date);

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

    model.tasks.insert(task_today.id, task_today);
    model.tasks.insert(task_tomorrow.id, task_tomorrow.clone());
    model
        .tasks
        .insert(task_next_week.id, task_next_week.clone());
    model.tasks.insert(task_no_date.id, task_no_date);

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
        .insert(task_with_project.id, task_with_project.clone());
    model.tasks.insert(task_no_project.id, task_no_project);

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
        .insert(task_yesterday.id, task_yesterday.clone());
    model
        .tasks
        .insert(task_last_week.id, task_last_week.clone());
    model.tasks.insert(task_today.id, task_today);
    model.tasks.insert(task_tomorrow.id, task_tomorrow);
    model.tasks.insert(task_no_date.id, task_no_date);

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

    model.tasks.insert(task_today.id, task_today);

    model.refresh_visible_tasks();

    // Today's tasks are not overdue
    assert!(model.visible_tasks.is_empty());
}

#[test]
fn test_view_overdue_excludes_no_due_date() {
    let mut model = Model::new();
    model.current_view = ViewId::Overdue;

    let task_no_date = Task::new("No due date");
    model.tasks.insert(task_no_date.id, task_no_date);

    model.refresh_visible_tasks();

    // Tasks without due dates are not overdue
    assert!(model.visible_tasks.is_empty());
}

#[test]
fn test_search_filter_matches_title() {
    let mut model = Model::new();

    let task_match = Task::new("Build the feature");
    let task_no_match = Task::new("Fix the bug");

    model.tasks.insert(task_match.id, task_match.clone());
    model.tasks.insert(task_no_match.id, task_no_match);

    model.filter.search_text = Some("build".to_string());
    model.refresh_visible_tasks();

    assert_eq!(model.visible_tasks.len(), 1);
    assert_eq!(model.visible_tasks[0], task_match.id);
}

#[test]
fn test_search_filter_case_insensitive() {
    let mut model = Model::new();

    let task = Task::new("Build Feature");
    model.tasks.insert(task.id, task);

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

    model.tasks.insert(task_with_tag.id, task_with_tag.clone());
    model.tasks.insert(task_no_tag.id, task_no_tag);

    model.filter.search_text = Some("urgent".to_string());
    model.refresh_visible_tasks();

    assert_eq!(model.visible_tasks.len(), 1);
    assert_eq!(model.visible_tasks[0], task_with_tag.id);
}

#[test]
fn test_search_filter_partial_match() {
    let mut model = Model::new();

    let task = Task::new("Implement authentication");
    model.tasks.insert(task.id, task);

    model.filter.search_text = Some("auth".to_string());
    model.refresh_visible_tasks();

    assert_eq!(model.visible_tasks.len(), 1);
}

#[test]
fn test_search_filter_empty_clears() {
    let mut model = Model::new();

    let task1 = Task::new("Task one");
    let task2 = Task::new("Task two");

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);

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
    let mut model = Model::new();

    let task_rust = Task::new("Task Rust").with_tags(vec!["rust".to_string()]);
    let task_python = Task::new("Task Python").with_tags(vec!["python".to_string()]);
    let task_both =
        Task::new("Task Both").with_tags(vec!["rust".to_string(), "python".to_string()]);
    let task_none = Task::new("Task None");

    model.tasks.insert(task_rust.id, task_rust.clone());
    model.tasks.insert(task_python.id, task_python);
    model.tasks.insert(task_both.id, task_both.clone());
    model.tasks.insert(task_none.id, task_none);

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
    let mut model = Model::new();

    let task_rust = Task::new("Task Rust").with_tags(vec!["rust".to_string()]);
    let task_both =
        Task::new("Task Both").with_tags(vec!["rust".to_string(), "python".to_string()]);
    let task_none = Task::new("Task None");

    model.tasks.insert(task_rust.id, task_rust);
    model.tasks.insert(task_both.id, task_both.clone());
    model.tasks.insert(task_none.id, task_none);

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
    model.tasks.insert(task.id, task.clone());

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

    model.tasks.insert(task_tagged.id, task_tagged);
    model.tasks.insert(task_untagged.id, task_untagged);

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

    model.tasks.insert(task_match.id, task_match.clone());
    model.tasks.insert(task_wrong_tag.id, task_wrong_tag);
    model.tasks.insert(task_wrong_title.id, task_wrong_title);

    // Both search and tag filter
    model.filter.search_text = Some("Important".to_string());
    model.filter.tags = Some(vec!["work".to_string()]);
    model.refresh_visible_tasks();

    // Only task_match matches both criteria
    assert_eq!(model.visible_tasks.len(), 1);
    assert!(model.visible_tasks.contains(&task_match.id));
}

#[test]
fn test_view_blocked_shows_tasks_with_unmet_dependencies() {
    let mut model = Model::new();
    model.current_view = ViewId::Blocked;

    // Task A is a prerequisite (incomplete)
    let task_a = Task::new("Prerequisite task");
    let task_a_id = task_a.id;

    // Task B depends on task A (blocked because A is not done)
    let mut task_b = Task::new("Blocked task");
    task_b.dependencies.push(task_a_id);

    // Task C has no dependencies
    let task_c = Task::new("Independent task");

    let task_b_id = task_b.id;
    model.tasks.insert(task_a.id, task_a);
    model.tasks.insert(task_b.id, task_b);
    model.tasks.insert(task_c.id, task_c);

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
    let task_a_id = task_a.id;

    // Task B depends on task A (NOT blocked because A is done)
    let mut task_b = Task::new("Unblocked task");
    task_b.dependencies.push(task_a_id);

    model.tasks.insert(task_a.id, task_a);
    model.tasks.insert(task_b.id, task_b);

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

    model.tasks.insert(task_with_tags.id, task_with_tags);
    model.tasks.insert(task_no_tags.id, task_no_tags.clone());

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

    model.tasks.insert(task_with_project.id, task_with_project);
    model
        .tasks
        .insert(task_no_project.id, task_no_project.clone());

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

    model.tasks.insert(recent_task.id, recent_task.clone());
    model.tasks.insert(old_task.id, old_task);

    model.refresh_visible_tasks();

    // Only the recent task should be visible (modified within 7 days)
    assert_eq!(model.visible_tasks.len(), 1);
    assert_eq!(model.visible_tasks[0], recent_task.id);
}
