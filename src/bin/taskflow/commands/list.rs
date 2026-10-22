//! List tasks command.

use std::collections::HashSet;
use std::fmt::Write as _;

use chrono::NaiveDate;
use tracing::error;

use taskflow::app::Model;
use taskflow::domain::filter_dsl::{evaluate_with_cache, parse, EvalContext, TaskLowerCache};
use taskflow::domain::{Priority, Task, TaskStatus};

use crate::cli::{Cli, ListFilters};
use crate::load_model_for_cli;
use taskflow::domain::today;

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

    let today = today();
    let tasks = select_tasks(&model, view, show_completed, limit, filters, today)?;
    print!(
        "{}",
        render_task_list(view, &tasks, filters.dsl_filter.as_deref(), today)
    );

    Ok(())
}

/// Select, sort and limit the tasks matching `view` and `filters`.
///
/// Returns an error when the DSL filter expression cannot be parsed or when
/// `--project` names no project (or more than one).
pub(crate) fn select_tasks<'a>(
    model: &'a Model,
    view: &str,
    show_completed: bool,
    limit: usize,
    filters: &ListFilters,
    today: NaiveDate,
) -> anyhow::Result<Vec<&'a Task>> {
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

    // An unknown project is an error, not "no filter".
    let project_id = match filters.project.as_deref() {
        Some(name) => Some(find_project(model, name)?),
        None => None,
    };

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
    Ok(tasks.into_iter().take(limit).collect())
}

/// How many project names to list in the "no such project" error before
/// falling back to a count.
const MAX_LISTED_PROJECTS: usize = 10;

/// Find the one project `name` refers to: an exact (case-insensitive) name
/// wins, otherwise a unique substring match. Anything else is an error.
fn find_project(model: &Model, name: &str) -> anyhow::Result<taskflow::domain::ProjectId> {
    let wanted = name.trim().to_lowercase();
    if let Some(project) = model
        .projects
        .values()
        .find(|p| p.name.to_lowercase() == wanted)
    {
        return Ok(project.id);
    }
    let mut matches: Vec<_> = model
        .projects
        .values()
        .filter(|p| !wanted.is_empty() && p.name.to_lowercase().contains(&wanted))
        .collect();
    matches.sort_by(|a, b| a.name.cmp(&b.name));
    match matches.as_slice() {
        [] => {
            let mut names: Vec<_> = model.projects.values().map(|p| p.name.as_str()).collect();
            names.sort_unstable();
            if names.is_empty() {
                anyhow::bail!("No project named '{name}' (there are no projects)");
            } else if names.len() <= MAX_LISTED_PROJECTS {
                anyhow::bail!(
                    "No project named '{name}'. Available projects: {}",
                    names.join(", ")
                );
            }
            anyhow::bail!(
                "No project named '{name}' ({} projects available)",
                names.len()
            );
        }
        [only] => Ok(only.id),
        many => {
            let names: Vec<_> = many.iter().map(|p| p.name.as_str()).collect();
            anyhow::bail!(
                "'{name}' matches several projects: {}. Give the full name.",
                names.join(", ")
            )
        }
    }
}

/// Render the task list exactly as `taskflow list` prints it.
pub(crate) fn render_task_list(
    view: &str,
    tasks: &[&Task],
    dsl_filter: Option<&str>,
    today: NaiveDate,
) -> String {
    let mut out = String::new();

    if tasks.is_empty() {
        out.push_str("No tasks found.\n");
        return out;
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
    let _ = writeln!(out, "{} ({} shown)", view_name, tasks.len());

    // Show active filter if set
    if let Some(dsl_str) = dsl_filter {
        let _ = writeln!(out, "Filter: {dsl_str}");
    }

    let _ = writeln!(out, "{}", "-".repeat(60));

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

        let _ = writeln!(
            out,
            "{} {} {}{}{}",
            status_icon,
            priority_icon,
            task.title,
            if due_str.is_empty() {
                String::new()
            } else {
                format!(" [{due_str}]")
            },
            tags_str
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Utc};

    use taskflow::app::Model;
    use taskflow::domain::{Priority, Project, Task, TaskStatus};

    use super::{list_tasks, render_task_list, select_tasks};
    use crate::cli::ListFilters;
    use crate::commands_tests::support::{
        add_project, add_task, data_file, seed_model, temp_dir, yaml_cli,
    };

    const TODAY: NaiveDate = match NaiveDate::from_ymd_opt(2026, 10, 21) {
        Some(d) => d,
        None => panic!("valid date"),
    };

    fn days(n: i64) -> NaiveDate {
        TODAY + chrono::Duration::days(n)
    }

    /// A model whose tasks each exercise one list view or filter.
    ///
    /// Only "Gamma upcoming" is untagged and without a project; only
    /// "Delta someday" has a scheduled date; only "Zeta blocked" waits on an
    /// incomplete dependency.
    fn fixture() -> Model {
        let mut model = Model::new();
        let backend = Project::new("Backend");
        let frontend = Project::new("Frontend");
        let backend_id = backend.id;
        let frontend_id = frontend.id;
        model.projects.insert(backend_id, backend);
        model.projects.insert(frontend_id, frontend);

        let mut created = 1;
        let mut push = |model: &mut Model, mut task: Task| -> taskflow::domain::TaskId {
            task.created_at = Utc
                .with_ymd_and_hms(2026, 1, created, 9, 0, 0)
                .single()
                .expect("valid timestamp");
            created += 1;
            let id = task.id;
            model.tasks.insert(id, task);
            id
        };

        let alpha = push(
            &mut model,
            Task::new("Alpha overdue")
                .with_priority(Priority::High)
                .with_due_date(days(-2))
                .with_tags(vec!["bug".to_string()])
                .with_project(backend_id),
        );
        push(
            &mut model,
            Task::new("Beta today")
                .with_priority(Priority::Urgent)
                .with_due_date(TODAY)
                .with_tags(vec!["bug".to_string(), "urgent".to_string()])
                .with_estimated_minutes(30)
                .with_project(backend_id),
        );
        push(
            &mut model,
            Task::new("Gamma upcoming")
                .with_priority(Priority::Medium)
                .with_due_date(days(3))
                .with_estimated_minutes(120),
        );
        let mut delta = Task::new("Delta someday")
            .with_priority(Priority::Low)
            .with_tags(vec!["docs".to_string()])
            .with_project(backend_id);
        delta.scheduled_date = Some(days(1));
        push(&mut model, delta);
        let epsilon = push(
            &mut model,
            Task::new("Epsilon done")
                .with_status(TaskStatus::Done)
                .with_due_date(TODAY)
                .with_tags(vec!["bug".to_string()])
                .with_project(frontend_id),
        );
        let mut zeta = Task::new("Zeta blocked")
            .with_tags(vec!["chore".to_string()])
            .with_project(frontend_id);
        zeta.dependencies.push(alpha);
        push(&mut model, zeta);
        let mut eta = Task::new("Eta started")
            .with_status(TaskStatus::InProgress)
            .with_tags(vec!["chore".to_string()])
            .with_project(frontend_id);
        eta.dependencies.push(epsilon);
        push(&mut model, eta);
        push(
            &mut model,
            Task::new("Theta cancelled")
                .with_status(TaskStatus::Cancelled)
                .with_tags(vec!["chore".to_string()])
                .with_project(frontend_id),
        );
        push(
            &mut model,
            Task::new("Iota far")
                .with_due_date(days(30))
                .with_tags(vec!["chore".to_string()])
                .with_project(frontend_id),
        );
        model
    }

    fn titles<'a>(tasks: &[&'a Task]) -> Vec<&'a str> {
        tasks.iter().map(|t| t.title.as_str()).collect()
    }

    fn sorted_titles<'a>(tasks: &[&'a Task]) -> Vec<&'a str> {
        let mut t = titles(tasks);
        t.sort_unstable();
        t
    }

    fn select<'a>(
        model: &'a Model,
        view: &str,
        show_completed: bool,
        filters: &ListFilters,
    ) -> Vec<&'a str> {
        let tasks = select_tasks(model, view, show_completed, 100, filters, TODAY)
            .expect("filters are valid");
        sorted_titles(&tasks)
    }

    #[test]
    fn all_view_hides_completed_and_cancelled_unless_requested() {
        let model = fixture();
        let filters = ListFilters::default();
        let open = select(&model, "all", false, &filters);
        assert_eq!(open.len(), 7);
        assert!(!open.contains(&"Epsilon done"));
        assert!(!open.contains(&"Theta cancelled"));

        let everything = select(&model, "all", true, &filters);
        assert_eq!(everything.len(), 9);
    }

    #[test]
    fn unknown_view_behaves_like_all() {
        let model = fixture();
        let filters = ListFilters::default();
        assert_eq!(
            select(&model, "whatever", false, &filters),
            select(&model, "ALL", false, &filters)
        );
    }

    #[test]
    fn date_views_select_by_due_date() {
        let model = fixture();
        let filters = ListFilters::default();
        assert_eq!(select(&model, "today", false, &filters), ["Beta today"]);
        assert_eq!(
            select(&model, "today", true, &filters),
            ["Beta today", "Epsilon done"]
        );
        assert_eq!(
            select(&model, "overdue", false, &filters),
            ["Alpha overdue"]
        );
        assert_eq!(
            select(&model, "upcoming", false, &filters),
            ["Gamma upcoming"],
            "upcoming covers the next seven days only"
        );
    }

    #[test]
    fn structural_views_select_blocked_untagged_projectless_and_scheduled() {
        let model = fixture();
        let filters = ListFilters::default();
        assert_eq!(
            select(&model, "blocked", false, &filters),
            ["Zeta blocked"],
            "a dependency on a completed task does not block"
        );
        assert_eq!(
            select(&model, "untagged", false, &filters),
            ["Gamma upcoming"]
        );
        assert_eq!(
            select(&model, "no-project", false, &filters),
            ["Gamma upcoming"]
        );
        assert_eq!(
            select(&model, "noproject", false, &filters),
            ["Gamma upcoming"]
        );
        assert_eq!(
            select(&model, "scheduled", false, &filters),
            ["Delta someday"]
        );
    }

    #[test]
    fn invalid_dsl_filter_is_an_error() {
        let model = fixture();
        let filters = ListFilters {
            dsl_filter: Some("status:".to_string()),
            ..ListFilters::default()
        };
        let err = select_tasks(&model, "all", false, 10, &filters, TODAY).unwrap_err();
        assert!(
            err.to_string().starts_with("Invalid filter expression: "),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn dsl_filter_replaces_the_simple_filters() {
        let model = fixture();
        let filters = ListFilters {
            dsl_filter: Some("priority:high".to_string()),
            // Would exclude Alpha if it were applied alongside the DSL.
            search: Some("beta".to_string()),
            ..ListFilters::default()
        };
        assert_eq!(select(&model, "all", false, &filters), ["Alpha overdue"]);
    }

    #[test]
    fn project_filter_matches_name_substring_case_insensitively() {
        let model = fixture();
        let filters = ListFilters {
            project: Some("BACK".to_string()),
            ..ListFilters::default()
        };
        assert_eq!(
            select(&model, "all", false, &filters),
            ["Alpha overdue", "Beta today", "Delta someday"]
        );
    }

    #[test]
    fn project_filter_prefers_an_exact_name_over_a_substring_match() {
        let mut model = Model::new();
        let api = Project::new("API");
        let api_v2 = Project::new("API v2");
        let (api_id, api_v2_id) = (api.id, api_v2.id);
        model.projects.insert(api_id, api);
        model.projects.insert(api_v2_id, api_v2);
        let old = Task::new("Old endpoint").with_project(api_id);
        let new = Task::new("New endpoint").with_project(api_v2_id);
        model.tasks.insert(old.id, old);
        model.tasks.insert(new.id, new);

        let filters = ListFilters {
            project: Some("api".to_string()),
            ..ListFilters::default()
        };
        assert_eq!(select(&model, "all", false, &filters), ["Old endpoint"]);

        let filters = ListFilters {
            project: Some("V2".to_string()),
            ..ListFilters::default()
        };
        assert_eq!(select(&model, "all", false, &filters), ["New endpoint"]);
    }

    #[test]
    fn unknown_project_is_an_error_listing_the_available_names() {
        let model = fixture();
        let filters = ListFilters {
            project: Some("Nope".to_string()),
            ..ListFilters::default()
        };
        let err = select_tasks(&model, "all", false, 100, &filters, TODAY)
            .expect_err("unknown project must not silently list everything");
        let msg = err.to_string();
        assert!(msg.contains("No project named 'Nope'"), "{msg}");
        assert!(msg.contains("Backend, Frontend"), "{msg}");

        // With no projects at all the message says so instead of listing.
        let empty = Model::new();
        let err = select_tasks(&empty, "all", false, 100, &filters, TODAY).unwrap_err();
        assert!(err.to_string().contains("there are no projects"), "{err}");

        // Blank names never match by substring.
        let filters = ListFilters {
            project: Some("  ".to_string()),
            ..ListFilters::default()
        };
        assert!(select_tasks(&model, "all", false, 100, &filters, TODAY).is_err());
    }

    #[test]
    fn ambiguous_project_substring_is_an_error() {
        let model = fixture();
        let filters = ListFilters {
            project: Some("end".to_string()),
            ..ListFilters::default()
        };
        let err = select_tasks(&model, "all", false, 100, &filters, TODAY).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("matches several projects"), "{msg}");
        assert!(msg.contains("Backend, Frontend"), "{msg}");
    }

    #[test]
    fn many_projects_are_counted_not_listed_in_the_error() {
        let mut model = Model::new();
        for i in 0..12 {
            let project = Project::new(format!("Project {i:02}"));
            model.projects.insert(project.id, project);
        }
        let filters = ListFilters {
            project: Some("zzz".to_string()),
            ..ListFilters::default()
        };
        let err = select_tasks(&model, "all", false, 100, &filters, TODAY).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("12 projects available"), "{msg}");
        assert!(!msg.contains("Project 00"), "{msg}");
    }

    #[test]
    fn tag_filter_requires_all_tags_unless_any_mode() {
        let model = fixture();
        let all_mode = ListFilters {
            tags: Some(vec!["BUG".to_string(), "urgent".to_string()]),
            ..ListFilters::default()
        };
        assert_eq!(select(&model, "all", false, &all_mode), ["Beta today"]);

        let any_mode = ListFilters {
            tags: Some(vec!["bug".to_string(), "docs".to_string()]),
            tags_any: true,
            ..ListFilters::default()
        };
        assert_eq!(
            select(&model, "all", false, &any_mode),
            ["Alpha overdue", "Beta today", "Delta someday"]
        );
    }

    #[test]
    fn priority_and_status_filters() {
        let model = fixture();
        let by_priority = ListFilters {
            priority: Some(vec![Priority::Low, Priority::Medium]),
            ..ListFilters::default()
        };
        assert_eq!(
            select(&model, "all", false, &by_priority),
            ["Delta someday", "Gamma upcoming"]
        );

        let empty_priority = ListFilters {
            priority: Some(Vec::new()),
            ..ListFilters::default()
        };
        assert_eq!(
            select(&model, "all", false, &empty_priority).len(),
            7,
            "an empty priority list does not filter"
        );

        let by_status = ListFilters {
            status: Some(vec![TaskStatus::InProgress]),
            ..ListFilters::default()
        };
        assert_eq!(select(&model, "all", false, &by_status), ["Eta started"]);
    }

    #[test]
    fn search_matches_title_or_tag() {
        let model = fixture();
        let by_title = ListFilters {
            search: Some("GAMMA".to_string()),
            ..ListFilters::default()
        };
        assert_eq!(select(&model, "all", false, &by_title), ["Gamma upcoming"]);

        let by_tag = ListFilters {
            search: Some("docs".to_string()),
            ..ListFilters::default()
        };
        assert_eq!(select(&model, "all", false, &by_tag), ["Delta someday"]);
    }

    #[test]
    fn due_date_range_filters_exclude_tasks_without_due_date() {
        let model = fixture();
        let before = ListFilters {
            due_before: Some(TODAY),
            ..ListFilters::default()
        };
        assert_eq!(
            select(&model, "all", false, &before),
            ["Alpha overdue", "Beta today"]
        );

        let after = ListFilters {
            due_after: Some(days(3)),
            ..ListFilters::default()
        };
        assert_eq!(
            select(&model, "all", false, &after),
            ["Gamma upcoming", "Iota far"]
        );
    }

    #[test]
    fn estimate_range_filters_exclude_tasks_without_estimate() {
        let model = fixture();
        let min = ListFilters {
            estimate_min: Some(60),
            ..ListFilters::default()
        };
        assert_eq!(select(&model, "all", false, &min), ["Gamma upcoming"]);

        let max = ListFilters {
            estimate_max: Some(60),
            ..ListFilters::default()
        };
        assert_eq!(select(&model, "all", false, &max), ["Beta today"]);
    }

    #[test]
    fn default_sort_is_due_date_then_priority() {
        let model = fixture();
        let filters = ListFilters::default();
        let tasks = select_tasks(&model, "all", false, 100, &filters, TODAY).unwrap();
        assert_eq!(
            &titles(&tasks)[..4],
            ["Alpha overdue", "Beta today", "Gamma upcoming", "Iota far"],
            "due-dated tasks come first in date order"
        );
        // No due date: by priority, Low before None.
        assert_eq!(tasks[4].title, "Delta someday");
        assert!(tasks[5..].iter().all(|t| t.priority == Priority::None));
    }

    #[test]
    fn sort_by_title_priority_created_and_reverse() {
        let model = fixture();

        let by_title = ListFilters {
            sort: "title".to_string(),
            ..ListFilters::default()
        };
        let tasks = select_tasks(&model, "all", false, 100, &by_title, TODAY).unwrap();
        assert_eq!(tasks.first().unwrap().title, "Alpha overdue");
        assert_eq!(tasks.last().unwrap().title, "Zeta blocked");

        let by_title_rev = ListFilters {
            sort: "Title".to_string(),
            reverse: true,
            ..ListFilters::default()
        };
        let tasks = select_tasks(&model, "all", false, 100, &by_title_rev, TODAY).unwrap();
        assert_eq!(tasks.first().unwrap().title, "Zeta blocked");

        let by_priority = ListFilters {
            sort: "priority".to_string(),
            ..ListFilters::default()
        };
        let tasks = select_tasks(&model, "all", false, 100, &by_priority, TODAY).unwrap();
        assert_eq!(tasks[0].title, "Beta today");
        assert_eq!(tasks[1].title, "Alpha overdue");
        assert_eq!(tasks[2].title, "Gamma upcoming");
        assert_eq!(tasks[3].title, "Delta someday");

        let by_priority_rev = ListFilters {
            sort: "priority".to_string(),
            reverse: true,
            ..ListFilters::default()
        };
        let tasks = select_tasks(&model, "all", false, 100, &by_priority_rev, TODAY).unwrap();
        assert_eq!(tasks.last().unwrap().title, "Beta today");

        let by_created = ListFilters {
            sort: "created".to_string(),
            ..ListFilters::default()
        };
        let tasks = select_tasks(&model, "all", false, 100, &by_created, TODAY).unwrap();
        assert_eq!(tasks.first().unwrap().title, "Alpha overdue");
        assert_eq!(tasks.last().unwrap().title, "Iota far");
    }

    #[test]
    fn limit_truncates_after_sorting() {
        let model = fixture();
        let filters = ListFilters {
            sort: "title".to_string(),
            ..ListFilters::default()
        };
        let tasks = select_tasks(&model, "all", false, 2, &filters, TODAY).unwrap();
        assert_eq!(titles(&tasks), ["Alpha overdue", "Beta today"]);
    }

    #[test]
    fn render_empty_list() {
        assert_eq!(
            render_task_list("all", &[], None, TODAY),
            "No tasks found.\n"
        );
    }

    #[test]
    fn render_headers_per_view() {
        let task = Task::new("Solo");
        let cases = [
            ("today", "Today's Tasks"),
            ("overdue", "Overdue Tasks"),
            ("UPCOMING", "Upcoming Tasks"),
            ("blocked", "Blocked Tasks"),
            ("untagged", "Untagged Tasks"),
            ("no-project", "Tasks Without Project"),
            ("noproject", "Tasks Without Project"),
            ("scheduled", "Scheduled Tasks"),
            ("all", "All Tasks"),
            ("anything", "All Tasks"),
        ];
        for (view, header) in cases {
            let out = render_task_list(view, &[&task], None, TODAY);
            let first = out.lines().next().unwrap();
            assert_eq!(first, format!("{header} (1 shown)"), "view {view}");
        }
    }

    #[test]
    fn render_shows_filter_line_and_separator() {
        let task = Task::new("Solo");
        let out = render_task_list("all", &[&task], Some("priority:high"), TODAY);
        let lines: Vec<_> = out.lines().collect();
        assert_eq!(lines[0], "All Tasks (1 shown)");
        assert_eq!(lines[1], "Filter: priority:high");
        assert_eq!(lines[2], "-".repeat(60));
        assert_eq!(lines[3], "○   Solo");
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn render_rows_show_status_priority_due_and_tags() {
        let overdue = Task::new("Late")
            .with_priority(Priority::High)
            .with_due_date(days(-2))
            .with_tags(vec!["bug".to_string(), "p1".to_string()]);
        let today = Task::new("Now")
            .with_priority(Priority::Urgent)
            .with_due_date(TODAY);
        let tomorrow = Task::new("Soon")
            .with_priority(Priority::Medium)
            .with_due_date(days(1))
            .with_status(TaskStatus::InProgress);
        let later = Task::new("Later")
            .with_priority(Priority::Low)
            .with_due_date(days(3))
            .with_status(TaskStatus::Blocked);
        let done = Task::new("Finished").with_status(TaskStatus::Done);
        let cancelled = Task::new("Dropped").with_status(TaskStatus::Cancelled);

        let out = render_task_list(
            "all",
            &[&overdue, &today, &tomorrow, &later, &done, &cancelled],
            None,
            TODAY,
        );
        let rows: Vec<_> = out.lines().skip(2).collect();
        assert_eq!(rows[0], "○ ❗ Late [10/19 (overdue)] #bug #p1");
        assert_eq!(rows[1], "○ ‼️ Now [today]");
        assert_eq!(rows[2], "~ • Soon [tomorrow]");
        assert_eq!(rows[3], "! · Later [10/24]");
        assert_eq!(rows[4], "✓   Finished");
        assert_eq!(rows[5], "✗   Dropped");
    }

    #[test]
    fn list_tasks_loads_from_the_configured_data_file() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        seed_model(&data, |model| {
            let pid = add_project(model, Project::new("Backend"));
            add_task(
                model,
                Task::new("Persisted task")
                    .with_due_date(TODAY)
                    .with_project(pid),
            );
        });
        let cli = yaml_cli(&data);

        let filters = ListFilters {
            project: Some("backend".to_string()),
            ..ListFilters::default()
        };
        list_tasks(&cli, "all", false, 20, &filters).expect("list succeeds");
        list_tasks(&cli, "today", true, 20, &ListFilters::default()).expect("view succeeds");
    }

    #[test]
    fn list_tasks_rejects_a_bad_filter_after_loading() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        seed_model(&data, |model| {
            add_task(model, Task::new("Persisted task"));
        });
        let cli = yaml_cli(&data);
        let filters = ListFilters {
            dsl_filter: Some("((status:todo".to_string()),
            ..ListFilters::default()
        };
        let err = list_tasks(&cli, "all", false, 20, &filters).unwrap_err();
        assert!(err.to_string().contains("Invalid filter expression"));
    }

    #[test]
    fn list_tasks_fails_when_storage_cannot_be_read() {
        let dir = temp_dir();
        // A directory is not a readable YAML file.
        let cli = yaml_cli(dir.path());
        let err = list_tasks(&cli, "all", false, 20, &ListFilters::default()).unwrap_err();
        assert!(
            err.to_string().contains("Could not load data from"),
            "unexpected error: {err}"
        );
    }
}
