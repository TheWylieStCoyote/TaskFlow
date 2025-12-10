//! Tests for network view component.

use super::*;
use crate::app::Model;
use crate::config::Theme;
use crate::domain::Task;
use ratatui::buffer::Buffer;

#[test]
fn test_network_empty_model() {
    let model = Model::new();
    let theme = Theme::default();
    let network = Network::new(&model, &theme, 0);
    assert!(network.get_root_tasks().is_empty());
}

#[test]
fn test_network_renders_without_panic() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let network = Network::new(&model, &theme, 0);

    let area = Rect::new(0, 0, 120, 30);
    let mut buffer = Buffer::empty(area);
    network.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_network_with_dependencies() {
    let mut model = Model::new();

    // Create parent task
    let parent = Task::new("Parent Task");
    let parent_id = parent.id;
    model.tasks.insert(parent_id, parent);

    // Create child task with dependency on parent
    let mut child = Task::new("Child Task");
    child.dependencies.push(parent_id);
    model.tasks.insert(child.id, child);

    model.refresh_visible_tasks();

    let theme = Theme::default();

    // Check roots first (before render consumes the widget)
    let network_for_roots = Network::new(&model, &theme, 0);
    let roots = network_for_roots.get_root_tasks();
    assert!(roots.contains(&parent_id));

    // Then test rendering
    let network = Network::new(&model, &theme, 0);
    let area = Rect::new(0, 0, 120, 30);
    let mut buffer = Buffer::empty(area);
    network.render(area, &mut buffer);
    assert!(buffer.area.width > 0);
}

#[test]
fn test_network_with_task_chain() {
    let mut model = Model::new();

    // Create first task
    let mut task1 = Task::new("First in chain");
    let task1_id = task1.id;

    // Create second task
    let task2 = Task::new("Second in chain");
    let task2_id = task2.id;

    // Link them in a chain
    task1.next_task_id = Some(task2_id);

    model.tasks.insert(task1_id, task1);
    model.tasks.insert(task2_id, task2);
    model.refresh_visible_tasks();

    let theme = Theme::default();

    // Check roots first (before render consumes the widget)
    let network_for_roots = Network::new(&model, &theme, 0);
    let roots = network_for_roots.get_root_tasks();
    assert!(roots.contains(&task1_id));

    // Then test rendering
    let network = Network::new(&model, &theme, 0);
    let area = Rect::new(0, 0, 120, 30);
    let mut buffer = Buffer::empty(area);
    network.render(area, &mut buffer);
    assert!(buffer.area.width > 0);
}

#[test]
fn test_network_with_completed_task() {
    let mut model = Model::new();

    let parent = Task::new("Parent Task");
    let parent_id = parent.id;
    model.tasks.insert(parent_id, parent);

    let mut child = Task::new("Child Task");
    child.dependencies.push(parent_id);
    child.toggle_complete();
    model.tasks.insert(child.id, child);

    model.refresh_visible_tasks();

    let theme = Theme::default();
    let network = Network::new(&model, &theme, 0);

    let area = Rect::new(0, 0, 120, 30);
    let mut buffer = Buffer::empty(area);
    network.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_network_narrow_area() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let network = Network::new(&model, &theme, 0);

    // Too narrow - should return early
    let area = Rect::new(0, 0, 20, 5);
    let mut buffer = Buffer::empty(area);
    network.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_network_get_selected_task_id() {
    let mut model = Model::new();

    let task1 = Task::new("Task 1");
    let task1_id = task1.id;
    let task2 = Task::new("Task 2");
    let task2_id = task2.id;

    // Link tasks to make them appear in network_tasks
    let mut task1 = task1;
    task1.next_task_id = Some(task2_id);
    model.tasks.insert(task1_id, task1);
    model.tasks.insert(task2_id, task2);
    model.refresh_visible_tasks();

    let theme = Theme::default();

    // Select first task
    let network = Network::new(&model, &theme, 0);
    let selected = network.get_selected_task_id();
    // The selected task depends on network_tasks order
    assert!(selected.is_some() || model.network_tasks().is_empty());
}

#[test]
fn test_network_stats_rendering() {
    let mut model = Model::new();

    // Create tasks with dependencies
    let parent = Task::new("Blocking Task");
    let parent_id = parent.id;
    model.tasks.insert(parent_id, parent);

    let mut blocked = Task::new("Blocked Task");
    blocked.dependencies.push(parent_id);
    model.tasks.insert(blocked.id, blocked);

    model.refresh_visible_tasks();

    let theme = Theme::default();
    let network = Network::new(&model, &theme, 0);

    let area = Rect::new(0, 0, 120, 30);
    let mut buffer = Buffer::empty(area);
    network.render(area, &mut buffer);

    // Check for stats content
    let mut found_stats = false;
    for y in 0..buffer.area.height {
        let line: String = (0..buffer.area.width)
            .filter_map(|x| buffer.cell((x, y)).map(ratatui::buffer::Cell::symbol))
            .collect();
        if line.contains("Statistics") {
            found_stats = true;
            break;
        }
    }
    assert!(found_stats);
}

#[test]
fn test_network_legend_rendering() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let network = Network::new(&model, &theme, 0);

    let area = Rect::new(0, 0, 120, 30);
    let mut buffer = Buffer::empty(area);
    network.render(area, &mut buffer);

    // Check for legend content
    let mut found_legend = false;
    for y in 0..buffer.area.height {
        let line: String = (0..buffer.area.width)
            .filter_map(|x| buffer.cell((x, y)).map(ratatui::buffer::Cell::symbol))
            .collect();
        if line.contains("Legend") {
            found_legend = true;
            break;
        }
    }
    assert!(found_legend);
}

#[test]
fn test_network_no_dependencies_message() {
    let mut model = Model::new();
    // Add tasks without any dependencies
    let task = Task::new("Standalone Task");
    model.tasks.insert(task.id, task);
    model.refresh_visible_tasks();

    let theme = Theme::default();
    let network = Network::new(&model, &theme, 0);

    let area = Rect::new(0, 0, 120, 30);
    let mut buffer = Buffer::empty(area);
    network.render(area, &mut buffer);

    // Check for "No task dependencies" message
    let mut found_message = false;
    for y in 0..buffer.area.height {
        let line: String = (0..buffer.area.width)
            .filter_map(|x| buffer.cell((x, y)).map(ratatui::buffer::Cell::symbol))
            .collect();
        if line.contains("No task dependencies") {
            found_message = true;
            break;
        }
    }
    assert!(found_message);
}

#[test]
fn test_network_selection_index() {
    let mut model = Model::new();

    // Create chain of tasks
    let mut task1 = Task::new("Task 1");
    let task1_id = task1.id;
    let mut task2 = Task::new("Task 2");
    let task2_id = task2.id;
    let task3 = Task::new("Task 3");
    let task3_id = task3.id;

    task1.next_task_id = Some(task2_id);
    task2.next_task_id = Some(task3_id);

    model.tasks.insert(task1_id, task1);
    model.tasks.insert(task2_id, task2);
    model.tasks.insert(task3_id, task3);
    model.refresh_visible_tasks();

    let theme = Theme::default();

    // Different selection indices
    for idx in 0..3 {
        let network = Network::new(&model, &theme, idx);
        let area = Rect::new(0, 0, 120, 30);
        let mut buffer = Buffer::empty(area);
        network.render(area, &mut buffer);
        assert!(buffer.area.width > 0);
    }
}
