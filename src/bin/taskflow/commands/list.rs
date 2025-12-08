//! List tasks command.

use chrono::Utc;

use taskflow::domain::{Priority, Task, TaskStatus};

use crate::cli::{Cli, ListFilters};
use crate::load_model_for_cli;

/// List tasks from the command line
pub fn list_tasks(
    cli: &Cli,
    view: &str,
    show_completed: bool,
    limit: usize,
    filters: &ListFilters,
) -> anyhow::Result<()> {
    // Load model with storage (fail fast on error)
    let model = load_model_for_cli(cli)?;

    let today = Utc::now().date_naive();

    // Helper: check if task has incomplete dependencies (blocked)
    let is_blocked = |task: &Task| -> bool {
        task.dependencies.iter().any(|dep_id| {
            model
                .tasks
                .get(dep_id)
                .is_some_and(|dep| !dep.status.is_complete())
        })
    };

    // Helper: find project ID by name
    let find_project_id = |name: &str| -> Option<taskflow::domain::ProjectId> {
        let name_lower = name.to_lowercase();
        model
            .projects
            .values()
            .find(|p| p.name.to_lowercase().contains(&name_lower))
            .map(|p| p.id)
    };

    let project_id = filters
        .project
        .as_ref()
        .and_then(|name| find_project_id(name));

    // Filter tasks based on all criteria
    let mut tasks: Vec<_> = model
        .tasks
        .values()
        .filter(|t| {
            // Filter by completion status (unless explicitly showing completed)
            if !show_completed && t.status.is_complete() {
                return false;
            }

            // Filter by view
            let view_match = match view.to_lowercase().as_str() {
                "today" => t.due_date.is_some_and(|d| d == today),
                "overdue" => t.due_date.is_some_and(|d| d < today) && !t.status.is_complete(),
                "upcoming" => t
                    .due_date
                    .is_some_and(|d| d > today && d <= today + chrono::Duration::days(7)),
                "blocked" => is_blocked(t),
                "untagged" => t.tags.is_empty(),
                "no-project" | "noproject" => t.project_id.is_none(),
                "scheduled" => t.scheduled_date.is_some(),
                _ => true, // "all" or any other value
            };
            if !view_match {
                return false;
            }

            // Filter by project
            if let Some(ref pid) = project_id {
                if t.project_id.as_ref() != Some(pid) {
                    return false;
                }
            }

            // Filter by tags
            if let Some(ref filter_tags) = filters.tags {
                let task_tags_lower: Vec<String> =
                    t.tags.iter().map(|tag| tag.to_lowercase()).collect();
                let filter_tags_lower: Vec<String> =
                    filter_tags.iter().map(|tag| tag.to_lowercase()).collect();

                let matches = if filters.tags_any {
                    // ANY mode: task has at least one of the filter tags
                    filter_tags_lower
                        .iter()
                        .any(|ft| task_tags_lower.contains(ft))
                } else {
                    // ALL mode: task has all of the filter tags
                    filter_tags_lower
                        .iter()
                        .all(|ft| task_tags_lower.contains(ft))
                };
                if !matches {
                    return false;
                }
            }

            // Filter by priority
            if let Some(ref priorities) = filters.priority {
                if !priorities.is_empty() && !priorities.contains(&t.priority) {
                    return false;
                }
            }

            // Filter by status
            if let Some(ref statuses) = filters.status {
                if !statuses.is_empty() && !statuses.contains(&t.status) {
                    return false;
                }
            }

            // Filter by search query
            if let Some(ref query) = filters.search {
                let query_lower = query.to_lowercase();
                let title_match = t.title.to_lowercase().contains(&query_lower);
                let tag_match = t
                    .tags
                    .iter()
                    .any(|tag| tag.to_lowercase().contains(&query_lower));
                if !title_match && !tag_match {
                    return false;
                }
            }

            true
        })
        .collect();

    // Sort tasks
    let priority_order = |p: &Priority| match p {
        Priority::Urgent => 0,
        Priority::High => 1,
        Priority::Medium => 2,
        Priority::Low => 3,
        Priority::None => 4,
    };

    tasks.sort_by(|a, b| {
        let cmp = match filters.sort.to_lowercase().as_str() {
            "priority" => priority_order(&a.priority).cmp(&priority_order(&b.priority)),
            "title" => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
            "created" => a.created_at.cmp(&b.created_at),
            _ => {
                // Default: due-date, then priority
                match (&a.due_date, &b.due_date) {
                    (Some(da), Some(db)) => da.cmp(db),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => priority_order(&a.priority).cmp(&priority_order(&b.priority)),
                }
            }
        };
        if filters.reverse {
            cmp.reverse()
        } else {
            cmp
        }
    });

    // Limit output
    let tasks: Vec<_> = tasks.into_iter().take(limit).collect();

    if tasks.is_empty() {
        println!("No tasks found.");
        return Ok(());
    }

    // Print header
    let view_name = match view.to_lowercase().as_str() {
        "today" => "Today's Tasks",
        "overdue" => "Overdue Tasks",
        "upcoming" => "Upcoming Tasks",
        "blocked" => "Blocked Tasks",
        "untagged" => "Untagged Tasks",
        "no-project" | "noproject" => "Tasks Without Project",
        "scheduled" => "Scheduled Tasks",
        _ => "All Tasks",
    };
    println!("{} ({} shown)", view_name, tasks.len());
    println!("{}", "-".repeat(60));

    // Print tasks
    for task in tasks {
        let status_icon = match task.status {
            TaskStatus::Done => "✓",
            TaskStatus::Cancelled => "✗",
            TaskStatus::InProgress => "~",
            TaskStatus::Blocked => "!",
            TaskStatus::Todo => "○",
        };

        let priority_icon = match task.priority {
            Priority::Urgent => "‼️",
            Priority::High => "❗",
            Priority::Medium => "•",
            Priority::Low => "·",
            Priority::None => " ",
        };

        let due_str = task
            .due_date
            .map(|d| {
                if d == today {
                    "today".to_string()
                } else if d == today + chrono::Duration::days(1) {
                    "tomorrow".to_string()
                } else if d < today {
                    format!("{} (overdue)", d.format("%m/%d"))
                } else {
                    d.format("%m/%d").to_string()
                }
            })
            .unwrap_or_default();

        let tags_str = if task.tags.is_empty() {
            String::new()
        } else {
            format!(
                " {}",
                task.tags
                    .iter()
                    .map(|t| format!("#{t}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        };

        println!(
            "{} {} {}{}{}",
            status_icon,
            priority_icon,
            task.title,
            if due_str.is_empty() {
                String::new()
            } else {
                format!(" [{}]", due_str)
            },
            tags_str
        );
    }

    Ok(())
}
