//! Tests for filtering module.

use chrono::{Datelike, Duration, Utc};

use crate::app::Model;
use crate::domain::{Priority, Project, SortField, SortOrder, Task, TaskStatus};

// ========================================================================
// View-Specific Task Filtering Tests
// ========================================================================

#[test]
fn test_kanban_column_tasks() {
    let mut model = Model::new();

    let todo = Task::new("Todo task").with_status(TaskStatus::Todo);
    let in_progress = Task::new("In progress").with_status(TaskStatus::InProgress);
    let blocked = Task::new("Blocked").with_status(TaskStatus::Blocked);
    let done = Task::new("Done").with_status(TaskStatus::Done);

    model.tasks.insert(todo.id, todo.clone());
    model.tasks.insert(in_progress.id, in_progress.clone());
    model.tasks.insert(blocked.id, blocked.clone());
    model.tasks.insert(done.id, done.clone());
    model.visible_tasks = vec![todo.id, in_progress.id, blocked.id, done.id];

    assert_eq!(model.kanban_column_tasks(0).len(), 1); // Todo
    assert_eq!(model.kanban_column_tasks(1).len(), 1); // InProgress
    assert_eq!(model.kanban_column_tasks(2).len(), 1); // Blocked
    assert_eq!(model.kanban_column_tasks(3).len(), 1); // Done
    assert!(model.kanban_column_tasks(4).is_empty()); // Invalid column
}

#[test]
fn test_eisenhower_quadrant_tasks() {
    let mut model = Model::new();
    let today = Utc::now().date_naive();

    // Q0: Urgent + Important (due within 2 days, high priority)
    let mut urgent_important = Task::new("Urgent Important");
    urgent_important.priority = Priority::High;
    urgent_important.due_date = Some(today + Duration::days(1));
    model
        .tasks
        .insert(urgent_important.id, urgent_important.clone());

    // Q1: Not Urgent + Important (due later, high priority)
    let mut not_urgent_important = Task::new("Not Urgent Important");
    not_urgent_important.priority = Priority::High;
    not_urgent_important.due_date = Some(today + Duration::days(10));
    model
        .tasks
        .insert(not_urgent_important.id, not_urgent_important.clone());

    // Q2: Urgent + Not Important (due soon, low priority)
    let mut urgent_not_important = Task::new("Urgent Not Important");
    urgent_not_important.priority = Priority::Low;
    urgent_not_important.due_date = Some(today);
    model
        .tasks
        .insert(urgent_not_important.id, urgent_not_important.clone());

    // Q3: Not Urgent + Not Important
    let mut not_urgent_not_important = Task::new("Not Urgent Not Important");
    not_urgent_not_important.priority = Priority::Low;
    not_urgent_not_important.due_date = Some(today + Duration::days(30));
    model.tasks.insert(
        not_urgent_not_important.id,
        not_urgent_not_important.clone(),
    );

    model.visible_tasks = vec![
        urgent_important.id,
        not_urgent_important.id,
        urgent_not_important.id,
        not_urgent_not_important.id,
    ];

    assert_eq!(model.eisenhower_quadrant_tasks(0).len(), 1); // Urgent + Important
    assert_eq!(model.eisenhower_quadrant_tasks(1).len(), 1); // Not Urgent + Important
    assert_eq!(model.eisenhower_quadrant_tasks(2).len(), 1); // Urgent + Not Important
    assert_eq!(model.eisenhower_quadrant_tasks(3).len(), 1); // Not Urgent + Not Important
    assert!(model.eisenhower_quadrant_tasks(5).is_empty()); // Invalid quadrant
}

#[test]
fn test_weekly_planner_day_tasks() {
    let mut model = Model::new();
    let today = Utc::now().date_naive();
    let days_since_monday = today.weekday().num_days_from_monday();
    let monday = today - Duration::days(i64::from(days_since_monday));

    // Task due on Monday (day 0)
    let mut monday_task = Task::new("Monday task");
    monday_task.due_date = Some(monday);
    model.tasks.insert(monday_task.id, monday_task.clone());

    // Task scheduled for Tuesday (day 1)
    let mut tuesday_task = Task::new("Tuesday task");
    tuesday_task.scheduled_date = Some(monday + Duration::days(1));
    model.tasks.insert(tuesday_task.id, tuesday_task.clone());

    model.visible_tasks = vec![monday_task.id, tuesday_task.id];

    assert_eq!(model.weekly_planner_day_tasks(0).len(), 1); // Monday
    assert_eq!(model.weekly_planner_day_tasks(1).len(), 1); // Tuesday
    assert!(model.weekly_planner_day_tasks(2).is_empty()); // Wednesday
    assert!(model.weekly_planner_day_tasks(7).is_empty()); // Invalid day
}

#[test]
fn test_network_tasks() {
    let mut model = Model::new();

    // Task with dependency
    let dep_target = Task::new("Dependency target");
    let mut dependent = Task::new("Dependent task");
    dependent.dependencies.push(dep_target.id);

    // Task in a chain
    let mut chain_start = Task::new("Chain start");
    let chain_end = Task::new("Chain end");
    chain_start.next_task_id = Some(chain_end.id);

    // Standalone task (should not be in network)
    let standalone = Task::new("Standalone");

    model.tasks.insert(dep_target.id, dep_target);
    model.tasks.insert(dependent.id, dependent);
    model.tasks.insert(chain_start.id, chain_start);
    model.tasks.insert(chain_end.id, chain_end);
    model.tasks.insert(standalone.id, standalone);

    let network_tasks = model.network_tasks();

    // Should include dep_target, dependent, chain_start, chain_end
    // but not standalone
    assert_eq!(network_tasks.len(), 4);
}

#[test]
fn test_get_tasks_grouped_by_project() {
    let mut model = Model::new();

    let project1 = Project::new("Alpha Project");
    let project2 = Project::new("Beta Project");
    model.projects.insert(project1.id, project1.clone());
    model.projects.insert(project2.id, project2.clone());

    let mut task1 = Task::new("Task in Alpha");
    task1.project_id = Some(project1.id);
    let mut task2 = Task::new("Task in Beta");
    task2.project_id = Some(project2.id);
    let task3 = Task::new("No project task");

    model.tasks.insert(task1.id, task1.clone());
    model.tasks.insert(task2.id, task2.clone());
    model.tasks.insert(task3.id, task3.clone());
    model.visible_tasks = vec![task1.id, task2.id, task3.id];

    let grouped = model.get_tasks_grouped_by_project();

    // Should have 3 groups: Alpha, Beta, No Project
    assert_eq!(grouped.len(), 3);

    // Alpha Project should come first (alphabetically)
    assert_eq!(grouped[0].1, "Alpha Project");

    // No Project should come last
    assert_eq!(grouped[2].1, "No Project");
}

#[test]
fn test_selected_task_id() {
    let mut model = Model::new();

    let task1 = Task::new("Task 1");
    let task2 = Task::new("Task 2");
    let task1_id = task1.id;
    let task2_id = task2.id;

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.visible_tasks = vec![task1_id, task2_id];

    model.selected_index = 0;
    assert_eq!(model.selected_task_id(), Some(task1_id));

    model.selected_index = 1;
    assert_eq!(model.selected_task_id(), Some(task2_id));

    model.selected_index = 10; // Out of bounds
    assert!(model.selected_task_id().is_none());
}

#[test]
fn test_selected_task() {
    let mut model = Model::new();

    let task = Task::new("Selected task");
    let task_id = task.id;
    model.tasks.insert(task.id, task);
    model.visible_tasks = vec![task_id];
    model.selected_index = 0;

    let selected = model.selected_task();
    assert!(selected.is_some());
    assert_eq!(selected.unwrap().title, "Selected task");
}

#[test]
fn test_selected_task_mut() {
    let mut model = Model::new();

    let task = Task::new("Mutable task");
    let task_id = task.id;
    model.tasks.insert(task.id, task);
    model.visible_tasks = vec![task_id];
    model.selected_index = 0;

    if let Some(selected) = model.selected_task_mut() {
        selected.title = "Modified title".to_string();
    }

    assert_eq!(model.tasks.get(&task_id).unwrap().title, "Modified title");
}

#[test]
fn test_refresh_visible_tasks_with_sort() {
    let mut model = Model::new();
    model.filtering.sort.field = SortField::Title;
    model.filtering.sort.order = SortOrder::Ascending;

    let task_c = Task::new("Charlie");
    let task_a = Task::new("Alpha");
    let task_b = Task::new("Bravo");

    model.tasks.insert(task_c.id, task_c.clone());
    model.tasks.insert(task_a.id, task_a.clone());
    model.tasks.insert(task_b.id, task_b.clone());

    model.refresh_visible_tasks();

    // Should be sorted alphabetically
    assert_eq!(model.visible_tasks.len(), 3);
    assert_eq!(
        model.tasks.get(&model.visible_tasks[0]).unwrap().title,
        "Alpha"
    );
    assert_eq!(
        model.tasks.get(&model.visible_tasks[1]).unwrap().title,
        "Bravo"
    );
    assert_eq!(
        model.tasks.get(&model.visible_tasks[2]).unwrap().title,
        "Charlie"
    );
}

#[test]
fn test_refresh_visible_tasks_with_subtasks() {
    let mut model = Model::new();

    let parent = Task::new("Parent");
    let parent_id = parent.id;
    let child1 = Task::new("Child 1").with_parent(parent_id);
    let child2 = Task::new("Child 2").with_parent(parent_id);

    model.tasks.insert(parent.id, parent);
    model.tasks.insert(child1.id, child1.clone());
    model.tasks.insert(child2.id, child2.clone());

    model.refresh_visible_tasks();

    // Parent should come before children
    let parent_idx = model
        .visible_tasks
        .iter()
        .position(|id| *id == parent_id)
        .unwrap();
    let child1_idx = model
        .visible_tasks
        .iter()
        .position(|id| *id == child1.id)
        .unwrap();
    let child2_idx = model
        .visible_tasks
        .iter()
        .position(|id| *id == child2.id)
        .unwrap();

    assert!(parent_idx < child1_idx);
    assert!(parent_idx < child2_idx);
}

#[test]
fn test_refresh_visible_tasks_hides_completed() {
    let mut model = Model::new();
    model.filtering.show_completed = false;

    let todo = Task::new("Todo");
    let done = Task::new("Done").with_status(TaskStatus::Done);

    model.tasks.insert(todo.id, todo.clone());
    model.tasks.insert(done.id, done);

    model.refresh_visible_tasks();

    // Should only show incomplete task
    assert_eq!(model.visible_tasks.len(), 1);
    assert_eq!(model.visible_tasks[0], todo.id);
}

#[test]
fn test_refresh_visible_tasks_shows_completed_when_enabled() {
    let mut model = Model::new();
    model.filtering.show_completed = true;

    let todo = Task::new("Todo");
    let done = Task::new("Done").with_status(TaskStatus::Done);

    model.tasks.insert(todo.id, todo);
    model.tasks.insert(done.id, done);

    model.refresh_visible_tasks();

    // Should show both tasks
    assert_eq!(model.visible_tasks.len(), 2);
}

#[test]
fn test_refresh_visible_tasks_by_priority_sort() {
    let mut model = Model::new();
    model.filtering.sort.field = SortField::Priority;
    model.filtering.sort.order = SortOrder::Ascending;

    let low = Task::new("Low priority").with_priority(Priority::Low);
    let urgent = Task::new("Urgent").with_priority(Priority::Urgent);
    let medium = Task::new("Medium").with_priority(Priority::Medium);

    model.tasks.insert(low.id, low.clone());
    model.tasks.insert(urgent.id, urgent.clone());
    model.tasks.insert(medium.id, medium.clone());

    model.refresh_visible_tasks();

    // Urgent should come first
    assert_eq!(
        model.tasks.get(&model.visible_tasks[0]).unwrap().priority,
        Priority::Urgent
    );
}

#[test]
fn test_refresh_visible_tasks_by_due_date_sort() {
    let mut model = Model::new();
    model.filtering.sort.field = SortField::DueDate;
    model.filtering.sort.order = SortOrder::Ascending;
    let today = Utc::now().date_naive();

    let mut soon = Task::new("Due soon");
    soon.due_date = Some(today + Duration::days(1));
    let mut later = Task::new("Due later");
    later.due_date = Some(today + Duration::days(10));
    let no_due = Task::new("No due date");

    model.tasks.insert(soon.id, soon.clone());
    model.tasks.insert(later.id, later.clone());
    model.tasks.insert(no_due.id, no_due);

    model.refresh_visible_tasks();

    // Soon should come first, no due date last
    assert_eq!(
        model.tasks.get(&model.visible_tasks[0]).unwrap().title,
        "Due soon"
    );
    assert_eq!(
        model.tasks.get(&model.visible_tasks[1]).unwrap().title,
        "Due later"
    );
    assert_eq!(
        model.tasks.get(&model.visible_tasks[2]).unwrap().title,
        "No due date"
    );
}
