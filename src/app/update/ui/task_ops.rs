//! Task operation handlers (move, reorder)

use crate::app::Model;

/// Handle moving a task up or down in the list order
pub fn handle_move_task(model: &mut Model, direction: i32) {
    if model.selected_index >= model.visible_tasks.len() {
        return;
    }

    let current_task_id = model.visible_tasks[model.selected_index].clone();

    // Get the current task
    let current_order = model
        .tasks
        .get(&current_task_id)
        .and_then(|t| t.sort_order)
        .unwrap_or(0);

    // Find the task to swap with
    let swap_index = if direction < 0 {
        // Moving up - find previous non-subtask at same level
        if model.selected_index == 0 {
            return;
        }
        model.selected_index - 1
    } else {
        // Moving down - find next task
        if model.selected_index >= model.visible_tasks.len() - 1 {
            return;
        }
        model.selected_index + 1
    };

    let swap_task_id = model.visible_tasks[swap_index].clone();

    // Get the swap task's order
    let swap_order = model
        .tasks
        .get(&swap_task_id)
        .and_then(|t| t.sort_order)
        .unwrap_or(0);

    // Swap the sort orders
    model.modify_task_with_undo(&current_task_id, |task| {
        task.sort_order = Some(swap_order);
    });
    model.modify_task_with_undo(&swap_task_id, |task| {
        task.sort_order = Some(current_order);
    });

    // Update selection to follow the moved task
    model.selected_index = swap_index;
    model.refresh_visible_tasks();
}
