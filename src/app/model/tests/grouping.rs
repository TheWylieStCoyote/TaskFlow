//! Project grouping tests.

use crate::app::model::{Model, SortSpec, ViewId};
use crate::domain::{Project, SortField, SortOrder, Task};

#[test]
fn test_get_tasks_grouped_by_project_basic() {
    let mut model = Model::new();
    model.current_view = ViewId::Projects;

    // Create two projects
    let project_a = Project::new("Alpha Project");
    let project_b = Project::new("Beta Project");
    let project_a_id = project_a.id;
    let project_b_id = project_b.id;

    model.projects.insert(project_a_id, project_a);
    model.projects.insert(project_b_id, project_b);

    // Create tasks for each project
    let task_a1 = Task::new("Alpha Task 1").with_project(project_a_id);
    let task_a2 = Task::new("Alpha Task 2").with_project(project_a_id);
    let task_b1 = Task::new("Beta Task 1").with_project(project_b_id);

    model.tasks.insert(task_a1.id, task_a1);
    model.tasks.insert(task_a2.id, task_a2);
    model.tasks.insert(task_b1.id, task_b1);

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
    let mut model = Model::new();
    model.current_view = ViewId::Projects;

    // Create projects out of alphabetical order
    let project_z = Project::new("Zebra");
    let project_a = Project::new("Apple");
    let project_m = Project::new("Mango");

    let z_id = project_z.id;
    let a_id = project_a.id;
    let m_id = project_m.id;

    model.projects.insert(z_id, project_z);
    model.projects.insert(a_id, project_a);
    model.projects.insert(m_id, project_m);

    // Create one task per project
    let task_z = Task::new("Z task").with_project(z_id);
    let task_a = Task::new("A task").with_project(a_id);
    let task_m = Task::new("M task").with_project(m_id);

    model.tasks.insert(task_z.id, task_z);
    model.tasks.insert(task_a.id, task_a);
    model.tasks.insert(task_m.id, task_m);

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
    let mut model = Model::new();
    model.current_view = ViewId::Projects;

    // Create one project
    let project = Project::new("My Project");
    let project_id = project.id;
    model.projects.insert(project_id, project);

    // Task with project
    let task_with = Task::new("With project").with_project(project_id);
    // Task without project (shouldn't appear in Projects view normally,
    // but test the grouping logic)
    let task_without = Task::new("Without project");

    model.tasks.insert(task_with.id, task_with);
    model.tasks.insert(task_without.id, task_without);

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
    let mut model = Model::new();
    model.current_view = ViewId::Projects;

    // Sort by title ascending
    model.sort = SortSpec {
        field: SortField::Title,
        order: SortOrder::Ascending,
    };

    let project = Project::new("Test Project");
    let project_id = project.id;
    model.projects.insert(project_id, project);

    // Create tasks with different titles (will be sorted alphabetically)
    let task_c = Task::new("Charlie").with_project(project_id);
    let task_a = Task::new("Alpha").with_project(project_id);
    let task_b = Task::new("Bravo").with_project(project_id);

    let task_a_id = task_a.id;
    let task_b_id = task_b.id;
    let task_c_id = task_c.id;

    model.tasks.insert(task_c.id, task_c);
    model.tasks.insert(task_a.id, task_a);
    model.tasks.insert(task_b.id, task_b);

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
