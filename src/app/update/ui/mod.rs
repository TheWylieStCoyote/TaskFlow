//! UI message dispatch handler.
//!
//! This module serves as the central dispatcher for all [`UiMessage`] variants.
//! It routes incoming UI messages to specialized sub-handlers based on message type,
//! following the TEA (The Elm Architecture) pattern used throughout TaskFlow.
//!
//! # Architecture
//!
//! The dispatcher pattern provides several benefits:
//! - **Separation of concerns**: Each sub-module handles a specific feature domain
//! - **Maintainability**: Related code is grouped together for easy navigation
//! - **Testability**: Sub-handlers can be tested independently
//!
//! # Sub-modules
//!
//! The following sub-modules handle specific UI message categories:
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`calendar`] | Calendar navigation and day selection |
//! | [`delete`] | Task and project deletion with confirmation |
//! | [`editors`] | Description, time log, and work log editors |
//! | [`filters`] | Search, tag filtering, and saved filter operations |
//! | [`input`] | Text input handling and quick-add parsing |
//! | [`keybindings`] | Keybinding editor with conflict detection |
//! | [`macros`] | Macro recording and playback |
//! | [`multi_select`] | Bulk selection and bulk operations |
//! | [`reviews`] | Daily and weekly review mode handling |
//! | [`task_ops`] | Task reordering and manipulation |
//! | [`templates`] | Template picker for task creation |
//! | [`time_tracking`] | Time entry management and Pomodoro timer |
//! | [`view_state`] | View switching, sidebar toggle, focus mode |
//!
//! # Message Flow
//!
//! ```text
//! UiMessage → handle_ui() → match msg {
//!     View state toggles    → view_state::*
//!     Delete operations     → delete::*
//!     Multi-select          → multi_select::*
//!     Calendar nav          → calendar::*
//!     Macros                → macros::*
//!     Templates             → templates::*
//!     Keybindings           → keybindings::*
//!     Time log              → time_tracking::*
//!     Work log              → editors::*
//!     Description editor    → editors::*
//!     Saved filters         → filters::*
//!     Reviews               → reviews::*
//!     Input handling        → inline in handle_ui()
//! }
//! ```
//!
//! # Example
//!
//! ```
//! use taskflow::app::{Model, Message, UiMessage, update};
//!
//! let mut model = Model::new();
//! assert!(model.show_sidebar); // Sidebar visible by default
//! update(&mut model, Message::Ui(UiMessage::ToggleSidebar));
//! assert!(!model.show_sidebar); // Sidebar now hidden
//! ```

mod calendar;
mod delete;
mod duplicates;
mod editors;
mod filters;
mod goals;
mod habits;
mod input;
mod keybindings;
mod macros;
mod multi_select;
mod reschedule;
mod reviews;
mod task_ops;
mod templates;
mod time_tracking;
mod view_state;
mod views;

use std::fmt::Write as _;

use crate::app::{Model, UiMessage, UndoAction};
use crate::ui::{InputMode, InputTarget};

use calendar::handle_ui_calendar;
use duplicates::handle_ui_duplicates;
use editors::{handle_ui_description_editor, handle_ui_work_log};
use filters::handle_ui_saved_filters;
use goals::handle_ui_goals;
use habits::handle_ui_habits;
use input::{handle_submit_input, start_input};
use keybindings::handle_ui_keybindings;
use macros::handle_ui_macros;
use reschedule::handle_ui_reschedule;
use reviews::{handle_ui_daily_review, handle_ui_weekly_review};
use task_ops::handle_move_task;
use templates::handle_ui_templates;
use time_tracking::handle_ui_time_log;
use views::handle_ui_views;

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

/// Handle a UI message by dispatching to the appropriate sub-handler.
///
/// This is the main entry point for UI message processing. It routes messages
/// to specialized handlers based on the message variant, updating the model
/// state accordingly.
///
/// # Arguments
///
/// * `model` - Mutable reference to the application model
/// * `msg` - The UI message to process
///
/// # Panics
///
/// This function does not panic under normal operation. Invalid state
/// transitions are handled gracefully with status messages.
#[allow(clippy::too_many_lines)]
pub fn handle_ui(model: &mut Model, msg: UiMessage) {
    match msg {
        // View state toggles - delegated to helper
        UiMessage::ToggleShowCompleted => view_state::toggle_show_completed(model),
        UiMessage::ToggleSidebar => view_state::toggle_sidebar(model),
        UiMessage::ShowHelp => view_state::show_help(model),
        UiMessage::HideHelp => view_state::hide_help(model),
        UiMessage::ToggleFocusMode => view_state::toggle_focus_mode(model),
        UiMessage::ToggleFullScreenFocus => view_state::toggle_full_screen_focus(model),
        UiMessage::AddToFocusQueue => view_state::add_to_focus_queue(model),
        UiMessage::ClearFocusQueue => view_state::clear_focus_queue(model),
        UiMessage::AdvanceFocusQueue => view_state::advance_focus_queue(model),
        // Input mode handling
        UiMessage::StartCreateTask => {
            start_input(model, InputTarget::Task, None);
        }
        UiMessage::StartQuickCapture => {
            start_input(model, InputTarget::QuickCapture, None);
        }
        UiMessage::StartCreateSubtask => {
            // Create a subtask under the currently selected task
            if let Some(parent_id) = model.selected_task_id() {
                if model.tasks.contains_key(&parent_id) {
                    start_input(model, InputTarget::Subtask(parent_id), None);
                }
            }
        }
        UiMessage::StartCreateProject => {
            start_input(model, InputTarget::Project, None);
        }
        UiMessage::StartEditProject => {
            // Edit the currently selected project (from sidebar)
            if let Some(ref project_id) = model.selected_project {
                if let Some(project) = model.projects.get(project_id) {
                    start_input(
                        model,
                        InputTarget::EditProject(*project_id),
                        Some(project.name.clone()),
                    );
                }
            } else {
                model.alerts.status_message =
                    Some("Select a project from the sidebar first".to_string());
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
                    model.storage.dirty = true;
                    model.refresh_visible_tasks();
                    model.alerts.status_message = Some(format!(
                        "Deleted project '{project_name}' (tasks unassigned)"
                    ));
                }
            } else {
                model.alerts.status_message =
                    Some("Select a project from the sidebar first".to_string());
            }
        }
        UiMessage::StartEditTask => {
            if let Some(task_id) = model.selected_task_id() {
                let prefill = model.tasks.get(&task_id).map(|t| t.title.clone());
                start_input(model, InputTarget::EditTask(task_id), prefill);
            }
        }
        UiMessage::StartEditDueDate => {
            if let Some(task_id) = model.selected_task_id() {
                let prefill = model
                    .tasks
                    .get(&task_id)
                    .and_then(|t| t.due_date.map(|d| d.format("%Y-%m-%d").to_string()));
                start_input(model, InputTarget::EditDueDate(task_id), prefill);
            }
        }
        UiMessage::StartEditScheduledDate => {
            if let Some(task_id) = model.selected_task_id() {
                let prefill = model
                    .tasks
                    .get(&task_id)
                    .and_then(|t| t.scheduled_date.map(|d| d.format("%Y-%m-%d").to_string()));
                start_input(model, InputTarget::EditScheduledDate(task_id), prefill);
            }
        }
        UiMessage::StartEditTags => {
            if let Some(task_id) = model.selected_task_id() {
                let prefill = model.tasks.get(&task_id).map(|t| t.tags.join(", "));
                start_input(model, InputTarget::EditTags(task_id), prefill);
            }
        }
        UiMessage::StartEditDescription => {
            if let Some(task_id) = model.selected_task_id() {
                let prefill = model
                    .tasks
                    .get(&task_id)
                    .and_then(|t| t.description.clone());
                start_input(model, InputTarget::EditDescription(task_id), prefill);
            }
        }
        UiMessage::StartEditEstimate => {
            if let Some(task_id) = model.selected_task_id() {
                let prefill = model
                    .tasks
                    .get(&task_id)
                    .and_then(|t| t.estimated_minutes.map(format_duration_input));

                // Show estimation suggestion if user has historical data and no current estimate
                if prefill.is_none() {
                    if let Some(task) = model.tasks.get(&task_id) {
                        let engine = crate::app::analytics::AnalyticsEngine::new(model);
                        // Try to suggest based on 60min default estimate
                        if let Some(suggestion) =
                            engine.suggest_estimate(60, task.project_id, &task.tags)
                        {
                            if suggestion.confidence >= 0.3 {
                                model.alerts.status_message = Some(format!(
                                    "Suggestion: {} ({})",
                                    format_duration_input(suggestion.suggested_minutes),
                                    suggestion.explanation
                                ));
                            }
                        }
                    }
                }

                start_input(model, InputTarget::EditEstimate(task_id), prefill);
            }
        }
        UiMessage::StartMoveToProject => {
            if let Some(task_id) = model.selected_task_id() {
                if model.tasks.contains_key(&task_id) {
                    model.input.mode = InputMode::Editing;
                    model.input.target = InputTarget::MoveToProject(task_id);
                    // Build project list string for display in input buffer
                    // Format: "0: None, 1: ProjectA, 2: ProjectB, ..."
                    let mut options = vec!["0: (none)".to_string()];
                    for (i, project) in model.projects.values().enumerate() {
                        options.push(format!("{}: {}", i + 1, project.name));
                    }
                    model.input.buffer = options.join(", ");
                    model.input.cursor = model.input.buffer.len();
                }
            }
        }
        UiMessage::StartSearch => {
            model.input.mode = InputMode::Editing;
            model.input.target = InputTarget::Search;
            // Pre-fill with existing search text if any
            model.input.buffer = model
                .filtering
                .filter
                .search_text
                .clone()
                .unwrap_or_default();
            model.input.cursor = model.input.buffer.len();
        }
        UiMessage::ClearSearch => {
            model.filtering.filter.search_text = None;
            model.refresh_visible_tasks();
        }
        UiMessage::StartFilterByTag => {
            model.input.mode = InputMode::Editing;
            model.input.target = InputTarget::FilterByTag;
            // Collect all unique tags from tasks
            let mut all_tags: Vec<String> = model
                .tasks
                .values()
                .flat_map(|t| t.tags.iter().cloned())
                .collect();
            all_tags.sort();
            all_tags.dedup();
            // Pre-fill with existing filter or show available tags as hint
            if let Some(ref tags) = model.filtering.filter.tags {
                model.input.buffer = tags.join(", ");
            } else if !all_tags.is_empty() {
                model.input.buffer = format!("Available: {}", all_tags.join(", "));
            } else {
                model.input.buffer.clear();
            }
            model.input.cursor = if model.filtering.filter.tags.is_some() {
                model.input.buffer.len()
            } else {
                0
            };
        }
        UiMessage::ClearTagFilter => {
            model.filtering.filter.tags = None;
            model.refresh_visible_tasks();
        }
        UiMessage::CycleSortField => {
            use crate::domain::SortField;
            model.filtering.sort.field = match model.filtering.sort.field {
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
            model.filtering.sort.order = match model.filtering.sort.order {
                SortOrder::Ascending => SortOrder::Descending,
                SortOrder::Descending => SortOrder::Ascending,
            };
            model.refresh_visible_tasks();
        }
        UiMessage::CancelInput => {
            model.input.clear();
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
                model.input.buffer.insert(model.input.cursor, c);
                model.input.cursor += 1;
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
            } else if model.input.cursor > 0 {
                model.input.cursor -= 1;
                model.input.buffer.remove(model.input.cursor);
            }
        }
        UiMessage::InputDelete => {
            if model.input.cursor < model.input.buffer.len() {
                model.input.buffer.remove(model.input.cursor);
            }
        }
        UiMessage::InputCursorLeft => {
            model.input.cursor = model.input.cursor.saturating_sub(1);
        }
        UiMessage::InputCursorRight => {
            if model.input.cursor < model.input.buffer.len() {
                model.input.cursor += 1;
            }
        }
        UiMessage::InputCursorStart => {
            model.input.cursor = 0;
        }
        UiMessage::InputCursorEnd => {
            model.input.cursor = model.input.buffer.len();
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
            if let Some(task_id) = model.selected_task_id() {
                if let Some(task) = model.tasks.get(&task_id) {
                    model.input.mode = InputMode::Editing;
                    model.input.target = InputTarget::EditDependencies(task_id);
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
                    model.input.buffer = buffer;
                    model.input.cursor = model.input.buffer.len();
                }
            }
        }
        UiMessage::StartEditRecurrence => {
            if let Some(task_id) = model.selected_task_id() {
                if let Some(task) = model.tasks.get(&task_id) {
                    model.input.mode = InputMode::Editing;
                    model.input.target = InputTarget::EditRecurrence(task_id);
                    // Show current recurrence setting
                    let current = match &task.recurrence {
                        Some(crate::domain::Recurrence::Daily) => "d (daily)",
                        Some(crate::domain::Recurrence::Weekly { .. }) => "w (weekly)",
                        Some(crate::domain::Recurrence::Monthly { .. }) => "m (monthly)",
                        Some(crate::domain::Recurrence::Yearly { .. }) => "y (yearly)",
                        None => "0 (none)",
                    };
                    model.input.buffer = format!("Current: {current}");
                    model.input.cursor = model.input.buffer.len();
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
            if let Some(task_id) = model.selected_task_id() {
                if model.tasks.contains_key(&task_id) {
                    model.input.mode = InputMode::Editing;
                    // Show current link if any
                    if let Some(task) = model.tasks.get(&task_id) {
                        if let Some(next_id) = &task.next_task_id {
                            if let Some(next_task) = model.tasks.get(next_id) {
                                model.input.buffer =
                                    format!("Currently linked to: {}", next_task.title);
                            } else {
                                model.input.buffer = String::new();
                            }
                        } else {
                            model.input.buffer = String::new();
                        }
                    }
                    model.input.target = InputTarget::LinkTask(task_id);
                    model.input.cursor = model.input.buffer.len();
                }
            }
        }
        UiMessage::UnlinkTask => {
            if let Some(task_id) = model.selected_task_id() {
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

        // Task snooze and quick reschedule - delegated to helper
        UiMessage::StartSnoozeTask
        | UiMessage::ClearSnooze
        | UiMessage::RescheduleTomorrow
        | UiMessage::RescheduleNextWeek
        | UiMessage::RescheduleNextMonday => {
            handle_ui_reschedule(model, msg);
        }

        // Habit tracking UI - delegated to helper
        UiMessage::StartCreateHabit
        | UiMessage::StartEditHabit(_)
        | UiMessage::HabitUp
        | UiMessage::HabitDown
        | UiMessage::HabitToggleToday
        | UiMessage::ShowHabitAnalytics
        | UiMessage::HideHabitAnalytics
        | UiMessage::HabitArchive
        | UiMessage::HabitDelete
        | UiMessage::HabitToggleShowArchived => {
            handle_ui_habits(model, msg);
        }

        // Goal/OKR tracking UI - delegated to helper
        UiMessage::StartCreateGoal
        | UiMessage::StartEditGoal(_)
        | UiMessage::StartCreateKeyResult => {
            handle_ui_goals(model, msg);
        }

        // View-specific selection - delegated to helper
        UiMessage::TimelineToggleDependencies
        | UiMessage::TimelineViewSelected
        | UiMessage::KanbanViewSelected
        | UiMessage::EisenhowerViewSelected
        | UiMessage::WeeklyPlannerViewSelected
        | UiMessage::NetworkViewSelected
        | UiMessage::ChainNext
        | UiMessage::ChainPrev => {
            handle_ui_views(model, msg);
        }

        // Burndown chart controls
        UiMessage::BurndownCycleWindow => {
            model.burndown_state.time_window = model.burndown_state.time_window.next();
            let label = model.burndown_state.time_window.label();
            model.alerts.status_message = Some(format!("Time window: {label}"));
        }
        UiMessage::BurndownToggleMode => {
            model.burndown_state.mode = model.burndown_state.mode.toggle();
            let label = model.burndown_state.mode.label();
            model.alerts.status_message = Some(format!("Mode: {label}"));
        }
        UiMessage::BurndownToggleScopeCreep => {
            model.burndown_state.show_scope_creep = !model.burndown_state.show_scope_creep;
            let status = if model.burndown_state.show_scope_creep {
                "enabled"
            } else {
                "disabled"
            };
            model.alerts.status_message = Some(format!("Scope creep display: {status}"));
        }

        // Duplicate detection controls - delegated to helper
        UiMessage::DismissDuplicate | UiMessage::MergeDuplicates | UiMessage::RefreshDuplicates => {
            handle_ui_duplicates(model, msg);
        }
        UiMessage::OpenInEditor => {
            // Open selected task's source file in editor (for git TODOs)
            // Extract data first to avoid borrow issues
            let git_location = model.selected_task().and_then(|task| {
                task.description.as_ref().and_then(|desc| {
                    desc.lines().next().and_then(|first_line| {
                        if first_line.starts_with("git:") {
                            let parts: Vec<&str> = first_line.splitn(3, ':').collect();
                            if parts.len() >= 3 {
                                Some((parts[1].to_string(), parts[2].to_string()))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                })
            });

            match git_location {
                Some((file, line)) => {
                    if let Ok(editor) = std::env::var("EDITOR") {
                        model.alerts.status_message =
                            Some(format!("Opening {file}:{line} in {editor}..."));
                        model.pending_editor_command = Some((editor, file, line));
                    } else {
                        model.alerts.status_message =
                            Some("Set $EDITOR environment variable to open files".to_string());
                    }
                }
                None => {
                    model.alerts.status_message = Some("Task has no git location info".to_string());
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
