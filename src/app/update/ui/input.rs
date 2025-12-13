//! Input submission handlers

use crate::app::{parse_date, parse_quick_add, Model, UndoAction};
use crate::domain::duplicate_detector::{find_similar_task, DEFAULT_SIMILARITY_THRESHOLD};
use crate::domain::TaskId;
use crate::ui::{InputMode, InputTarget};

use crate::app::update::system::handle_execute_import;

/// Start input editing mode with the given target.
///
/// Sets the input mode to Editing, assigns the target, and optionally
/// pre-fills the buffer. Cursor is placed at the end of the buffer.
pub fn start_input(model: &mut Model, target: InputTarget, prefill: Option<String>) {
    model.input.mode = InputMode::Editing;
    model.input.target = target;
    if let Some(text) = prefill {
        model.input.buffer = text;
        model.input.cursor = model.input.buffer.len();
    } else {
        model.input.buffer.clear();
        model.input.cursor = 0;
    }
}

/// Enter focus mode for a specific task, finding its position in visible_tasks.
///
/// Returns true if the task was found and focus mode was entered.
pub fn enter_focus_for_task(model: &mut Model, task_id: TaskId) -> bool {
    if let Some(pos) = model.visible_tasks.iter().position(|id| *id == task_id) {
        model.selected_index = pos;
        model.focus_mode = true;
        true
    } else {
        false
    }
}

/// Handle input submission
#[allow(clippy::too_many_lines)]
pub fn handle_submit_input(model: &mut Model) {
    let input = model.input.buffer.trim().to_string();
    match &model.input.target {
        InputTarget::Task => {
            if input.is_empty() {
                model.alerts.status_message = Some("Task title cannot be empty".to_string());
            } else {
                let task = create_task_from_quick_add(&input, model, None);
                let task_id = task.id;

                // Check for similar existing tasks before inserting
                if let Some((similar_task, similarity)) = find_similar_task(
                    &task.title,
                    task.project_id,
                    &model.tasks,
                    DEFAULT_SIMILARITY_THRESHOLD,
                    None,
                ) {
                    model.alerts.status_message = Some(format!(
                        "Warning: Similar task ({:.0}%): \"{}\"",
                        similarity * 100.0,
                        similar_task.title
                    ));
                }

                // Insert first (moves task), then sync by id
                model
                    .undo_stack
                    .push(UndoAction::TaskCreated(Box::new(task.clone())));
                model.tasks.insert(task_id, task);
                model.sync_task_by_id(&task_id);
                model.refresh_visible_tasks();
            }
        }
        InputTarget::QuickCapture => {
            if input.is_empty() {
                model.alerts.status_message = Some("Task title cannot be empty".to_string());
            } else {
                let task = create_task_from_quick_add(&input, model, None);
                let task_id = task.id;

                // Check for similar existing tasks before inserting
                let duplicate_warning = find_similar_task(
                    &task.title,
                    task.project_id,
                    &model.tasks,
                    DEFAULT_SIMILARITY_THRESHOLD,
                    None,
                )
                .map(|(similar_task, similarity)| {
                    format!(
                        "Warning: Similar task ({:.0}%): \"{}\"",
                        similarity * 100.0,
                        similar_task.title
                    )
                });

                // Insert first (moves task), then sync by id
                model
                    .undo_stack
                    .push(UndoAction::TaskCreated(Box::new(task.clone())));
                model.tasks.insert(task_id, task);
                model.sync_task_by_id(&task_id);
                model.refresh_visible_tasks();
                // Show confirmation and stay ready for another capture
                // Get title from HashMap to avoid extra clone
                model.alerts.status_message = duplicate_warning.or_else(|| {
                    model
                        .tasks
                        .get(&task_id)
                        .map(|t| format!("Task created: {}", t.title))
                });
                model.input.buffer.clear();
                model.input.cursor = 0;
                // Don't reset input_mode - stay in QuickCapture mode
                return;
            }
        }
        InputTarget::Subtask(parent_id) => {
            if !input.is_empty() {
                let task = create_task_from_quick_add(&input, model, Some(*parent_id));
                let task_id = task.id;
                // Insert first (moves task), then sync by id
                model
                    .undo_stack
                    .push(UndoAction::TaskCreated(Box::new(task.clone())));
                model.tasks.insert(task_id, task);
                model.sync_task_by_id(&task_id);
                model.refresh_visible_tasks();
            }
        }
        InputTarget::EditTask(task_id) => {
            let task_id = *task_id;
            if !input.is_empty() {
                model.modify_task_with_undo(&task_id, |task| {
                    task.title.clone_from(&input);
                });
                model.refresh_visible_tasks();
            }
        }
        InputTarget::EditDueDate(task_id) => {
            let task_id = *task_id;
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
            let task_id = *task_id;
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
        InputTarget::EditScheduledTime(task_id) => {
            let task_id = *task_id;
            // Parse time range - empty clears, invalid shows error
            // Time parsing will be implemented in Task 4, full handler in Task 5
            if input.is_empty() {
                model.modify_task_with_undo(&task_id, |task| {
                    task.scheduled_start_time = None;
                    task.scheduled_end_time = None;
                });
                model.alerts.status_message = Some("Time block cleared".to_string());
            } else {
                // TODO: Implement time parsing in Task 4/5
                model.alerts.status_message = Some("Time parsing not yet implemented".to_string());
            }
            model.refresh_visible_tasks();
        }
        InputTarget::EditTags(task_id) => {
            let task_id = *task_id;
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
            let task_id = *task_id;
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
        InputTarget::EditEstimate(task_id) => {
            let task_id = *task_id;
            // Parse duration - empty clears, invalid keeps old
            let new_estimate = if input.is_empty() {
                Some(None) // Explicitly clear
            } else {
                super::parse_duration_input(&input).map(Some)
            };
            if let Some(estimate) = new_estimate {
                model.modify_task_with_undo(&task_id, |task| {
                    task.estimated_minutes = estimate;
                });
                // Show feedback
                if let Some(mins) = estimate {
                    model.alerts.status_message = Some(format!(
                        "Estimate set to {}",
                        super::format_duration_input(mins)
                    ));
                } else {
                    model.alerts.status_message = Some("Estimate cleared".to_string());
                }
            } else {
                model.alerts.status_message =
                    Some("Invalid duration format (try: 30m, 1h, 1h30m)".to_string());
            }
            model.refresh_visible_tasks();
        }
        InputTarget::Project => {
            if !input.is_empty() {
                let project = crate::domain::Project::new(input);
                let project_id = project.id;
                // Clone for undo stack, then move into projects map
                model
                    .undo_stack
                    .push(UndoAction::ProjectCreated(Box::new(project.clone())));
                model.projects.insert(project_id, project);
                model.sync_project_by_id(&project_id);
            }
        }
        InputTarget::EditProject(project_id) => {
            let project_id = *project_id;
            // Only rename if input is non-empty and different from current name
            let should_rename = !input.is_empty()
                && model
                    .projects
                    .get(&project_id)
                    .is_some_and(|p| p.name != input);
            if should_rename {
                let new_name = input.clone();
                model.modify_project_with_undo(&project_id, |project| {
                    project.name.clone_from(&new_name);
                });
                model.alerts.status_message = Some(format!("Renamed project to '{new_name}'"));
            }
        }
        InputTarget::Search => {
            if input.is_empty() {
                model.filtering.filter.search_text = None;
            } else {
                model.filtering.filter.search_text = Some(input);
            }
            model.refresh_visible_tasks();
        }
        InputTarget::MoveToProject(task_id) => {
            let task_id = *task_id;
            // Parse the number input to select a project
            if let Ok(choice) = input.parse::<usize>() {
                let project_ids: Vec<_> = model.projects.keys().copied().collect();
                let new_project = if choice == 0 {
                    Some(None) // Remove from project
                } else {
                    project_ids.get(choice - 1).copied().map(Some) // Move to project or None if invalid
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
                model.filtering.filter.tags = None;
            } else {
                // Parse comma-separated tags, trim whitespace, filter empty
                let tags: Vec<String> = input
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if tags.is_empty() {
                    model.filtering.filter.tags = None;
                } else {
                    model.filtering.filter.tags = Some(tags);
                }
            }
            model.refresh_visible_tasks();
        }
        InputTarget::BulkMoveToProject => {
            if let Ok(choice) = input.parse::<usize>() {
                let project_ids: Vec<_> = model.projects.keys().copied().collect();
                let target_project = if choice == 0 {
                    None
                } else {
                    project_ids.get(choice - 1).copied()
                };

                // Move all selected tasks
                let tasks_to_move: Vec<_> = model.multi_select.selected.iter().copied().collect();
                for task_id in tasks_to_move {
                    let proj = target_project;
                    model.modify_task_with_undo(&task_id, |task| {
                        task.project_id = proj;
                    });
                }
                model.multi_select.selected.clear();
                model.multi_select.mode = false;
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
                let tasks_to_update: Vec<_> = model.multi_select.selected.iter().copied().collect();
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
                model.multi_select.selected.clear();
                model.multi_select.mode = false;
                model.refresh_visible_tasks();
            } else {
                model.alerts.status_message = Some(
                    "Invalid status: enter 1-5 (Todo/InProgress/Blocked/Done/Cancelled)"
                        .to_string(),
                );
            }
        }
        InputTarget::EditDependencies(task_id) => {
            let task_id = *task_id;
            // Parse task numbers from input
            let dep_indices: Vec<usize> = input
                .split(|c: char| !c.is_ascii_digit())
                .filter_map(|s| s.parse::<usize>().ok())
                .collect();

            // Convert indices to task IDs (can't depend on self)
            let new_deps: Vec<_> = dep_indices
                .iter()
                .filter_map(|i| model.visible_tasks.get(i.saturating_sub(1)).copied())
                .filter(|id| *id != task_id)
                .collect();

            model.modify_task_with_undo(&task_id, |task| {
                task.dependencies = new_deps;
            });
            model.refresh_visible_tasks();
        }
        InputTarget::EditRecurrence(task_id) => {
            let task_id = *task_id;
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
            let task_id = *task_id;
            // Parse the input - support task number or task title search
            let target_task_id = if let Ok(num) = input.parse::<usize>() {
                // User entered a task number
                model.visible_tasks.get(num.saturating_sub(1)).copied()
            } else {
                // User entered a task title - find matching task
                let input_lower = input.to_lowercase();
                model
                    .tasks
                    .iter()
                    .find(|(id, t)| {
                        **id != task_id && t.title.to_lowercase().contains(&input_lower)
                    })
                    .map(|(id, _)| *id)
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
                    model.filtering.filter.clone(),
                    model.filtering.sort.clone(),
                );
                let filter_id = saved_filter.id.clone();
                model.saved_filters.insert(filter_id.clone(), saved_filter);
                model.active_saved_filter = Some(filter_id);
                model.storage.dirty = true;
                model.alerts.status_message = Some(format!("Saved filter: {input}"));
            }
        }
        InputTarget::SnoozeTask(task_id) => {
            let task_id = *task_id;
            if input.is_empty() {
                // Clear snooze
                if let Some(task) = model.tasks.get_mut(&task_id) {
                    task.clear_snooze();
                }
                model.sync_task_by_id(&task_id);
                model.alerts.status_message = Some("Snooze cleared".to_string());
            } else if let Some(date) = parse_date(&input) {
                // Set snooze date
                if let Some(task) = model.tasks.get_mut(&task_id) {
                    task.snooze_until_date(date);
                }
                model.sync_task_by_id(&task_id);
                model.alerts.status_message =
                    Some(format!("Snoozed until {}", date.format("%Y-%m-%d")));
            } else {
                model.alerts.status_message = Some("Invalid date format".to_string());
            }
            model.refresh_visible_tasks();
        }
        InputTarget::NewHabit => {
            if !input.is_empty() {
                let habit = crate::domain::Habit::new(input.clone());
                let id = habit.id;
                model.sync_habit(&habit);
                model.habits.insert(id, habit);
                model.refresh_visible_habits();
                model.alerts.status_message = Some("Habit created".to_string());
            }
        }
        InputTarget::EditHabit(habit_id) => {
            let habit_id = *habit_id;
            if !input.is_empty() {
                if let Some(habit) = model.habits.get_mut(&habit_id) {
                    habit.name.clone_from(&input);
                    habit.updated_at = chrono::Utc::now();
                }
                model.sync_habit_by_id(&habit_id);
                model.refresh_visible_habits();
                model.alerts.status_message = Some("Habit updated".to_string());
            }
        }
        InputTarget::GoalName => {
            if !input.is_empty() {
                crate::app::update::goal::handle_goal(
                    model,
                    crate::app::GoalMessage::Create(input.clone()),
                );
                model.alerts.status_message = Some("Goal created".to_string());
            }
        }
        InputTarget::EditGoalName(goal_id) => {
            let goal_id = *goal_id;
            if !input.is_empty() {
                crate::app::update::goal::handle_goal(
                    model,
                    crate::app::GoalMessage::UpdateName {
                        id: goal_id,
                        name: input.clone(),
                    },
                );
                model.alerts.status_message = Some("Goal updated".to_string());
            }
        }
        InputTarget::KeyResultName(goal_id) => {
            let goal_id = *goal_id;
            if !input.is_empty() {
                crate::app::update::goal::handle_goal(
                    model,
                    crate::app::GoalMessage::CreateKeyResult {
                        goal_id,
                        name: input.clone(),
                    },
                );
                model.alerts.status_message = Some("Key result created".to_string());
            }
        }
    }
    model.input.mode = InputMode::Normal;
    model.input.target = InputTarget::default();
    model.input.buffer.clear();
    model.input.cursor = 0;
}

/// Create a task from quick add input, applying parsed metadata
#[must_use]
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
            .map(|p| p.id)
        {
            task.project_id = Some(project_id);
        }
    }

    task
}
