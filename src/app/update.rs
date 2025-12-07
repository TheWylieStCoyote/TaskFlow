use std::fmt::Write as _;

use crate::domain::TaskId;
use crate::ui::{InputMode, InputTarget};

use super::{
    parse_date, parse_quick_add, FocusPane, Message, Model, NavigationMessage, PomodoroMessage,
    RunningState, SystemMessage, TaskMessage, TimeMessage, UiMessage, UndoAction, ViewId,
    SIDEBAR_FIRST_PROJECT_INDEX, SIDEBAR_PROJECTS_HEADER_INDEX, SIDEBAR_SEPARATOR_INDEX,
};

/// Main update function - heart of TEA pattern
pub fn update(model: &mut Model, message: Message) {
    // Record message if we're recording a macro
    if model.macro_state.is_recording() {
        model.macro_state.record(&message);
    }

    match message {
        Message::Navigation(msg) => handle_navigation(model, msg),
        Message::Task(msg) => handle_task(model, msg),
        Message::Time(msg) => handle_time(model, msg),
        Message::Pomodoro(msg) => handle_pomodoro(model, msg),
        Message::Ui(msg) => handle_ui(model, msg),
        Message::System(msg) => handle_system(model, msg),
        Message::None => {}
    }
}

#[allow(clippy::too_many_lines)]
fn handle_navigation(model: &mut Model, msg: NavigationMessage) {
    match msg {
        NavigationMessage::Up => match model.focus_pane {
            FocusPane::TaskList => {
                if model.current_view == ViewId::Calendar {
                    if model.calendar_state.focus_task_list {
                        // Navigate tasks in calendar task list
                        if model.selected_index > 0 {
                            model.selected_index -= 1;
                        }
                    } else {
                        // In calendar grid, up moves to previous week (or wraps)
                        handle_calendar_up(model);
                    }
                } else if model.selected_index > 0 {
                    model.selected_index -= 1;
                }
            }
            FocusPane::Sidebar => {
                if model.sidebar_selected > 0 {
                    model.sidebar_selected -= 1;
                    // Skip separator
                    if model.sidebar_selected == SIDEBAR_SEPARATOR_INDEX {
                        model.sidebar_selected = SIDEBAR_SEPARATOR_INDEX - 1;
                    }
                }
            }
        },
        NavigationMessage::Down => match model.focus_pane {
            FocusPane::TaskList => {
                if model.current_view == ViewId::Calendar {
                    if model.calendar_state.focus_task_list {
                        // Navigate tasks in calendar task list
                        let task_count = model.tasks_for_selected_day().len();
                        if model.selected_index < task_count.saturating_sub(1) {
                            model.selected_index += 1;
                        }
                    } else {
                        // In calendar grid, down moves to next week (or wraps)
                        handle_calendar_down(model);
                    }
                } else if model.selected_index < model.visible_tasks.len().saturating_sub(1) {
                    model.selected_index += 1;
                }
            }
            FocusPane::Sidebar => {
                let max_index = model.sidebar_item_count().saturating_sub(1);
                if model.sidebar_selected < max_index {
                    model.sidebar_selected += 1;
                    // Skip separator
                    if model.sidebar_selected == SIDEBAR_SEPARATOR_INDEX {
                        model.sidebar_selected = SIDEBAR_SEPARATOR_INDEX + 1;
                    }
                }
            }
        },
        NavigationMessage::First => match model.focus_pane {
            FocusPane::TaskList => model.selected_index = 0,
            FocusPane::Sidebar => model.sidebar_selected = 0,
        },
        NavigationMessage::Last => match model.focus_pane {
            FocusPane::TaskList => {
                if !model.visible_tasks.is_empty() {
                    model.selected_index = model.visible_tasks.len() - 1;
                }
            }
            FocusPane::Sidebar => {
                model.sidebar_selected = model.sidebar_item_count().saturating_sub(1);
            }
        },
        NavigationMessage::PageUp => match model.focus_pane {
            FocusPane::TaskList => {
                model.selected_index = model.selected_index.saturating_sub(10);
            }
            FocusPane::Sidebar => {
                model.sidebar_selected = model.sidebar_selected.saturating_sub(5);
            }
        },
        NavigationMessage::PageDown => match model.focus_pane {
            FocusPane::TaskList => {
                let max_index = model.visible_tasks.len().saturating_sub(1);
                model.selected_index = (model.selected_index + 10).min(max_index);
            }
            FocusPane::Sidebar => {
                let max_index = model.sidebar_item_count().saturating_sub(1);
                model.sidebar_selected = (model.sidebar_selected + 5).min(max_index);
            }
        },
        NavigationMessage::Select(index) => {
            if index < model.visible_tasks.len() {
                model.selected_index = index;
            }
        }
        NavigationMessage::GoToView(view_id) => {
            model.current_view = view_id;
            model.selected_index = 0;
            model.selected_project = None;
            model.refresh_visible_tasks();
        }
        NavigationMessage::FocusSidebar => {
            if model.show_sidebar {
                model.focus_pane = FocusPane::Sidebar;
            }
        }
        NavigationMessage::FocusTaskList => {
            model.focus_pane = FocusPane::TaskList;
        }
        NavigationMessage::SelectSidebarItem => {
            handle_sidebar_selection(model);
        }
        NavigationMessage::CalendarPrevMonth => {
            if model.calendar_state.month == 1 {
                model.calendar_state.month = 12;
                model.calendar_state.year -= 1;
            } else {
                model.calendar_state.month -= 1;
            }
            // Adjust selected day if it exceeds days in new month
            let days_in_month =
                days_in_month(model.calendar_state.year, model.calendar_state.month);
            if let Some(day) = model.calendar_state.selected_day {
                if day > days_in_month {
                    model.calendar_state.selected_day = Some(days_in_month);
                }
            }
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        NavigationMessage::CalendarNextMonth => {
            if model.calendar_state.month == 12 {
                model.calendar_state.month = 1;
                model.calendar_state.year += 1;
            } else {
                model.calendar_state.month += 1;
            }
            // Adjust selected day if it exceeds days in new month
            let days_in_month =
                days_in_month(model.calendar_state.year, model.calendar_state.month);
            if let Some(day) = model.calendar_state.selected_day {
                if day > days_in_month {
                    model.calendar_state.selected_day = Some(days_in_month);
                }
            }
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        NavigationMessage::CalendarSelectDay(day) => {
            model.calendar_state.selected_day = Some(day);
            model.calendar_state.focus_task_list = false; // Reset focus to grid
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        NavigationMessage::CalendarFocusTaskList => {
            if model.current_view == ViewId::Calendar {
                // Only focus task list if there are tasks for the selected day
                if !model.tasks_for_selected_day().is_empty() {
                    model.calendar_state.focus_task_list = true;
                    model.selected_index = 0;
                }
            }
        }
        NavigationMessage::CalendarFocusGrid => {
            model.calendar_state.focus_task_list = false;
        }
        NavigationMessage::ReportsNextPanel => {
            if model.current_view == ViewId::Reports {
                model.report_panel = model.report_panel.next();
            }
        }
        NavigationMessage::ReportsPrevPanel => {
            if model.current_view == ViewId::Reports {
                model.report_panel = model.report_panel.prev();
            }
        }
    }
}

/// Helper to get days in a month
fn days_in_month(year: i32, month: u32) -> u32 {
    use chrono::{Datelike, NaiveDate};
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .and_then(|d| d.pred_opt())
    .map(|d| d.day())
    .unwrap_or(28)
}

/// Handle calendar up navigation (move to previous week)
fn handle_calendar_up(model: &mut Model) {
    if let Some(day) = model.calendar_state.selected_day {
        if day > 7 {
            model.calendar_state.selected_day = Some(day - 7);
        } else {
            // Move to previous month, last row
            if model.calendar_state.month == 1 {
                model.calendar_state.month = 12;
                model.calendar_state.year -= 1;
            } else {
                model.calendar_state.month -= 1;
            }
            let days = days_in_month(model.calendar_state.year, model.calendar_state.month);
            // Try to land on same weekday in last week
            let new_day = days - (7 - day);
            model.calendar_state.selected_day = Some(new_day.max(1));
        }
        model.calendar_state.focus_task_list = false; // Reset focus to grid
        model.selected_index = 0;
        model.refresh_visible_tasks();
    }
}

/// Handle calendar down navigation (move to next week)
fn handle_calendar_down(model: &mut Model) {
    if let Some(day) = model.calendar_state.selected_day {
        let days = days_in_month(model.calendar_state.year, model.calendar_state.month);
        if day + 7 <= days {
            model.calendar_state.selected_day = Some(day + 7);
        } else {
            // Move to next month, first row
            if model.calendar_state.month == 12 {
                model.calendar_state.month = 1;
                model.calendar_state.year += 1;
            } else {
                model.calendar_state.month += 1;
            }
            // Try to land on same weekday in first week
            let new_day = (day + 7) - days;
            model.calendar_state.selected_day = Some(new_day.min(7));
        }
        model.calendar_state.focus_task_list = false; // Reset focus to grid
        model.selected_index = 0;
        model.refresh_visible_tasks();
    }
}

fn handle_sidebar_selection(model: &mut Model) {
    let selected = model.sidebar_selected;

    // Sidebar layout - see SIDEBAR_* constants in model.rs:
    // 0-11: View items (All Tasks, Today, Upcoming, Overdue, Scheduled,
    //       Calendar, Dashboard, Reports, Blocked, Untagged, No Project, Recent)
    // SIDEBAR_SEPARATOR_INDEX (12): Separator (skip)
    // SIDEBAR_PROJECTS_HEADER_INDEX (13): "Projects" header
    // SIDEBAR_FIRST_PROJECT_INDEX+ (14+): Individual projects

    // Helper to activate a view
    let activate_view = |model: &mut Model, view: ViewId| {
        model.current_view = view;
        model.selected_project = None;
        model.focus_pane = FocusPane::TaskList;
        model.selected_index = 0;
        model.refresh_visible_tasks();
    };

    match selected {
        0 => activate_view(model, ViewId::TaskList),
        1 => activate_view(model, ViewId::Today),
        2 => activate_view(model, ViewId::Upcoming),
        3 => activate_view(model, ViewId::Overdue),
        4 => activate_view(model, ViewId::Scheduled),
        5 => activate_view(model, ViewId::Calendar),
        6 => activate_view(model, ViewId::Dashboard),
        7 => activate_view(model, ViewId::Reports),
        8 => activate_view(model, ViewId::Blocked),
        9 => activate_view(model, ViewId::Untagged),
        10 => activate_view(model, ViewId::NoProject),
        11 => activate_view(model, ViewId::RecentlyModified),
        n if n == SIDEBAR_SEPARATOR_INDEX => {} // Separator, do nothing
        n if n == SIDEBAR_PROJECTS_HEADER_INDEX => {
            // Projects header - go to Projects view showing all project tasks
            activate_view(model, ViewId::Projects);
        }
        n if n >= SIDEBAR_FIRST_PROJECT_INDEX => {
            // Select a specific project
            let project_index = n - SIDEBAR_FIRST_PROJECT_INDEX;
            let project_ids: Vec<_> = model.projects.keys().cloned().collect();
            if let Some(project_id) = project_ids.get(project_index) {
                model.current_view = ViewId::TaskList;
                model.selected_project = Some(project_id.clone());
                model.focus_pane = FocusPane::TaskList;
                model.selected_index = 0;
                model.refresh_visible_tasks();
            }
        }
        _ => {}
    }
}

#[allow(clippy::too_many_lines)]
fn handle_task(model: &mut Model, msg: TaskMessage) {
    match msg {
        TaskMessage::ToggleComplete => {
            // Get the task id first to avoid borrow issues
            let task_id = model.visible_tasks.get(model.selected_index).cloned();

            if let Some(id) = task_id {
                // Check if completing a recurring task
                let next_task = model.tasks.get(&id).and_then(|task| {
                    if task.status != crate::domain::TaskStatus::Done && task.recurrence.is_some() {
                        // Create next occurrence
                        Some(create_next_recurring_task(task))
                    } else {
                        None
                    }
                });

                // Check for task chain - if completing and has next_task_id, schedule it
                let chain_next_id = model.tasks.get(&id).and_then(|task| {
                    if task.status != crate::domain::TaskStatus::Done {
                        task.next_task_id.clone()
                    } else {
                        None
                    }
                });

                // Check if we're completing (not uncompleting) to cascade to descendants
                let is_completing = model
                    .tasks
                    .get(&id)
                    .is_some_and(|t| t.status != crate::domain::TaskStatus::Done);

                // Get all descendants BEFORE modifying the task
                let descendants = if is_completing {
                    model.get_all_descendants(&id)
                } else {
                    Vec::new()
                };

                if let Some(task) = model.tasks.get_mut(&id) {
                    let before = task.clone();
                    task.toggle_complete();
                    let after = task.clone();
                    model.sync_task(&after);
                    model.undo_stack.push(UndoAction::TaskModified {
                        before: Box::new(before),
                        after: Box::new(after),
                    });
                }

                // Auto-complete all descendants when completing a parent task
                for descendant_id in descendants {
                    if let Some(descendant) = model.tasks.get_mut(&descendant_id) {
                        if !descendant.status.is_complete() {
                            let before = descendant.clone();
                            descendant.status = crate::domain::TaskStatus::Done;
                            descendant.updated_at = chrono::Utc::now();
                            let after = descendant.clone();
                            model.sync_task(&after);
                            model.undo_stack.push(UndoAction::TaskModified {
                                before: Box::new(before),
                                after: Box::new(after),
                            });
                        }
                    }
                }

                // Add the next recurring task if one was created
                if let Some(new_task) = next_task {
                    model.sync_task(&new_task);
                    model
                        .undo_stack
                        .push(UndoAction::TaskCreated(Box::new(new_task.clone())));
                    model.tasks.insert(new_task.id.clone(), new_task);
                }

                // Auto-schedule the next task in chain for today
                if let Some(next_id) = chain_next_id {
                    if let Some(next_task) = model.tasks.get_mut(&next_id) {
                        let before = next_task.clone();
                        next_task.scheduled_date = Some(chrono::Local::now().date_naive());
                        next_task.updated_at = chrono::Utc::now();
                        let after = next_task.clone();
                        model.sync_task(&after);
                        model.undo_stack.push(UndoAction::TaskModified {
                            before: Box::new(before),
                            after: Box::new(after),
                        });
                    }
                }
            }
            model.refresh_visible_tasks();
        }
        TaskMessage::SetStatus(task_id, status) => {
            if let Some(task) = model.tasks.get_mut(&task_id) {
                let before = task.clone();
                task.status = status;
                task.updated_at = chrono::Utc::now();
                let after = task.clone();
                model.sync_task(&after);
                model.undo_stack.push(UndoAction::TaskModified {
                    before: Box::new(before),
                    after: Box::new(after),
                });
            }
            model.refresh_visible_tasks();
        }
        TaskMessage::SetPriority(task_id, priority) => {
            if let Some(task) = model.tasks.get_mut(&task_id) {
                let before = task.clone();
                task.priority = priority;
                task.updated_at = chrono::Utc::now();
                let after = task.clone();
                model.sync_task(&after);
                model.undo_stack.push(UndoAction::TaskModified {
                    before: Box::new(before),
                    after: Box::new(after),
                });
            }
            model.refresh_visible_tasks();
        }
        TaskMessage::CyclePriority => {
            use crate::domain::Priority;
            let task_id = model.visible_tasks.get(model.selected_index).cloned();

            if let Some(id) = task_id {
                if let Some(task) = model.tasks.get_mut(&id) {
                    let before = task.clone();
                    task.priority = match task.priority {
                        Priority::None => Priority::Low,
                        Priority::Low => Priority::Medium,
                        Priority::Medium => Priority::High,
                        Priority::High => Priority::Urgent,
                        Priority::Urgent => Priority::None,
                    };
                    task.updated_at = chrono::Utc::now();
                    let after = task.clone();
                    model.sync_task(&after);
                    model.undo_stack.push(UndoAction::TaskModified {
                        before: Box::new(before),
                        after: Box::new(after),
                    });
                }
            }
            model.refresh_visible_tasks();
        }
        TaskMessage::Create(title) => {
            let task = crate::domain::Task::new(title).with_priority(model.default_priority);
            model.sync_task(&task);
            model
                .undo_stack
                .push(UndoAction::TaskCreated(Box::new(task.clone())));
            model.tasks.insert(task.id.clone(), task);
            model.refresh_visible_tasks();
        }
        TaskMessage::Delete(task_id) => {
            if let Some(task) = model.tasks.remove(&task_id) {
                model.delete_task_from_storage(&task_id);
                model
                    .undo_stack
                    .push(UndoAction::TaskDeleted(Box::new(task)));
            }
            model.refresh_visible_tasks();
        }
        TaskMessage::MoveToProject(task_id, project_id) => {
            if let Some(task) = model.tasks.get_mut(&task_id) {
                let before = task.clone();
                task.project_id = project_id;
                task.updated_at = chrono::Utc::now();
                let after = task.clone();
                model.sync_task(&after);
                model.undo_stack.push(UndoAction::TaskModified {
                    before: Box::new(before),
                    after: Box::new(after),
                });
            }
            model.refresh_visible_tasks();
        }
    }
}

#[allow(clippy::too_many_lines)]
fn handle_ui(model: &mut Model, msg: UiMessage) {
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
                        let task =
                            create_task_from_quick_add(&input, model, Some(parent_id.clone()));
                        model.sync_task(&task);
                        model
                            .undo_stack
                            .push(UndoAction::TaskCreated(Box::new(task.clone())));
                        model.tasks.insert(task.id.clone(), task);
                        model.refresh_visible_tasks();
                    }
                }
                InputTarget::EditTask(task_id) => {
                    if !input.is_empty() {
                        if let Some(task) = model.tasks.get_mut(task_id) {
                            let before = task.clone();
                            task.title = input;
                            task.updated_at = chrono::Utc::now();
                            let after = task.clone();
                            model.sync_task(&after);
                            model.undo_stack.push(UndoAction::TaskModified {
                                before: Box::new(before),
                                after: Box::new(after),
                            });
                        }
                        model.refresh_visible_tasks();
                    }
                }
                InputTarget::EditDueDate(task_id) => {
                    if let Some(task) = model.tasks.get_mut(task_id) {
                        let before = task.clone();
                        // Empty input clears the due date
                        if input.is_empty() {
                            task.due_date = None;
                        } else if let Some(date) = parse_date(&input) {
                            task.due_date = Some(date);
                        }
                        // If parsing fails, just ignore (keep old date)
                        task.updated_at = chrono::Utc::now();
                        let after = task.clone();
                        model.sync_task(&after);
                        model.undo_stack.push(UndoAction::TaskModified {
                            before: Box::new(before),
                            after: Box::new(after),
                        });
                    }
                    model.refresh_visible_tasks();
                }
                InputTarget::EditScheduledDate(task_id) => {
                    if let Some(task) = model.tasks.get_mut(task_id) {
                        let before = task.clone();
                        // Empty input clears the scheduled date
                        if input.is_empty() {
                            task.scheduled_date = None;
                        } else if let Some(date) = parse_date(&input) {
                            task.scheduled_date = Some(date);
                        }
                        // If parsing fails, just ignore (keep old date)
                        task.updated_at = chrono::Utc::now();
                        let after = task.clone();
                        model.sync_task(&after);
                        model.undo_stack.push(UndoAction::TaskModified {
                            before: Box::new(before),
                            after: Box::new(after),
                        });
                    }
                    model.refresh_visible_tasks();
                }
                InputTarget::EditTags(task_id) => {
                    if let Some(task) = model.tasks.get_mut(task_id) {
                        let before = task.clone();
                        // Parse comma-separated tags, trim whitespace, filter empty
                        task.tags = input
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        task.updated_at = chrono::Utc::now();
                        let after = task.clone();
                        model.sync_task(&after);
                        model.undo_stack.push(UndoAction::TaskModified {
                            before: Box::new(before),
                            after: Box::new(after),
                        });
                    }
                    model.refresh_visible_tasks();
                }
                InputTarget::EditDescription(task_id) => {
                    if let Some(task) = model.tasks.get_mut(task_id) {
                        let before = task.clone();
                        // Empty input clears the description
                        task.description = if input.is_empty() { None } else { Some(input) };
                        task.updated_at = chrono::Utc::now();
                        let after = task.clone();
                        model.sync_task(&after);
                        model.undo_stack.push(UndoAction::TaskModified {
                            before: Box::new(before),
                            after: Box::new(after),
                        });
                    }
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
                InputTarget::Search => {
                    if input.is_empty() {
                        model.filter.search_text = None;
                    } else {
                        model.filter.search_text = Some(input);
                    }
                    model.refresh_visible_tasks();
                }
                InputTarget::MoveToProject(task_id) => {
                    // Parse the number input to select a project
                    if let Ok(choice) = input.parse::<usize>() {
                        if let Some(task) = model.tasks.get_mut(task_id) {
                            let before = task.clone();
                            let project_ids: Vec<_> = model.projects.keys().cloned().collect();
                            if choice == 0 {
                                // Remove from project
                                task.project_id = None;
                            } else if let Some(project_id) = project_ids.get(choice - 1) {
                                // Move to selected project
                                task.project_id = Some(project_id.clone());
                            }
                            task.updated_at = chrono::Utc::now();
                            let after = task.clone();
                            model.sync_task(&after);
                            model.undo_stack.push(UndoAction::TaskModified {
                                before: Box::new(before),
                                after: Box::new(after),
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
                            if let Some(task) = model.tasks.get_mut(&task_id) {
                                let before = task.clone();
                                task.project_id.clone_from(&target_project);
                                task.updated_at = chrono::Utc::now();
                                let after = task.clone();
                                model.sync_task(&after);
                                model.undo_stack.push(UndoAction::TaskModified {
                                    before: Box::new(before),
                                    after: Box::new(after),
                                });
                            }
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
                        let tasks_to_update: Vec<_> =
                            model.selected_tasks.iter().cloned().collect();
                        for task_id in tasks_to_update {
                            if let Some(task) = model.tasks.get_mut(&task_id) {
                                let before = task.clone();
                                task.status = new_status;
                                task.updated_at = chrono::Utc::now();
                                if new_status.is_complete() && task.completed_at.is_none() {
                                    task.completed_at = Some(chrono::Utc::now());
                                } else if !new_status.is_complete() {
                                    task.completed_at = None;
                                }
                                let after = task.clone();
                                model.sync_task(&after);
                                model.undo_stack.push(UndoAction::TaskModified {
                                    before: Box::new(before),
                                    after: Box::new(after),
                                });
                            }
                        }
                        model.selected_tasks.clear();
                        model.multi_select_mode = false;
                        model.refresh_visible_tasks();
                    }
                }
                InputTarget::EditDependencies(task_id) => {
                    // Parse task numbers from input
                    let dep_indices: Vec<usize> = input
                        .split(|c: char| !c.is_ascii_digit())
                        .filter_map(|s| s.parse::<usize>().ok())
                        .collect();

                    // Convert indices to task IDs
                    let new_deps: Vec<_> = dep_indices
                        .iter()
                        .filter_map(|i| model.visible_tasks.get(i.saturating_sub(1)).cloned())
                        .filter(|id| id != task_id) // Can't depend on self
                        .collect();

                    if let Some(task) = model.tasks.get_mut(task_id) {
                        let before = task.clone();
                        task.dependencies = new_deps;
                        task.updated_at = chrono::Utc::now();
                        let after = task.clone();
                        model.sync_task(&after);
                        model.undo_stack.push(UndoAction::TaskModified {
                            before: Box::new(before),
                            after: Box::new(after),
                        });
                    }
                    model.refresh_visible_tasks();
                }
                InputTarget::EditRecurrence(task_id) => {
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

                    if let Some(task) = model.tasks.get_mut(task_id) {
                        let before = task.clone();
                        task.recurrence = new_recurrence;
                        task.updated_at = chrono::Utc::now();
                        let after = task.clone();
                        model.sync_task(&after);
                        model.undo_stack.push(UndoAction::TaskModified {
                            before: Box::new(before),
                            after: Box::new(after),
                        });
                    }
                    model.refresh_visible_tasks();
                }
                InputTarget::LinkTask(task_id) => {
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
                                *id != task_id && t.title.to_lowercase().contains(&input_lower)
                            })
                            .map(|(id, _)| id.clone())
                    };

                    if let Some(next_id) = target_task_id {
                        // Don't allow linking to self
                        if next_id != *task_id {
                            if let Some(task) = model.tasks.get_mut(task_id) {
                                let before = task.clone();
                                task.next_task_id = Some(next_id);
                                task.updated_at = chrono::Utc::now();
                                let after = task.clone();
                                model.sync_task(&after);
                                model.undo_stack.push(UndoAction::TaskModified {
                                    before: Box::new(before),
                                    after: Box::new(after),
                                });
                            }
                        }
                    }
                }
                InputTarget::ImportFilePath(_format) => {
                    // File path entered, execute the import
                    handle_execute_import(model);
                    // Don't reset input mode here - handle_execute_import does it
                    // and may show preview dialog
                    return;
                }
            }
            model.input_mode = InputMode::Normal;
            model.input_target = InputTarget::default();
            model.input_buffer.clear();
            model.cursor_position = 0;
        }
        UiMessage::InputChar(c) => {
            model.input_buffer.insert(model.cursor_position, c);
            model.cursor_position += 1;
        }
        UiMessage::InputBackspace => {
            if model.cursor_position > 0 {
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
                    model.delete_task_from_storage(&id);
                    model
                        .undo_stack
                        .push(UndoAction::TaskDeleted(Box::new(task)));
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
                        model.delete_task_from_storage(&task_id);
                        model
                            .undo_stack
                            .push(UndoAction::TaskDeleted(Box::new(task)));
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
                if let Some(task) = model.tasks.get_mut(&task_id) {
                    if task.next_task_id.is_some() {
                        let before = task.clone();
                        task.next_task_id = None;
                        task.updated_at = chrono::Utc::now();
                        let after = task.clone();
                        model.sync_task(&after);
                        model.undo_stack.push(UndoAction::TaskModified {
                            before: Box::new(before),
                            after: Box::new(after),
                        });
                    }
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
    }
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
    if let Some(current_task) = model.tasks.get_mut(&current_task_id) {
        let before = current_task.clone();
        current_task.sort_order = Some(swap_order);
        current_task.updated_at = chrono::Utc::now();
        let after = current_task.clone();
        model.sync_task(&after);
        model.undo_stack.push(UndoAction::TaskModified {
            before: Box::new(before),
            after: Box::new(after),
        });
    }

    if let Some(swap_task) = model.tasks.get_mut(&swap_task_id) {
        let before = swap_task.clone();
        swap_task.sort_order = Some(current_order);
        swap_task.updated_at = chrono::Utc::now();
        let after = swap_task.clone();
        model.sync_task(&after);
        model.undo_stack.push(UndoAction::TaskModified {
            before: Box::new(before),
            after: Box::new(after),
        });
    }

    // Update selection to follow the moved task
    model.selected_index = swap_index;
    model.refresh_visible_tasks();
}

/// Create the next occurrence of a recurring task
fn create_next_recurring_task(task: &crate::domain::Task) -> crate::domain::Task {
    use crate::domain::Recurrence;
    use chrono::{Datelike, Duration, NaiveDate, Utc};

    let today = Utc::now().date_naive();
    let base_date = task.due_date.unwrap_or(today);

    let next_due = match &task.recurrence {
        Some(Recurrence::Daily) => base_date + Duration::days(1),
        Some(Recurrence::Weekly { days }) => {
            if days.is_empty() {
                // Default: same day next week
                base_date + Duration::weeks(1)
            } else {
                // Find next day in the list after today
                let current_weekday = today.weekday();
                let mut min_days = 7i64;
                for day in days {
                    let days_until = (i64::from(day.num_days_from_monday())
                        - i64::from(current_weekday.num_days_from_monday())
                        + 7)
                        % 7;
                    let days_until = if days_until == 0 { 7 } else { days_until };
                    if days_until < min_days {
                        min_days = days_until;
                    }
                }
                today + Duration::days(min_days)
            }
        }
        Some(Recurrence::Monthly { day }) => {
            let next_month = if base_date.month() == 12 {
                NaiveDate::from_ymd_opt(base_date.year() + 1, 1, *day)
            } else {
                NaiveDate::from_ymd_opt(base_date.year(), base_date.month() + 1, *day)
            };
            // Handle invalid dates (e.g., Feb 30) by using last day of month
            next_month.unwrap_or_else(|| {
                let year = if base_date.month() == 12 {
                    base_date.year() + 1
                } else {
                    base_date.year()
                };
                let month = if base_date.month() == 12 {
                    1
                } else {
                    base_date.month() + 1
                };
                // Get last day of the target month (first of next month minus 1 day)
                NaiveDate::from_ymd_opt(
                    if month == 12 { year + 1 } else { year },
                    if month == 12 { 1 } else { month + 1 },
                    1,
                )
                .expect("day 1 of any month always exists")
                    - Duration::days(1)
            })
        }
        Some(Recurrence::Yearly { month, day }) => {
            let next_year = base_date.year() + 1;
            // Try exact date, fall back to 28th if invalid (e.g., Feb 30)
            NaiveDate::from_ymd_opt(next_year, *month, *day).unwrap_or_else(|| {
                NaiveDate::from_ymd_opt(next_year, *month, 28)
                    .expect("day 28 always exists in any month")
            })
        }
        None => today + Duration::days(1), // Shouldn't happen
    };

    crate::domain::Task::new(&task.title)
        .with_priority(task.priority)
        .with_due_date(next_due)
        .with_tags(task.tags.clone())
        .with_recurrence(task.recurrence.clone())
        .with_project_opt(task.project_id.clone())
        .with_description_opt(task.description.clone())
}

fn handle_time(model: &mut Model, msg: TimeMessage) {
    match msg {
        TimeMessage::StartTracking => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                model.start_time_tracking(task_id);
            }
        }
        TimeMessage::StopTracking => {
            model.stop_time_tracking();
        }
        TimeMessage::ToggleTracking => {
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).cloned() {
                if model.is_tracking_task(&task_id) {
                    model.stop_time_tracking();
                } else {
                    model.start_time_tracking(task_id);
                }
            }
        }
    }
}

fn handle_system(model: &mut Model, msg: SystemMessage) {
    match msg {
        SystemMessage::Quit => {
            // Stop any running timer before quitting
            model.stop_time_tracking();
            model.running = RunningState::Quitting;
        }
        SystemMessage::Save => {
            let _ = model.save();
        }
        SystemMessage::Undo => {
            if let Some(action) = model.undo_stack.pop_for_undo() {
                match action {
                    UndoAction::TaskCreated(task) => {
                        // Undo create by deleting the task
                        model.delete_task_from_storage(&task.id);
                        model.tasks.remove(&task.id);
                    }
                    UndoAction::TaskDeleted(task) => {
                        // Undo delete by restoring the task
                        model.sync_task(&task);
                        model.tasks.insert(task.id.clone(), *task);
                    }
                    UndoAction::TaskModified { before, after: _ } => {
                        // Undo modify by restoring previous state
                        model.sync_task(&before);
                        model.tasks.insert(before.id.clone(), *before);
                    }
                    UndoAction::ProjectCreated(project) => {
                        // Undo project create by removing it
                        model.projects.remove(&project.id);
                        model.dirty = true;
                    }
                    UndoAction::ProjectDeleted(project) => {
                        // Undo project delete by restoring it
                        model.sync_project(&project);
                        model.projects.insert(project.id.clone(), *project);
                    }
                    UndoAction::ProjectModified { before, after: _ } => {
                        // Undo modify by restoring previous state
                        model.sync_project(&before);
                        model.projects.insert(before.id.clone(), *before);
                    }
                }
                model.refresh_visible_tasks();
            }
        }
        SystemMessage::Redo => {
            if let Some(action) = model.undo_stack.pop_for_redo() {
                match action {
                    UndoAction::TaskCreated(task) => {
                        // Redo create by restoring the task
                        model.sync_task(&task);
                        model.tasks.insert(task.id.clone(), *task);
                    }
                    UndoAction::TaskDeleted(task) => {
                        // Redo delete by removing the task
                        model.delete_task_from_storage(&task.id);
                        model.tasks.remove(&task.id);
                    }
                    UndoAction::TaskModified { before, after: _ } => {
                        // Redo modify: the redo stack holds the inverse, so "before" is the state we want
                        model.sync_task(&before);
                        model.tasks.insert(before.id.clone(), *before);
                    }
                    UndoAction::ProjectCreated(project) => {
                        // Redo project create by restoring it
                        model.sync_project(&project);
                        model.projects.insert(project.id.clone(), *project);
                    }
                    UndoAction::ProjectDeleted(project) => {
                        // Redo project delete by removing it
                        model.projects.remove(&project.id);
                        model.dirty = true;
                    }
                    UndoAction::ProjectModified { before, after: _ } => {
                        // Redo modify: the redo stack holds the inverse, so "before" is the state we want
                        model.sync_project(&before);
                        model.projects.insert(before.id.clone(), *before);
                    }
                }
                model.refresh_visible_tasks();
            }
        }
        SystemMessage::Resize { width, height } => {
            model.terminal_size = (width, height);
        }
        SystemMessage::Tick => {
            // Handle periodic updates (e.g., timer display)
            // Clear status message after a tick
            model.status_message = None;
        }
        SystemMessage::ExportCsv => {
            handle_export_csv(model);
        }
        SystemMessage::ExportIcs => {
            handle_export_ics(model);
        }
        SystemMessage::ExportChainsDot => {
            handle_export_chains_dot(model);
        }
        SystemMessage::ExportChainsMermaid => {
            handle_export_chains_mermaid(model);
        }
        SystemMessage::ExportReportMarkdown => {
            handle_export_report_markdown(model);
        }
        SystemMessage::ExportReportHtml => {
            handle_export_report_html(model);
        }
        SystemMessage::StartImportCsv => {
            handle_start_import(model, crate::storage::ImportFormat::Csv);
        }
        SystemMessage::StartImportIcs => {
            handle_start_import(model, crate::storage::ImportFormat::Ics);
        }
        SystemMessage::ExecuteImport => {
            handle_execute_import(model);
        }
        SystemMessage::ConfirmImport => {
            handle_confirm_import(model);
        }
        SystemMessage::CancelImport => {
            handle_cancel_import(model);
        }
    }
}

fn handle_export_csv(model: &mut Model) {
    use crate::storage::{export_to_string, ExportFormat};

    let tasks = model.tasks_for_export();
    match export_to_string(&tasks, ExportFormat::Csv) {
        Ok(content) => {
            // Determine export path
            let export_path = model
                .data_path
                .as_ref()
                .map(|p| p.with_extension("csv"))
                .unwrap_or_else(|| std::path::PathBuf::from("tasks.csv"));

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.status_message = Some(format!(
                        "Exported {} tasks to {}",
                        tasks.len(),
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.status_message = Some(format!("Export failed: {e}"));
        }
    }
}

fn handle_export_ics(model: &mut Model) {
    use crate::storage::{export_to_string, ExportFormat};

    let tasks = model.tasks_for_export();
    match export_to_string(&tasks, ExportFormat::Ics) {
        Ok(content) => {
            // Determine export path
            let export_path = model
                .data_path
                .as_ref()
                .map(|p| p.with_extension("ics"))
                .unwrap_or_else(|| std::path::PathBuf::from("tasks.ics"));

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.status_message = Some(format!(
                        "Exported {} tasks to {}",
                        tasks.len(),
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.status_message = Some(format!("Export failed: {e}"));
        }
    }
}

fn handle_export_chains_dot(model: &mut Model) {
    use crate::storage::{export_chains_to_string, ExportFormat};

    match export_chains_to_string(&model.tasks, ExportFormat::Dot) {
        Ok(content) => {
            // Determine export path
            let export_path = model
                .data_path
                .as_ref()
                .map(|p| p.with_extension("dot"))
                .unwrap_or_else(|| std::path::PathBuf::from("task_chains.dot"));

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.status_message = Some(format!(
                        "Exported task chains to {} (use Graphviz to render)",
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.status_message = Some(format!("Export failed: {e}"));
        }
    }
}

fn handle_export_chains_mermaid(model: &mut Model) {
    use crate::storage::{export_chains_to_string, ExportFormat};

    match export_chains_to_string(&model.tasks, ExportFormat::Mermaid) {
        Ok(content) => {
            // Determine export path
            let export_path = model
                .data_path
                .as_ref()
                .map(|p| p.with_extension("md"))
                .unwrap_or_else(|| std::path::PathBuf::from("task_chains.md"));

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.status_message = Some(format!(
                        "Exported task chains to {} (Mermaid diagram)",
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.status_message = Some(format!("Export failed: {e}"));
        }
    }
}

fn handle_export_report_markdown(model: &mut Model) {
    use crate::app::analytics::AnalyticsEngine;
    use crate::domain::analytics::ReportConfig;
    use crate::storage::export_report_to_markdown_string;

    let config = ReportConfig::last_n_days(30);
    let engine = AnalyticsEngine::new(model);
    let report = engine.generate_report(&config);

    match export_report_to_markdown_string(&report) {
        Ok(content) => {
            let export_path = model
                .data_path
                .as_ref()
                .map(|p| p.with_extension("report.md"))
                .unwrap_or_else(|| std::path::PathBuf::from("taskflow_report.md"));

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.status_message = Some(format!(
                        "Exported analytics report to {}",
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.status_message = Some(format!("Export failed: {e}"));
        }
    }
}

fn handle_export_report_html(model: &mut Model) {
    use crate::app::analytics::AnalyticsEngine;
    use crate::domain::analytics::ReportConfig;
    use crate::storage::export_report_to_html_string;

    let config = ReportConfig::last_n_days(30);
    let engine = AnalyticsEngine::new(model);
    let report = engine.generate_report(&config);

    match export_report_to_html_string(&report) {
        Ok(content) => {
            let export_path = model
                .data_path
                .as_ref()
                .map(|p| p.with_extension("report.html"))
                .unwrap_or_else(|| std::path::PathBuf::from("taskflow_report.html"));

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.status_message = Some(format!(
                        "Exported analytics report to {}",
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.status_message = Some(format!("Export failed: {e}"));
        }
    }
}

fn handle_start_import(model: &mut Model, format: crate::storage::ImportFormat) {
    use crate::ui::InputTarget;

    model.input_mode = InputMode::Editing;
    model.input_target = InputTarget::ImportFilePath(format);
    model.input_buffer.clear();
    model.cursor_position = 0;
}

fn handle_execute_import(model: &mut Model) {
    use crate::storage::{
        apply_merge_strategy, import_from_csv, import_from_ics, ImportFormat, ImportOptions,
        MergeStrategy,
    };
    use crate::ui::InputTarget;
    use std::fs::File;
    use std::io::BufReader;

    let format = match &model.input_target {
        InputTarget::ImportFilePath(fmt) => *fmt,
        _ => return,
    };

    let file_path = model.input_buffer.trim();
    if file_path.is_empty() {
        model.status_message = Some("No file path provided".to_string());
        model.input_mode = InputMode::Normal;
        model.input_target = InputTarget::Task;
        return;
    }

    // Open the file
    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(e) => {
            model.status_message = Some(format!("Failed to open file: {e}"));
            model.input_mode = InputMode::Normal;
            model.input_target = InputTarget::Task;
            return;
        }
    };

    let reader = BufReader::new(file);
    let options = ImportOptions {
        merge_strategy: MergeStrategy::Skip,
        validate: true,
        dry_run: false,
    };

    // Parse the file
    let mut result = match format {
        ImportFormat::Csv => match import_from_csv(reader, &options) {
            Ok(r) => r,
            Err(e) => {
                model.status_message = Some(format!("Import failed: {e}"));
                model.input_mode = InputMode::Normal;
                model.input_target = InputTarget::Task;
                return;
            }
        },
        ImportFormat::Ics => match import_from_ics(reader, &options) {
            Ok(r) => r,
            Err(e) => {
                model.status_message = Some(format!("Import failed: {e}"));
                model.input_mode = InputMode::Normal;
                model.input_target = InputTarget::Task;
                return;
            }
        },
    };

    // Apply duplicate detection
    apply_merge_strategy(&mut result, &model.tasks, options.merge_strategy);

    // Reset input mode
    model.input_mode = InputMode::Normal;
    model.input_target = InputTarget::Task;
    model.input_buffer.clear();

    // If there are tasks to import, show preview
    if result.imported.is_empty() && result.skipped.is_empty() && result.errors.is_empty() {
        model.status_message = Some("No tasks found in file".to_string());
        return;
    }

    // Store the result and show preview
    let import_count = result.imported.len();
    let skip_count = result.skipped.len();
    let error_count = result.errors.len();

    model.pending_import = Some(result);
    model.show_import_preview = true;
    model.status_message = Some(format!(
        "Preview: {} to import, {} skipped, {} errors. Press Enter to confirm, Esc to cancel.",
        import_count, skip_count, error_count
    ));
}

fn handle_confirm_import(model: &mut Model) {
    if let Some(result) = model.pending_import.take() {
        let count = result.imported.len();

        // Add all imported tasks
        for task in result.imported {
            model.sync_task(&task);
            model.tasks.insert(task.id.clone(), task);
        }

        model.dirty = true;
        model.show_import_preview = false;
        model.refresh_visible_tasks();
        model.status_message = Some(format!("Imported {} tasks", count));
    }
}

fn handle_cancel_import(model: &mut Model) {
    model.pending_import = None;
    model.show_import_preview = false;
    model.status_message = Some("Import cancelled".to_string());
}

fn handle_pomodoro(model: &mut Model, msg: PomodoroMessage) {
    use crate::domain::PomodoroSession;

    match msg {
        PomodoroMessage::Start { goal_cycles } => {
            // Start a new session for the selected task
            if let Some(task) = model.selected_task() {
                let task_id = task.id.clone();
                model.pomodoro_session = Some(PomodoroSession::new(
                    task_id,
                    &model.pomodoro_config,
                    goal_cycles,
                ));
                // Automatically enter focus mode
                model.focus_mode = true;
                model.status_message =
                    Some(format!("Pomodoro started: {} cycle goal", goal_cycles));
            } else {
                model.status_message = Some("Select a task to start Pomodoro".to_string());
            }
        }
        PomodoroMessage::Pause => {
            if let Some(ref mut session) = model.pomodoro_session {
                if !session.paused {
                    session.paused = true;
                    session.paused_at = Some(chrono::Utc::now());
                }
            }
        }
        PomodoroMessage::Resume => {
            if let Some(ref mut session) = model.pomodoro_session {
                if session.paused {
                    // Add elapsed pause time to total paused duration
                    if let Some(pause_start) = session.paused_at {
                        let pause_duration =
                            (chrono::Utc::now() - pause_start).num_seconds().max(0) as u32;
                        session.paused_duration_secs += pause_duration;
                    }
                    session.paused = false;
                    session.paused_at = None;
                }
            }
        }
        PomodoroMessage::TogglePause => {
            if let Some(ref mut session) = model.pomodoro_session {
                if session.paused {
                    // Resuming - add elapsed pause time
                    if let Some(pause_start) = session.paused_at {
                        let pause_duration =
                            (chrono::Utc::now() - pause_start).num_seconds().max(0) as u32;
                        session.paused_duration_secs += pause_duration;
                    }
                    session.paused = false;
                    session.paused_at = None;
                } else {
                    // Pausing - record pause start
                    session.paused = true;
                    session.paused_at = Some(chrono::Utc::now());
                }
            }
        }
        PomodoroMessage::Skip => {
            if model.pomodoro_session.is_some() {
                transition_pomodoro_phase(model);
            }
        }
        PomodoroMessage::Stop => {
            if model.pomodoro_session.is_some() {
                model.pomodoro_session = None;
                model.status_message = Some("Pomodoro session stopped".to_string());
            }
        }
        PomodoroMessage::Tick => {
            let should_transition = if let Some(ref mut session) = model.pomodoro_session {
                if !session.paused && session.remaining_secs > 0 {
                    session.remaining_secs -= 1;
                }
                session.remaining_secs == 0
            } else {
                false
            };

            if should_transition {
                transition_pomodoro_phase(model);
            }
        }
        PomodoroMessage::SetWorkDuration(mins) => {
            model.pomodoro_config.work_duration_mins = mins.max(1);
        }
        PomodoroMessage::SetShortBreak(mins) => {
            model.pomodoro_config.short_break_mins = mins.max(1);
        }
        PomodoroMessage::SetLongBreak(mins) => {
            model.pomodoro_config.long_break_mins = mins.max(1);
        }
        PomodoroMessage::SetCyclesBeforeLongBreak(cycles) => {
            model.pomodoro_config.cycles_before_long_break = cycles.max(1);
        }
        PomodoroMessage::IncrementGoal => {
            if let Some(ref mut session) = model.pomodoro_session {
                session.session_goal += 1;
            }
        }
        PomodoroMessage::DecrementGoal => {
            if let Some(ref mut session) = model.pomodoro_session {
                if session.session_goal > 1 {
                    session.session_goal -= 1;
                }
            }
        }
    }
}

fn transition_pomodoro_phase(model: &mut Model) {
    use crate::domain::PomodoroPhase;

    let (next_phase, next_remaining, cycles_completed, message) = {
        let session = match model.pomodoro_session.as_ref() {
            Some(s) => s,
            None => return,
        };

        match session.phase {
            PomodoroPhase::Work => {
                // Record the completed work cycle
                let new_cycles = session.cycles_completed + 1;

                // Determine if long break or short break
                if new_cycles > 0
                    && new_cycles % model.pomodoro_config.cycles_before_long_break == 0
                {
                    (
                        PomodoroPhase::LongBreak,
                        model.pomodoro_config.long_break_mins * 60,
                        new_cycles,
                        format!("🎉 Cycle {} complete! Time for a long break.", new_cycles),
                    )
                } else {
                    (
                        PomodoroPhase::ShortBreak,
                        model.pomodoro_config.short_break_mins * 60,
                        new_cycles,
                        format!("🍅 Cycle {} complete! Take a short break.", new_cycles),
                    )
                }
            }
            PomodoroPhase::ShortBreak | PomodoroPhase::LongBreak => (
                PomodoroPhase::Work,
                model.pomodoro_config.work_duration_mins * 60,
                session.cycles_completed,
                "☕ Break over! Back to work.".to_string(),
            ),
        }
    };

    // Update session
    if let Some(ref mut session) = model.pomodoro_session {
        // Record stats when completing a work phase
        if session.phase == PomodoroPhase::Work {
            model
                .pomodoro_stats
                .record_cycle(model.pomodoro_config.work_duration_mins);
        }

        session.phase = next_phase;
        session.cycles_completed = cycles_completed;
        // Reset phase timing (sets remaining_secs, phase_started_at, clears pause state)
        session.reset_phase_timing(next_remaining);

        // Check if goal reached
        if session.goal_reached() && next_phase == PomodoroPhase::Work {
            model.status_message = Some(format!(
                "🎊 Goal reached! {} cycles completed. Keep going or stop.",
                session.cycles_completed
            ));
        } else {
            model.status_message = Some(message);
        }
    }
}

/// Create a task from quick add input, applying parsed metadata
fn create_task_from_quick_add(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Priority, Task, TaskStatus};

    fn create_test_model_with_tasks() -> Model {
        let mut model = Model::new();

        for i in 0..5 {
            let task = Task::new(format!("Task {}", i));
            model.tasks.insert(task.id.clone(), task);
        }
        model.refresh_visible_tasks();
        model
    }

    // Navigation tests
    #[test]
    fn test_navigation_up() {
        let mut model = create_test_model_with_tasks();
        model.selected_index = 2;

        update(&mut model, Message::Navigation(NavigationMessage::Up));

        assert_eq!(model.selected_index, 1);
    }

    #[test]
    fn test_navigation_up_stops_at_zero() {
        let mut model = create_test_model_with_tasks();
        model.selected_index = 0;

        update(&mut model, Message::Navigation(NavigationMessage::Up));

        assert_eq!(model.selected_index, 0);
    }

    #[test]
    fn test_navigation_down() {
        let mut model = create_test_model_with_tasks();
        model.selected_index = 2;

        update(&mut model, Message::Navigation(NavigationMessage::Down));

        assert_eq!(model.selected_index, 3);
    }

    #[test]
    fn test_navigation_down_stops_at_max() {
        let mut model = create_test_model_with_tasks();
        model.selected_index = 4;

        update(&mut model, Message::Navigation(NavigationMessage::Down));

        assert_eq!(model.selected_index, 4);
    }

    #[test]
    fn test_navigation_first() {
        let mut model = create_test_model_with_tasks();
        model.selected_index = 3;

        update(&mut model, Message::Navigation(NavigationMessage::First));

        assert_eq!(model.selected_index, 0);
    }

    #[test]
    fn test_navigation_last() {
        let mut model = create_test_model_with_tasks();
        model.selected_index = 0;

        update(&mut model, Message::Navigation(NavigationMessage::Last));

        assert_eq!(model.selected_index, 4);
    }

    #[test]
    fn test_navigation_page_up() {
        let mut model = create_test_model_with_tasks();
        model.selected_index = 4;

        update(&mut model, Message::Navigation(NavigationMessage::PageUp));

        assert_eq!(model.selected_index, 0); // saturating_sub from 4 - 10
    }

    #[test]
    fn test_navigation_page_down() {
        let mut model = create_test_model_with_tasks();
        model.selected_index = 0;

        update(&mut model, Message::Navigation(NavigationMessage::PageDown));

        assert_eq!(model.selected_index, 4); // capped at max
    }

    #[test]
    fn test_navigation_select() {
        let mut model = create_test_model_with_tasks();
        model.selected_index = 0;

        update(
            &mut model,
            Message::Navigation(NavigationMessage::Select(3)),
        );

        assert_eq!(model.selected_index, 3);
    }

    #[test]
    fn test_navigation_select_invalid_ignored() {
        let mut model = create_test_model_with_tasks();
        model.selected_index = 2;

        update(
            &mut model,
            Message::Navigation(NavigationMessage::Select(100)),
        );

        assert_eq!(model.selected_index, 2); // unchanged
    }

    #[test]
    fn test_navigation_go_to_view() {
        let mut model = create_test_model_with_tasks();
        model.selected_index = 3;
        model.current_view = super::super::ViewId::TaskList;

        update(
            &mut model,
            Message::Navigation(NavigationMessage::GoToView(super::super::ViewId::Today)),
        );

        assert_eq!(model.current_view, super::super::ViewId::Today);
        assert_eq!(model.selected_index, 0); // reset to 0
    }

    // Task tests
    #[test]
    fn test_task_toggle_complete() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Task should be Todo initially
        assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Todo);

        update(&mut model, Message::Task(TaskMessage::ToggleComplete));

        assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Done);
    }

    #[test]
    fn test_task_set_status() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        update(
            &mut model,
            Message::Task(TaskMessage::SetStatus(
                task_id.clone(),
                TaskStatus::InProgress,
            )),
        );

        assert_eq!(
            model.tasks.get(&task_id).unwrap().status,
            TaskStatus::InProgress
        );
    }

    #[test]
    fn test_task_set_priority() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        update(
            &mut model,
            Message::Task(TaskMessage::SetPriority(task_id.clone(), Priority::Urgent)),
        );

        assert_eq!(
            model.tasks.get(&task_id).unwrap().priority,
            Priority::Urgent
        );
    }

    #[test]
    fn test_task_create() {
        let mut model = Model::new();
        assert!(model.tasks.is_empty());

        update(
            &mut model,
            Message::Task(TaskMessage::Create("New task".to_string())),
        );

        assert_eq!(model.tasks.len(), 1);
        let task = model.tasks.values().next().unwrap();
        assert_eq!(task.title, "New task");
    }

    #[test]
    fn test_task_delete() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();
        let initial_count = model.tasks.len();

        update(
            &mut model,
            Message::Task(TaskMessage::Delete(task_id.clone())),
        );

        assert_eq!(model.tasks.len(), initial_count - 1);
        assert!(!model.tasks.contains_key(&task_id));
    }

    // Time tests
    #[test]
    fn test_time_toggle_tracking_start() {
        let mut model = create_test_model_with_tasks();
        assert!(model.active_time_entry.is_none());

        update(&mut model, Message::Time(TimeMessage::ToggleTracking));

        assert!(model.active_time_entry.is_some());
    }

    #[test]
    fn test_time_toggle_tracking_stop() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();
        model.start_time_tracking(task_id);

        update(&mut model, Message::Time(TimeMessage::ToggleTracking));

        assert!(model.active_time_entry.is_none());
    }

    // UI tests
    #[test]
    fn test_ui_toggle_show_completed() {
        let mut model = Model::new();
        assert!(!model.show_completed);

        update(&mut model, Message::Ui(UiMessage::ToggleShowCompleted));

        assert!(model.show_completed);

        update(&mut model, Message::Ui(UiMessage::ToggleShowCompleted));

        assert!(!model.show_completed);
    }

    #[test]
    fn test_ui_toggle_sidebar() {
        let mut model = Model::new();
        assert!(model.show_sidebar);

        update(&mut model, Message::Ui(UiMessage::ToggleSidebar));

        assert!(!model.show_sidebar);
    }

    #[test]
    fn test_ui_input_char() {
        let mut model = Model::new();
        model.input_mode = InputMode::Editing;

        update(&mut model, Message::Ui(UiMessage::InputChar('H')));
        update(&mut model, Message::Ui(UiMessage::InputChar('i')));

        assert_eq!(model.input_buffer, "Hi");
        assert_eq!(model.cursor_position, 2);
    }

    #[test]
    fn test_ui_input_backspace() {
        let mut model = Model::new();
        model.input_mode = InputMode::Editing;
        model.input_buffer = "Hello".to_string();
        model.cursor_position = 5;

        update(&mut model, Message::Ui(UiMessage::InputBackspace));

        assert_eq!(model.input_buffer, "Hell");
        assert_eq!(model.cursor_position, 4);
    }

    #[test]
    fn test_ui_input_cursor_movement() {
        let mut model = Model::new();
        model.input_mode = InputMode::Editing;
        model.input_buffer = "Hello".to_string();
        model.cursor_position = 3;

        update(&mut model, Message::Ui(UiMessage::InputCursorLeft));
        assert_eq!(model.cursor_position, 2);

        update(&mut model, Message::Ui(UiMessage::InputCursorRight));
        assert_eq!(model.cursor_position, 3);

        update(&mut model, Message::Ui(UiMessage::InputCursorStart));
        assert_eq!(model.cursor_position, 0);

        update(&mut model, Message::Ui(UiMessage::InputCursorEnd));
        assert_eq!(model.cursor_position, 5);
    }

    #[test]
    fn test_ui_submit_input_creates_task() {
        let mut model = Model::new();
        model.input_mode = InputMode::Editing;
        model.input_buffer = "New task from input".to_string();

        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        assert_eq!(model.input_mode, InputMode::Normal);
        assert!(model.input_buffer.is_empty());
        assert_eq!(model.tasks.len(), 1);
        let task = model.tasks.values().next().unwrap();
        assert_eq!(task.title, "New task from input");
    }

    #[test]
    fn test_ui_submit_input_empty_ignored() {
        let mut model = Model::new();
        model.input_mode = InputMode::Editing;
        model.input_buffer = "   ".to_string(); // whitespace only

        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        assert_eq!(model.input_mode, InputMode::Normal);
        assert!(model.tasks.is_empty()); // no task created
    }

    #[test]
    fn test_ui_cancel_input() {
        let mut model = Model::new();
        model.input_mode = InputMode::Editing;
        model.input_buffer = "Some text".to_string();
        model.cursor_position = 5;

        update(&mut model, Message::Ui(UiMessage::CancelInput));

        assert_eq!(model.input_mode, InputMode::Normal);
        assert!(model.input_buffer.is_empty());
        assert_eq!(model.cursor_position, 0);
    }

    // System tests
    #[test]
    fn test_system_quit_stops_timer() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();
        model.start_time_tracking(task_id);

        assert!(model.active_time_entry.is_some());

        update(&mut model, Message::System(SystemMessage::Quit));

        assert!(model.active_time_entry.is_none());
        assert_eq!(model.running, RunningState::Quitting);
    }

    #[test]
    fn test_system_resize() {
        let mut model = Model::new();

        update(
            &mut model,
            Message::System(SystemMessage::Resize {
                width: 120,
                height: 40,
            }),
        );

        assert_eq!(model.terminal_size, (120, 40));
    }

    #[test]
    fn test_show_help() {
        let mut model = Model::new();
        assert!(!model.show_help);

        update(&mut model, Message::Ui(UiMessage::ShowHelp));

        assert!(model.show_help);

        update(&mut model, Message::Ui(UiMessage::HideHelp));

        assert!(!model.show_help);
    }

    #[test]
    fn test_task_create_uses_default_priority() {
        let mut model = Model::new();
        model.default_priority = Priority::High;

        update(
            &mut model,
            Message::Task(TaskMessage::Create("High priority task".to_string())),
        );

        let task = model.tasks.values().next().unwrap();
        assert_eq!(task.title, "High priority task");
        assert_eq!(task.priority, Priority::High);
    }

    #[test]
    fn test_submit_input_uses_default_priority() {
        let mut model = Model::new();
        model.input_mode = InputMode::Editing;
        model.input_buffer = "Task via input".to_string();
        model.default_priority = Priority::Urgent;

        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        let task = model.tasks.values().next().unwrap();
        assert_eq!(task.title, "Task via input");
        assert_eq!(task.priority, Priority::Urgent);
    }

    // Sidebar navigation tests
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
        use crate::domain::Project;

        let mut model = Model::new();
        // Add a project
        let project = Project::new("Test Project");
        let project_id = project.id.clone();
        model.projects.insert(project.id.clone(), project);

        // Add a task with this project
        let mut task = Task::new("Task in project");
        task.project_id = Some(project_id.clone());
        model.tasks.insert(task.id.clone(), task);

        // Add a task without project
        let task2 = Task::new("Task without project");
        model.tasks.insert(task2.id.clone(), task2);

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
        use crate::domain::Project;

        let mut model = Model::new();
        let project = Project::new("Test Project");
        let project_id = project.id.clone();
        model.projects.insert(project.id.clone(), project);
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

    // Project creation tests
    #[test]
    fn test_start_create_project() {
        let mut model = Model::new();
        assert_eq!(model.input_mode, InputMode::Normal);
        assert_eq!(model.input_target, InputTarget::Task); // Default

        update(&mut model, Message::Ui(UiMessage::StartCreateProject));

        assert_eq!(model.input_mode, InputMode::Editing);
        assert_eq!(model.input_target, InputTarget::Project);
        assert!(model.input_buffer.is_empty());
    }

    #[test]
    fn test_submit_input_creates_project() {
        let mut model = Model::new();
        assert!(model.projects.is_empty());

        // Start project creation
        update(&mut model, Message::Ui(UiMessage::StartCreateProject));

        // Type project name
        for c in "My New Project".chars() {
            update(&mut model, Message::Ui(UiMessage::InputChar(c)));
        }

        // Submit
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Project should be created
        assert_eq!(model.projects.len(), 1);
        let project = model.projects.values().next().unwrap();
        assert_eq!(project.name, "My New Project");

        // Should return to normal mode
        assert_eq!(model.input_mode, InputMode::Normal);
        assert_eq!(model.input_target, InputTarget::Task); // Reset to default
    }

    #[test]
    fn test_cancel_project_creation() {
        let mut model = Model::new();

        // Start project creation
        update(&mut model, Message::Ui(UiMessage::StartCreateProject));

        // Type something
        update(&mut model, Message::Ui(UiMessage::InputChar('T')));

        // Cancel
        update(&mut model, Message::Ui(UiMessage::CancelInput));

        // No project should be created
        assert!(model.projects.is_empty());
        assert_eq!(model.input_mode, InputMode::Normal);
        assert!(model.input_buffer.is_empty());
    }

    #[test]
    fn test_empty_project_name_not_created() {
        let mut model = Model::new();

        // Start project creation
        update(&mut model, Message::Ui(UiMessage::StartCreateProject));

        // Submit with empty name
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // No project should be created
        assert!(model.projects.is_empty());
        assert_eq!(model.input_mode, InputMode::Normal);
    }

    // Task editing tests
    #[test]
    fn test_start_edit_task() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();
        let original_title = model.tasks.get(&task_id).unwrap().title.clone();

        update(&mut model, Message::Ui(UiMessage::StartEditTask));

        assert_eq!(model.input_mode, InputMode::Editing);
        assert_eq!(model.input_target, InputTarget::EditTask(task_id));
        assert_eq!(model.input_buffer, original_title);
        assert_eq!(model.cursor_position, original_title.len());
    }

    #[test]
    fn test_edit_task_title() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Start editing
        update(&mut model, Message::Ui(UiMessage::StartEditTask));

        // Clear and type new title
        model.input_buffer.clear();
        model.cursor_position = 0;
        for c in "Updated Title".chars() {
            update(&mut model, Message::Ui(UiMessage::InputChar(c)));
        }

        // Submit
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Title should be updated
        let task = model.tasks.get(&task_id).unwrap();
        assert_eq!(task.title, "Updated Title");
        assert_eq!(model.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_cancel_edit_task() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();
        let original_title = model.tasks.get(&task_id).unwrap().title.clone();

        // Start editing
        update(&mut model, Message::Ui(UiMessage::StartEditTask));

        // Type something
        model.input_buffer = "Changed".to_string();

        // Cancel
        update(&mut model, Message::Ui(UiMessage::CancelInput));

        // Title should NOT be changed
        let task = model.tasks.get(&task_id).unwrap();
        assert_eq!(task.title, original_title);
        assert_eq!(model.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_cycle_priority() {
        use crate::domain::Priority;
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Set initial priority to None
        model.tasks.get_mut(&task_id).unwrap().priority = Priority::None;

        // Cycle through priorities
        update(&mut model, Message::Task(TaskMessage::CyclePriority));
        assert_eq!(model.tasks.get(&task_id).unwrap().priority, Priority::Low);

        update(&mut model, Message::Task(TaskMessage::CyclePriority));
        assert_eq!(
            model.tasks.get(&task_id).unwrap().priority,
            Priority::Medium
        );

        update(&mut model, Message::Task(TaskMessage::CyclePriority));
        assert_eq!(model.tasks.get(&task_id).unwrap().priority, Priority::High);

        update(&mut model, Message::Task(TaskMessage::CyclePriority));
        assert_eq!(
            model.tasks.get(&task_id).unwrap().priority,
            Priority::Urgent
        );

        update(&mut model, Message::Task(TaskMessage::CyclePriority));
        assert_eq!(model.tasks.get(&task_id).unwrap().priority, Priority::None);
    }

    #[test]
    fn test_edit_due_date() {
        use chrono::NaiveDate;
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Start editing due date
        update(&mut model, Message::Ui(UiMessage::StartEditDueDate));

        assert_eq!(model.input_mode, InputMode::Editing);
        assert!(matches!(model.input_target, InputTarget::EditDueDate(_)));

        // Type a date
        model.input_buffer = "2025-12-25".to_string();
        model.cursor_position = model.input_buffer.len();

        // Submit
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Due date should be set
        let task = model.tasks.get(&task_id).unwrap();
        assert_eq!(
            task.due_date,
            Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap())
        );
    }

    #[test]
    fn test_clear_due_date() {
        use chrono::NaiveDate;
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Set an initial due date
        model.tasks.get_mut(&task_id).unwrap().due_date =
            Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());

        // Start editing due date
        update(&mut model, Message::Ui(UiMessage::StartEditDueDate));

        // Clear the buffer
        model.input_buffer.clear();
        model.cursor_position = 0;

        // Submit empty
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Due date should be cleared
        let task = model.tasks.get(&task_id).unwrap();
        assert!(task.due_date.is_none());
    }

    #[test]
    fn test_invalid_due_date_keeps_old() {
        use chrono::NaiveDate;
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Set an initial due date
        let original_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        model.tasks.get_mut(&task_id).unwrap().due_date = Some(original_date);

        // Start editing due date
        update(&mut model, Message::Ui(UiMessage::StartEditDueDate));

        // Type invalid date
        model.input_buffer = "not-a-date".to_string();
        model.cursor_position = model.input_buffer.len();

        // Submit
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Due date should be unchanged
        let task = model.tasks.get(&task_id).unwrap();
        assert_eq!(task.due_date, Some(original_date));
    }

    // Tag management tests
    #[test]
    fn test_start_edit_tags() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Add some initial tags
        model.tasks.get_mut(&task_id).unwrap().tags =
            vec!["work".to_string(), "urgent".to_string()];

        update(&mut model, Message::Ui(UiMessage::StartEditTags));

        assert_eq!(model.input_mode, InputMode::Editing);
        assert!(matches!(model.input_target, InputTarget::EditTags(_)));
        assert_eq!(model.input_buffer, "work, urgent");
    }

    #[test]
    fn test_edit_tags_add_new() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Task has no tags initially
        assert!(model.tasks.get(&task_id).unwrap().tags.is_empty());

        // Start editing tags
        update(&mut model, Message::Ui(UiMessage::StartEditTags));

        // Type new tags
        model.input_buffer = "feature, bug, priority".to_string();
        model.cursor_position = model.input_buffer.len();

        // Submit
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Tags should be set
        let task = model.tasks.get(&task_id).unwrap();
        assert_eq!(task.tags, vec!["feature", "bug", "priority"]);
    }

    #[test]
    fn test_edit_tags_clear() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Set initial tags
        model.tasks.get_mut(&task_id).unwrap().tags = vec!["work".to_string()];

        // Start editing tags
        update(&mut model, Message::Ui(UiMessage::StartEditTags));

        // Clear input
        model.input_buffer.clear();
        model.cursor_position = 0;

        // Submit empty
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Tags should be cleared
        let task = model.tasks.get(&task_id).unwrap();
        assert!(task.tags.is_empty());
    }

    #[test]
    fn test_edit_tags_trims_whitespace() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Start editing tags
        update(&mut model, Message::Ui(UiMessage::StartEditTags));

        // Type tags with extra whitespace
        model.input_buffer = "  work  ,  play  , rest ".to_string();
        model.cursor_position = model.input_buffer.len();

        // Submit
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Tags should be trimmed
        let task = model.tasks.get(&task_id).unwrap();
        assert_eq!(task.tags, vec!["work", "play", "rest"]);
    }

    #[test]
    fn test_edit_tags_filters_empty() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Start editing tags
        update(&mut model, Message::Ui(UiMessage::StartEditTags));

        // Type tags with empty entries
        model.input_buffer = "work,,, ,play".to_string();
        model.cursor_position = model.input_buffer.len();

        // Submit
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Only non-empty tags should remain
        let task = model.tasks.get(&task_id).unwrap();
        assert_eq!(task.tags, vec!["work", "play"]);
    }

    #[test]
    fn test_cancel_edit_tags() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Set initial tags
        let original_tags = vec!["original".to_string()];
        model.tasks.get_mut(&task_id).unwrap().tags = original_tags.clone();

        // Start editing
        update(&mut model, Message::Ui(UiMessage::StartEditTags));

        // Type something different
        model.input_buffer = "new, tags, here".to_string();

        // Cancel
        update(&mut model, Message::Ui(UiMessage::CancelInput));

        // Tags should NOT be changed
        let task = model.tasks.get(&task_id).unwrap();
        assert_eq!(task.tags, original_tags);
        assert_eq!(model.input_mode, InputMode::Normal);
    }

    // Description editing tests
    #[test]
    fn test_start_edit_description_enters_edit_mode() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Task starts with no description
        assert!(model.tasks.get(&task_id).unwrap().description.is_none());

        update(&mut model, Message::Ui(UiMessage::StartEditDescription));

        assert_eq!(model.input_mode, InputMode::Editing);
        assert!(matches!(
            model.input_target,
            InputTarget::EditDescription(_)
        ));
        assert!(model.input_buffer.is_empty());
    }

    #[test]
    fn test_start_edit_description_prefills_existing() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Set existing description
        model.tasks.get_mut(&task_id).unwrap().description =
            Some("Existing notes here".to_string());

        update(&mut model, Message::Ui(UiMessage::StartEditDescription));

        assert_eq!(model.input_buffer, "Existing notes here");
    }

    #[test]
    fn test_edit_description_add_new() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Start editing description
        update(&mut model, Message::Ui(UiMessage::StartEditDescription));

        // Type new description
        model.input_buffer = "This is a detailed task description".to_string();
        model.cursor_position = model.input_buffer.len();

        // Submit
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Description should be set
        let task = model.tasks.get(&task_id).unwrap();
        assert_eq!(
            task.description,
            Some("This is a detailed task description".to_string())
        );
    }

    #[test]
    fn test_edit_description_clear() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Set initial description
        model.tasks.get_mut(&task_id).unwrap().description = Some("Old description".to_string());

        // Start editing
        update(&mut model, Message::Ui(UiMessage::StartEditDescription));

        // Clear input
        model.input_buffer.clear();
        model.cursor_position = 0;

        // Submit empty
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Description should be cleared
        let task = model.tasks.get(&task_id).unwrap();
        assert!(task.description.is_none());
    }

    #[test]
    fn test_edit_description_undo() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Start with no description
        assert!(model.tasks.get(&task_id).unwrap().description.is_none());

        // Add a description
        update(&mut model, Message::Ui(UiMessage::StartEditDescription));
        model.input_buffer = "New description".to_string();
        model.cursor_position = model.input_buffer.len();
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Verify description was set
        assert_eq!(
            model.tasks.get(&task_id).unwrap().description,
            Some("New description".to_string())
        );

        // Undo
        update(&mut model, Message::System(SystemMessage::Undo));

        // Description should be gone
        assert!(model.tasks.get(&task_id).unwrap().description.is_none());
    }

    // Move to project tests
    #[test]
    fn test_start_move_to_project() {
        use crate::domain::Project;

        let mut model = create_test_model_with_tasks();
        let _task_id = model.visible_tasks[0].clone();

        // Add some projects
        let project1 = Project::new("Project Alpha");
        let project2 = Project::new("Project Beta");
        model.projects.insert(project1.id.clone(), project1);
        model.projects.insert(project2.id.clone(), project2);

        // Start move to project
        update(&mut model, Message::Ui(UiMessage::StartMoveToProject));

        assert_eq!(model.input_mode, InputMode::Editing);
        assert!(matches!(model.input_target, InputTarget::MoveToProject(_)));
        // Input buffer should contain project list
        assert!(model.input_buffer.contains("0: (none)"));
    }

    #[test]
    fn test_move_to_project_assign() {
        use crate::domain::Project;

        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Initially no project
        assert!(model.tasks.get(&task_id).unwrap().project_id.is_none());

        // Add a project
        let project = Project::new("Test Project");
        let project_id = project.id.clone();
        model.projects.insert(project.id.clone(), project);

        // Start move to project
        update(&mut model, Message::Ui(UiMessage::StartMoveToProject));

        // Type "1" to select the first project
        model.input_buffer = "1".to_string();
        model.cursor_position = 1;

        // Submit
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Task should now belong to the project
        let task = model.tasks.get(&task_id).unwrap();
        assert_eq!(task.project_id, Some(project_id));
        assert_eq!(model.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_move_to_project_remove() {
        use crate::domain::Project;

        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Add a project and assign task to it
        let project = Project::new("Test Project");
        let project_id = project.id.clone();
        model.projects.insert(project.id.clone(), project);
        model.tasks.get_mut(&task_id).unwrap().project_id = Some(project_id);

        // Start move to project
        update(&mut model, Message::Ui(UiMessage::StartMoveToProject));

        // Type "0" to remove from project
        model.input_buffer = "0".to_string();
        model.cursor_position = 1;

        // Submit
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Task should no longer belong to any project
        let task = model.tasks.get(&task_id).unwrap();
        assert!(task.project_id.is_none());
    }

    #[test]
    fn test_move_to_project_undo() {
        use crate::domain::Project;

        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Add a project
        let project = Project::new("Test Project");
        let project_id = project.id.clone();
        model.projects.insert(project.id.clone(), project);

        // Move task to project
        update(&mut model, Message::Ui(UiMessage::StartMoveToProject));
        model.input_buffer = "1".to_string();
        model.cursor_position = 1;
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Verify task is in project
        assert_eq!(
            model.tasks.get(&task_id).unwrap().project_id,
            Some(project_id)
        );

        // Undo
        update(&mut model, Message::System(SystemMessage::Undo));

        // Task should no longer be in project
        assert!(model.tasks.get(&task_id).unwrap().project_id.is_none());
    }

    #[test]
    fn test_move_to_project_invalid_input_ignored() {
        use crate::domain::Project;

        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Add a project
        let project = Project::new("Test Project");
        model.projects.insert(project.id.clone(), project);

        // Start move to project
        update(&mut model, Message::Ui(UiMessage::StartMoveToProject));

        // Type invalid input
        model.input_buffer = "abc".to_string();
        model.cursor_position = 3;

        // Submit
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Task should not have changed
        let task = model.tasks.get(&task_id).unwrap();
        assert!(task.project_id.is_none());
    }

    #[test]
    fn test_move_to_project_out_of_range_ignored() {
        use crate::domain::Project;

        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Add one project
        let project = Project::new("Test Project");
        model.projects.insert(project.id.clone(), project);

        // Start move to project
        update(&mut model, Message::Ui(UiMessage::StartMoveToProject));

        // Type index out of range (99)
        model.input_buffer = "99".to_string();
        model.cursor_position = 2;

        // Submit
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Task should not have changed (out of range index is ignored)
        let task = model.tasks.get(&task_id).unwrap();
        assert!(task.project_id.is_none());
    }

    // Tag filter tests
    #[test]
    fn test_start_filter_by_tag() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Add tags to task
        model.tasks.get_mut(&task_id).unwrap().tags =
            vec!["work".to_string(), "urgent".to_string()];

        update(&mut model, Message::Ui(UiMessage::StartFilterByTag));

        assert_eq!(model.input_mode, InputMode::Editing);
        assert!(matches!(model.input_target, InputTarget::FilterByTag));
        // Input buffer should show available tags
        assert!(model.input_buffer.contains("Available:"));
        assert!(model.input_buffer.contains("urgent"));
        assert!(model.input_buffer.contains("work"));
    }

    #[test]
    fn test_filter_by_tag_submit() {
        let mut model = Model::new();

        // Create one tagged task and one untagged
        let task_tagged = Task::new("Tagged task").with_tags(vec!["work".to_string()]);
        let task_untagged = Task::new("Untagged task");

        model
            .tasks
            .insert(task_tagged.id.clone(), task_tagged.clone());
        model.tasks.insert(task_untagged.id.clone(), task_untagged);
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks.len(), 2);

        // Start filter
        update(&mut model, Message::Ui(UiMessage::StartFilterByTag));

        // Type tag to filter
        model.input_buffer = "work".to_string();
        model.cursor_position = 4;

        // Submit
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Only tagged task should be visible
        assert_eq!(model.filter.tags, Some(vec!["work".to_string()]));
        assert_eq!(model.visible_tasks.len(), 1);
        assert!(model.visible_tasks.contains(&task_tagged.id));
    }

    #[test]
    fn test_filter_by_tag_multiple_tags() {
        let mut model = Model::new();

        // Create tasks with different tags
        let task_work =
            Task::new("Work task").with_tags(vec!["work".to_string(), "urgent".to_string()]);
        let task_home = Task::new("Home task").with_tags(vec!["home".to_string()]);
        let task_work_only = Task::new("Work only").with_tags(vec!["work".to_string()]);

        model.tasks.insert(task_work.id.clone(), task_work.clone());
        model.tasks.insert(task_home.id.clone(), task_home);
        model
            .tasks
            .insert(task_work_only.id.clone(), task_work_only.clone());
        model.refresh_visible_tasks();

        // Start filter
        update(&mut model, Message::Ui(UiMessage::StartFilterByTag));

        // Type multiple tags (Any mode will match tasks with either)
        model.input_buffer = "work, urgent".to_string();
        model.cursor_position = model.input_buffer.len();

        // Submit
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Both work tasks should be visible (Any mode)
        assert_eq!(model.visible_tasks.len(), 2);
        assert!(model.visible_tasks.contains(&task_work.id));
        assert!(model.visible_tasks.contains(&task_work_only.id));
    }

    #[test]
    fn test_clear_tag_filter() {
        let mut model = Model::new();

        // Add one tagged task and one untagged
        let task_tagged = Task::new("Tagged").with_tags(vec!["work".to_string()]);
        let task_untagged = Task::new("Untagged");

        model.tasks.insert(task_tagged.id.clone(), task_tagged);
        model.tasks.insert(task_untagged.id.clone(), task_untagged);
        model.refresh_visible_tasks();

        // Set tag filter
        model.filter.tags = Some(vec!["work".to_string()]);
        model.refresh_visible_tasks();
        assert_eq!(model.visible_tasks.len(), 1);

        // Clear filter
        update(&mut model, Message::Ui(UiMessage::ClearTagFilter));

        assert!(model.filter.tags.is_none());
        assert_eq!(model.visible_tasks.len(), 2);
    }

    #[test]
    fn test_filter_by_tag_empty_clears() {
        let mut model = create_test_model_with_tasks();

        // Set initial tag filter
        model.filter.tags = Some(vec!["work".to_string()]);

        // Start filter
        update(&mut model, Message::Ui(UiMessage::StartFilterByTag));

        // Clear input
        model.input_buffer.clear();
        model.cursor_position = 0;

        // Submit empty
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Filter should be cleared
        assert!(model.filter.tags.is_none());
    }

    #[test]
    fn test_filter_by_tag_preserves_existing() {
        let mut model = create_test_model_with_tasks();

        // Set initial tag filter
        model.filter.tags = Some(vec!["work".to_string()]);

        // Start filter - should pre-fill with existing
        update(&mut model, Message::Ui(UiMessage::StartFilterByTag));

        assert_eq!(model.input_buffer, "work");
        assert_eq!(model.cursor_position, 4);
    }

    // Undo tests
    #[test]
    fn test_undo_task_create() {
        let mut model = Model::new();
        assert!(model.tasks.is_empty());
        assert!(model.undo_stack.is_empty());

        // Create a task
        update(
            &mut model,
            Message::Task(TaskMessage::Create("New task".to_string())),
        );

        assert_eq!(model.tasks.len(), 1);
        assert_eq!(model.undo_stack.len(), 1);

        // Undo should remove the task
        update(&mut model, Message::System(SystemMessage::Undo));

        assert!(model.tasks.is_empty());
        assert!(model.undo_stack.is_empty());
    }

    #[test]
    fn test_undo_task_delete() {
        let mut model = create_test_model_with_tasks();
        let initial_count = model.tasks.len();
        let task_id = model.visible_tasks[0].clone();
        let original_title = model.tasks.get(&task_id).unwrap().title.clone();

        // Delete the task via confirm dialog path
        model.selected_index = 0;
        update(&mut model, Message::Ui(UiMessage::ShowDeleteConfirm));
        update(&mut model, Message::Ui(UiMessage::ConfirmDelete));

        assert_eq!(model.tasks.len(), initial_count - 1);
        assert!(!model.tasks.contains_key(&task_id));

        // Undo should restore the task
        update(&mut model, Message::System(SystemMessage::Undo));

        assert_eq!(model.tasks.len(), initial_count);
        let restored_task = model.tasks.get(&task_id).unwrap();
        assert_eq!(restored_task.title, original_title);
    }

    #[test]
    fn test_undo_task_toggle_complete() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Task starts as Todo
        assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Todo);

        // Toggle complete
        update(&mut model, Message::Task(TaskMessage::ToggleComplete));

        assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Done);

        // Undo should restore to Todo
        update(&mut model, Message::System(SystemMessage::Undo));

        assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Todo);
    }

    #[test]
    fn test_undo_task_edit_title() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();
        let original_title = model.tasks.get(&task_id).unwrap().title.clone();

        // Edit the title
        update(&mut model, Message::Ui(UiMessage::StartEditTask));
        model.input_buffer = "Changed Title".to_string();
        model.cursor_position = model.input_buffer.len();
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        assert_eq!(model.tasks.get(&task_id).unwrap().title, "Changed Title");

        // Undo should restore original title
        update(&mut model, Message::System(SystemMessage::Undo));

        assert_eq!(model.tasks.get(&task_id).unwrap().title, original_title);
    }

    #[test]
    fn test_undo_task_cycle_priority() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Set initial priority
        model.tasks.get_mut(&task_id).unwrap().priority = Priority::None;

        // Cycle priority
        update(&mut model, Message::Task(TaskMessage::CyclePriority));

        assert_eq!(model.tasks.get(&task_id).unwrap().priority, Priority::Low);

        // Undo should restore to None
        update(&mut model, Message::System(SystemMessage::Undo));

        assert_eq!(model.tasks.get(&task_id).unwrap().priority, Priority::None);
    }

    #[test]
    fn test_undo_project_create() {
        let mut model = Model::new();
        assert!(model.projects.is_empty());

        // Create a project
        update(&mut model, Message::Ui(UiMessage::StartCreateProject));
        for c in "My Project".chars() {
            update(&mut model, Message::Ui(UiMessage::InputChar(c)));
        }
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        assert_eq!(model.projects.len(), 1);

        // Undo should remove the project
        update(&mut model, Message::System(SystemMessage::Undo));

        assert!(model.projects.is_empty());
    }

    #[test]
    fn test_undo_multiple_actions() {
        let mut model = Model::new();

        // Create three tasks
        for i in 1..=3 {
            update(
                &mut model,
                Message::Task(TaskMessage::Create(format!("Task {}", i))),
            );
        }

        assert_eq!(model.tasks.len(), 3);
        assert_eq!(model.undo_stack.len(), 3);

        // Undo all three
        update(&mut model, Message::System(SystemMessage::Undo));
        assert_eq!(model.tasks.len(), 2);

        update(&mut model, Message::System(SystemMessage::Undo));
        assert_eq!(model.tasks.len(), 1);

        update(&mut model, Message::System(SystemMessage::Undo));
        assert!(model.tasks.is_empty());
        assert!(model.undo_stack.is_empty());
    }

    #[test]
    fn test_undo_empty_stack() {
        let mut model = Model::new();
        assert!(model.undo_stack.is_empty());

        // Undo with empty stack should do nothing
        update(&mut model, Message::System(SystemMessage::Undo));

        assert!(model.undo_stack.is_empty());
    }

    #[test]
    fn test_undo_edit_due_date() {
        use chrono::NaiveDate;
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Set initial due date
        let original_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        model.tasks.get_mut(&task_id).unwrap().due_date = Some(original_date);

        // Edit due date
        update(&mut model, Message::Ui(UiMessage::StartEditDueDate));
        model.input_buffer = "2025-12-25".to_string();
        model.cursor_position = model.input_buffer.len();
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        assert_eq!(
            model.tasks.get(&task_id).unwrap().due_date,
            Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap())
        );

        // Undo should restore original date
        update(&mut model, Message::System(SystemMessage::Undo));

        assert_eq!(
            model.tasks.get(&task_id).unwrap().due_date,
            Some(original_date)
        );
    }

    #[test]
    fn test_undo_edit_tags() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Set initial tags
        model.tasks.get_mut(&task_id).unwrap().tags = vec!["original".to_string()];

        // Edit tags
        update(&mut model, Message::Ui(UiMessage::StartEditTags));
        model.input_buffer = "new, tags".to_string();
        model.cursor_position = model.input_buffer.len();
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        assert_eq!(model.tasks.get(&task_id).unwrap().tags, vec!["new", "tags"]);

        // Undo should restore original tags
        update(&mut model, Message::System(SystemMessage::Undo));

        assert_eq!(
            model.tasks.get(&task_id).unwrap().tags,
            vec!["original".to_string()]
        );
    }

    // Redo tests
    #[test]
    fn test_redo_task_create() {
        let mut model = Model::new();

        // Create a task
        update(
            &mut model,
            Message::Task(TaskMessage::Create("New task".to_string())),
        );
        let task_id = model.visible_tasks[0].clone();
        assert_eq!(model.tasks.len(), 1);

        // Undo should remove the task
        update(&mut model, Message::System(SystemMessage::Undo));
        assert!(model.tasks.is_empty());
        assert!(model.undo_stack.can_redo());

        // Redo should restore the task
        update(&mut model, Message::System(SystemMessage::Redo));
        assert_eq!(model.tasks.len(), 1);
        assert!(model.tasks.contains_key(&task_id));
        assert!(!model.undo_stack.can_redo());
    }

    #[test]
    fn test_redo_task_delete() {
        let mut model = create_test_model_with_tasks();
        let initial_count = model.tasks.len();
        let task_id = model.visible_tasks[0].clone();

        // Delete the task
        model.selected_index = 0;
        update(&mut model, Message::Ui(UiMessage::ShowDeleteConfirm));
        update(&mut model, Message::Ui(UiMessage::ConfirmDelete));
        assert_eq!(model.tasks.len(), initial_count - 1);

        // Undo should restore the task
        update(&mut model, Message::System(SystemMessage::Undo));
        assert_eq!(model.tasks.len(), initial_count);

        // Redo should delete it again
        update(&mut model, Message::System(SystemMessage::Redo));
        assert_eq!(model.tasks.len(), initial_count - 1);
        assert!(!model.tasks.contains_key(&task_id));
    }

    #[test]
    fn test_redo_task_modify() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();
        let original_title = model.tasks.get(&task_id).unwrap().title.clone();

        // Edit the title
        update(&mut model, Message::Ui(UiMessage::StartEditTask));
        model.input_buffer = "New Title".to_string();
        model.cursor_position = model.input_buffer.len();
        update(&mut model, Message::Ui(UiMessage::SubmitInput));
        assert_eq!(model.tasks.get(&task_id).unwrap().title, "New Title");

        // Undo should restore original
        update(&mut model, Message::System(SystemMessage::Undo));
        assert_eq!(model.tasks.get(&task_id).unwrap().title, original_title);

        // Redo should apply the change again
        update(&mut model, Message::System(SystemMessage::Redo));
        assert_eq!(model.tasks.get(&task_id).unwrap().title, "New Title");
    }

    #[test]
    fn test_redo_project_create() {
        let mut model = Model::new();

        // Create a project
        update(&mut model, Message::Ui(UiMessage::StartCreateProject));
        for c in "My Project".chars() {
            update(&mut model, Message::Ui(UiMessage::InputChar(c)));
        }
        update(&mut model, Message::Ui(UiMessage::SubmitInput));
        assert_eq!(model.projects.len(), 1);
        let project_id = model.projects.keys().next().unwrap().clone();

        // Undo should remove the project
        update(&mut model, Message::System(SystemMessage::Undo));
        assert!(model.projects.is_empty());

        // Redo should restore the project
        update(&mut model, Message::System(SystemMessage::Redo));
        assert_eq!(model.projects.len(), 1);
        assert!(model.projects.contains_key(&project_id));
    }

    #[test]
    fn test_new_action_clears_redo() {
        let mut model = Model::new();

        // Create and undo a task
        update(
            &mut model,
            Message::Task(TaskMessage::Create("Task 1".to_string())),
        );
        update(&mut model, Message::System(SystemMessage::Undo));
        assert!(model.undo_stack.can_redo());

        // New action should clear redo
        update(
            &mut model,
            Message::Task(TaskMessage::Create("Task 2".to_string())),
        );
        assert!(!model.undo_stack.can_redo());
    }

    #[test]
    fn test_multiple_undo_redo() {
        let mut model = Model::new();

        // Create 3 tasks
        for i in 1..=3 {
            update(
                &mut model,
                Message::Task(TaskMessage::Create(format!("Task {}", i))),
            );
        }
        assert_eq!(model.tasks.len(), 3);

        // Undo all 3
        update(&mut model, Message::System(SystemMessage::Undo));
        update(&mut model, Message::System(SystemMessage::Undo));
        update(&mut model, Message::System(SystemMessage::Undo));
        assert!(model.tasks.is_empty());
        assert_eq!(model.undo_stack.redo_len(), 3);

        // Redo 2
        update(&mut model, Message::System(SystemMessage::Redo));
        update(&mut model, Message::System(SystemMessage::Redo));
        assert_eq!(model.tasks.len(), 2);
        assert_eq!(model.undo_stack.redo_len(), 1);

        // Undo 1
        update(&mut model, Message::System(SystemMessage::Undo));
        assert_eq!(model.tasks.len(), 1);
        assert_eq!(model.undo_stack.redo_len(), 2);
    }

    #[test]
    fn test_redo_empty_does_nothing() {
        let mut model = Model::new();
        assert!(!model.undo_stack.can_redo());

        // Redo with empty stack should do nothing
        update(&mut model, Message::System(SystemMessage::Redo));
        assert!(model.tasks.is_empty());
    }

    // Subtask tests
    #[test]
    fn test_start_create_subtask() {
        let mut model = create_test_model_with_tasks();
        let _parent_id = model.visible_tasks[0].clone();

        update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));

        assert_eq!(model.input_mode, InputMode::Editing);
        assert!(matches!(model.input_target, InputTarget::Subtask(_)));
        assert!(model.input_buffer.is_empty());
    }

    #[test]
    fn test_start_create_subtask_no_selection() {
        let mut model = Model::new();
        // No tasks, so no selection

        update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));

        // Should remain in normal mode since there's no parent task
        assert_eq!(model.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_submit_subtask_creates_with_parent() {
        let mut model = create_test_model_with_tasks();
        let parent_id = model.visible_tasks[0].clone();
        let initial_count = model.tasks.len();

        // Start creating subtask
        update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));

        // Type subtask name
        model.input_buffer = "My subtask".to_string();
        model.cursor_position = model.input_buffer.len();

        // Submit
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Should have one more task
        assert_eq!(model.tasks.len(), initial_count + 1);

        // Find the new subtask
        let subtask = model
            .tasks
            .values()
            .find(|t| t.title == "My subtask")
            .expect("Subtask should exist");

        // Should have parent_task_id set
        assert_eq!(subtask.parent_task_id, Some(parent_id));
    }

    #[test]
    fn test_subtask_inherits_default_priority() {
        let mut model = create_test_model_with_tasks();
        model.default_priority = Priority::High;

        // Start creating subtask
        update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));
        model.input_buffer = "Priority subtask".to_string();
        model.cursor_position = model.input_buffer.len();
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        let subtask = model
            .tasks
            .values()
            .find(|t| t.title == "Priority subtask")
            .expect("Subtask should exist");

        assert_eq!(subtask.priority, Priority::High);
    }

    #[test]
    fn test_cancel_subtask_creation() {
        let mut model = create_test_model_with_tasks();
        let initial_count = model.tasks.len();

        // Start creating subtask
        update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));

        // Type something
        model.input_buffer = "Will be cancelled".to_string();

        // Cancel
        update(&mut model, Message::Ui(UiMessage::CancelInput));

        // No new task should be created
        assert_eq!(model.tasks.len(), initial_count);
        assert_eq!(model.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_subtask_empty_name_not_created() {
        let mut model = create_test_model_with_tasks();
        let initial_count = model.tasks.len();

        // Start creating subtask
        update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));

        // Submit with empty name
        model.input_buffer = "   ".to_string();
        model.cursor_position = model.input_buffer.len();
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // No new task should be created
        assert_eq!(model.tasks.len(), initial_count);
        assert_eq!(model.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_subtask_undo() {
        let mut model = create_test_model_with_tasks();
        let initial_count = model.tasks.len();

        // Create subtask
        update(&mut model, Message::Ui(UiMessage::StartCreateSubtask));
        model.input_buffer = "Subtask to undo".to_string();
        model.cursor_position = model.input_buffer.len();
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        assert_eq!(model.tasks.len(), initial_count + 1);

        // Undo
        update(&mut model, Message::System(SystemMessage::Undo));

        assert_eq!(model.tasks.len(), initial_count);
        assert!(!model.tasks.values().any(|t| t.title == "Subtask to undo"));
    }

    // Bulk operation tests
    #[test]
    fn test_toggle_multi_select() {
        let mut model = create_test_model_with_tasks();

        assert!(!model.multi_select_mode);

        update(&mut model, Message::Ui(UiMessage::ToggleMultiSelect));
        assert!(model.multi_select_mode);

        update(&mut model, Message::Ui(UiMessage::ToggleMultiSelect));
        assert!(!model.multi_select_mode);
    }

    #[test]
    fn test_toggle_task_selection() {
        let mut model = create_test_model_with_tasks();
        model.multi_select_mode = true;
        let task_id = model.visible_tasks[0].clone();

        assert!(!model.selected_tasks.contains(&task_id));

        update(&mut model, Message::Ui(UiMessage::ToggleTaskSelection));
        assert!(model.selected_tasks.contains(&task_id));

        update(&mut model, Message::Ui(UiMessage::ToggleTaskSelection));
        assert!(!model.selected_tasks.contains(&task_id));
    }

    #[test]
    fn test_toggle_task_selection_not_in_multi_mode() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Not in multi-select mode
        update(&mut model, Message::Ui(UiMessage::ToggleTaskSelection));

        // Should not select anything
        assert!(!model.selected_tasks.contains(&task_id));
    }

    #[test]
    fn test_select_all() {
        let mut model = create_test_model_with_tasks();
        let task_count = model.visible_tasks.len();

        assert!(!model.multi_select_mode);
        assert!(model.selected_tasks.is_empty());

        update(&mut model, Message::Ui(UiMessage::SelectAll));

        assert!(model.multi_select_mode);
        assert_eq!(model.selected_tasks.len(), task_count);
    }

    #[test]
    fn test_clear_selection() {
        let mut model = create_test_model_with_tasks();
        model.multi_select_mode = true;
        model.selected_tasks = model.visible_tasks.iter().cloned().collect();

        update(&mut model, Message::Ui(UiMessage::ClearSelection));

        assert!(!model.multi_select_mode);
        assert!(model.selected_tasks.is_empty());
    }

    #[test]
    fn test_bulk_delete() {
        let mut model = create_test_model_with_tasks();
        let initial_count = model.tasks.len();

        // Select first two tasks
        model.multi_select_mode = true;
        let task1 = model.visible_tasks[0].clone();
        let task2 = model.visible_tasks[1].clone();
        model.selected_tasks.insert(task1);
        model.selected_tasks.insert(task2);

        update(&mut model, Message::Ui(UiMessage::BulkDelete));

        assert_eq!(model.tasks.len(), initial_count - 2);
        assert!(!model.multi_select_mode);
        assert!(model.selected_tasks.is_empty());
    }

    #[test]
    fn test_bulk_delete_not_in_multi_mode() {
        let mut model = create_test_model_with_tasks();
        let initial_count = model.tasks.len();

        // Not in multi-select mode
        update(&mut model, Message::Ui(UiMessage::BulkDelete));

        // Nothing should be deleted
        assert_eq!(model.tasks.len(), initial_count);
    }

    #[test]
    fn test_exiting_multi_select_clears_selection() {
        let mut model = create_test_model_with_tasks();
        model.multi_select_mode = true;
        model.selected_tasks = model.visible_tasks.iter().cloned().collect();

        // Exit multi-select mode
        update(&mut model, Message::Ui(UiMessage::ToggleMultiSelect));

        assert!(!model.multi_select_mode);
        assert!(model.selected_tasks.is_empty());
    }

    // Recurrence tests
    #[test]
    fn test_set_recurrence_daily() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Start editing recurrence
        update(&mut model, Message::Ui(UiMessage::StartEditRecurrence));
        assert_eq!(model.input_mode, InputMode::Editing);

        // Set to daily
        model.input_buffer = "d".to_string();
        model.cursor_position = 1;
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        let task = model.tasks.get(&task_id).unwrap();
        assert!(matches!(
            task.recurrence,
            Some(crate::domain::Recurrence::Daily)
        ));
    }

    #[test]
    fn test_set_recurrence_weekly() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        update(&mut model, Message::Ui(UiMessage::StartEditRecurrence));
        model.input_buffer = "w".to_string();
        model.cursor_position = 1;
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        let task = model.tasks.get(&task_id).unwrap();
        assert!(matches!(
            task.recurrence,
            Some(crate::domain::Recurrence::Weekly { .. })
        ));
    }

    #[test]
    fn test_clear_recurrence() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // First set recurrence
        if let Some(task) = model.tasks.get_mut(&task_id) {
            task.recurrence = Some(crate::domain::Recurrence::Daily);
        }

        // Now clear it
        update(&mut model, Message::Ui(UiMessage::StartEditRecurrence));
        model.input_buffer = "0".to_string();
        model.cursor_position = 1;
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        let task = model.tasks.get(&task_id).unwrap();
        assert!(task.recurrence.is_none());
    }

    #[test]
    fn test_completing_recurring_task_creates_next() {
        use chrono::NaiveDate;
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();
        let initial_count = model.tasks.len();

        // Set task as recurring with a due date
        if let Some(task) = model.tasks.get_mut(&task_id) {
            task.recurrence = Some(crate::domain::Recurrence::Daily);
            task.due_date = Some(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap());
        }
        model.refresh_visible_tasks();

        // Complete the task
        update(&mut model, Message::Task(TaskMessage::ToggleComplete));

        // Should have created a new task
        assert_eq!(model.tasks.len(), initial_count + 1);

        // The new task should have the same title and be recurring
        let new_tasks: Vec<_> = model
            .tasks
            .values()
            .filter(|t| t.id != task_id && t.recurrence.is_some())
            .collect();
        assert_eq!(new_tasks.len(), 1);
        let new_task = new_tasks[0];
        assert!(new_task.recurrence.is_some());
        assert!(new_task.due_date.is_some());
    }

    #[test]
    fn test_completing_non_recurring_task_no_new_task() {
        let mut model = create_test_model_with_tasks();
        let initial_count = model.tasks.len();

        // Complete a non-recurring task
        update(&mut model, Message::Task(TaskMessage::ToggleComplete));

        // Should NOT create a new task
        assert_eq!(model.tasks.len(), initial_count);
    }

    #[test]
    fn test_recurrence_undo() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Set recurrence
        update(&mut model, Message::Ui(UiMessage::StartEditRecurrence));
        model.input_buffer = "d".to_string();
        model.cursor_position = 1;
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        assert!(model.tasks.get(&task_id).unwrap().recurrence.is_some());

        // Undo
        update(&mut model, Message::System(SystemMessage::Undo));

        assert!(model.tasks.get(&task_id).unwrap().recurrence.is_none());
    }

    // Task chain tests
    #[test]
    fn test_start_link_task_enters_editing_mode() {
        let mut model = create_test_model_with_tasks();
        assert_eq!(model.input_mode, InputMode::Normal);

        update(&mut model, Message::Ui(UiMessage::StartLinkTask));

        assert_eq!(model.input_mode, InputMode::Editing);
        assert!(matches!(model.input_target, InputTarget::LinkTask(_)));
    }

    #[test]
    fn test_start_link_task_shows_current_link() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();
        let target_id = model.visible_tasks[1].clone();
        let target_title = model.tasks.get(&target_id).unwrap().title.clone();

        // Set existing link
        model.tasks.get_mut(&task_id).unwrap().next_task_id = Some(target_id.clone());

        update(&mut model, Message::Ui(UiMessage::StartLinkTask));

        // Should show the linked task title
        assert_eq!(
            model.input_buffer,
            format!("Currently linked to: {target_title}")
        );
    }

    #[test]
    fn test_link_task_by_number() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();
        let target_id = model.visible_tasks[2].clone();

        update(&mut model, Message::Ui(UiMessage::StartLinkTask));

        // Enter task number "3" (1-indexed)
        model.input_buffer = "3".to_string();
        model.cursor_position = 1;

        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Should link to the third task
        assert_eq!(
            model.tasks.get(&task_id).unwrap().next_task_id,
            Some(target_id)
        );
    }

    #[test]
    fn test_link_task_by_title_search() {
        let mut model = Model::new();

        // Create tasks with distinct titles
        let task1 = Task::new("First task");
        let task2 = Task::new("Second task");
        let task3 = Task::new("Target unique title");
        let task1_id = task1.id.clone();
        let task3_id = task3.id.clone();

        model.tasks.insert(task1.id.clone(), task1);
        model.tasks.insert(task2.id.clone(), task2);
        model.tasks.insert(task3.id.clone(), task3);
        model.refresh_visible_tasks();

        // Find the visible index for task1
        let task1_visible_idx = model
            .visible_tasks
            .iter()
            .position(|id| *id == task1_id)
            .expect("task1 should be in visible_tasks");
        model.selected_index = task1_visible_idx;

        update(&mut model, Message::Ui(UiMessage::StartLinkTask));

        // Enter part of target title
        model.input_buffer = "Target unique".to_string();
        model.cursor_position = model.input_buffer.len();

        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Should link to the task with matching title
        assert_eq!(
            model.tasks.get(&task1_id).unwrap().next_task_id,
            Some(task3_id)
        );
    }

    #[test]
    fn test_link_task_prevents_self_linking() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        update(&mut model, Message::Ui(UiMessage::StartLinkTask));

        // Try to link task 1 to itself
        model.input_buffer = "1".to_string();
        model.cursor_position = 1;

        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Should NOT create self-link
        assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());
    }

    #[test]
    fn test_link_task_undo() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();
        let target_id = model.visible_tasks[1].clone();

        // Link task
        update(&mut model, Message::Ui(UiMessage::StartLinkTask));
        model.input_buffer = "2".to_string();
        model.cursor_position = 1;
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        assert_eq!(
            model.tasks.get(&task_id).unwrap().next_task_id,
            Some(target_id)
        );

        // Undo
        update(&mut model, Message::System(SystemMessage::Undo));

        assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());
    }

    #[test]
    fn test_unlink_task_removes_link() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();
        let target_id = model.visible_tasks[1].clone();

        // Set existing link
        model.tasks.get_mut(&task_id).unwrap().next_task_id = Some(target_id);

        update(&mut model, Message::Ui(UiMessage::UnlinkTask));

        assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());
    }

    #[test]
    fn test_unlink_task_when_not_linked_is_noop() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();

        // Ensure no link exists
        assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());

        update(&mut model, Message::Ui(UiMessage::UnlinkTask));

        // Should still be None, no error
        assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());
    }

    #[test]
    fn test_unlink_task_undo() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();
        let target_id = model.visible_tasks[1].clone();

        // Set existing link
        model.tasks.get_mut(&task_id).unwrap().next_task_id = Some(target_id.clone());

        // Unlink
        update(&mut model, Message::Ui(UiMessage::UnlinkTask));
        assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());

        // Undo
        update(&mut model, Message::System(SystemMessage::Undo));

        assert_eq!(
            model.tasks.get(&task_id).unwrap().next_task_id,
            Some(target_id)
        );
    }

    #[test]
    fn test_completing_chained_task_schedules_next() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();
        let next_id = model.visible_tasks[1].clone();

        // Link tasks
        model.tasks.get_mut(&task_id).unwrap().next_task_id = Some(next_id.clone());

        // Next task should have no scheduled date initially
        assert!(model.tasks.get(&next_id).unwrap().scheduled_date.is_none());

        // Complete the first task
        update(&mut model, Message::Task(TaskMessage::ToggleComplete));

        // Next task should now be scheduled for today (local time)
        let today = chrono::Local::now().date_naive();
        assert_eq!(
            model.tasks.get(&next_id).unwrap().scheduled_date,
            Some(today)
        );
    }

    #[test]
    fn test_completing_unchained_task_no_scheduling() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0].clone();
        let other_id = model.visible_tasks[1].clone();

        // No link - task is standalone
        assert!(model.tasks.get(&task_id).unwrap().next_task_id.is_none());

        // Other task has no scheduled date
        assert!(model.tasks.get(&other_id).unwrap().scheduled_date.is_none());

        // Complete the first task
        update(&mut model, Message::Task(TaskMessage::ToggleComplete));

        // Other task should NOT be scheduled
        assert!(model.tasks.get(&other_id).unwrap().scheduled_date.is_none());
    }

    // === Pomodoro Timer Tests ===

    #[test]
    fn test_pomodoro_start() {
        let mut model = create_test_model_with_tasks();

        assert!(model.pomodoro_session.is_none());
        assert!(!model.focus_mode);

        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
        );

        assert!(model.pomodoro_session.is_some());
        assert!(model.focus_mode);

        let session = model.pomodoro_session.as_ref().unwrap();
        assert_eq!(session.session_goal, 4);
        assert_eq!(session.cycles_completed, 0);
        assert!(!session.paused);
    }

    #[test]
    fn test_pomodoro_pause_resume() {
        let mut model = create_test_model_with_tasks();
        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
        );

        assert!(!model.pomodoro_session.as_ref().unwrap().paused);

        update(&mut model, Message::Pomodoro(PomodoroMessage::Pause));
        assert!(model.pomodoro_session.as_ref().unwrap().paused);

        update(&mut model, Message::Pomodoro(PomodoroMessage::Resume));
        assert!(!model.pomodoro_session.as_ref().unwrap().paused);
    }

    #[test]
    fn test_pomodoro_toggle_pause() {
        let mut model = create_test_model_with_tasks();
        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
        );

        assert!(!model.pomodoro_session.as_ref().unwrap().paused);

        update(&mut model, Message::Pomodoro(PomodoroMessage::TogglePause));
        assert!(model.pomodoro_session.as_ref().unwrap().paused);

        update(&mut model, Message::Pomodoro(PomodoroMessage::TogglePause));
        assert!(!model.pomodoro_session.as_ref().unwrap().paused);
    }

    #[test]
    fn test_pomodoro_stop() {
        let mut model = create_test_model_with_tasks();
        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
        );

        assert!(model.pomodoro_session.is_some());

        update(&mut model, Message::Pomodoro(PomodoroMessage::Stop));
        assert!(model.pomodoro_session.is_none());
    }

    #[test]
    fn test_pomodoro_tick_decrements_time() {
        let mut model = create_test_model_with_tasks();
        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
        );

        let initial_remaining = model.pomodoro_session.as_ref().unwrap().remaining_secs;

        update(&mut model, Message::Pomodoro(PomodoroMessage::Tick));

        assert_eq!(
            model.pomodoro_session.as_ref().unwrap().remaining_secs,
            initial_remaining - 1
        );
    }

    #[test]
    fn test_pomodoro_tick_paused_no_decrement() {
        let mut model = create_test_model_with_tasks();
        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
        );
        update(&mut model, Message::Pomodoro(PomodoroMessage::Pause));

        let initial_remaining = model.pomodoro_session.as_ref().unwrap().remaining_secs;

        update(&mut model, Message::Pomodoro(PomodoroMessage::Tick));

        // Time should not decrement when paused
        assert_eq!(
            model.pomodoro_session.as_ref().unwrap().remaining_secs,
            initial_remaining
        );
    }

    #[test]
    fn test_pomodoro_skip_phase() {
        use crate::domain::PomodoroPhase;

        let mut model = create_test_model_with_tasks();
        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
        );

        // Should be in Work phase
        assert_eq!(
            model.pomodoro_session.as_ref().unwrap().phase,
            PomodoroPhase::Work
        );

        // Skip to break
        update(&mut model, Message::Pomodoro(PomodoroMessage::Skip));

        // Should now be in ShortBreak phase and cycle completed
        assert_eq!(
            model.pomodoro_session.as_ref().unwrap().phase,
            PomodoroPhase::ShortBreak
        );
        assert_eq!(model.pomodoro_session.as_ref().unwrap().cycles_completed, 1);
    }

    #[test]
    fn test_pomodoro_goal_adjustment() {
        let mut model = create_test_model_with_tasks();
        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
        );

        assert_eq!(model.pomodoro_session.as_ref().unwrap().session_goal, 4);

        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::IncrementGoal),
        );
        assert_eq!(model.pomodoro_session.as_ref().unwrap().session_goal, 5);

        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::DecrementGoal),
        );
        assert_eq!(model.pomodoro_session.as_ref().unwrap().session_goal, 4);

        // Cannot go below 1
        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::DecrementGoal),
        );
        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::DecrementGoal),
        );
        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::DecrementGoal),
        );
        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::DecrementGoal),
        );
        assert_eq!(model.pomodoro_session.as_ref().unwrap().session_goal, 1);
    }

    #[test]
    fn test_pomodoro_config_changes() {
        let mut model = Model::new();

        assert_eq!(model.pomodoro_config.work_duration_mins, 25);

        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::SetWorkDuration(30)),
        );
        assert_eq!(model.pomodoro_config.work_duration_mins, 30);

        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::SetShortBreak(10)),
        );
        assert_eq!(model.pomodoro_config.short_break_mins, 10);

        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::SetLongBreak(20)),
        );
        assert_eq!(model.pomodoro_config.long_break_mins, 20);

        update(
            &mut model,
            Message::Pomodoro(PomodoroMessage::SetCyclesBeforeLongBreak(3)),
        );
        assert_eq!(model.pomodoro_config.cycles_before_long_break, 3);
    }

    // ============ Keybindings Editor Tests ============

    #[test]
    fn test_show_keybindings_editor() {
        let mut model = Model::new();
        assert!(!model.show_keybindings_editor);

        update(&mut model, Message::Ui(UiMessage::ShowKeybindingsEditor));
        assert!(model.show_keybindings_editor);
        assert_eq!(model.keybinding_selected, 0);
        assert!(!model.keybinding_capturing);
    }

    #[test]
    fn test_hide_keybindings_editor() {
        let mut model = Model::new();
        model.show_keybindings_editor = true;
        model.keybinding_capturing = true;

        update(&mut model, Message::Ui(UiMessage::HideKeybindingsEditor));
        assert!(!model.show_keybindings_editor);
        assert!(!model.keybinding_capturing);
    }

    #[test]
    fn test_keybindings_navigation() {
        let mut model = Model::new();
        model.show_keybindings_editor = true;
        model.keybinding_selected = 5;

        update(&mut model, Message::Ui(UiMessage::KeybindingsUp));
        assert_eq!(model.keybinding_selected, 4);

        update(&mut model, Message::Ui(UiMessage::KeybindingsDown));
        assert_eq!(model.keybinding_selected, 5);

        // Navigate up at 0 should stay at 0
        model.keybinding_selected = 0;
        update(&mut model, Message::Ui(UiMessage::KeybindingsUp));
        assert_eq!(model.keybinding_selected, 0);
    }

    #[test]
    fn test_start_edit_keybinding() {
        let mut model = Model::new();
        model.show_keybindings_editor = true;

        update(&mut model, Message::Ui(UiMessage::StartEditKeybinding));
        assert!(model.keybinding_capturing);
        assert!(model.status_message.is_some());
    }

    #[test]
    fn test_cancel_edit_keybinding() {
        let mut model = Model::new();
        model.show_keybindings_editor = true;
        model.keybinding_capturing = true;
        model.status_message = Some("Press a key...".to_string());

        update(&mut model, Message::Ui(UiMessage::CancelEditKeybinding));
        assert!(!model.keybinding_capturing);
        assert!(model.status_message.is_none());
    }

    #[test]
    fn test_apply_keybinding() {
        let mut model = Model::new();
        model.show_keybindings_editor = true;
        model.keybinding_capturing = true;

        // Get the first binding's action
        let bindings = model.keybindings.sorted_bindings();
        let (_, first_action) = &bindings[0];
        let original_action = first_action.clone();

        // Apply a new key to that action
        update(
            &mut model,
            Message::Ui(UiMessage::ApplyKeybinding("z".to_string())),
        );

        assert!(!model.keybinding_capturing);
        // The action should now be bound to 'z'
        assert_eq!(model.keybindings.get_action("z"), Some(&original_action));
    }

    #[test]
    fn test_reset_all_keybindings() {
        let mut model = Model::new();
        model.show_keybindings_editor = true;

        // Modify a keybinding
        model
            .keybindings
            .set_binding("z".to_string(), crate::config::Action::Quit);

        // Verify it was changed
        assert_eq!(
            model.keybindings.get_action("z"),
            Some(&crate::config::Action::Quit)
        );

        // Reset all
        update(&mut model, Message::Ui(UiMessage::ResetAllKeybindings));

        // Should be back to default (z is not a default binding)
        assert_eq!(model.keybindings.get_action("z"), None);
        assert!(model.status_message.is_some());
    }

    // Calendar focus tests

    #[test]
    fn test_calendar_focus_toggle() {
        let mut model = Model::new().with_sample_data();
        model.current_view = ViewId::Calendar;
        model.refresh_visible_tasks();

        // Initially focus should be on calendar grid
        assert!(!model.calendar_state.focus_task_list);

        // Focus task list (should work if there are tasks)
        update(
            &mut model,
            Message::Navigation(NavigationMessage::CalendarFocusTaskList),
        );

        // Should be focused on task list if there are tasks for the day
        if !model.tasks_for_selected_day().is_empty() {
            assert!(model.calendar_state.focus_task_list);
        }

        // Focus back to grid
        update(
            &mut model,
            Message::Navigation(NavigationMessage::CalendarFocusGrid),
        );
        assert!(!model.calendar_state.focus_task_list);
    }

    #[test]
    fn test_calendar_task_navigation() {
        use chrono::Datelike;

        let mut model = Model::new();

        // Add multiple tasks for the same day
        let today = chrono::Utc::now().date_naive();
        let task1 = crate::domain::Task::new("Task 1").with_due_date(today);
        let task2 = crate::domain::Task::new("Task 2").with_due_date(today);
        let task3 = crate::domain::Task::new("Task 3").with_due_date(today);

        model.tasks.insert(task1.id.clone(), task1);
        model.tasks.insert(task2.id.clone(), task2);
        model.tasks.insert(task3.id.clone(), task3);

        model.current_view = ViewId::Calendar;
        model.calendar_state.selected_day = Some(today.day());
        model.calendar_state.year = today.year();
        model.calendar_state.month = today.month();
        model.refresh_visible_tasks();

        // Focus on task list
        model.calendar_state.focus_task_list = true;
        model.selected_index = 0;

        // Navigate down
        update(&mut model, Message::Navigation(NavigationMessage::Down));
        assert_eq!(model.selected_index, 1);

        // Navigate down again
        update(&mut model, Message::Navigation(NavigationMessage::Down));
        assert_eq!(model.selected_index, 2);

        // Navigate down at end should stay at end
        update(&mut model, Message::Navigation(NavigationMessage::Down));
        assert_eq!(model.selected_index, 2);

        // Navigate up
        update(&mut model, Message::Navigation(NavigationMessage::Up));
        assert_eq!(model.selected_index, 1);
    }

    #[test]
    fn test_calendar_focus_reset_on_day_change() {
        let mut model = Model::new().with_sample_data();
        model.current_view = ViewId::Calendar;
        model.calendar_state.selected_day = Some(15);
        model.calendar_state.focus_task_list = true;

        // Select a new day
        update(
            &mut model,
            Message::Navigation(NavigationMessage::CalendarSelectDay(20)),
        );

        // Focus should be reset to grid
        assert!(!model.calendar_state.focus_task_list);
        assert_eq!(model.calendar_state.selected_day, Some(20));
    }

    #[test]
    fn test_calendar_focus_only_with_tasks() {
        let mut model = Model::new();
        model.current_view = ViewId::Calendar;
        model.calendar_state.selected_day = Some(15);
        model.refresh_visible_tasks();

        // No tasks for the day, focus should not switch
        assert!(!model.calendar_state.focus_task_list);

        update(
            &mut model,
            Message::Navigation(NavigationMessage::CalendarFocusTaskList),
        );

        // Should still be on grid since there are no tasks
        assert!(!model.calendar_state.focus_task_list);
    }

    #[test]
    fn test_calendar_task_actions_when_focused() {
        use chrono::Datelike;

        let mut model = Model::new();

        // Add a task for today
        let today = chrono::Utc::now().date_naive();
        let task = crate::domain::Task::new("Test task").with_due_date(today);
        let task_id = task.id.clone();
        model.tasks.insert(task_id.clone(), task);

        model.current_view = ViewId::Calendar;
        model.calendar_state.selected_day = Some(today.day());
        model.calendar_state.year = today.year();
        model.calendar_state.month = today.month();
        model.calendar_state.focus_task_list = true;
        model.refresh_visible_tasks();
        model.selected_index = 0;

        // Task should be Todo initially
        assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Todo);

        // Toggle complete
        update(&mut model, Message::Task(TaskMessage::ToggleComplete));

        // Task should now be Done
        assert_eq!(model.tasks.get(&task_id).unwrap().status, TaskStatus::Done);
    }

    // Import tests
    #[test]
    fn test_start_import_csv_sets_input_mode() {
        let mut model = Model::new();

        update(&mut model, Message::System(SystemMessage::StartImportCsv));

        assert_eq!(model.input_mode, InputMode::Editing);
        assert!(matches!(
            model.input_target,
            InputTarget::ImportFilePath(crate::storage::ImportFormat::Csv)
        ));
        assert!(model.input_buffer.is_empty());
    }

    #[test]
    fn test_start_import_ics_sets_input_mode() {
        let mut model = Model::new();

        update(&mut model, Message::System(SystemMessage::StartImportIcs));

        assert_eq!(model.input_mode, InputMode::Editing);
        assert!(matches!(
            model.input_target,
            InputTarget::ImportFilePath(crate::storage::ImportFormat::Ics)
        ));
    }

    #[test]
    fn test_cancel_import_resets_state() {
        let mut model = Model::new();

        // Set up pending import state
        model.show_import_preview = true;
        model.pending_import = Some(crate::storage::ImportResult {
            imported: vec![],
            skipped: vec![],
            errors: vec![],
        });

        update(&mut model, Message::System(SystemMessage::CancelImport));

        assert!(!model.show_import_preview);
        assert!(model.pending_import.is_none());
        assert!(model.status_message.is_some());
        assert!(model.status_message.as_ref().unwrap().contains("cancelled"));
    }

    #[test]
    fn test_confirm_import_adds_tasks() {
        let mut model = Model::new();

        // Create a task to import
        let task = Task::new("Imported Task");

        model.show_import_preview = true;
        model.pending_import = Some(crate::storage::ImportResult {
            imported: vec![task.clone()],
            skipped: vec![],
            errors: vec![],
        });

        update(&mut model, Message::System(SystemMessage::ConfirmImport));

        assert!(!model.show_import_preview);
        assert!(model.pending_import.is_none());
        assert_eq!(model.tasks.len(), 1);
        assert!(model.tasks.values().any(|t| t.title == "Imported Task"));
        assert!(model.status_message.is_some());
        assert!(model
            .status_message
            .as_ref()
            .unwrap()
            .contains("Imported 1"));
    }

    #[test]
    fn test_confirm_import_multiple_tasks() {
        let mut model = Model::new();

        // Create multiple tasks to import
        let task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");
        let task3 = Task::new("Task 3");

        model.show_import_preview = true;
        model.pending_import = Some(crate::storage::ImportResult {
            imported: vec![task1, task2, task3],
            skipped: vec![],
            errors: vec![],
        });

        update(&mut model, Message::System(SystemMessage::ConfirmImport));

        assert_eq!(model.tasks.len(), 3);
        assert!(model
            .status_message
            .as_ref()
            .unwrap()
            .contains("Imported 3"));
    }

    #[test]
    fn test_import_empty_path_shows_error() {
        use crate::ui::InputTarget;

        let mut model = Model::new();

        // Set up for file path input
        model.input_mode = InputMode::Editing;
        model.input_target = InputTarget::ImportFilePath(crate::storage::ImportFormat::Csv);
        model.input_buffer = "   ".to_string(); // Whitespace only

        // Submit the input
        update(&mut model, Message::Ui(UiMessage::SubmitInput));

        // Should show error, not crash
        assert!(model.status_message.is_some());
        assert!(model
            .status_message
            .as_ref()
            .unwrap()
            .contains("No file path"));
    }

    #[test]
    fn test_reports_panel_navigation() {
        use crate::ui::ReportPanel;

        let mut model = Model::new().with_sample_data();
        model.current_view = ViewId::Reports;
        assert_eq!(model.report_panel, ReportPanel::Overview);

        // Navigate to next panel
        update(
            &mut model,
            Message::Navigation(NavigationMessage::ReportsNextPanel),
        );
        assert_eq!(model.report_panel, ReportPanel::Velocity);

        // Navigate to next panel again
        update(
            &mut model,
            Message::Navigation(NavigationMessage::ReportsNextPanel),
        );
        assert_eq!(model.report_panel, ReportPanel::Tags);

        // Navigate back
        update(
            &mut model,
            Message::Navigation(NavigationMessage::ReportsPrevPanel),
        );
        assert_eq!(model.report_panel, ReportPanel::Velocity);
    }

    #[test]
    fn test_reports_navigation_only_works_in_reports_view() {
        use crate::ui::ReportPanel;

        let mut model = Model::new().with_sample_data();
        model.current_view = ViewId::TaskList; // Not in reports view
        assert_eq!(model.report_panel, ReportPanel::Overview);

        // Try to navigate - should have no effect
        update(
            &mut model,
            Message::Navigation(NavigationMessage::ReportsNextPanel),
        );
        assert_eq!(model.report_panel, ReportPanel::Overview); // Unchanged
    }

    #[test]
    fn test_sidebar_select_reports_view() {
        let mut model = Model::new().with_sample_data();
        model.focus_pane = FocusPane::Sidebar;
        model.sidebar_selected = 7; // Reports view index

        update(
            &mut model,
            Message::Navigation(NavigationMessage::SelectSidebarItem),
        );

        assert_eq!(model.current_view, ViewId::Reports);
        assert_eq!(model.focus_pane, FocusPane::TaskList);
    }

    #[test]
    fn test_completing_parent_cascades_to_descendants() {
        use crate::domain::{Task, TaskStatus};

        let mut model = Model::new();

        // Create a 3-level hierarchy: root -> child -> grandchild
        let root = Task::new("Root Task");
        let mut child = Task::new("Child Task");
        child.parent_task_id = Some(root.id.clone());
        let mut grandchild = Task::new("Grandchild Task");
        grandchild.parent_task_id = Some(child.id.clone());

        let root_id = root.id.clone();
        let child_id = child.id.clone();
        let grandchild_id = grandchild.id.clone();

        model.tasks.insert(root.id.clone(), root);
        model.tasks.insert(child.id.clone(), child);
        model.tasks.insert(grandchild.id.clone(), grandchild);
        model.refresh_visible_tasks();

        // Select the root task
        model.selected_index = model
            .visible_tasks
            .iter()
            .position(|id| id == &root_id)
            .unwrap();

        // All tasks should be Todo initially
        assert_eq!(model.tasks.get(&root_id).unwrap().status, TaskStatus::Todo);
        assert_eq!(model.tasks.get(&child_id).unwrap().status, TaskStatus::Todo);
        assert_eq!(
            model.tasks.get(&grandchild_id).unwrap().status,
            TaskStatus::Todo
        );

        // Complete the root task
        update(&mut model, Message::Task(TaskMessage::ToggleComplete));

        // All tasks should now be Done
        assert_eq!(model.tasks.get(&root_id).unwrap().status, TaskStatus::Done);
        assert_eq!(model.tasks.get(&child_id).unwrap().status, TaskStatus::Done);
        assert_eq!(
            model.tasks.get(&grandchild_id).unwrap().status,
            TaskStatus::Done
        );
    }

    #[test]
    fn test_uncompleting_parent_does_not_affect_descendants() {
        use crate::domain::{Task, TaskStatus};

        let mut model = Model::new();
        model.show_completed = true; // Show completed tasks so we can select them

        // Create a hierarchy with all tasks completed
        let mut root = Task::new("Root Task");
        root.status = TaskStatus::Done;
        let mut child = Task::new("Child Task");
        child.parent_task_id = Some(root.id.clone());
        child.status = TaskStatus::Done;

        let root_id = root.id.clone();
        let child_id = child.id.clone();

        model.tasks.insert(root.id.clone(), root);
        model.tasks.insert(child.id.clone(), child);
        model.refresh_visible_tasks();

        // Select the root task
        model.selected_index = model
            .visible_tasks
            .iter()
            .position(|id| id == &root_id)
            .unwrap();

        // Both should be Done
        assert_eq!(model.tasks.get(&root_id).unwrap().status, TaskStatus::Done);
        assert_eq!(model.tasks.get(&child_id).unwrap().status, TaskStatus::Done);

        // Uncomplete the root task
        update(&mut model, Message::Task(TaskMessage::ToggleComplete));

        // Root should be Todo, but child stays Done (intentional design)
        assert_eq!(model.tasks.get(&root_id).unwrap().status, TaskStatus::Todo);
        assert_eq!(model.tasks.get(&child_id).unwrap().status, TaskStatus::Done);
    }

    #[test]
    fn test_cascade_completion_undo() {
        use crate::domain::{Task, TaskStatus};

        let mut model = Model::new();

        // Create a hierarchy: root -> child
        let root = Task::new("Root Task");
        let mut child = Task::new("Child Task");
        child.parent_task_id = Some(root.id.clone());

        let root_id = root.id.clone();
        let child_id = child.id.clone();

        model.tasks.insert(root.id.clone(), root);
        model.tasks.insert(child.id.clone(), child);
        model.refresh_visible_tasks();

        // Select the root task
        model.selected_index = model
            .visible_tasks
            .iter()
            .position(|id| id == &root_id)
            .unwrap();

        // Complete the root (cascades to child)
        update(&mut model, Message::Task(TaskMessage::ToggleComplete));

        assert_eq!(model.tasks.get(&root_id).unwrap().status, TaskStatus::Done);
        assert_eq!(model.tasks.get(&child_id).unwrap().status, TaskStatus::Done);

        // Undo should restore child first (last pushed to undo stack)
        update(&mut model, Message::System(SystemMessage::Undo));
        assert_eq!(model.tasks.get(&child_id).unwrap().status, TaskStatus::Todo);
        assert_eq!(model.tasks.get(&root_id).unwrap().status, TaskStatus::Done);

        // Undo again to restore root
        update(&mut model, Message::System(SystemMessage::Undo));
        assert_eq!(model.tasks.get(&root_id).unwrap().status, TaskStatus::Todo);
    }

    #[test]
    fn test_delete_blocked_for_task_with_subtasks() {
        use crate::domain::Task;

        let mut model = Model::new();

        // Create a parent with a child
        let parent = Task::new("Parent Task");
        let mut child = Task::new("Child Task");
        child.parent_task_id = Some(parent.id.clone());

        let parent_id = parent.id.clone();

        model.tasks.insert(parent.id.clone(), parent);
        model.tasks.insert(child.id.clone(), child);
        model.refresh_visible_tasks();

        // Select the parent task
        model.selected_index = model
            .visible_tasks
            .iter()
            .position(|id| id == &parent_id)
            .unwrap();

        // Try to delete - should be blocked
        update(&mut model, Message::Ui(UiMessage::ShowDeleteConfirm));

        // Confirm dialog should NOT be shown
        assert!(!model.show_confirm_delete);

        // Error message should be set
        assert!(model.status_message.is_some());
        assert!(model
            .status_message
            .as_ref()
            .unwrap()
            .contains("has subtasks"));
    }

    #[test]
    fn test_delete_allowed_for_task_without_subtasks() {
        use crate::domain::Task;

        let mut model = Model::new();

        // Create a task without children
        let task = Task::new("Standalone Task");
        let task_id = task.id.clone();

        model.tasks.insert(task.id.clone(), task);
        model.refresh_visible_tasks();

        // Select the task
        model.selected_index = model
            .visible_tasks
            .iter()
            .position(|id| id == &task_id)
            .unwrap();

        // Try to delete - should show confirm dialog
        update(&mut model, Message::Ui(UiMessage::ShowDeleteConfirm));

        // Confirm dialog should be shown
        assert!(model.show_confirm_delete);
    }

    #[test]
    fn test_delete_subtask_allowed() {
        use crate::domain::Task;

        let mut model = Model::new();

        // Create parent -> child hierarchy
        let parent = Task::new("Parent Task");
        let mut child = Task::new("Child Task");
        child.parent_task_id = Some(parent.id.clone());

        let child_id = child.id.clone();

        model.tasks.insert(parent.id.clone(), parent);
        model.tasks.insert(child.id.clone(), child);
        model.refresh_visible_tasks();

        // Select the child task (leaf node)
        model.selected_index = model
            .visible_tasks
            .iter()
            .position(|id| id == &child_id)
            .unwrap();

        // Try to delete child - should be allowed (it has no subtasks)
        update(&mut model, Message::Ui(UiMessage::ShowDeleteConfirm));

        // Confirm dialog should be shown
        assert!(model.show_confirm_delete);
    }
}
