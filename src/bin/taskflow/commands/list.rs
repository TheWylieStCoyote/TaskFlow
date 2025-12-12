//! List tasks command.

use std::collections::HashSet;

use chrono::Utc;
use tracing::error;

use taskflow::domain::filter_dsl::{evaluate_with_cache, parse, EvalContext, TaskLowerCache};
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

    // Parse DSL filter if provided
    let dsl_expr = if let Some(ref dsl_str) = filters.dsl_filter {
        match parse(dsl_str) {
            Ok(expr) => Some(expr),
            Err(e) => {
                error!(error = %e, "Filter parse error");
                eprintln!("Filter parse error: {e}");
                return Err(anyhow::anyhow!("Invalid filter expression: {e}"));
            }
        }
    } else {
        None
    };

    // Create evaluation context for DSL filter
    let eval_ctx = EvalContext::new(&model.projects);

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

    // Pre-compute lowercase filter tags ONCE (outside the filter closure)
    let filter_tags_lower: Option<HashSet<String>> = filters
        .tags
        .as_ref()
        .map(|tags| tags.iter().map(|tag| tag.to_lowercase()).collect());

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

            // If DSL filter is provided, use it exclusively for remaining filtering
            if let Some(ref expr) = dsl_expr {
                let cache = TaskLowerCache::new(t);
                return evaluate_with_cache(expr, &cache, &eval_ctx);
            }

            // Filter by project
            if let Some(ref pid) = project_id {
                if t.project_id.as_ref() != Some(pid) {
                    return false;
                }
            }

            // Filter by tags (using pre-computed filter_tags_lower)
            if let Some(ref ftl) = filter_tags_lower {
                // Use HashSet for O(1) contains lookup
                let task_tags_lower: HashSet<String> =
                    t.tags.iter().map(|tag| tag.to_lowercase()).collect();

                let matches = if filters.tags_any {
                    // ANY mode: task has at least one of the filter tags
                    ftl.iter().any(|ft| task_tags_lower.contains(ft))
                } else {
                    // ALL mode: task has all of the filter tags
                    ftl.iter().all(|ft| task_tags_lower.contains(ft))
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

            // Filter by due date range
            if let Some(before) = filters.due_before {
                match t.due_date {
                    Some(due) if due <= before => {}
                    _ => return false,
                }
            }
            if let Some(after) = filters.due_after {
                match t.due_date {
                    Some(due) if due >= after => {}
                    _ => return false,
                }
            }

            // Filter by time estimate
            if let Some(min) = filters.estimate_min {
                match t.estimated_minutes {
                    Some(est) if est >= min => {}
                    _ => return false,
                }
            }
            if let Some(max) = filters.estimate_max {
                match t.estimated_minutes {
                    Some(est) if est <= max => {}
                    _ => return false,
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

    // Pre-compute sort field ONCE (not per-comparison)
    let sort_field = filters.sort.to_lowercase();

    // Use sort_by_cached_key for title to avoid repeated to_lowercase() calls
    if sort_field == "title" {
        tasks.sort_by_cached_key(|t| t.title.to_lowercase());
        if filters.reverse {
            tasks.reverse();
        }
    } else {
        tasks.sort_by(|a, b| {
            let cmp = match sort_field.as_str() {
                "priority" => priority_order(&a.priority).cmp(&priority_order(&b.priority)),
                "created" => a.created_at.cmp(&b.created_at),
                _ => {
                    // Default: due-date, then priority
                    match (&a.due_date, &b.due_date) {
                        (Some(da), Some(db)) => da.cmp(db),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => {
                            priority_order(&a.priority).cmp(&priority_order(&b.priority))
                        }
                    }
                }
            };
            if filters.reverse {
                cmp.reverse()
            } else {
                cmp
            }
        });
    }

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

    // Show active filter if set
    if let Some(ref dsl_str) = filters.dsl_filter {
        println!("Filter: {}", dsl_str);
    }

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
