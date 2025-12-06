//! API functions exposed to Rhai scripts.
//!
//! These functions are registered with the Rhai engine and can be called
//! from user scripts. They don't directly modify state - instead they
//! push actions to a queue that the engine returns.

use rhai::{Engine, Map, Scope};
use std::sync::{Arc, Mutex};

use super::actions::ScriptAction;
use crate::domain::{Priority, Task, TaskId, TaskStatus};

/// Shared action queue for collecting script results (thread-safe).
pub type ActionQueue = Arc<Mutex<Vec<ScriptAction>>>;

/// Creates a new action queue.
#[must_use]
pub fn new_action_queue() -> ActionQueue {
    Arc::new(Mutex::new(Vec::new()))
}

/// Registers all API functions with the Rhai engine.
pub fn register_api(engine: &mut Engine, actions: ActionQueue) {
    // Clone queue for each closure
    let actions_create = actions.clone();
    let actions_create_opts = actions.clone();
    let actions_complete = actions.clone();
    let actions_set_status = actions.clone();
    let actions_set_priority = actions.clone();
    let actions_add_tag = actions.clone();
    let actions_remove_tag = actions.clone();
    let actions_start = actions.clone();
    let actions_stop = actions.clone();
    let actions_log = actions.clone();
    let actions_notify = actions;

    // Task creation (simple)
    engine.register_fn("create_task", move |title: &str| {
        if let Ok(mut queue) = actions_create.lock() {
            queue.push(ScriptAction::CreateTask {
                title: title.to_string(),
                priority: None,
                due_in_days: None,
                tags: Vec::new(),
                project_name: None,
            });
        }
    });

    // Task creation with options
    engine.register_fn(
        "create_task_with_options",
        move |title: &str, options: Map| {
            let priority = options
                .get("priority")
                .and_then(|v| v.clone().into_string().ok())
                .and_then(|s| parse_priority(&s));

            let due_in_days = options
                .get("due_in_days")
                .and_then(|v| v.clone().as_int().ok())
                .map(|i| i as i32);

            let tags = options
                .get("tags")
                .and_then(|v| v.clone().into_typed_array::<String>().ok())
                .unwrap_or_default();

            let project_name = options
                .get("project")
                .and_then(|v| v.clone().into_string().ok());

            if let Ok(mut queue) = actions_create_opts.lock() {
                queue.push(ScriptAction::CreateTask {
                    title: title.to_string(),
                    priority,
                    due_in_days,
                    tags,
                    project_name,
                });
            }
        },
    );

    // Complete task
    engine.register_fn("complete_task", move |id: &str| {
        if let Ok(uuid) = uuid::Uuid::parse_str(id) {
            if let Ok(mut queue) = actions_complete.lock() {
                queue.push(ScriptAction::CompleteTask {
                    task_id: TaskId(uuid),
                });
            }
        }
    });

    // Set task status
    engine.register_fn("set_status", move |id: &str, status: &str| {
        if let (Ok(uuid), Some(status)) = (uuid::Uuid::parse_str(id), parse_status(status)) {
            if let Ok(mut queue) = actions_set_status.lock() {
                queue.push(ScriptAction::SetTaskStatus {
                    task_id: TaskId(uuid),
                    status,
                });
            }
        }
    });

    // Set task priority
    engine.register_fn("set_priority", move |id: &str, priority: &str| {
        if let (Ok(uuid), Some(priority)) = (uuid::Uuid::parse_str(id), parse_priority(priority)) {
            if let Ok(mut queue) = actions_set_priority.lock() {
                queue.push(ScriptAction::SetTaskPriority {
                    task_id: TaskId(uuid),
                    priority,
                });
            }
        }
    });

    // Add tag
    engine.register_fn("add_tag", move |id: &str, tag: &str| {
        if let Ok(uuid) = uuid::Uuid::parse_str(id) {
            if let Ok(mut queue) = actions_add_tag.lock() {
                queue.push(ScriptAction::AddTag {
                    task_id: TaskId(uuid),
                    tag: tag.to_string(),
                });
            }
        }
    });

    // Remove tag
    engine.register_fn("remove_tag", move |id: &str, tag: &str| {
        if let Ok(uuid) = uuid::Uuid::parse_str(id) {
            if let Ok(mut queue) = actions_remove_tag.lock() {
                queue.push(ScriptAction::RemoveTag {
                    task_id: TaskId(uuid),
                    tag: tag.to_string(),
                });
            }
        }
    });

    // Start time tracking
    engine.register_fn("start_tracking", move |id: &str| {
        if let Ok(uuid) = uuid::Uuid::parse_str(id) {
            if let Ok(mut queue) = actions_start.lock() {
                queue.push(ScriptAction::StartTracking {
                    task_id: TaskId(uuid),
                });
            }
        }
    });

    // Stop time tracking
    engine.register_fn("stop_tracking", move || {
        if let Ok(mut queue) = actions_stop.lock() {
            queue.push(ScriptAction::StopTracking);
        }
    });

    // Log message
    engine.register_fn("log", move |msg: &str| {
        if let Ok(mut queue) = actions_log.lock() {
            queue.push(ScriptAction::Log {
                message: msg.to_string(),
            });
        }
    });

    // Notify user
    engine.register_fn("notify", move |msg: &str| {
        if let Ok(mut queue) = actions_notify.lock() {
            queue.push(ScriptAction::Notify {
                message: msg.to_string(),
            });
        }
    });

    // Register date utilities (pure functions, no side effects)
    engine.register_fn("today", || 0_i64);
    engine.register_fn("tomorrow", || 1_i64);
    engine.register_fn("next_week", || 7_i64);
}

/// A script-friendly wrapper around Task data.
/// This is Clone + Send + Sync safe for Rhai.
#[derive(Debug, Clone)]
pub struct ScriptTask {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub tags: Vec<String>,
    pub is_complete: bool,
    pub is_overdue: bool,
    pub is_due_today: bool,
}

impl From<&Task> for ScriptTask {
    fn from(task: &Task) -> Self {
        Self {
            id: task.id.to_string(),
            title: task.title.clone(),
            status: format!("{:?}", task.status).to_lowercase(),
            priority: format!("{:?}", task.priority).to_lowercase(),
            tags: task.tags.clone(),
            is_complete: task.status.is_complete(),
            is_overdue: task.is_overdue(),
            is_due_today: task.is_due_today(),
        }
    }
}

/// Registers the ScriptTask type and its accessors.
pub fn register_task_type(engine: &mut Engine) {
    // Register ScriptTask as a custom type
    engine.register_type_with_name::<ScriptTask>("Task");

    // Property getters
    engine.register_get("id", |t: &mut ScriptTask| t.id.clone());
    engine.register_get("title", |t: &mut ScriptTask| t.title.clone());
    engine.register_get("status", |t: &mut ScriptTask| t.status.clone());
    engine.register_get("priority", |t: &mut ScriptTask| t.priority.clone());
    engine.register_get("tags", |t: &mut ScriptTask| t.tags.clone());
    engine.register_get("is_complete", |t: &mut ScriptTask| t.is_complete);
    engine.register_get("is_overdue", |t: &mut ScriptTask| t.is_overdue);
    engine.register_get("is_due_today", |t: &mut ScriptTask| t.is_due_today);

    // Tag checking method
    engine.register_fn("has_tag", |t: &mut ScriptTask, tag: &str| {
        t.tags.iter().any(|x| x == tag)
    });
}

/// Creates a scope with the given task as context.
#[must_use]
pub fn create_task_scope(task: &Task) -> Scope<'static> {
    let mut scope = Scope::new();
    scope.push("task", ScriptTask::from(task));
    scope
}

/// Creates a scope for status change events.
#[must_use]
pub fn create_status_change_scope(
    task: &Task,
    old_status: TaskStatus,
    new_status: TaskStatus,
) -> Scope<'static> {
    let mut scope = Scope::new();
    scope.push("task", ScriptTask::from(task));
    scope.push("old_status", format!("{old_status:?}").to_lowercase());
    scope.push("new_status", format!("{new_status:?}").to_lowercase());
    scope
}

/// Creates a scope for priority change events.
#[must_use]
pub fn create_priority_change_scope(
    task: &Task,
    old_priority: Priority,
    new_priority: Priority,
) -> Scope<'static> {
    let mut scope = Scope::new();
    scope.push("task", ScriptTask::from(task));
    scope.push("old_priority", format!("{old_priority:?}").to_lowercase());
    scope.push("new_priority", format!("{new_priority:?}").to_lowercase());
    scope
}

/// Creates a scope for time tracking events.
#[must_use]
pub fn create_time_tracking_scope(task: &Task, duration_mins: Option<u32>) -> Scope<'static> {
    let mut scope = Scope::new();
    scope.push("task", ScriptTask::from(task));
    if let Some(mins) = duration_mins {
        scope.push("duration_minutes", mins as i64);
    }
    scope
}

fn parse_priority(s: &str) -> Option<Priority> {
    match s.to_lowercase().as_str() {
        "urgent" => Some(Priority::Urgent),
        "high" => Some(Priority::High),
        "medium" => Some(Priority::Medium),
        "low" => Some(Priority::Low),
        "none" => Some(Priority::None),
        _ => None,
    }
}

fn parse_status(s: &str) -> Option<TaskStatus> {
    match s.to_lowercase().as_str() {
        "todo" => Some(TaskStatus::Todo),
        "in_progress" | "inprogress" => Some(TaskStatus::InProgress),
        "blocked" => Some(TaskStatus::Blocked),
        "done" => Some(TaskStatus::Done),
        "cancelled" => Some(TaskStatus::Cancelled),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_priority() {
        assert_eq!(parse_priority("high"), Some(Priority::High));
        assert_eq!(parse_priority("HIGH"), Some(Priority::High));
        assert_eq!(parse_priority("invalid"), None);
    }

    #[test]
    fn test_parse_status() {
        assert_eq!(parse_status("todo"), Some(TaskStatus::Todo));
        assert_eq!(parse_status("in_progress"), Some(TaskStatus::InProgress));
        assert_eq!(parse_status("invalid"), None);
    }

    #[test]
    fn test_create_task_scope() {
        let task = Task::new("Test task");
        let scope = create_task_scope(&task);
        assert!(scope.contains("task"));
    }

    #[test]
    fn test_script_task_from_task() {
        let task = Task::new("Test task")
            .with_priority(Priority::High)
            .with_tags(vec!["important".to_string()]);

        let script_task = ScriptTask::from(&task);

        assert_eq!(script_task.title, "Test task");
        assert_eq!(script_task.priority, "high");
        assert!(script_task.tags.contains(&"important".to_string()));
    }
}
