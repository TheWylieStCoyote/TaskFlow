//! Tests for Model query methods (tasks_for_day, contexts, sidebar, etc.)

use crate::app::model::Model;
use crate::domain::{Habit, Task};
use chrono::{NaiveDate, Utc};

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

// ── tasks_for_export ──────────────────────────────────────────────────────────

#[test]
fn test_tasks_for_export_empty() {
    let model = Model::new();
    assert!(model.tasks_for_export().is_empty());
}

#[test]
fn test_tasks_for_export_includes_all_tasks() {
    let mut model = Model::new();
    model.tasks.insert(Task::new("A").id, Task::new("A"));
    model.tasks.insert(Task::new("B").id, Task::new("B"));
    assert_eq!(model.tasks_for_export().len(), 2);
}

// ── all_contexts ──────────────────────────────────────────────────────────────

#[test]
fn test_all_contexts_empty() {
    let model = Model::new();
    assert!(model.all_contexts().is_empty());
}

#[test]
fn test_all_contexts_from_task_tags() {
    let mut model = Model::new();
    let mut task = Task::new("Task");
    task.tags = vec!["@home".into(), "@work".into()];
    model.tasks.insert(task.id, task);
    model.rebuild_caches();

    let contexts = model.all_contexts();
    assert!(contexts.contains(&"@home".to_string()));
    assert!(contexts.contains(&"@work".to_string()));
}

#[test]
fn test_all_contexts_only_at_prefixed() {
    let mut model = Model::new();
    let mut task = Task::new("Task");
    task.tags = vec!["@home".into(), "work".into(), "@office".into()];
    model.tasks.insert(task.id, task);
    model.rebuild_caches();

    let contexts = model.all_contexts();
    // "work" without @ is not a context
    assert!(contexts.iter().all(|c| c.starts_with('@')));
}

#[test]
fn test_all_contexts_sorted() {
    let mut model = Model::new();
    let mut task = Task::new("Task");
    task.tags = vec!["@zeta".into(), "@alpha".into(), "@beta".into()];
    model.tasks.insert(task.id, task);
    model.rebuild_caches();

    let contexts = model.all_contexts();
    let mut sorted = contexts.clone();
    sorted.sort();
    assert_eq!(contexts, sorted);
}

// ── sidebar_item_count ────────────────────────────────────────────────────────

#[test]
fn test_sidebar_item_count_empty_model() {
    let model = Model::new();
    // Should not panic, result is deterministic
    let count = model.sidebar_item_count();
    assert!(count > 0);
}

#[test]
fn test_sidebar_item_count_increases_with_more_projects() {
    use crate::domain::Project;
    let mut model = Model::new();
    // .max(1) means the first project doesn't change the count, but 2+ do
    model
        .projects
        .insert(Project::new("P1").id, Project::new("P1"));
    let count_one = model.sidebar_item_count();

    model
        .projects
        .insert(Project::new("P2").id, Project::new("P2"));
    let count_two = model.sidebar_item_count();

    assert!(count_two > count_one);
}

// ── tasks_for_day ─────────────────────────────────────────────────────────────

#[test]
fn test_tasks_for_day_empty() {
    let model = Model::new();
    let tasks = model.tasks_for_day(date(2024, 6, 1));
    assert!(tasks.is_empty());
}

#[test]
fn test_tasks_for_day_with_due_task() {
    let mut model = Model::new();
    let mut task = Task::new("Due task");
    let d = date(2024, 6, 15);
    task.due_date = Some(d);
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.rebuild_caches();

    let tasks = model.tasks_for_day(d);
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, task_id);
}

#[test]
fn test_tasks_for_day_no_match() {
    let mut model = Model::new();
    let mut task = Task::new("Due task");
    task.due_date = Some(date(2024, 6, 15));
    model.tasks.insert(task.id, task);
    model.rebuild_caches();

    // A different day — no tasks
    let tasks = model.tasks_for_day(date(2024, 6, 16));
    assert!(tasks.is_empty());
}

// ── task_count_for_day ────────────────────────────────────────────────────────

#[test]
fn test_task_count_for_day_empty() {
    let model = Model::new();
    assert_eq!(model.task_count_for_day(date(2024, 1, 1)), 0);
}

#[test]
fn test_task_count_for_day_counts_incomplete() {
    let mut model = Model::new();
    let mut task = Task::new("Task");
    task.due_date = Some(date(2024, 3, 10));
    model.tasks.insert(task.id, task);
    model.rebuild_caches();

    assert_eq!(model.task_count_for_day(date(2024, 3, 10)), 1);
}

#[test]
fn test_task_count_for_day_excludes_completed_when_hidden() {
    use crate::domain::TaskStatus;

    let mut model = Model::new();
    let mut task = Task::new("Done task");
    task.due_date = Some(date(2024, 3, 10));
    task.status = TaskStatus::Done;
    model.tasks.insert(task.id, task);
    model.filtering.show_completed = false;

    // Completed task should not be counted when show_completed is false
    assert_eq!(model.task_count_for_day(date(2024, 3, 10)), 0);
}

// ── has_overdue_on_day ────────────────────────────────────────────────────────

#[test]
fn test_has_overdue_on_day_no_tasks() {
    let model = Model::new();
    let past = date(2020, 1, 1);
    assert!(!model.has_overdue_on_day(past));
}

#[test]
fn test_has_overdue_on_day_future_day_not_overdue() {
    let model = Model::new();
    // Future dates can't be overdue
    let future = Utc::now().date_naive() + chrono::TimeDelta::days(10);
    assert!(!model.has_overdue_on_day(future));
}

#[test]
fn test_has_overdue_on_day_past_with_incomplete_task() {
    let mut model = Model::new();
    let past = date(2020, 6, 1); // definitely in the past
    let mut task = Task::new("Old task");
    task.due_date = Some(past);
    model.tasks.insert(task.id, task);

    assert!(model.has_overdue_on_day(past));
}

// ── work_logs_for_task / work_log_count_for_task ──────────────────────────────

#[test]
fn test_work_logs_for_task_empty() {
    let model = Model::new();
    let task = Task::new("Task");
    assert!(model.work_logs_for_task(&task.id).is_empty());
}

#[test]
fn test_work_log_count_for_task_empty() {
    let model = Model::new();
    let task = Task::new("Task");
    assert_eq!(model.work_log_count_for_task(&task.id), 0);
}

// ── refresh_visible_habits ────────────────────────────────────────────────────

#[test]
fn test_refresh_visible_habits_empty() {
    let mut model = Model::new();
    model.refresh_visible_habits();
    assert!(model.visible_habits.is_empty());
    assert!(model.selected_habit().is_none());
}

#[test]
fn test_refresh_visible_habits_shows_active() {
    let mut model = Model::new();
    let habit = Habit::new("Exercise");
    let habit_id = habit.id;
    model.habits.insert(habit_id, habit);
    model.refresh_visible_habits();

    assert_eq!(model.visible_habits.len(), 1);
    assert!(model.selected_habit().is_some());
}

#[test]
fn test_refresh_visible_habits_hides_archived() {
    let mut model = Model::new();
    let mut habit = Habit::new("Old habit");
    habit.archived = true;
    model.habits.insert(habit.id, habit);
    model.habit_view.show_archived = false;
    model.refresh_visible_habits();

    assert!(model.visible_habits.is_empty());
}

#[test]
fn test_refresh_visible_habits_shows_archived_when_flag_set() {
    let mut model = Model::new();
    let mut habit = Habit::new("Old habit");
    habit.archived = true;
    model.habits.insert(habit.id, habit);
    model.habit_view.show_archived = true;
    model.refresh_visible_habits();

    assert_eq!(model.visible_habits.len(), 1);
}

#[test]
fn test_refresh_visible_habits_sorted_by_name() {
    let mut model = Model::new();
    model
        .habits
        .insert(Habit::new("Yoga").id, Habit::new("Yoga"));
    model
        .habits
        .insert(Habit::new("Exercise").id, Habit::new("Exercise"));
    model
        .habits
        .insert(Habit::new("Meditate").id, Habit::new("Meditate"));
    model.refresh_visible_habits();

    let names: Vec<_> = model
        .visible_habits
        .iter()
        .filter_map(|id| model.habits.get(id))
        .map(|h| h.name.as_str())
        .collect();

    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
}

// ── tasks_for_selected_day ────────────────────────────────────────────────────

#[test]
fn test_tasks_for_selected_day_no_day_selected() {
    let mut model = Model::new();
    model.calendar_state.selected_day = None;
    assert!(model.tasks_for_selected_day().is_empty());
}

#[test]
fn test_tasks_for_selected_day_with_day() {
    let mut model = Model::new();
    // Set up a task due on 2024-06-15
    let mut task = Task::new("Calendar task");
    task.due_date = Some(date(2024, 6, 15));
    model.tasks.insert(task.id, task);
    model.rebuild_caches();

    // Select day 15 in June 2024
    model.calendar_state.year = 2024;
    model.calendar_state.month = 6;
    model.calendar_state.selected_day = Some(15);

    let tasks = model.tasks_for_selected_day();
    assert_eq!(tasks.len(), 1);
}

// ── habits_for_export ─────────────────────────────────────────────────────────

#[test]
fn test_habits_for_export_empty() {
    let model = Model::new();
    assert!(model.habits_for_export().is_empty());
}

#[test]
fn test_habits_for_export_includes_all() {
    let mut model = Model::new();
    model.habits.insert(Habit::new("H1").id, Habit::new("H1"));
    model.habits.insert(Habit::new("H2").id, Habit::new("H2"));
    assert_eq!(model.habits_for_export().len(), 2);
}
