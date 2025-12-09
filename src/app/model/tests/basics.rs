//! Basic Model tests (construction, defaults, selection).

use crate::app::model::{InputMode, Model, RunningState};
use crate::domain::Task;

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

    // Sample data creates ~88 tasks across 10 projects
    assert!(model.tasks.len() >= 80);
    assert_eq!(model.projects.len(), 10);
    // Some are completed, so visible should be less than total
    assert!(model.visible_tasks.len() < model.tasks.len());
}

#[test]
fn test_model_selected_task_returns_correct() {
    let mut model = Model::new();

    let task1 = Task::new("Task 1");
    let task2 = Task::new("Task 2");

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
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
        model.tasks.insert(task.id, task);
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
fn test_model_dirty_flag() {
    let mut model = Model::new();
    assert!(!model.dirty);

    let task = Task::new("Task");
    model.tasks.insert(task.id, task);

    model.start_time_tracking(model.tasks.keys().next().cloned().unwrap());
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
