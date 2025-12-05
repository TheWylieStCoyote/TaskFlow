use crate::ui::{InputMode, InputTarget};

use super::{
    FocusPane, Message, Model, NavigationMessage, RunningState, SystemMessage, TaskMessage,
    TimeMessage, UiMessage, UndoAction, ViewId,
};

/// Main update function - heart of TEA pattern
pub fn update(model: &mut Model, message: Message) {
    match message {
        Message::Navigation(msg) => handle_navigation(model, msg),
        Message::Task(msg) => handle_task(model, msg),
        Message::Time(msg) => handle_time(model, msg),
        Message::Ui(msg) => handle_ui(model, msg),
        Message::System(msg) => handle_system(model, msg),
        Message::None => {}
    }
}

fn handle_navigation(model: &mut Model, msg: NavigationMessage) {
    match msg {
        NavigationMessage::Up => match model.focus_pane {
            FocusPane::TaskList => {
                if model.current_view == ViewId::Calendar {
                    // In calendar view, up moves to previous week (or wraps)
                    handle_calendar_up(model);
                } else if model.selected_index > 0 {
                    model.selected_index -= 1;
                }
            }
            FocusPane::Sidebar => {
                if model.sidebar_selected > 0 {
                    model.sidebar_selected -= 1;
                    // Skip separator (index 5)
                    if model.sidebar_selected == 5 {
                        model.sidebar_selected = 4;
                    }
                }
            }
        },
        NavigationMessage::Down => match model.focus_pane {
            FocusPane::TaskList => {
                if model.current_view == ViewId::Calendar {
                    // In calendar view, down moves to next week (or wraps)
                    handle_calendar_down(model);
                } else if model.selected_index < model.visible_tasks.len().saturating_sub(1) {
                    model.selected_index += 1;
                }
            }
            FocusPane::Sidebar => {
                let max_index = model.sidebar_item_count().saturating_sub(1);
                if model.sidebar_selected < max_index {
                    model.sidebar_selected += 1;
                    // Skip separator (index 5)
                    if model.sidebar_selected == 5 {
                        model.sidebar_selected = 6;
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
            model.selected_index = 0;
            model.refresh_visible_tasks();
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
            model.selected_index = 0;
            model.refresh_visible_tasks();
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
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
    }
}

/// Handle calendar down navigation (move to next week)
fn handle_calendar_down(model: &mut Model) {
    if let Some(day) = model.calendar_state.selected_day {
        let days = days_in_month(model.calendar_state.year, model.calendar_state.month);
        if day + 7 <= days {
            model.calendar_state.selected_day = Some(day + 7);
            model.selected_index = 0;
            model.refresh_visible_tasks();
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
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
    }
}

fn handle_sidebar_selection(model: &mut Model) {
    let selected = model.sidebar_selected;

    // Sidebar layout:
    // 0: All Tasks (TaskList view)
    // 1: Today
    // 2: Upcoming
    // 3: Overdue
    // 4: Calendar
    // 5: Separator (skip)
    // 6: "Projects" header (skip or go to Projects view)
    // 7+: Individual projects

    match selected {
        0 => {
            model.current_view = ViewId::TaskList;
            model.selected_project = None;
            model.focus_pane = FocusPane::TaskList;
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        1 => {
            model.current_view = ViewId::Today;
            model.selected_project = None;
            model.focus_pane = FocusPane::TaskList;
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        2 => {
            model.current_view = ViewId::Upcoming;
            model.selected_project = None;
            model.focus_pane = FocusPane::TaskList;
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        3 => {
            model.current_view = ViewId::Overdue;
            model.selected_project = None;
            model.focus_pane = FocusPane::TaskList;
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        4 => {
            model.current_view = ViewId::Calendar;
            model.selected_project = None;
            model.focus_pane = FocusPane::TaskList;
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        5 => {} // Separator, do nothing
        6 => {
            // Projects header - go to Projects view showing all project tasks
            model.current_view = ViewId::Projects;
            model.selected_project = None;
            model.focus_pane = FocusPane::TaskList;
            model.selected_index = 0;
            model.refresh_visible_tasks();
        }
        n if n >= 7 => {
            // Select a specific project
            let project_index = n - 7;
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

fn handle_task(model: &mut Model, msg: TaskMessage) {
    match msg {
        TaskMessage::ToggleComplete => {
            // Get the task id first to avoid borrow issues
            let task_id = model.visible_tasks.get(model.selected_index).cloned();

            if let Some(id) = task_id {
                // Check if completing a recurring task
                let next_task = if let Some(task) = model.tasks.get(&id) {
                    if task.status != crate::domain::TaskStatus::Done && task.recurrence.is_some() {
                        // Create next occurrence
                        Some(create_next_recurring_task(task))
                    } else {
                        None
                    }
                } else {
                    None
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

                // Add the next recurring task if one was created
                if let Some(new_task) = next_task {
                    model.sync_task(&new_task);
                    model
                        .undo_stack
                        .push(UndoAction::TaskCreated(Box::new(new_task.clone())));
                    model.tasks.insert(new_task.id.clone(), new_task);
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
                        let task =
                            crate::domain::Task::new(input).with_priority(model.default_priority);
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
                        let task = crate::domain::Task::new(input)
                            .with_priority(model.default_priority)
                            .with_parent(parent_id.clone());
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
                    use chrono::NaiveDate;
                    if let Some(task) = model.tasks.get_mut(task_id) {
                        let before = task.clone();
                        // Empty input clears the due date
                        if input.is_empty() {
                            task.due_date = None;
                        } else if let Ok(date) = NaiveDate::parse_from_str(&input, "%Y-%m-%d") {
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
                                task.project_id = target_project.clone();
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
                        '0' | 'n' | 'N' => None,
                        _ => None, // Invalid input clears recurrence
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
            if model.selected_task().is_some() {
                model.show_confirm_delete = true;
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
                                buffer.push_str(&format!("{}{}: {}, ", marker, i + 1, t.title));
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
                    model.input_buffer = format!("Current: {}", current);
                    model.cursor_position = model.input_buffer.len();
                }
            }
        }
        UiMessage::CalendarPrevDay => {
            if model.current_view == ViewId::Calendar {
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
                        let days =
                            days_in_month(model.calendar_state.year, model.calendar_state.month);
                        model.calendar_state.selected_day = Some(days);
                    }
                    model.selected_index = 0;
                    model.refresh_visible_tasks();
                }
            }
        }
        UiMessage::CalendarNextDay => {
            if model.current_view == ViewId::Calendar {
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
        }
    }
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
                    let days_until = (day.num_days_from_monday() as i64
                        - current_weekday.num_days_from_monday() as i64
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
                // Get last day of the target month
                NaiveDate::from_ymd_opt(
                    if month == 12 { year + 1 } else { year },
                    if month == 12 { 1 } else { month + 1 },
                    1,
                )
                .unwrap()
                    - Duration::days(1)
            })
        }
        Some(Recurrence::Yearly { month, day }) => {
            let next_year = base_date.year() + 1;
            NaiveDate::from_ymd_opt(next_year, *month, *day)
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(next_year, *month, 28).unwrap())
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
                }
                model.refresh_visible_tasks();
            }
        }
        SystemMessage::Resize { width, height } => {
            model.terminal_size = (width, height);
        }
        SystemMessage::Tick => {
            // Handle periodic updates (e.g., timer display)
        }
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
        assert!(model.tasks.get(&task_id).is_none());
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
        model.sidebar_selected = 4; // Calendar (before separator at 5)

        // Move down should skip separator (5) and go to Projects header (6)
        update(&mut model, Message::Navigation(NavigationMessage::Down));
        assert_eq!(model.sidebar_selected, 6);

        // Move up should skip separator and go back to Calendar (4)
        update(&mut model, Message::Navigation(NavigationMessage::Up));
        assert_eq!(model.sidebar_selected, 4);
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
        model.sidebar_selected = 7; // First project (index 7 = after header items including Calendar)

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
        assert!(model.tasks.get(&task_id).is_none());

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
        assert!(model.tasks.get(&task_id).is_none());
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
}
