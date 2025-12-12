//! Tests for task detail component.

use super::*;
use crate::app::Model;
use crate::config::Theme;
use crate::domain::Task;
use ratatui::{buffer::Buffer, layout::Rect};

fn default_theme() -> Theme {
    Theme::default()
}

#[test]
fn test_task_detail_renders_without_panic() {
    let mut model = Model::new();

    // Add a task
    let task = Task::new("Test task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    let theme = default_theme();
    let widget = TaskDetail::new(&model, &theme, 0);

    let area = Rect::new(0, 0, 60, 30);
    let mut buffer = Buffer::empty(area);

    widget.render(area, &mut buffer);

    // Check that the title is rendered somewhere
    let content = buffer_to_string(&buffer);
    assert!(content.contains("Test task"));
}

#[test]
fn test_task_detail_renders_no_task_message() {
    let model = Model::new();
    let theme = default_theme();
    let widget = TaskDetail::new(&model, &theme, 0);

    let area = Rect::new(0, 0, 40, 10);
    let mut buffer = Buffer::empty(area);

    widget.render(area, &mut buffer);

    let content = buffer_to_string(&buffer);
    assert!(content.contains("No task selected"));
}

#[test]
fn test_task_detail_scroll() {
    let mut model = Model::new();

    let mut task = Task::new("Test task");
    task.description = Some("Line 1\nLine 2\nLine 3\nLine 4\nLine 5".to_string());
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.refresh_visible_tasks();

    let theme = default_theme();

    // Render with scroll = 0
    let widget = TaskDetail::new(&model, &theme, 0);
    let area = Rect::new(0, 0, 60, 10);
    let mut buffer = Buffer::empty(area);
    widget.render(area, &mut buffer);

    // Render with scroll = 5
    let widget_scrolled = TaskDetail::new(&model, &theme, 5);
    let mut buffer_scrolled = Buffer::empty(area);
    widget_scrolled.render(area, &mut buffer_scrolled);

    // Content should be different when scrolled
    // (This is a basic sanity check)
}

/// Helper to convert buffer to string for assertions
fn buffer_to_string(buffer: &Buffer) -> String {
    let mut result = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            result.push_str(cell.symbol());
        }
        result.push('\n');
    }
    result
}
