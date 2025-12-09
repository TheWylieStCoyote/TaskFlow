//! Tests for the task list widget.

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use super::*;
use crate::app::Model;
use crate::config::Theme;
use crate::domain::{Priority, Task, TaskStatus};

/// Helper to render a widget into a buffer
fn render_widget<W: Widget>(widget: W, width: u16, height: u16) -> Buffer {
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    widget.render(area, &mut buffer);
    buffer
}

/// Extract text content from buffer
fn buffer_content(buffer: &Buffer) -> String {
    let mut content = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            content.push(
                buffer
                    .cell((x, y))
                    .map_or(' ', |c| c.symbol().chars().next().unwrap_or(' ')),
            );
        }
        content.push('\n');
    }
    content
}

#[test]
fn test_task_list_renders_title() {
    let model = Model::new();
    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 60, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Tasks"),
        "Task list title should be visible"
    );
}

#[test]
fn test_task_list_renders_empty_list() {
    let model = Model::new();
    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 60, 20);

    // Should render without panic
    let _ = buffer_content(&buffer);
}

#[test]
fn test_task_list_renders_task_titles() {
    let mut model = Model::new();
    let task = Task::new("Test Task Title");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.visible_tasks.push(task_id);

    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 60, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Test Task Title"),
        "Task title should be visible"
    );
}

#[test]
fn test_task_list_renders_priority_indicator() {
    let mut model = Model::new();
    let task = Task::new("Urgent Task").with_priority(Priority::Urgent);
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.visible_tasks.push(task_id);

    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 60, 20);
    let content = buffer_content(&buffer);

    // Urgent tasks show "!!!!"
    assert!(
        content.contains("!!!!") || content.contains("Urgent"),
        "Priority indicator should be visible"
    );
}

#[test]
fn test_task_list_renders_completed_task() {
    let mut model = Model::new();
    let task = Task::new("Completed Task").with_status(TaskStatus::Done);
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.visible_tasks.push(task_id);
    model.show_completed = true;

    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 60, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Completed Task"),
        "Completed task title should be visible"
    );
}

#[test]
fn test_task_list_renders_status_symbol() {
    let mut model = Model::new();
    let task = Task::new("In Progress Task").with_status(TaskStatus::InProgress);
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.visible_tasks.push(task_id);

    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 60, 20);
    let content = buffer_content(&buffer);

    // Should have status symbol (varies by status)
    assert!(
        content.contains("In Progress Task"),
        "Task with status should be visible"
    );
}

#[test]
fn test_task_list_renders_tags() {
    let mut model = Model::new();
    let task = Task::new("Tagged Task").with_tags(vec!["rust".into(), "test".into()]);
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.visible_tasks.push(task_id);

    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 80, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("#rust") || content.contains("#test"),
        "Tags should be visible"
    );
}

#[test]
fn test_task_list_renders_due_date() {
    let mut model = Model::new();
    let today = chrono::Utc::now().date_naive();
    let task = Task::new("Due Task").with_due_date(today);
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.visible_tasks.push(task_id);

    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 80, 20);
    let content = buffer_content(&buffer);

    // Due date shown in format [MM/DD]
    assert!(content.contains('['), "Due date bracket should be visible");
}

#[test]
fn test_task_list_renders_with_sample_data() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 80, 30);
    let content = buffer_content(&buffer);

    // Sample data contains various tasks
    assert!(
        content.contains("Tasks"),
        "Task list title should be visible"
    );
}

#[test]
fn test_task_list_grouped_view() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Projects;
    model.refresh_visible_tasks();

    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 80, 30);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("by Project"),
        "Grouped title should be visible"
    );
}

#[test]
fn test_task_list_description_indicator() {
    let mut model = Model::new();
    let task = Task::new("Task with Notes").with_description("Some important notes".to_string());
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.visible_tasks.push(task_id);

    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 80, 20);
    let content = buffer_content(&buffer);

    // [+] indicator for tasks with descriptions
    assert!(
        content.contains("[+]"),
        "Description indicator should be visible"
    );
}
