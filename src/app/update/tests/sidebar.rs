//! Sidebar navigation tests.

use crate::app::{
    update::update, FocusPane, Message, Model, NavigationMessage, ViewId,
    SIDEBAR_FIRST_PROJECT_INDEX, SIDEBAR_PROJECTS_HEADER_INDEX, SIDEBAR_SEPARATOR_INDEX,
};
use crate::domain::{Project, Task};

#[test]
fn test_focus_sidebar() {
    let mut model = Model::new();
    assert_eq!(model.focus_pane, FocusPane::TaskList);

    update(
        &mut model,
        Message::Navigation(NavigationMessage::FocusSidebar),
    );

    assert_eq!(model.focus_pane, FocusPane::Sidebar);
}

#[test]
fn test_focus_task_list() {
    let mut model = Model::new();
    model.focus_pane = FocusPane::Sidebar;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::FocusTaskList),
    );

    assert_eq!(model.focus_pane, FocusPane::TaskList);
}

#[test]
fn test_sidebar_navigation_up_down() {
    let mut model = Model::new().with_sample_data();
    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = 0;

    // Move down
    update(&mut model, Message::Navigation(NavigationMessage::Down));
    assert_eq!(model.sidebar_selected, 1);

    // Move down again
    update(&mut model, Message::Navigation(NavigationMessage::Down));
    assert_eq!(model.sidebar_selected, 2);

    // Move up
    update(&mut model, Message::Navigation(NavigationMessage::Up));
    assert_eq!(model.sidebar_selected, 1);
}

#[test]
fn test_sidebar_navigation_skips_separator() {
    let mut model = Model::new().with_sample_data();
    model.focus_pane = FocusPane::Sidebar;
    // Position at last view item (just before separator)
    model.sidebar_selected = SIDEBAR_SEPARATOR_INDEX - 1;

    // Move down should skip separator and go to Projects header
    update(&mut model, Message::Navigation(NavigationMessage::Down));
    assert_eq!(model.sidebar_selected, SIDEBAR_PROJECTS_HEADER_INDEX);

    // Move up should skip separator and go back to last view item
    update(&mut model, Message::Navigation(NavigationMessage::Up));
    assert_eq!(model.sidebar_selected, SIDEBAR_SEPARATOR_INDEX - 1);
}

#[test]
fn test_sidebar_select_view() {
    let mut model = Model::new().with_sample_data();
    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = 1; // Today view

    update(
        &mut model,
        Message::Navigation(NavigationMessage::SelectSidebarItem),
    );

    assert_eq!(model.current_view, ViewId::Today);
    assert!(model.selected_project.is_none());
}

#[test]
fn test_sidebar_select_overdue_view() {
    let mut model = Model::new().with_sample_data();
    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = 3; // Overdue view

    update(
        &mut model,
        Message::Navigation(NavigationMessage::SelectSidebarItem),
    );

    assert_eq!(model.current_view, ViewId::Overdue);
    assert!(model.selected_project.is_none());
    assert_eq!(model.focus_pane, FocusPane::TaskList);
}

#[test]
fn test_sidebar_select_project() {
    let mut model = Model::new();
    // Add a project
    let project = Project::new("Test Project");
    let project_id = project.id;
    model.projects.insert(project.id, project);

    // Add a task with this project
    let mut task = Task::new("Task in project");
    task.project_id = Some(project_id);
    model.tasks.insert(task.id, task);

    // Add a task without project
    let task2 = Task::new("Task without project");
    model.tasks.insert(task2.id, task2);

    model.refresh_visible_tasks();
    assert_eq!(model.visible_tasks.len(), 2);

    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = SIDEBAR_FIRST_PROJECT_INDEX; // First project

    update(
        &mut model,
        Message::Navigation(NavigationMessage::SelectSidebarItem),
    );

    // Project should be selected
    assert_eq!(model.selected_project, Some(project_id));
    // Only project tasks should be visible
    assert_eq!(model.visible_tasks.len(), 1);
}

#[test]
fn test_sidebar_select_all_tasks_clears_project_filter() {
    let mut model = Model::new();
    let project = Project::new("Test Project");
    let project_id = project.id;
    model.projects.insert(project.id, project);
    model.selected_project = Some(project_id);

    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = 0; // All Tasks

    update(
        &mut model,
        Message::Navigation(NavigationMessage::SelectSidebarItem),
    );

    assert!(model.selected_project.is_none());
    assert_eq!(model.current_view, ViewId::TaskList);
}
