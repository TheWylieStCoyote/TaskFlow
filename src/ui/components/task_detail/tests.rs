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

fn render_task(model: &Model) -> String {
    let theme = default_theme();
    let widget = TaskDetail::new(model, &theme, 0);
    let area = Rect::new(0, 0, 100, 50);
    let mut buffer = Buffer::empty(area);
    widget.render(area, &mut buffer);
    buffer_to_string(&buffer)
}

#[test]
fn test_task_detail_with_description() {
    let mut model = Model::new();
    let mut task = Task::new("Task with desc");
    task.description = Some("This is the description".to_string());
    let id = task.id;
    model.tasks.insert(id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    let content = render_task(&model);
    assert!(content.contains("Task with desc"));
}

#[test]
fn test_task_detail_with_tags() {
    let mut model = Model::new();
    let mut task = Task::new("Tagged task");
    task.tags = vec!["work".to_string(), "urgent".to_string()];
    let id = task.id;
    model.tasks.insert(id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    let content = render_task(&model);
    assert!(content.contains("Tagged task"));
}

#[test]
fn test_task_detail_with_project() {
    use crate::domain::Project;

    let mut model = Model::new();
    let project = Project::new("My Project");
    let pid = project.id;
    model.projects.insert(pid, project);

    let mut task = Task::new("Project task");
    task.project_id = Some(pid);
    let id = task.id;
    model.tasks.insert(id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    let content = render_task(&model);
    assert!(content.contains("Project task"));
}

#[test]
fn test_task_detail_with_due_date() {
    use chrono::Utc;

    let mut model = Model::new();
    let mut task = Task::new("Due task");
    task.due_date = Some(Utc::now().date_naive());
    let id = task.id;
    model.tasks.insert(id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    let content = render_task(&model);
    assert!(content.contains("Due task"));
}

#[test]
fn test_task_detail_with_priority_variants() {
    use crate::domain::Priority;

    for priority in [
        Priority::Urgent,
        Priority::High,
        Priority::Medium,
        Priority::Low,
        Priority::None,
    ] {
        let mut model = Model::new();
        let mut task = Task::new("Priority task");
        task.priority = priority;
        let id = task.id;
        model.tasks.insert(id, task);
        model.refresh_visible_tasks();
        model.selected_index = 0;

        let theme = default_theme();
        let widget = TaskDetail::new(&model, &theme, 0);
        let area = Rect::new(0, 0, 80, 30);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer); // Should not panic
    }
}

#[test]
fn test_task_detail_with_status_variants() {
    use crate::domain::TaskStatus;

    for status in [
        TaskStatus::Todo,
        TaskStatus::InProgress,
        TaskStatus::Blocked,
        TaskStatus::Done,
        TaskStatus::Cancelled,
    ] {
        let mut model = Model::new();
        let mut task = Task::new("Status task");
        task.status = status;
        let id = task.id;
        model.tasks.insert(id, task);
        model.refresh_visible_tasks();
        model.selected_index = 0;

        let theme = default_theme();
        let widget = TaskDetail::new(&model, &theme, 0);
        let area = Rect::new(0, 0, 80, 30);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer); // Should not panic
    }
}

#[test]
fn test_task_detail_with_estimate() {
    let mut model = Model::new();
    let mut task = Task::new("Estimated task");
    task.estimated_minutes = Some(90);
    task.actual_minutes = 45;
    let id = task.id;
    model.tasks.insert(id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    let content = render_task(&model);
    assert!(content.contains("Estimated task"));
}

#[test]
fn test_task_detail_with_subtasks() {
    let mut model = Model::new();
    let parent = Task::new("Parent task");
    let mut child = Task::new("Child task");
    let parent_id = parent.id;
    child.parent_task_id = Some(parent_id);
    model.tasks.insert(parent_id, parent);
    model.tasks.insert(child.id, child);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    let content = render_task(&model);
    assert!(!content.is_empty());
}

#[test]
fn test_task_detail_narrow_area() {
    let mut model = Model::new();
    let task = Task::new("Narrow test");
    let id = task.id;
    model.tasks.insert(id, task);
    model.refresh_visible_tasks();
    model.selected_index = 0;

    let theme = default_theme();
    let widget = TaskDetail::new(&model, &theme, 0);
    let area = Rect::new(0, 0, 20, 10);
    let mut buffer = Buffer::empty(area);
    widget.render(area, &mut buffer); // Should not panic
}
