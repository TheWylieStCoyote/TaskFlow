//! UI message handlers
//!
//! Handles all user interface messages including:
//! - Input mode handling (create, edit, search)
//! - View controls (toggle completed, sidebar)
//! - Multi-select and bulk operations
//! - Calendar navigation
//! - Macro recording/playback
//! - Template picker
//! - Keybindings editor

use std::fmt::Write as _;

use crate::app::{parse_date, parse_quick_add, Model, UiMessage, UndoAction, ViewId};
use crate::domain::TaskId;
use crate::ui::{InputMode, InputTarget};

use super::navigation::days_in_month;
use super::system::handle_execute_import;

/// Handle UI messages
#[allow(clippy::too_many_lines)]
pub fn handle_ui(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ToggleShowCompleted => {
            model.show_completed = !model.show_completed;
            model.refresh_visible_tasks();
        }
        UiMessage::ToggleSidebar => {
            model.show_sidebar = !model.show_sidebar;
        }
        UiMessage::ShowHelp => {
            model.show_help = true;
        }
        UiMessage::HideHelp => {
            model.show_help = false;
        }
        UiMessage::ToggleFocusMode => {
            // Only toggle focus mode if there's a selected task
            if model.selected_task().is_some() {
                model.focus_mode = !model.focus_mode;
            }
        }
        // Input mode handling
        UiMessage::StartCreateTask => {
            model.input_mode = InputMode::Editing;
            model.input_target = InputTarget::Task;
            model.input_buffer.clear();
            model.cursor_position = 0;
        }
        UiMessage::StartCreateSubtask => {
            // Create a subtask under the currently selected task
            if let Some(parent_id) = model.visible_tasks.get(model.selected_index).cloned() {
                if model.tasks.contains_key(&parent_id) {
                    model.input_mode = InputMode::Editing;
                    model.input_target = InputTarget::Subtask(parent_id);
                    model.input_buffer.clear();
                    model.cursor_position = 0;
                }
            }
        }
        UiMessage::StartCreateProject => {
            model.input_mode = InputMode::Editing;
            model.input_target = InputTarget::Project;
            model.input_buffer.clear();
            model.cursor_position = 0;
        }
        UiMessage::StartEditProject => {
            // Edit the currently selected project (from sidebar)
            if let Some(ref project_id) = model.selected_project.clone() {
                if let Some(project) = model.projects.get(project_id) {
                    model.input_mode = InputMode::Editing;
                    model.input_target = InputTarget::EditProject(project_id.clone());
                    model.input_buffer = project.name.clone();
                    model.cursor_position = model.input_buffer.len();
                }
            } else {
                model.status_message = Some("Select a project from the sidebar first".to_string());
            }
        }
        UiMessage::DeleteProject => {
            // Delete the currently selected project (from sidebar)
            if let Some(ref project_id) = model.selected_project.clone() {
                if let Some(project) = model.projects.remove(project_id) {
                    // Find tasks in this project
                    let tasks_to_unassign: Vec<_> = model
                        .tasks
                        .iter()
                        .filter(|(_, task)| task.project_id.as_ref() == Some(project_id))
                        .map(|(id, _)| id.clone())
                        .collect();

                    // Unassign tasks from this project
                    for task_id in tasks_to_unassign {
                        if let Some(task) = model.tasks.get_mut(&task_id) {
                            task.project_id = None;
                            let task_clone = task.clone();
                            model.sync_task(&task_clone);
                        }
                    }

                    // Push undo action
                    model
                        .undo_stack
                        .push(UndoAction::ProjectDeleted(Box::new(project.clone())));
                    // Clear selected project
                    model.selected_project = None;
                    model.dirty = true;
                    model.refresh_visible_tasks();
                    model.status_message = Some(format!(
                        "Deleted project '{}' (tasks unassigned)",
                        project.name
                    ));
                }
            } else {
                model.status_message = Some("Select a project from the sidebar first".to_string());
            }
        }
        UiMessage::StartEditTask => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                if let Some(task) = model.tasks.get(&task_id) {
                    model.input_mode = InputMode::Editing;
                    model.input_target = InputTarget::EditTask(task_id);
                    model.input_buffer = task.title.clone();
                    model.cursor_position = model.input_buffer.len();
                }
            }
        }
        UiMessage::StartEditDueDate => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                if let Some(task) = model.tasks.get(&task_id) {
                    model.input_mode = InputMode::Editing;
                    model.input_target = InputTarget::EditDueDate(task_id);
                    // Pre-fill with existing due date or empty
                    model.input_buffer = task
                        .due_date
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_default();
                    model.cursor_position = model.input_buffer.len();
                }
            }
        }
        UiMessage::StartEditScheduledDate => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                if let Some(task) = model.tasks.get(&task_id) {
                    model.input_mode = InputMode::Editing;
                    model.input_target = InputTarget::EditScheduledDate(task_id);
                    // Pre-fill with existing scheduled date or empty
                    model.input_buffer = task
                        .scheduled_date
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_default();
                    model.cursor_position = model.input_buffer.len();
                }
            }
        }
        UiMessage::StartEditTags => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                if let Some(task) = model.tasks.get(&task_id) {
                    model.input_mode = InputMode::Editing;
                    model.input_target = InputTarget::EditTags(task_id);
                    // Pre-fill with existing tags as comma-separated
                    model.input_buffer = task.tags.join(", ");
                    model.cursor_position = model.input_buffer.len();
                }
            }
        }
        UiMessage::StartEditDescription => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                if let Some(task) = model.tasks.get(&task_id) {
                    model.input_mode = InputMode::Editing;
                    model.input_target = InputTarget::EditDescription(task_id);
                    // Pre-fill with existing description
                    model.input_buffer = task.description.clone().unwrap_or_default();
                    model.cursor_position = model.input_buffer.len();
                }
            }
        }
        UiMessage::StartMoveToProject => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                if model.tasks.contains_key(&task_id) {
                    model.input_mode = InputMode::Editing;
                    model.input_target = InputTarget::MoveToProject(task_id);
                    // Build project list string for display in input buffer
                    // Format: "0: None, 1: ProjectA, 2: ProjectB, ..."
                    let mut options = vec!["0: (none)".to_string()];
                    for (i, project) in model.projects.values().enumerate() {
                        options.push(format!("{}: {}", i + 1, project.name));
                    }
                    model.input_buffer = options.join(", ");
                    model.cursor_position = model.input_buffer.len();
                }
            }
        }
        UiMessage::StartSearch => {
            model.input_mode = InputMode::Editing;
            model.input_target = InputTarget::Search;
            // Pre-fill with existing search text if any
            model.input_buffer = model.filter.search_text.clone().unwrap_or_default();
            model.cursor_position = model.input_buffer.len();
        }
        UiMessage::ClearSearch => {
            model.filter.search_text = None;
            model.refresh_visible_tasks();
        }
        UiMessage::StartFilterByTag => {
            model.input_mode = InputMode::Editing;
            model.input_target = InputTarget::FilterByTag;
            // Collect all unique tags from tasks
            let mut all_tags: Vec<String> = model
                .tasks
                .values()
                .flat_map(|t| t.tags.iter().cloned())
                .collect();
            all_tags.sort();
            all_tags.dedup();
            // Pre-fill with existing filter or show available tags as hint
            if let Some(ref tags) = model.filter.tags {
                model.input_buffer = tags.join(", ");
            } else if !all_tags.is_empty() {
                model.input_buffer = format!("Available: {}", all_tags.join(", "));
            } else {
                model.input_buffer.clear();
            }
            model.cursor_position = if model.filter.tags.is_some() {
                model.input_buffer.len()
            } else {
                0
            };
        }
        UiMessage::ClearTagFilter => {
            model.filter.tags = None;
            model.refresh_visible_tasks();
        }
        UiMessage::CycleSortField => {
            use crate::domain::SortField;
            model.sort.field = match model.sort.field {
                SortField::CreatedAt => SortField::Priority,
                SortField::Priority => SortField::DueDate,
                SortField::DueDate => SortField::Title,
                SortField::Title => SortField::Status,
                SortField::Status => SortField::UpdatedAt,
                SortField::UpdatedAt => SortField::CreatedAt,
            };
            model.refresh_visible_tasks();
        }
        UiMessage::ToggleSortOrder => {
            use crate::domain::SortOrder;
            model.sort.order = match model.sort.order {
                SortOrder::Ascending => SortOrder::Descending,
                SortOrder::Descending => SortOrder::Ascending,
            };
            model.refresh_visible_tasks();
        }
        UiMessage::CancelInput => {
            model.input_mode = InputMode::Normal;
            model.input_target = InputTarget::default();
            model.input_buffer.clear();
            model.cursor_position = 0;
        }
        UiMessage::SubmitInput => {
            handle_submit_input(model);
        }
        UiMessage::InputChar(c) => {
            // Check if we're editing time log
            if model.show_time_log
                && matches!(
                    model.time_log_mode,
                    crate::ui::TimeLogMode::EditStart | crate::ui::TimeLogMode::EditEnd
                )
            {
                model.time_log_buffer.push(c);
            } else {
                model.input_buffer.insert(model.cursor_position, c);
                model.cursor_position += 1;
            }
        }
        UiMessage::InputBackspace => {
            // Check if we're editing time log
            if model.show_time_log
                && matches!(
                    model.time_log_mode,
                    crate::ui::TimeLogMode::EditStart | crate::ui::TimeLogMode::EditEnd
                )
            {
                model.time_log_buffer.pop();
            } else if model.cursor_position > 0 {
                model.cursor_position -= 1;
                model.input_buffer.remove(model.cursor_position);
            }
        }
        UiMessage::InputDelete => {
            if model.cursor_position < model.input_buffer.len() {
                model.input_buffer.remove(model.cursor_position);
            }
        }
        UiMessage::InputCursorLeft => {
            model.cursor_position = model.cursor_position.saturating_sub(1);
        }
        UiMessage::InputCursorRight => {
            if model.cursor_position < model.input_buffer.len() {
                model.cursor_position += 1;
            }
        }
        UiMessage::InputCursorStart => {
            model.cursor_position = 0;
        }
        UiMessage::InputCursorEnd => {
            model.cursor_position = model.input_buffer.len();
        }
        // Delete confirmation
        UiMessage::ShowDeleteConfirm => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                if model.has_subtasks(&task_id) {
                    model.status_message = Some(
                        "Cannot delete: task has subtasks. Delete subtasks first.".to_string(),
                    );
                } else {
                    model.show_confirm_delete = true;
                }
            }
        }
        UiMessage::ConfirmDelete => {
            if let Some(id) = model.visible_tasks.get(model.selected_index).cloned() {
                if let Some(task) = model.tasks.remove(&id) {
                    // Collect time entries for this task before deleting
                    let task_entries: Vec<_> = model
                        .time_entries
                        .values()
                        .filter(|e| e.task_id == id)
                        .cloned()
                        .collect();

                    // Clear active time entry if it belongs to this task
                    if model
                        .active_time_entry
                        .as_ref()
                        .and_then(|entry_id| model.time_entries.get(entry_id))
                        .is_some_and(|e| e.task_id == id)
                    {
                        model.active_time_entry = None;
                    }

                    // Delete time entries (collect IDs first to avoid borrow issues)
                    let entry_ids: Vec<_> = task_entries.iter().map(|e| e.id.clone()).collect();
                    for entry_id in entry_ids {
                        model.delete_time_entry(&entry_id);
                    }

                    model.delete_task_from_storage(&id);
                    model.undo_stack.push(UndoAction::TaskDeleted {
                        task: Box::new(task),
                        time_entries: task_entries,
                    });
                }
                model.refresh_visible_tasks();
            }
            model.show_confirm_delete = false;
        }
        UiMessage::CancelDelete => {
            model.show_confirm_delete = false;
        }
        // Multi-select / Bulk operations
        UiMessage::ToggleMultiSelect => {
            model.multi_select_mode = !model.multi_select_mode;
            if !model.multi_select_mode {
                // Exiting multi-select mode clears selection
                model.selected_tasks.clear();
            }
        }
        UiMessage::ToggleTaskSelection => {
            if model.multi_select_mode {
                if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                    if model.selected_tasks.contains(&task_id) {
                        model.selected_tasks.remove(&task_id);
                    } else {
                        model.selected_tasks.insert(task_id);
                    }
                }
            }
        }
        UiMessage::SelectAll => {
            model.multi_select_mode = true;
            model.selected_tasks = model.visible_tasks.iter().cloned().collect();
        }
        UiMessage::ClearSelection => {
            model.selected_tasks.clear();
            model.multi_select_mode = false;
        }
        UiMessage::BulkDelete => {
            if model.multi_select_mode && !model.selected_tasks.is_empty() {
                // Delete all selected tasks
                let tasks_to_delete: Vec<_> = model.selected_tasks.iter().cloned().collect();
                for task_id in tasks_to_delete {
                    if let Some(task) = model.tasks.remove(&task_id) {
                        // Collect time entries for this task before deleting
                        let task_entries: Vec<_> = model
                            .time_entries
                            .values()
                            .filter(|e| e.task_id == task_id)
                            .cloned()
                            .collect();

                        // Clear active time entry if it belongs to this task
                        if model
                            .active_time_entry
                            .as_ref()
                            .and_then(|id| model.time_entries.get(id))
                            .is_some_and(|e| e.task_id == task_id)
                        {
                            model.active_time_entry = None;
                        }

                        // Delete time entries (collect IDs first to avoid borrow issues)
                        let entry_ids: Vec<_> = task_entries.iter().map(|e| e.id.clone()).collect();
                        for entry_id in entry_ids {
                            model.delete_time_entry(&entry_id);
                        }

                        model.delete_task_from_storage(&task_id);
                        model.undo_stack.push(UndoAction::TaskDeleted {
                            task: Box::new(task),
                            time_entries: task_entries,
                        });
                    }
                }
                model.selected_tasks.clear();
                model.multi_select_mode = false;
                model.refresh_visible_tasks();
            }
        }
        UiMessage::StartBulkMoveToProject => {
            if model.multi_select_mode && !model.selected_tasks.is_empty() {
                model.input_mode = InputMode::Editing;
                model.input_target = InputTarget::BulkMoveToProject;
                // Build project list string
                let mut options = vec!["0: (none)".to_string()];
                for (i, project) in model.projects.values().enumerate() {
                    options.push(format!("{}: {}", i + 1, project.name));
                }
                model.input_buffer = options.join(", ");
                model.cursor_position = model.input_buffer.len();
            }
        }
        UiMessage::StartBulkSetStatus => {
            if model.multi_select_mode && !model.selected_tasks.is_empty() {
                model.input_mode = InputMode::Editing;
                model.input_target = InputTarget::BulkSetStatus;
                model.input_buffer =
                    "1: Todo, 2: In Progress, 3: Blocked, 4: Done, 5: Cancelled".to_string();
                model.cursor_position = model.input_buffer.len();
            }
        }
        UiMessage::StartEditDependencies => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                if let Some(task) = model.tasks.get(&task_id) {
                    model.input_mode = InputMode::Editing;
                    model.input_target = InputTarget::EditDependencies(task_id.clone());
                    // Build list of available tasks with numbers
                    let mut buffer = String::new();
                    for (i, id) in model.visible_tasks.iter().enumerate() {
                        if *id != task.id {
                            if let Some(t) = model.tasks.get(id) {
                                let is_dep = task.dependencies.contains(id);
                                let marker = if is_dep { "*" } else { "" };
                                let _ = write!(buffer, "{}{}: {}, ", marker, i + 1, t.title);
                            }
                        }
                    }
                    if buffer.ends_with(", ") {
                        buffer.truncate(buffer.len() - 2);
                    }
                    model.input_buffer = buffer;
                    model.cursor_position = model.input_buffer.len();
                }
            }
        }
        UiMessage::StartEditRecurrence => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                if let Some(task) = model.tasks.get(&task_id) {
                    model.input_mode = InputMode::Editing;
                    model.input_target = InputTarget::EditRecurrence(task_id);
                    // Show current recurrence setting
                    let current = match &task.recurrence {
                        Some(crate::domain::Recurrence::Daily) => "d (daily)",
                        Some(crate::domain::Recurrence::Weekly { .. }) => "w (weekly)",
                        Some(crate::domain::Recurrence::Monthly { .. }) => "m (monthly)",
                        Some(crate::domain::Recurrence::Yearly { .. }) => "y (yearly)",
                        None => "0 (none)",
                    };
                    model.input_buffer = format!("Current: {current}");
                    model.cursor_position = model.input_buffer.len();
                }
            }
        }
        // Manual ordering
        UiMessage::MoveTaskUp => {
            handle_move_task(model, -1);
        }
        UiMessage::MoveTaskDown => {
            handle_move_task(model, 1);
        }
        // Task chains
        UiMessage::StartLinkTask => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                if model.tasks.contains_key(&task_id) {
                    model.input_mode = InputMode::Editing;
                    // Show current link if any
                    if let Some(task) = model.tasks.get(&task_id) {
                        if let Some(next_id) = &task.next_task_id {
                            if let Some(next_task) = model.tasks.get(next_id) {
                                model.input_buffer =
                                    format!("Currently linked to: {}", next_task.title);
                            } else {
                                model.input_buffer = String::new();
                            }
                        } else {
                            model.input_buffer = String::new();
                        }
                    }
                    model.input_target = InputTarget::LinkTask(task_id);
                    model.cursor_position = model.input_buffer.len();
                }
            }
        }
        UiMessage::UnlinkTask => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                // Only unlink if currently linked
                let is_linked = model
                    .tasks
                    .get(&task_id)
                    .is_some_and(|t| t.next_task_id.is_some());
                if is_linked {
                    model.modify_task_with_undo(&task_id, |task| {
                        task.next_task_id = None;
                    });
                }
            }
        }
        // Calendar navigation - delegated to helper
        UiMessage::CalendarPrevDay | UiMessage::CalendarNextDay => {
            handle_ui_calendar(model, msg);
        }
        // Macro recording/playback - delegated to helper
        UiMessage::StartRecordMacro | UiMessage::StopRecordMacro | UiMessage::PlayMacro(_) => {
            handle_ui_macros(model, msg);
        }
        // Template picker - delegated to helper
        UiMessage::ShowTemplates | UiMessage::HideTemplates | UiMessage::SelectTemplate(_) => {
            handle_ui_templates(model, msg);
        }
        // Keybindings editor - delegated to helper
        UiMessage::ShowKeybindingsEditor
        | UiMessage::HideKeybindingsEditor
        | UiMessage::KeybindingsUp
        | UiMessage::KeybindingsDown
        | UiMessage::StartEditKeybinding
        | UiMessage::CancelEditKeybinding
        | UiMessage::ApplyKeybinding(_)
        | UiMessage::ResetKeybinding
        | UiMessage::ResetAllKeybindings
        | UiMessage::SaveKeybindings
        | UiMessage::DismissOverdueAlert => {
            handle_ui_keybindings(model, msg);
        }
        // Time log editor - delegated to helper
        UiMessage::ShowTimeLog
        | UiMessage::HideTimeLog
        | UiMessage::TimeLogUp
        | UiMessage::TimeLogDown
        | UiMessage::TimeLogEditStart
        | UiMessage::TimeLogEditEnd
        | UiMessage::TimeLogConfirmDelete
        | UiMessage::TimeLogCancel
        | UiMessage::TimeLogSubmit
        | UiMessage::TimeLogAddEntry
        | UiMessage::TimeLogDelete => {
            handle_ui_time_log(model, msg);
        }
        // Saved filters - delegated to helper
        UiMessage::ShowSavedFilters
        | UiMessage::HideSavedFilters
        | UiMessage::SavedFilterUp
        | UiMessage::SavedFilterDown
        | UiMessage::ApplySavedFilter
        | UiMessage::SaveCurrentFilter
        | UiMessage::DeleteSavedFilter
        | UiMessage::ClearSavedFilter => {
            handle_ui_saved_filters(model, msg);
        }
        // Daily review - delegated to helper
        UiMessage::ShowDailyReview
        | UiMessage::HideDailyReview
        | UiMessage::DailyReviewNext
        | UiMessage::DailyReviewPrev
        | UiMessage::DailyReviewUp
        | UiMessage::DailyReviewDown
        | UiMessage::DailyReviewComplete => {
            handle_ui_daily_review(model, msg);
        }
        // Weekly review - delegated to helper
        UiMessage::ShowWeeklyReview
        | UiMessage::HideWeeklyReview
        | UiMessage::WeeklyReviewNext
        | UiMessage::WeeklyReviewPrev
        | UiMessage::WeeklyReviewUp
        | UiMessage::WeeklyReviewDown => {
            handle_ui_weekly_review(model, msg);
        }

        // Task snooze
        UiMessage::StartSnoozeTask => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                model.input_mode = InputMode::Editing;
                model.input_target = InputTarget::SnoozeTask(task_id);
                model.input_buffer.clear();
                model.cursor_position = 0;
            }
        }
        UiMessage::ClearSnooze => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                if let Some(task) = model.tasks.get_mut(&task_id) {
                    task.clear_snooze();
                    let task_clone = task.clone();
                    model.sync_task(&task_clone);
                    model.status_message = Some("Snooze cleared".to_string());
                    model.refresh_visible_tasks();
                }
            }
        }
    }
}

/// Handle input submission
#[allow(clippy::too_many_lines)]
fn handle_submit_input(model: &mut Model) {
    let input = model.input_buffer.trim().to_string();
    match &model.input_target {
        InputTarget::Task => {
            if !input.is_empty() {
                let task = create_task_from_quick_add(&input, model, None);
                model.sync_task(&task);
                model
                    .undo_stack
                    .push(UndoAction::TaskCreated(Box::new(task.clone())));
                model.tasks.insert(task.id.clone(), task);
                model.refresh_visible_tasks();
            }
        }
        InputTarget::Subtask(parent_id) => {
            if !input.is_empty() {
                let task = create_task_from_quick_add(&input, model, Some(parent_id.clone()));
                model.sync_task(&task);
                model
                    .undo_stack
                    .push(UndoAction::TaskCreated(Box::new(task.clone())));
                model.tasks.insert(task.id.clone(), task);
                model.refresh_visible_tasks();
            }
        }
        InputTarget::EditTask(task_id) => {
            let task_id = task_id.clone();
            if !input.is_empty() {
                model.modify_task_with_undo(&task_id, |task| {
                    task.title = input.clone();
                });
                model.refresh_visible_tasks();
            }
        }
        InputTarget::EditDueDate(task_id) => {
            let task_id = task_id.clone();
            // Parse date outside closure - empty clears, invalid keeps old
            let new_due = if input.is_empty() {
                Some(None) // Explicitly clear
            } else {
                parse_date(&input).map(Some) // Some(Some(date)) or None (invalid)
            };
            if let Some(due_date) = new_due {
                model.modify_task_with_undo(&task_id, |task| {
                    task.due_date = due_date;
                });
            }
            model.refresh_visible_tasks();
        }
        InputTarget::EditScheduledDate(task_id) => {
            let task_id = task_id.clone();
            // Parse date outside closure - empty clears, invalid keeps old
            let new_scheduled = if input.is_empty() {
                Some(None) // Explicitly clear
            } else {
                parse_date(&input).map(Some) // Some(Some(date)) or None (invalid)
            };
            if let Some(scheduled_date) = new_scheduled {
                model.modify_task_with_undo(&task_id, |task| {
                    task.scheduled_date = scheduled_date;
                });
            }
            model.refresh_visible_tasks();
        }
        InputTarget::EditTags(task_id) => {
            let task_id = task_id.clone();
            // Parse comma-separated tags outside closure
            let tags: Vec<String> = input
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            model.modify_task_with_undo(&task_id, |task| {
                task.tags = tags;
            });
            model.refresh_visible_tasks();
        }
        InputTarget::EditDescription(task_id) => {
            let task_id = task_id.clone();
            let description = if input.is_empty() {
                None
            } else {
                Some(input.clone())
            };
            model.modify_task_with_undo(&task_id, |task| {
                task.description = description;
            });
            model.refresh_visible_tasks();
        }
        InputTarget::Project => {
            if !input.is_empty() {
                let project = crate::domain::Project::new(input);
                model.sync_project(&project);
                model
                    .undo_stack
                    .push(UndoAction::ProjectCreated(Box::new(project.clone())));
                model.projects.insert(project.id.clone(), project);
            }
        }
        InputTarget::EditProject(project_id) => {
            let project_id = project_id.clone();
            // Only rename if input is non-empty and different from current name
            let should_rename = !input.is_empty()
                && model
                    .projects
                    .get(&project_id)
                    .is_some_and(|p| p.name != input);
            if should_rename {
                let new_name = input.clone();
                model.modify_project_with_undo(&project_id, |project| {
                    project.name = new_name.clone();
                });
                model.status_message = Some(format!("Renamed project to '{new_name}'"));
            }
        }
        InputTarget::Search => {
            if input.is_empty() {
                model.filter.search_text = None;
            } else {
                model.filter.search_text = Some(input);
            }
            model.refresh_visible_tasks();
        }
        InputTarget::MoveToProject(task_id) => {
            let task_id = task_id.clone();
            // Parse the number input to select a project
            if let Ok(choice) = input.parse::<usize>() {
                let project_ids: Vec<_> = model.projects.keys().cloned().collect();
                let new_project = if choice == 0 {
                    Some(None) // Remove from project
                } else {
                    project_ids.get(choice - 1).cloned().map(Some) // Move to project or None if invalid
                };
                if let Some(project_id) = new_project {
                    model.modify_task_with_undo(&task_id, |task| {
                        task.project_id = project_id;
                    });
                    model.refresh_visible_tasks();
                }
            }
        }
        InputTarget::FilterByTag => {
            if input.is_empty() || input.starts_with("Available:") {
                // Clear the tag filter
                model.filter.tags = None;
            } else {
                // Parse comma-separated tags, trim whitespace, filter empty
                let tags: Vec<String> = input
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if tags.is_empty() {
                    model.filter.tags = None;
                } else {
                    model.filter.tags = Some(tags);
                }
            }
            model.refresh_visible_tasks();
        }
        InputTarget::BulkMoveToProject => {
            if let Ok(choice) = input.parse::<usize>() {
                let project_ids: Vec<_> = model.projects.keys().cloned().collect();
                let target_project = if choice == 0 {
                    None
                } else {
                    project_ids.get(choice - 1).cloned()
                };

                // Move all selected tasks
                let tasks_to_move: Vec<_> = model.selected_tasks.iter().cloned().collect();
                for task_id in tasks_to_move {
                    let proj = target_project.clone();
                    model.modify_task_with_undo(&task_id, |task| {
                        task.project_id = proj;
                    });
                }
                model.selected_tasks.clear();
                model.multi_select_mode = false;
                model.refresh_visible_tasks();
            }
        }
        InputTarget::BulkSetStatus => {
            use crate::domain::TaskStatus;
            let status = match input.parse::<usize>() {
                Ok(1) => Some(TaskStatus::Todo),
                Ok(2) => Some(TaskStatus::InProgress),
                Ok(3) => Some(TaskStatus::Blocked),
                Ok(4) => Some(TaskStatus::Done),
                Ok(5) => Some(TaskStatus::Cancelled),
                _ => None,
            };

            if let Some(new_status) = status {
                let tasks_to_update: Vec<_> = model.selected_tasks.iter().cloned().collect();
                for task_id in tasks_to_update {
                    model.modify_task_with_undo(&task_id, |task| {
                        task.status = new_status;
                        if new_status.is_complete() && task.completed_at.is_none() {
                            task.completed_at = Some(chrono::Utc::now());
                        } else if !new_status.is_complete() {
                            task.completed_at = None;
                        }
                    });
                }
                model.selected_tasks.clear();
                model.multi_select_mode = false;
                model.refresh_visible_tasks();
            }
        }
        InputTarget::EditDependencies(task_id) => {
            let task_id = task_id.clone();
            // Parse task numbers from input
            let dep_indices: Vec<usize> = input
                .split(|c: char| !c.is_ascii_digit())
                .filter_map(|s| s.parse::<usize>().ok())
                .collect();

            // Convert indices to task IDs (can't depend on self)
            let new_deps: Vec<_> = dep_indices
                .iter()
                .filter_map(|i| model.visible_tasks.get(i.saturating_sub(1)).cloned())
                .filter(|id| *id != task_id)
                .collect();

            model.modify_task_with_undo(&task_id, |task| {
                task.dependencies = new_deps;
            });
            model.refresh_visible_tasks();
        }
        InputTarget::EditRecurrence(task_id) => {
            let task_id = task_id.clone();
            use crate::domain::Recurrence;
            use chrono::Datelike;
            // Parse recurrence from input (first char: d, w, m, y, 0)
            let first_char = input.chars().next().unwrap_or('0');
            let new_recurrence = match first_char {
                'd' | 'D' => Some(Recurrence::Daily),
                'w' | 'W' => Some(Recurrence::Weekly {
                    days: vec![], // Default to all days
                }),
                'm' | 'M' => Some(Recurrence::Monthly {
                    day: chrono::Utc::now().date_naive().day(),
                }),
                'y' | 'Y' => {
                    let today = chrono::Utc::now().date_naive();
                    Some(Recurrence::Yearly {
                        month: today.month(),
                        day: today.day(),
                    })
                }
                _ => None, // Invalid input or explicit 0/n/N clears recurrence
            };

            model.modify_task_with_undo(&task_id, |task| {
                task.recurrence = new_recurrence;
            });
            model.refresh_visible_tasks();
        }
        InputTarget::LinkTask(task_id) => {
            let task_id = task_id.clone();
            // Parse the input - support task number or task title search
            let target_task_id = if let Ok(num) = input.parse::<usize>() {
                // User entered a task number
                model.visible_tasks.get(num.saturating_sub(1)).cloned()
            } else {
                // User entered a task title - find matching task
                let input_lower = input.to_lowercase();
                model
                    .tasks
                    .iter()
                    .find(|(id, t)| {
                        **id != task_id && t.title.to_lowercase().contains(&input_lower)
                    })
                    .map(|(id, _)| id.clone())
            };

            // Don't allow linking to self
            if let Some(next_id) = target_task_id.filter(|id| *id != task_id) {
                model.modify_task_with_undo(&task_id, |task| {
                    task.next_task_id = Some(next_id);
                });
            }
        }
        InputTarget::ImportFilePath(_format) => {
            // File path entered, execute the import
            handle_execute_import(model);
            // Don't reset input mode here - handle_execute_import does it
            // and may show preview dialog
            return;
        }
        InputTarget::SavedFilterName => {
            if !input.is_empty() {
                // Create a new saved filter from current filter settings
                let saved_filter = crate::domain::SavedFilter::new(
                    input.clone(),
                    model.filter.clone(),
                    model.sort.clone(),
                );
                let filter_id = saved_filter.id.clone();
                model.saved_filters.insert(filter_id.clone(), saved_filter);
                model.active_saved_filter = Some(filter_id);
                model.dirty = true;
                model.status_message = Some(format!("Saved filter: {input}"));
            }
        }
        InputTarget::SnoozeTask(task_id) => {
            let task_id = task_id.clone();
            if input.is_empty() {
                // Clear snooze
                if let Some(task) = model.tasks.get_mut(&task_id) {
                    task.clear_snooze();
                    let task_clone = task.clone();
                    model.sync_task(&task_clone);
                    model.status_message = Some("Snooze cleared".to_string());
                }
            } else if let Some(date) = parse_date(&input) {
                // Set snooze date
                if let Some(task) = model.tasks.get_mut(&task_id) {
                    task.snooze_until_date(date);
                    let task_clone = task.clone();
                    model.sync_task(&task_clone);
                    model.status_message =
                        Some(format!("Snoozed until {}", date.format("%Y-%m-%d")));
                }
            } else {
                model.status_message = Some("Invalid date format".to_string());
            }
            model.refresh_visible_tasks();
        }
    }
    model.input_mode = InputMode::Normal;
    model.input_target = InputTarget::default();
    model.input_buffer.clear();
    model.cursor_position = 0;
}

/// Create a task from quick add input, applying parsed metadata
pub fn create_task_from_quick_add(
    input: &str,
    model: &Model,
    parent_id: Option<TaskId>,
) -> crate::domain::Task {
    let parsed = parse_quick_add(input);

    // Use parsed title, or original input if title is empty
    let title = if parsed.title.is_empty() {
        input.to_string()
    } else {
        parsed.title
    };

    // Start with basic task
    let mut task = crate::domain::Task::new(title);

    // Apply parent if provided
    if let Some(pid) = parent_id {
        task = task.with_parent(pid);
    }

    // Apply parsed priority, or default priority if none specified
    if let Some(priority) = parsed.priority {
        task.priority = priority;
    } else {
        task.priority = model.default_priority;
    }

    // Apply tags
    if !parsed.tags.is_empty() {
        task = task.with_tags(parsed.tags);
    }

    // Apply due date
    if let Some(due) = parsed.due_date {
        task = task.with_due_date(due);
    }

    // Apply scheduled date
    if let Some(sched) = parsed.scheduled_date {
        task.scheduled_date = Some(sched);
    }

    // Apply project by name (find matching project)
    if let Some(ref project_name) = parsed.project_name {
        let project_name_lower = project_name.to_lowercase();
        if let Some(project_id) = model
            .projects
            .values()
            .find(|p| p.name.to_lowercase() == project_name_lower)
            .map(|p| p.id.clone())
        {
            task.project_id = Some(project_id);
        }
    }

    task
}

/// Handle calendar navigation messages
fn handle_ui_calendar(model: &mut Model, msg: UiMessage) {
    if model.current_view != ViewId::Calendar {
        return;
    }

    match msg {
        UiMessage::CalendarPrevDay => {
            if let Some(day) = model.calendar_state.selected_day {
                if day > 1 {
                    model.calendar_state.selected_day = Some(day - 1);
                } else {
                    // Go to previous month's last day
                    if model.calendar_state.month == 1 {
                        model.calendar_state.month = 12;
                        model.calendar_state.year -= 1;
                    } else {
                        model.calendar_state.month -= 1;
                    }
                    let days = days_in_month(model.calendar_state.year, model.calendar_state.month);
                    model.calendar_state.selected_day = Some(days);
                }
                model.selected_index = 0;
                model.refresh_visible_tasks();
            }
        }
        UiMessage::CalendarNextDay => {
            if let Some(day) = model.calendar_state.selected_day {
                let days = days_in_month(model.calendar_state.year, model.calendar_state.month);
                if day < days {
                    model.calendar_state.selected_day = Some(day + 1);
                } else {
                    // Go to next month's first day
                    if model.calendar_state.month == 12 {
                        model.calendar_state.month = 1;
                        model.calendar_state.year += 1;
                    } else {
                        model.calendar_state.month += 1;
                    }
                    model.calendar_state.selected_day = Some(1);
                }
                model.selected_index = 0;
                model.refresh_visible_tasks();
            }
        }
        _ => {}
    }
}

/// Handle macro recording and playback messages
fn handle_ui_macros(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::StartRecordMacro => {
            if model.macro_state.is_recording() {
                // Already recording - treat as entering slot number mode
                model.pending_macro_slot = Some(0); // Will be set by digit input
                model.status_message = Some("Press 0-9 to select macro slot".to_string());
            } else if let Some(slot) = model.pending_macro_slot.take() {
                // We have a pending slot, start recording
                if model.macro_state.start_recording(slot) {
                    model.status_message = Some(format!("Recording macro {slot}..."));
                }
            } else {
                // First press - prompt for slot
                model.pending_macro_slot = Some(0);
                model.status_message = Some("Press 0-9 to start recording macro".to_string());
            }
        }
        UiMessage::StopRecordMacro => {
            if let Some(slot) = model.pending_macro_slot.take() {
                if model.macro_state.is_recording() {
                    if model.macro_state.stop_recording(slot) {
                        model.status_message = Some(format!("Macro {slot} saved"));
                    } else {
                        model.status_message = Some("Macro was empty, not saved".to_string());
                    }
                }
            } else if model.macro_state.is_recording() {
                // No slot specified, cancel recording
                model.macro_state.cancel_recording();
                model.status_message = Some("Recording cancelled".to_string());
            }
        }
        UiMessage::PlayMacro(slot) => {
            // Playback is handled in main.rs by dispatching stored messages
            if model.macro_state.has_macro(slot) {
                model.status_message = Some(format!("Playing macro {slot}..."));
            } else {
                model.status_message = Some(format!("No macro in slot {slot}"));
            }
        }
        _ => {}
    }
}

/// Handle template picker messages
fn handle_ui_templates(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ShowTemplates => {
            model.show_templates = true;
            model.template_selected = 0;
        }
        UiMessage::HideTemplates => {
            model.show_templates = false;
        }
        UiMessage::SelectTemplate(index) => {
            if let Some(template) = model.template_manager.get(index) {
                // Create a new task from the template
                let mut task = template.create_task();

                // Apply default priority from settings if template has none
                if task.priority == crate::domain::Priority::None {
                    task.priority = model.default_priority;
                }

                // Push undo action
                model
                    .undo_stack
                    .push(UndoAction::TaskCreated(Box::new(task.clone())));

                // Store the task
                model.sync_task(&task);
                model.tasks.insert(task.id.clone(), task.clone());

                // Start editing the task title
                model.input_mode = InputMode::Editing;
                model.input_target = InputTarget::EditTask(task.id);
                model.input_buffer = task.title;
                model.cursor_position = model.input_buffer.len();

                model.show_templates = false;
                model.refresh_visible_tasks();
            }
        }
        _ => {}
    }
}

/// Handle keybindings editor messages
fn handle_ui_keybindings(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ShowKeybindingsEditor => {
            model.show_keybindings_editor = true;
            model.keybinding_selected = 0;
            model.keybinding_capturing = false;
        }
        UiMessage::HideKeybindingsEditor => {
            model.show_keybindings_editor = false;
            model.keybinding_capturing = false;
        }
        UiMessage::KeybindingsUp => {
            if model.keybinding_selected > 0 {
                model.keybinding_selected -= 1;
            }
        }
        UiMessage::KeybindingsDown => {
            let bindings = model.keybindings.sorted_bindings();
            if model.keybinding_selected < bindings.len().saturating_sub(1) {
                model.keybinding_selected += 1;
            }
        }
        UiMessage::StartEditKeybinding => {
            model.keybinding_capturing = true;
            model.status_message = Some("Press a key combination...".to_string());
        }
        UiMessage::CancelEditKeybinding => {
            model.keybinding_capturing = false;
            model.status_message = None;
        }
        UiMessage::ApplyKeybinding(new_key) => {
            let bindings = model.keybindings.sorted_bindings();
            if let Some((_, action)) = bindings.get(model.keybinding_selected) {
                // Check for conflict
                if let Some(existing_action) = model.keybindings.get_action(&new_key) {
                    if existing_action != action {
                        model.status_message = Some(format!(
                            "Key '{new_key}' already bound to {:?}. Overwriting...",
                            existing_action
                        ));
                    }
                }
                model
                    .keybindings
                    .set_binding(new_key.clone(), action.clone());
                model.status_message = Some(format!("Bound '{new_key}' to {:?}", action));
            }
            model.keybinding_capturing = false;
        }
        UiMessage::ResetKeybinding => {
            let bindings = model.keybindings.sorted_bindings();
            if let Some((_, action)) = bindings.get(model.keybinding_selected) {
                // Find the default key for this action
                let defaults = crate::config::Keybindings::default();
                if let Some(default_key) = defaults.key_for_action(action) {
                    model
                        .keybindings
                        .set_binding(default_key.clone(), action.clone());
                    model.status_message = Some(format!(
                        "Reset {:?} to default key '{}'",
                        action, default_key
                    ));
                } else {
                    model.status_message = Some("No default binding for this action".to_string());
                }
            }
        }
        UiMessage::ResetAllKeybindings => {
            model.keybindings = crate::config::Keybindings::default();
            model.status_message = Some("All keybindings reset to defaults".to_string());
        }
        UiMessage::SaveKeybindings => match model.keybindings.save() {
            Ok(()) => {
                model.status_message = Some("Keybindings saved".to_string());
            }
            Err(e) => {
                model.status_message = Some(format!("Failed to save keybindings: {e}"));
            }
        },
        UiMessage::DismissOverdueAlert => {
            model.show_overdue_alert = false;
        }
        _ => {}
    }
}

/// Handle time log editor messages
fn handle_ui_time_log(model: &mut Model, msg: UiMessage) {
    use crate::ui::TimeLogMode;
    use chrono::{NaiveTime, Utc};

    match msg {
        UiMessage::ShowTimeLog => {
            // Only show if a task is selected
            if model.selected_index < model.visible_tasks.len() {
                model.show_time_log = true;
                model.time_log_selected = 0;
                model.time_log_mode = TimeLogMode::Browse;
                model.time_log_buffer.clear();
            }
        }
        UiMessage::HideTimeLog => {
            model.show_time_log = false;
            model.time_log_mode = TimeLogMode::Browse;
            model.time_log_buffer.clear();
        }
        UiMessage::TimeLogUp => {
            if model.time_log_selected > 0 {
                model.time_log_selected -= 1;
            }
        }
        UiMessage::TimeLogDown => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index) {
                let entries = model.time_entries_for_task(task_id);
                if model.time_log_selected < entries.len().saturating_sub(1) {
                    model.time_log_selected += 1;
                }
            }
        }
        UiMessage::TimeLogEditStart => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                let entries = model.time_entries_for_task(&task_id);
                if let Some(entry) = entries.get(model.time_log_selected) {
                    let start_time = entry.started_at.format("%H:%M").to_string();
                    model.time_log_mode = TimeLogMode::EditStart;
                    model.time_log_buffer = start_time;
                }
            }
        }
        UiMessage::TimeLogEditEnd => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                let entries = model.time_entries_for_task(&task_id);
                if let Some(entry) = entries.get(model.time_log_selected) {
                    // Can't edit end time of running entry
                    if entry.is_running() {
                        model.status_message =
                            Some("Cannot edit end time of running entry".to_string());
                        return;
                    }
                    let end_time = entry
                        .ended_at
                        .map(|t| t.format("%H:%M").to_string())
                        .unwrap_or_default();
                    model.time_log_mode = TimeLogMode::EditEnd;
                    model.time_log_buffer = end_time;
                }
            }
        }
        UiMessage::TimeLogConfirmDelete => {
            model.time_log_mode = TimeLogMode::ConfirmDelete;
        }
        UiMessage::TimeLogCancel => {
            model.time_log_mode = TimeLogMode::Browse;
            model.time_log_buffer.clear();
        }
        UiMessage::TimeLogSubmit => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                let entries = model.time_entries_for_task(&task_id);
                if let Some(entry) = entries.get(model.time_log_selected) {
                    let entry_id = entry.id.clone();

                    // Parse the time from buffer (HH:MM format)
                    if let Ok(time) = NaiveTime::parse_from_str(&model.time_log_buffer, "%H:%M") {
                        if let Some(entry) = model.time_entries.get_mut(&entry_id) {
                            let before = entry.clone();

                            match model.time_log_mode {
                                TimeLogMode::EditStart => {
                                    // Update start time, keeping the same date
                                    let date = entry.started_at.date_naive();
                                    if let Some(new_dt) =
                                        date.and_time(time).and_local_timezone(Utc).single()
                                    {
                                        entry.started_at = new_dt;
                                        // Recalculate duration if entry is completed
                                        if let Some(ended_at) = entry.ended_at {
                                            let duration =
                                                ended_at.signed_duration_since(entry.started_at);
                                            entry.duration_minutes =
                                                Some(duration.num_minutes().max(0) as u32);
                                        }
                                    }
                                }
                                TimeLogMode::EditEnd => {
                                    // Update end time, keeping the same date
                                    let date = entry.started_at.date_naive();
                                    if let Some(new_dt) =
                                        date.and_time(time).and_local_timezone(Utc).single()
                                    {
                                        entry.ended_at = Some(new_dt);
                                        let duration =
                                            new_dt.signed_duration_since(entry.started_at);
                                        entry.duration_minutes =
                                            Some(duration.num_minutes().max(0) as u32);
                                    }
                                }
                                _ => {}
                            }

                            let after = entry.clone();
                            model.undo_stack.push(UndoAction::TimeEntryModified {
                                before: Box::new(before),
                                after: Box::new(after.clone()),
                            });
                            model.sync_time_entry(&after);
                            model.status_message = Some("Time entry updated".to_string());
                        }
                    } else {
                        model.status_message = Some("Invalid time format. Use HH:MM".to_string());
                        return;
                    }
                }
            }
            model.time_log_mode = TimeLogMode::Browse;
            model.time_log_buffer.clear();
        }
        UiMessage::TimeLogAddEntry => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                // Create a new 30-minute entry ending now
                let ended_at = Utc::now();
                let started_at = ended_at - chrono::Duration::minutes(30);

                let mut entry = crate::domain::TimeEntry::start(task_id);
                entry.started_at = started_at;
                entry.ended_at = Some(ended_at);
                entry.duration_minutes = Some(30);

                model
                    .undo_stack
                    .push(UndoAction::TimeEntryStarted(Box::new(entry.clone())));
                model.sync_time_entry(&entry);
                model.time_entries.insert(entry.id.clone(), entry);
                model.time_log_selected = 0; // New entry will be at top (sorted by date)
                model.status_message = Some("Added 30-minute time entry".to_string());
            }
        }
        UiMessage::TimeLogDelete => {
            if model.time_log_mode == TimeLogMode::ConfirmDelete {
                if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                    let entries = model.time_entries_for_task(&task_id);
                    if let Some(entry) = entries.get(model.time_log_selected) {
                        let entry_id = entry.id.clone();

                        // Can't delete if it's the active entry
                        if model.active_time_entry.as_ref() == Some(&entry_id) {
                            model.status_message =
                                Some("Cannot delete running time entry".to_string());
                            model.time_log_mode = TimeLogMode::Browse;
                            return;
                        }

                        if let Some(removed) = model.time_entries.remove(&entry_id) {
                            model
                                .undo_stack
                                .push(UndoAction::TimeEntryDeleted(Box::new(removed.clone())));
                            model.delete_time_entry(&entry_id);

                            // Adjust selection
                            let remaining = model.time_entries_for_task(&task_id);
                            if model.time_log_selected >= remaining.len() && !remaining.is_empty() {
                                model.time_log_selected = remaining.len() - 1;
                            }
                            model.status_message = Some("Time entry deleted".to_string());
                        }
                    }
                }
                model.time_log_mode = TimeLogMode::Browse;
            }
        }
        _ => {}
    }
}

/// Handle saved filter UI messages
fn handle_ui_saved_filters(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ShowSavedFilters => {
            model.show_saved_filter_picker = true;
            model.saved_filter_selected = 0;
        }
        UiMessage::HideSavedFilters => {
            model.show_saved_filter_picker = false;
        }
        UiMessage::SavedFilterUp => {
            if model.saved_filter_selected > 0 {
                model.saved_filter_selected -= 1;
            }
        }
        UiMessage::SavedFilterDown => {
            let count = model.saved_filters.len();
            if count > 0 && model.saved_filter_selected < count - 1 {
                model.saved_filter_selected += 1;
            }
        }
        UiMessage::ApplySavedFilter => {
            // Get the sorted filter list and find selected filter
            let mut filter_list: Vec<_> = model.saved_filters.values().collect();
            filter_list.sort_by(|a, b| a.name.cmp(&b.name));

            if let Some(saved_filter) = filter_list.get(model.saved_filter_selected) {
                // Clone data we need before modifying model
                let filter = saved_filter.filter.clone();
                let sort = saved_filter.sort.clone();
                let filter_id = saved_filter.id.clone();
                let filter_name = saved_filter.name.clone();

                // Apply the filter and sort
                model.filter = filter;
                model.sort = sort;
                model.active_saved_filter = Some(filter_id);
                model.show_saved_filter_picker = false;
                model.refresh_visible_tasks();
                model.status_message = Some(format!("Applied filter: {filter_name}"));
            }
        }
        UiMessage::SaveCurrentFilter => {
            // Start input mode to name the filter
            model.input_mode = InputMode::Editing;
            model.input_target = InputTarget::SavedFilterName;
            model.input_buffer.clear();
            model.cursor_position = 0;
            model.show_saved_filter_picker = false;
        }
        UiMessage::DeleteSavedFilter => {
            // Get the sorted filter list
            let mut filter_list: Vec<_> = model.saved_filters.values().collect();
            filter_list.sort_by(|a, b| a.name.cmp(&b.name));

            if let Some(saved_filter) = filter_list.get(model.saved_filter_selected) {
                let id_to_remove = saved_filter.id.clone();
                let name = saved_filter.name.clone();

                // Clear active filter if it's being deleted
                if model.active_saved_filter.as_ref() == Some(&id_to_remove) {
                    model.active_saved_filter = None;
                }

                model.saved_filters.remove(&id_to_remove);
                model.dirty = true;

                // Adjust selection
                if model.saved_filter_selected > 0
                    && model.saved_filter_selected >= model.saved_filters.len()
                {
                    model.saved_filter_selected = model.saved_filters.len().saturating_sub(1);
                }

                model.status_message = Some(format!("Deleted filter: {name}"));
            }
        }
        UiMessage::ClearSavedFilter => {
            model.active_saved_filter = None;
            model.filter = crate::domain::Filter::default();
            model.sort = crate::domain::SortSpec::default();
            model.refresh_visible_tasks();
            model.status_message = Some("Filter cleared".to_string());
        }
        _ => {}
    }
}

/// Handle daily review UI messages
fn handle_ui_daily_review(model: &mut Model, msg: UiMessage) {
    use crate::ui::DailyReviewPhase;

    match msg {
        UiMessage::ShowDailyReview => {
            model.show_daily_review = true;
            model.daily_review_phase = DailyReviewPhase::Welcome;
            model.daily_review_selected = 0;
        }
        UiMessage::HideDailyReview => {
            model.show_daily_review = false;
        }
        UiMessage::DailyReviewNext => {
            model.daily_review_phase = model.daily_review_phase.next();
            model.daily_review_selected = 0;
        }
        UiMessage::DailyReviewPrev => {
            model.daily_review_phase = model.daily_review_phase.prev();
            model.daily_review_selected = 0;
        }
        UiMessage::DailyReviewUp => {
            if model.daily_review_selected > 0 {
                model.daily_review_selected -= 1;
            }
        }
        UiMessage::DailyReviewDown => {
            // Get the task count for current phase
            let today = chrono::Utc::now().date_naive();
            let count = match model.daily_review_phase {
                DailyReviewPhase::OverdueTasks => model
                    .tasks
                    .values()
                    .filter(|t| !t.status.is_complete() && t.due_date.is_some_and(|d| d < today))
                    .count(),
                DailyReviewPhase::TodayTasks => model
                    .tasks
                    .values()
                    .filter(|t| !t.status.is_complete() && t.due_date == Some(today))
                    .count(),
                DailyReviewPhase::ScheduledTasks => model
                    .tasks
                    .values()
                    .filter(|t| {
                        !t.status.is_complete()
                            && t.scheduled_date == Some(today)
                            && t.due_date != Some(today)
                    })
                    .count(),
                _ => 0,
            };
            if count > 0 && model.daily_review_selected < count - 1 {
                model.daily_review_selected += 1;
            }
        }
        UiMessage::DailyReviewComplete => {
            // Get the task at the current selection and toggle its completion
            let today = chrono::Utc::now().date_naive();
            let task_ids: Vec<_> = match model.daily_review_phase {
                DailyReviewPhase::OverdueTasks => model
                    .tasks
                    .values()
                    .filter(|t| !t.status.is_complete() && t.due_date.is_some_and(|d| d < today))
                    .map(|t| t.id.clone())
                    .collect(),
                DailyReviewPhase::TodayTasks => model
                    .tasks
                    .values()
                    .filter(|t| !t.status.is_complete() && t.due_date == Some(today))
                    .map(|t| t.id.clone())
                    .collect(),
                DailyReviewPhase::ScheduledTasks => model
                    .tasks
                    .values()
                    .filter(|t| {
                        !t.status.is_complete()
                            && t.scheduled_date == Some(today)
                            && t.due_date != Some(today)
                    })
                    .map(|t| t.id.clone())
                    .collect(),
                _ => vec![],
            };

            if let Some(task_id) = task_ids.get(model.daily_review_selected).cloned() {
                model.modify_task_with_undo(&task_id, |task| {
                    task.toggle_complete();
                });
                model.status_message = Some("Task completed!".to_string());

                // Adjust selection if we just removed an item
                let new_count = task_ids.len().saturating_sub(1);
                if model.daily_review_selected >= new_count && new_count > 0 {
                    model.daily_review_selected = new_count - 1;
                }
            }
        }
        _ => {}
    }
}

/// Handle weekly review UI messages
fn handle_ui_weekly_review(model: &mut Model, msg: UiMessage) {
    use crate::ui::WeeklyReviewPhase;

    match msg {
        UiMessage::ShowWeeklyReview => {
            model.show_weekly_review = true;
            model.weekly_review_phase = WeeklyReviewPhase::Welcome;
            model.weekly_review_selected = 0;
        }
        UiMessage::HideWeeklyReview => {
            model.show_weekly_review = false;
        }
        UiMessage::WeeklyReviewNext => {
            model.weekly_review_phase = model.weekly_review_phase.next();
            model.weekly_review_selected = 0;
        }
        UiMessage::WeeklyReviewPrev => {
            model.weekly_review_phase = model.weekly_review_phase.prev();
            model.weekly_review_selected = 0;
        }
        UiMessage::WeeklyReviewUp => {
            if model.weekly_review_selected > 0 {
                model.weekly_review_selected -= 1;
            }
        }
        UiMessage::WeeklyReviewDown => {
            // Get the count for current phase
            let today = chrono::Utc::now().date_naive();
            let week_ago = today - chrono::Duration::days(7);
            let week_ahead = today + chrono::Duration::days(7);

            let count = match model.weekly_review_phase {
                WeeklyReviewPhase::CompletedTasks => model
                    .tasks
                    .values()
                    .filter(|t| {
                        t.status.is_complete()
                            && t.completed_at.is_some_and(|d| d.date_naive() >= week_ago)
                    })
                    .count(),
                WeeklyReviewPhase::OverdueTasks => model
                    .tasks
                    .values()
                    .filter(|t| !t.status.is_complete() && t.due_date.is_some_and(|d| d < today))
                    .count(),
                WeeklyReviewPhase::UpcomingWeek => model
                    .tasks
                    .values()
                    .filter(|t| {
                        !t.status.is_complete()
                            && t.due_date.is_some_and(|d| d >= today && d <= week_ahead)
                    })
                    .count(),
                WeeklyReviewPhase::StaleProjects => {
                    // Count stale projects
                    model
                        .projects
                        .iter()
                        .filter(|(id, _)| {
                            let task_count = model
                                .tasks
                                .values()
                                .filter(|t| {
                                    t.project_id.as_ref() == Some(*id) && !t.status.is_complete()
                                })
                                .count();
                            let has_recent = model.tasks.values().any(|t| {
                                t.project_id.as_ref() == Some(*id)
                                    && t.updated_at.date_naive() >= week_ago
                            });
                            task_count > 0 && !has_recent
                        })
                        .count()
                }
                _ => 0,
            };

            if count > 0 && model.weekly_review_selected < count - 1 {
                model.weekly_review_selected += 1;
            }
        }
        _ => {}
    }
}

/// Handle moving a task up or down in the list order
fn handle_move_task(model: &mut Model, direction: i32) {
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
