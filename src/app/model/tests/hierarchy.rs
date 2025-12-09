//! Task hierarchy tests (depth, ancestors, descendants, cycles, subtasks).

use crate::app::Model;
use crate::domain::{Task, TaskId, TaskStatus};

#[test]
fn test_task_depth_root_task() {
    let mut model = Model::new();
    let task = Task::new("Root");
    let task_id = task.id;
    model.tasks.insert(task.id, task);
    assert_eq!(model.task_depth(&task_id), 0);
}

#[test]
fn test_task_depth_nested() {
    let mut model = Model::new();
    let root = Task::new("Root");
    let child = Task::new("Child").with_parent(root.id);
    let grandchild = Task::new("Grandchild").with_parent(child.id);

    let root_id = root.id;
    let child_id = child.id;
    let grandchild_id = grandchild.id;

    model.tasks.insert(root.id, root);
    model.tasks.insert(child.id, child);
    model.tasks.insert(grandchild.id, grandchild);

    assert_eq!(model.task_depth(&root_id), 0);
    assert_eq!(model.task_depth(&child_id), 1);
    assert_eq!(model.task_depth(&grandchild_id), 2);
}

#[test]
fn test_task_depth_missing_parent() {
    let mut model = Model::new();
    // Create a task with a parent_task_id that doesn't exist
    let orphan_parent_id = TaskId::new();
    let orphan = Task::new("Orphan").with_parent(orphan_parent_id);
    let orphan_id = orphan.id;
    model.tasks.insert(orphan.id, orphan);

    // Returns 1 because the function counts parent hops: orphan → missing parent (1 hop).
    // Note that orphaned tasks will display indented even though their parent doesn't exist.
    assert_eq!(model.task_depth(&orphan_id), 1);
}

#[test]
fn test_get_all_descendants_empty() {
    let mut model = Model::new();
    let task = Task::new("Standalone");
    let task_id = task.id;
    model.tasks.insert(task.id, task);

    let descendants = model.get_all_descendants(&task_id);
    assert!(descendants.is_empty());
}

#[test]
fn test_get_all_descendants_nested() {
    let mut model = Model::new();
    let root = Task::new("Root");
    let child1 = Task::new("Child1").with_parent(root.id);
    let child2 = Task::new("Child2").with_parent(root.id);
    let grandchild = Task::new("Grandchild").with_parent(child1.id);

    let root_id = root.id;
    let child1_id = child1.id;
    let child2_id = child2.id;
    let grandchild_id = grandchild.id;

    model.tasks.insert(root.id, root);
    model.tasks.insert(child1.id, child1);
    model.tasks.insert(child2.id, child2);
    model.tasks.insert(grandchild.id, grandchild);

    let descendants = model.get_all_descendants(&root_id);
    assert_eq!(descendants.len(), 3);
    assert!(descendants.contains(&child1_id));
    assert!(descendants.contains(&child2_id));
    assert!(descendants.contains(&grandchild_id));
}

#[test]
fn test_get_all_ancestors_empty() {
    let mut model = Model::new();
    let task = Task::new("Root");
    let task_id = task.id;
    model.tasks.insert(task.id, task);

    let ancestors = model.get_all_ancestors(&task_id);
    assert!(ancestors.is_empty());
}

#[test]
fn test_get_all_ancestors_nested() {
    let mut model = Model::new();
    let root = Task::new("Root");
    let child = Task::new("Child").with_parent(root.id);
    let grandchild = Task::new("Grandchild").with_parent(child.id);

    let root_id = root.id;
    let child_id = child.id;
    let grandchild_id = grandchild.id;

    model.tasks.insert(root.id, root);
    model.tasks.insert(child.id, child);
    model.tasks.insert(grandchild.id, grandchild);

    let ancestors = model.get_all_ancestors(&grandchild_id);
    assert_eq!(ancestors.len(), 2);
    assert_eq!(ancestors[0], child_id); // Direct parent first
    assert_eq!(ancestors[1], root_id); // Then grandparent
}

#[test]
fn test_would_create_cycle_self_reference() {
    let mut model = Model::new();
    let task = Task::new("Task");
    let task_id = task.id;
    model.tasks.insert(task.id, task);

    assert!(model.would_create_cycle(&task_id, &task_id));
}

#[test]
fn test_would_create_cycle_descendant() {
    let mut model = Model::new();
    let root = Task::new("Root");
    let child = Task::new("Child").with_parent(root.id);
    let grandchild = Task::new("Grandchild").with_parent(child.id);

    let root_id = root.id;
    let child_id = child.id;
    let grandchild_id = grandchild.id;

    model.tasks.insert(root.id, root);
    model.tasks.insert(child.id, child);
    model.tasks.insert(grandchild.id, grandchild);

    // Setting root's parent to grandchild would create a cycle
    assert!(model.would_create_cycle(&root_id, &grandchild_id));
    assert!(model.would_create_cycle(&root_id, &child_id));

    // Setting grandchild's parent to a new task is fine
    let new_task = Task::new("New");
    let new_task_id = new_task.id;
    model.tasks.insert(new_task.id, new_task);
    assert!(!model.would_create_cycle(&grandchild_id, &new_task_id));
}

#[test]
fn test_has_subtasks() {
    let mut model = Model::new();
    let parent = Task::new("Parent");
    let child = Task::new("Child").with_parent(parent.id);
    let standalone = Task::new("Standalone");

    let parent_id = parent.id;
    let standalone_id = standalone.id;

    model.tasks.insert(parent.id, parent);
    model.tasks.insert(child.id, child);
    model.tasks.insert(standalone.id, standalone);

    assert!(model.has_subtasks(&parent_id));
    assert!(!model.has_subtasks(&standalone_id));
}

#[test]
fn test_subtask_progress_recursive() {
    let mut model = Model::new();
    let root = Task::new("Root");
    let child1 = Task::new("Child1")
        .with_parent(root.id)
        .with_status(TaskStatus::Done);
    let child2 = Task::new("Child2").with_parent(root.id);
    let grandchild = Task::new("Grandchild")
        .with_parent(child2.id)
        .with_status(TaskStatus::Done);

    let root_id = root.id;

    model.tasks.insert(root.id, root);
    model.tasks.insert(child1.id, child1);
    model.tasks.insert(child2.id, child2);
    model.tasks.insert(grandchild.id, grandchild);

    let (completed, total) = model.subtask_progress(&root_id);
    assert_eq!(total, 3); // child1, child2, grandchild
    assert_eq!(completed, 2); // child1, grandchild
}

#[test]
fn test_subtask_percentage() {
    let mut model = Model::new();
    let root = Task::new("Root");
    let child1 = Task::new("Child1")
        .with_parent(root.id)
        .with_status(TaskStatus::Done);
    let child2 = Task::new("Child2").with_parent(root.id);

    let root_id = root.id;

    model.tasks.insert(root.id, root);
    model.tasks.insert(child1.id, child1);
    model.tasks.insert(child2.id, child2);

    // 1 of 2 completed = 50%
    assert_eq!(model.subtask_percentage(&root_id), Some(50));
}

#[test]
fn test_subtask_percentage_no_subtasks() {
    let mut model = Model::new();
    let task = Task::new("Standalone");
    let task_id = task.id;
    model.tasks.insert(task.id, task);

    assert_eq!(model.subtask_percentage(&task_id), None);
}

#[test]
fn test_refresh_visible_tasks_deep_nesting_order() {
    // Test that visible_tasks orders: Root -> Child -> Grandchild -> Root2
    let mut model = Model::new();

    let root1 = Task::new("Root1");
    let child1 = Task::new("Child1").with_parent(root1.id);
    let grandchild = Task::new("Grandchild").with_parent(child1.id);
    let root2 = Task::new("Root2");

    let root1_id = root1.id;
    let child1_id = child1.id;
    let grandchild_id = grandchild.id;
    let root2_id = root2.id;

    // Insert in random order
    model.tasks.insert(grandchild.id, grandchild);
    model.tasks.insert(root2.id, root2);
    model.tasks.insert(child1.id, child1);
    model.tasks.insert(root1.id, root1);

    model.refresh_visible_tasks();

    // Check ordering: should be Root1 -> Child1 -> Grandchild -> Root2
    // (roots sorted by created_at, subtasks inserted after their parents)
    let root1_pos = model
        .visible_tasks
        .iter()
        .position(|id| id == &root1_id)
        .unwrap();
    let child1_pos = model
        .visible_tasks
        .iter()
        .position(|id| id == &child1_id)
        .unwrap();
    let grandchild_pos = model
        .visible_tasks
        .iter()
        .position(|id| id == &grandchild_id)
        .unwrap();
    let root2_pos = model
        .visible_tasks
        .iter()
        .position(|id| id == &root2_id)
        .unwrap();

    // Child1 should come after Root1
    assert!(child1_pos > root1_pos, "Child1 should appear after Root1");

    // Grandchild should come after Child1
    assert!(
        grandchild_pos > child1_pos,
        "Grandchild should appear after Child1"
    );

    // Grandchild should come before Root2 (if Root2 comes after Root1)
    // This ensures the hierarchy is kept together
    if root2_pos > root1_pos {
        assert!(
            grandchild_pos < root2_pos,
            "Grandchild should appear before Root2"
        );
    }
}
