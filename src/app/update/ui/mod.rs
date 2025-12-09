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

mod calendar;
mod delete;
mod editors;
mod filters;
mod input;
mod keybindings;
mod macros;
mod multi_select;
mod reviews;
mod task_ops;
mod templates;
mod time_tracking;
mod view_state;

use std::fmt::Write as _;

use crate::app::{Model, UiMessage, UndoAction};
use crate::ui::{InputMode, InputTarget};

use calendar::handle_ui_calendar;
use editors::{handle_ui_description_editor, handle_ui_work_log};
use filters::handle_ui_saved_filters;
use input::handle_submit_input;
use keybindings::handle_ui_keybindings;
use macros::handle_ui_macros;
use reviews::{handle_ui_daily_review, handle_ui_weekly_review};
use task_ops::handle_move_task;
use templates::handle_ui_templates;
use time_tracking::handle_ui_time_log;

// Re-export for external use
pub use input::create_task_from_quick_add;

/// Format duration in minutes as a human-readable string (e.g., "1h30m" or "45m")
fn format_duration_input(minutes: u32) -> String {
    let hours = minutes / 60;
    let mins = minutes % 60;
    match (hours, mins) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h{m}m"),
    }
}

/// Parse a duration string (e.g., "1h30m", "90m", "1.5h", "2h") into minutes.
/// Returns None if the input is empty or invalid.
fn parse_duration_input(input: &str) -> Option<u32> {
    let input = input.trim().to_lowercase();
    if input.is_empty() {
        return None;
    }

    // Try parsing as plain number (assume minutes)
    if let Ok(mins) = input.parse::<u32>() {
        return Some(mins);
    }

    // Try parsing as decimal hours (e.g., "1.5h")
    if let Some(hours_str) = input.strip_suffix('h') {
        if let Ok(hours) = hours_str.parse::<f64>() {
            return Some((hours * 60.0).round() as u32);
        }
    }

    // Try parsing as minutes only (e.g., "90m")
    if let Some(mins_str) = input.strip_suffix('m') {
        if !mins_str.contains('h') {
            if let Ok(mins) = mins_str.parse::<u32>() {
                return Some(mins);
            }
        }
    }

    // Try parsing as hours and minutes (e.g., "1h30m" or "1h 30m")
    let input = input.replace(' ', "");
    if let Some(h_pos) = input.find('h') {
        let hours_str = &input[..h_pos];
        let rest = &input[h_pos + 1..];

        if let Ok(hours) = hours_str.parse::<u32>() {
            let mins = if rest.is_empty() {
                0
            } else if let Some(mins_str) = rest.strip_suffix('m') {
                mins_str.parse::<u32>().unwrap_or(0)
            } else {
                rest.parse::<u32>().unwrap_or(0)
            };
            return Some(hours * 60 + mins);
        }
    }

    None
}

/// Handle UI messages
#[allow(clippy::too_many_lines)]
pub fn handle_ui(model: &mut Model, msg: UiMessage) {
    match msg {
        // View state toggles - delegated to helper
        UiMessage::ToggleShowCompleted => view_state::toggle_show_completed(model),
        UiMessage::ToggleSidebar => view_state::toggle_sidebar(model),
        UiMessage::ShowHelp => view_state::show_help(model),
        UiMessage::HideHelp => view_state::hide_help(model),
        UiMessage::ToggleFocusMode => view_state::toggle_focus_mode(model),
        // Input mode handling
        UiMessage::StartCreateTask => {
            model.input_mode = InputMode::Editing;
            model.input_target = InputTarget::Task;
            model.input_buffer.clear();
            model.cursor_position = 0;
        }
        UiMessage::StartQuickCapture => {
            model.input_mode = InputMode::Editing;
            model.input_target = InputTarget::QuickCapture;
            model.input_buffer.clear();
            model.cursor_position = 0;
        }
        UiMessage::StartCreateSubtask => {
            // Create a subtask under the currently selected task
            if let Some(parent_id) = model.visible_tasks.get(model.selected_index).copied() {
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
            if let Some(ref project_id) = model.selected_project {
                if let Some(project) = model.projects.get(project_id) {
                    model.input_mode = InputMode::Editing;
                    model.input_target = InputTarget::EditProject(*project_id);
                    model.input_buffer.clone_from(&project.name);
                    model.cursor_position = model.input_buffer.len();
                }
            } else {
                model.status_message = Some("Select a project from the sidebar first".to_string());
            }
        }
        UiMessage::DeleteProject => {
            // Delete the currently selected project (from sidebar)
            if let Some(project_id) = model.selected_project {
                if let Some(project) = model.projects.remove(&project_id) {
                    let project_name = project.name.clone();

                    // Find tasks in this project
                    let tasks_to_unassign: Vec<_> = model
                        .tasks
                        .iter()
                        .filter(|(_, task)| task.project_id.as_ref() == Some(&project_id))
                        .map(|(id, _)| *id)
                        .collect();

                    // Unassign tasks from this project
                    for task_id in tasks_to_unassign {
                        if let Some(task) = model.tasks.get_mut(&task_id) {
                            task.project_id = None;
                        }
                        model.sync_task_by_id(&task_id);
                    }

                    // Push undo action (project is already owned from remove)
                    model
                        .undo_stack
                        .push(UndoAction::ProjectDeleted(Box::new(project)));
                    // Clear selected project
                    model.selected_project = None;
                    model.dirty = true;
                    model.refresh_visible_tasks();
                    model.status_message = Some(format!(
                        "Deleted project '{project_name}' (tasks unassigned)"
                    ));
                }
            } else {
                model.status_message = Some("Select a project from the sidebar first".to_string());
            }
        }
        UiMessage::StartEditTask => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).copied() {
                if let Some(task) = model.tasks.get(&task_id) {
                    model.input_mode = InputMode::Editing;
                    model.input_target = InputTarget::EditTask(task_id);
                    model.input_buffer = task.title.clone();
                    model.cursor_position = model.input_buffer.len();
                }
            }
        }
        UiMessage::StartEditDueDate => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).copied() {
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
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).copied() {
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
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).copied() {
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
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).copied() {
                if let Some(task) = model.tasks.get(&task_id) {
                    model.input_mode = InputMode::Editing;
                    model.input_target = InputTarget::EditDescription(task_id);
                    // Pre-fill with existing description
                    model.input_buffer = task.description.clone().unwrap_or_default();
                    model.cursor_position = model.input_buffer.len();
                }
            }
        }
        UiMessage::StartEditEstimate => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).copied() {
                if let Some(task) = model.tasks.get(&task_id) {
                    model.input_mode = InputMode::Editing;
                    model.input_target = InputTarget::EditEstimate(task_id);
                    // Pre-fill with existing estimate in human-readable format
                    model.input_buffer = task
                        .estimated_minutes
                        .map(format_duration_input)
                        .unwrap_or_default();
                    model.cursor_position = model.input_buffer.len();
                }
            }
        }
        UiMessage::StartMoveToProject => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).copied() {
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
            if model.time_log.visible
                && matches!(
                    model.time_log.mode,
                    crate::ui::TimeLogMode::EditStart | crate::ui::TimeLogMode::EditEnd
                )
            {
                model.time_log.buffer.push(c);
            } else {
                model.input_buffer.insert(model.cursor_position, c);
                model.cursor_position += 1;
            }
        }
        UiMessage::InputBackspace => {
            // Check if we're editing time log
            if model.time_log.visible
                && matches!(
                    model.time_log.mode,
                    crate::ui::TimeLogMode::EditStart | crate::ui::TimeLogMode::EditEnd
                )
            {
                model.time_log.buffer.pop();
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
        // Delete confirmation - delegated to helper
        UiMessage::ShowDeleteConfirm => delete::show_delete_confirm(model),
        UiMessage::ConfirmDelete => delete::confirm_delete(model),
        UiMessage::CancelDelete => delete::cancel_delete(model),
        // Multi-select / Bulk operations - delegated to helper
        UiMessage::ToggleMultiSelect => multi_select::toggle_multi_select(model),
        UiMessage::ToggleTaskSelection => multi_select::toggle_task_selection(model),
        UiMessage::SelectAll => multi_select::select_all(model),
        UiMessage::ClearSelection => multi_select::clear_selection(model),
        UiMessage::BulkDelete => multi_select::bulk_delete(model),
        UiMessage::StartBulkMoveToProject => multi_select::start_bulk_move_to_project(model),
        UiMessage::StartBulkSetStatus => multi_select::start_bulk_set_status(model),
        UiMessage::StartEditDependencies => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).copied() {
                if let Some(task) = model.tasks.get(&task_id) {
                    model.input_mode = InputMode::Editing;
                    model.input_target = InputTarget::EditDependencies(task_id);
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
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).copied() {
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
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).copied() {
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
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).copied() {
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
        | UiMessage::DismissOverdueAlert
        | UiMessage::DismissStorageErrorAlert => {
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
        // Work log editor - delegated to helper
        UiMessage::ShowWorkLog
        | UiMessage::HideWorkLog
        | UiMessage::WorkLogUp
        | UiMessage::WorkLogDown
        | UiMessage::WorkLogView
        | UiMessage::WorkLogAdd
        | UiMessage::WorkLogEdit
        | UiMessage::WorkLogConfirmDelete
        | UiMessage::WorkLogCancel
        | UiMessage::WorkLogSubmit
        | UiMessage::WorkLogDelete
        | UiMessage::WorkLogInputChar(_)
        | UiMessage::WorkLogInputBackspace
        | UiMessage::WorkLogInputDelete
        | UiMessage::WorkLogCursorLeft
        | UiMessage::WorkLogCursorRight
        | UiMessage::WorkLogCursorUp
        | UiMessage::WorkLogCursorDown
        | UiMessage::WorkLogNewline
        | UiMessage::WorkLogCursorHome
        | UiMessage::WorkLogCursorEnd
        | UiMessage::WorkLogSearchStart
        | UiMessage::WorkLogSearchCancel
        | UiMessage::WorkLogSearchApply
        | UiMessage::WorkLogSearchClear
        | UiMessage::WorkLogSearchChar(_)
        | UiMessage::WorkLogSearchBackspace => {
            handle_ui_work_log(model, msg);
        }
        // Description editor - delegated to helper
        UiMessage::StartEditDescriptionMultiline
        | UiMessage::HideDescriptionEditor
        | UiMessage::DescriptionSubmit
        | UiMessage::DescriptionInputChar(_)
        | UiMessage::DescriptionInputBackspace
        | UiMessage::DescriptionInputDelete
        | UiMessage::DescriptionCursorLeft
        | UiMessage::DescriptionCursorRight
        | UiMessage::DescriptionCursorUp
        | UiMessage::DescriptionCursorDown
        | UiMessage::DescriptionNewline
        | UiMessage::DescriptionCursorHome
        | UiMessage::DescriptionCursorEnd => {
            handle_ui_description_editor(model, msg);
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
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).copied() {
                model.input_mode = InputMode::Editing;
                model.input_target = InputTarget::SnoozeTask(task_id);
                model.input_buffer.clear();
                model.cursor_position = 0;
            }
        }
        UiMessage::ClearSnooze => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).copied() {
                if let Some(task) = model.tasks.get_mut(&task_id) {
                    task.clear_snooze();
                }
                model.sync_task_by_id(&task_id);
                model.status_message = Some("Snooze cleared".to_string());
                model.refresh_visible_tasks();
            }
        }

        // Habit tracking UI
        UiMessage::StartCreateHabit => {
            model.input_mode = InputMode::Editing;
            model.input_target = InputTarget::NewHabit;
            model.input_buffer.clear();
            model.cursor_position = 0;
        }
        UiMessage::StartEditHabit(habit_id) => {
            if let Some(habit) = model.habits.get(&habit_id) {
                model.input_mode = InputMode::Editing;
                model.input_target = InputTarget::EditHabit(habit_id);
                model.input_buffer.clone_from(&habit.name);
                model.cursor_position = habit.name.len();
            }
        }
        UiMessage::HabitUp => {
            if model.habit_view.selected > 0 {
                model.habit_view.selected -= 1;
            }
        }
        UiMessage::HabitDown => {
            if !model.visible_habits.is_empty()
                && model.habit_view.selected < model.visible_habits.len() - 1
            {
                model.habit_view.selected += 1;
            }
        }
        UiMessage::HabitToggleToday => {
            if let Some(&habit_id) = model.visible_habits.get(model.habit_view.selected) {
                let today = chrono::Utc::now().date_naive();
                if let Some(habit) = model.habits.get_mut(&habit_id) {
                    let currently_completed = habit.is_completed_on(today);
                    habit.check_in(today, !currently_completed, None);
                }
                model.sync_habit_by_id(&habit_id);
            }
        }
        UiMessage::ShowHabitAnalytics => {
            model.habit_view.show_analytics = true;
        }
        UiMessage::HideHabitAnalytics => {
            model.habit_view.show_analytics = false;
        }
        UiMessage::HabitArchive => {
            if let Some(&habit_id) = model.visible_habits.get(model.habit_view.selected) {
                if let Some(habit) = model.habits.get_mut(&habit_id) {
                    habit.archived = true;
                    habit.updated_at = chrono::Utc::now();
                }
                model.sync_habit_by_id(&habit_id);
                model.refresh_visible_habits();
            }
        }
        UiMessage::HabitDelete => {
            if let Some(&habit_id) = model.visible_habits.get(model.habit_view.selected) {
                model.habits.remove(&habit_id);
                model.delete_habit_from_storage(&habit_id);
                model.refresh_visible_habits();
            }
        }
        UiMessage::HabitToggleShowArchived => {
            model.habit_view.show_archived = !model.habit_view.show_archived;
            model.refresh_visible_habits();
        }
        UiMessage::TimelineToggleDependencies => {
            model.timeline_state.show_dependencies = !model.timeline_state.show_dependencies;
        }
        UiMessage::TimelineViewSelected => {
            // Get timeline tasks (filtered and sorted same as timeline widget)
            let timeline_tasks: Vec<_> = model
                .visible_tasks
                .iter()
                .filter_map(|id| model.tasks.get(id))
                .filter(|t| t.scheduled_date.is_some() || t.due_date.is_some())
                .collect();

            // Get the selected task from timeline
            if let Some(task) = timeline_tasks.get(model.timeline_state.selected_task_index) {
                let task_id = task.id;
                // Find this task's position in visible_tasks
                if let Some(pos) = model.visible_tasks.iter().position(|id| *id == task_id) {
                    model.selected_index = pos;
                    model.focus_mode = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod duration_tests {
    use super::*;

    #[test]
    fn test_format_duration_input() {
        assert_eq!(format_duration_input(30), "30m");
        assert_eq!(format_duration_input(60), "1h");
        assert_eq!(format_duration_input(90), "1h30m");
        assert_eq!(format_duration_input(120), "2h");
        assert_eq!(format_duration_input(0), "0m");
        assert_eq!(format_duration_input(5), "5m");
    }

    #[test]
    fn test_parse_duration_plain_minutes() {
        assert_eq!(parse_duration_input("30"), Some(30));
        assert_eq!(parse_duration_input("0"), Some(0));
        assert_eq!(parse_duration_input("120"), Some(120));
    }

    #[test]
    fn test_parse_duration_minutes_suffix() {
        assert_eq!(parse_duration_input("30m"), Some(30));
        assert_eq!(parse_duration_input("90m"), Some(90));
        assert_eq!(parse_duration_input("0m"), Some(0));
    }

    #[test]
    fn test_parse_duration_hours() {
        assert_eq!(parse_duration_input("1h"), Some(60));
        assert_eq!(parse_duration_input("2h"), Some(120));
        assert_eq!(parse_duration_input("0h"), Some(0));
    }

    #[test]
    fn test_parse_duration_decimal_hours() {
        assert_eq!(parse_duration_input("1.5h"), Some(90));
        assert_eq!(parse_duration_input("0.5h"), Some(30));
        assert_eq!(parse_duration_input("2.25h"), Some(135));
    }

    #[test]
    fn test_parse_duration_hours_and_minutes() {
        assert_eq!(parse_duration_input("1h30m"), Some(90));
        assert_eq!(parse_duration_input("2h15m"), Some(135));
        assert_eq!(parse_duration_input("1h0m"), Some(60));
        assert_eq!(parse_duration_input("0h30m"), Some(30));
    }

    #[test]
    fn test_parse_duration_with_spaces() {
        assert_eq!(parse_duration_input("1h 30m"), Some(90));
        assert_eq!(parse_duration_input(" 30m "), Some(30));
        assert_eq!(parse_duration_input("  2h  "), Some(120));
    }

    #[test]
    fn test_parse_duration_case_insensitive() {
        assert_eq!(parse_duration_input("1H30M"), Some(90));
        assert_eq!(parse_duration_input("2H"), Some(120));
        assert_eq!(parse_duration_input("30M"), Some(30));
    }

    #[test]
    fn test_parse_duration_empty_and_invalid() {
        assert_eq!(parse_duration_input(""), None);
        assert_eq!(parse_duration_input("   "), None);
        assert_eq!(parse_duration_input("abc"), None);
        assert_eq!(parse_duration_input("h"), None);
        assert_eq!(parse_duration_input("m"), None);
    }

    #[test]
    fn test_format_and_parse_roundtrip() {
        for mins in [0, 15, 30, 45, 60, 90, 120, 135, 180] {
            let formatted = format_duration_input(mins);
            let parsed = parse_duration_input(&formatted);
            assert_eq!(parsed, Some(mins), "Roundtrip failed for {mins} minutes");
        }
    }
}
