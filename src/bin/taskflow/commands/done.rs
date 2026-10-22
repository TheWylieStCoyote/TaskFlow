//! Mark task done command.

use tracing::warn;

use taskflow::domain::TaskStatus;

use crate::cli::Cli;
use crate::load_model_for_cli;

/// Mark a task as done from the command line
pub fn mark_task_done(
    cli: &Cli,
    query_words: &[String],
    project_filter: Option<&str>,
    tags_filter: Option<&[String]>,
) -> anyhow::Result<()> {
    let query = query_words.join(" ").to_lowercase();
    if query.trim().is_empty() {
        warn!("Empty search query provided for done command");
        eprintln!("Error: Search query cannot be empty");
        eprintln!("Usage: taskflow done <search query>");
        std::process::exit(1);
    }

    // Load model with storage (fail fast on error)
    let mut model = load_model_for_cli(cli)?;

    // Find project ID by name for filtering
    let project_id = project_filter.and_then(|name| {
        let name_lower = name.to_lowercase();
        model
            .projects
            .iter()
            .find(|(_, p)| p.name.to_lowercase().contains(&name_lower))
            .map(|(id, _)| *id)
    });

    // Find matching tasks (case-insensitive title search + optional filters)
    let matches: Vec<_> = model
        .tasks
        .values()
        .filter(|t| {
            // Basic filter: not complete and title matches
            if t.status.is_complete() || !t.title.to_lowercase().contains(&query) {
                return false;
            }

            // Project filter: if specified, must match
            if let Some(ref proj_id) = project_id {
                if t.project_id.as_ref() != Some(proj_id) {
                    return false;
                }
            }

            // Tags filter: must have ALL specified tags
            if let Some(filter_tags) = tags_filter {
                let task_tags_lower: Vec<String> =
                    t.tags.iter().map(|tag| tag.to_lowercase()).collect();
                let has_all = filter_tags.iter().all(|ft| {
                    task_tags_lower
                        .iter()
                        .any(|tt| tt.contains(&ft.to_lowercase()))
                });
                if !has_all {
                    return false;
                }
            }

            true
        })
        .collect();

    match matches.len() {
        0 => {
            warn!(query = %query, "No matching tasks found for done command");
            eprintln!("No matching incomplete tasks found for: \"{}\"", query);
            eprintln!();
            eprintln!("Tips:");
            eprintln!("  - Check spelling of your search query");
            eprintln!("  - Use 'taskflow list' to see available tasks");
            if project_filter.is_some() || tags_filter.is_some() {
                eprintln!("  - Try without --project or --tags filters");
            }
            std::process::exit(1);
        }
        1 => {
            let task_id = matches[0].id;
            let task_title = matches[0].title.clone();

            // Mark as done
            if let Some(task) = model.tasks.get_mut(&task_id) {
                task.status = TaskStatus::Done;
                task.completed_at = Some(chrono::Utc::now());
            }
            model.sync_task_by_id(&task_id);
            if let Err(e) = model.save() {
                warn!(error = %e, "Could not save after marking task done");
                eprintln!("Warning: Could not save: {e}");
            }

            println!("✓ Completed: {}", task_title);
        }
        n => {
            println!("Multiple tasks match '{}' ({} found):", query, n);
            for (i, task) in matches.iter().enumerate() {
                let due_str = task
                    .due_date
                    .map(|d| format!(" [{}]", d.format("%m/%d")))
                    .unwrap_or_default();
                println!("  {}. {}{}", i + 1, task.title, due_str);
            }
            eprintln!();
            eprintln!("Tips:");
            eprintln!("  - Use a more specific search query");
            eprintln!("  - Add --project <name> to filter by project");
            eprintln!("  - Add --tags <tag> to filter by tag");
            std::process::exit(1);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use taskflow::app::Model;
    use taskflow::domain::{Project, Task, TaskStatus};

    use super::mark_task_done;
    use crate::commands_tests::support::{
        add_project, add_task, assert_exit_one, data_file, open_model, run_exit_case, seed_model,
        temp_dir, yaml_cli,
    };

    fn words(items: &[&str]) -> Vec<String> {
        items.iter().map(ToString::to_string).collect()
    }

    fn status_of(model: &Model, title: &str) -> TaskStatus {
        model
            .tasks
            .values()
            .find(|t| t.title == title)
            .unwrap_or_else(|| panic!("task {title} exists"))
            .status
    }

    #[test]
    fn single_match_is_completed_and_saved() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        seed_model(&data, |model| {
            add_task(model, Task::new("Write report"));
            add_task(model, Task::new("Other task"));
        });

        mark_task_done(&yaml_cli(&data), &words(&["WRITE"]), None, None).expect("done succeeds");

        let model = open_model(&data);
        assert_eq!(status_of(&model, "Write report"), TaskStatus::Done);
        assert_eq!(status_of(&model, "Other task"), TaskStatus::Todo);
        let done = model
            .tasks
            .values()
            .find(|t| t.title == "Write report")
            .unwrap();
        assert!(done.completed_at.is_some());
    }

    #[test]
    fn already_complete_tasks_are_ignored_when_matching() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        seed_model(&data, |model| {
            add_task(model, Task::new("Ship v1").with_status(TaskStatus::Done));
            add_task(model, Task::new("Ship v2"));
        });

        mark_task_done(&yaml_cli(&data), &words(&["ship"]), None, None).expect("unambiguous");

        let model = open_model(&data);
        assert_eq!(status_of(&model, "Ship v2"), TaskStatus::Done);
    }

    #[test]
    fn project_filter_disambiguates() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        seed_model(&data, |model| {
            let backend = add_project(model, Project::new("Backend"));
            let frontend = add_project(model, Project::new("Frontend"));
            add_task(
                model,
                Task::new("Write report (backend)").with_project(backend),
            );
            add_task(
                model,
                Task::new("Write report (frontend)").with_project(frontend),
            );
        });

        mark_task_done(
            &yaml_cli(&data),
            &words(&["write", "report"]),
            Some("FRONT"),
            None,
        )
        .expect("project narrows to one task");

        let model = open_model(&data);
        assert_eq!(
            status_of(&model, "Write report (frontend)"),
            TaskStatus::Done
        );
        assert_eq!(
            status_of(&model, "Write report (backend)"),
            TaskStatus::Todo
        );
    }

    #[test]
    fn tags_filter_disambiguates() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        seed_model(&data, |model| {
            add_task(
                model,
                Task::new("Deploy prod").with_tags(vec!["prod".to_string()]),
            );
            add_task(
                model,
                Task::new("Deploy staging").with_tags(vec!["staging".to_string()]),
            );
        });

        mark_task_done(
            &yaml_cli(&data),
            &words(&["deploy"]),
            None,
            Some(&words(&["PROD"])),
        )
        .expect("tag narrows to one task");

        let model = open_model(&data);
        assert_eq!(status_of(&model, "Deploy prod"), TaskStatus::Done);
        assert_eq!(status_of(&model, "Deploy staging"), TaskStatus::Todo);
    }

    #[test]
    fn empty_query_exits_with_status_one() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        let output = run_exit_case("done_empty_query", &data, None);
        let text = assert_exit_one(&output);
        assert!(text.contains("Error: Search query cannot be empty"));
        assert!(!data.exists());
    }

    #[test]
    fn no_match_exits_with_status_one_and_filter_tip() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        seed_model(&data, |model| {
            add_task(model, Task::new("Something else"));
        });

        let output = run_exit_case("done_no_match", &data, None);
        let text = assert_exit_one(&output);
        assert!(text.contains("No matching incomplete tasks found for: \"nonexistent\""));
        assert!(text.contains("Try without --project or --tags filters"));
    }

    #[test]
    fn multiple_matches_exit_with_status_one_and_change_nothing() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        seed_model(&data, |model| {
            add_task(model, Task::new("Fix bug in parser"));
            add_task(model, Task::new("Fix bug in lexer"));
        });

        let output = run_exit_case("done_multiple", &data, None);
        let text = assert_exit_one(&output);
        assert!(text.contains("Multiple tasks match 'fix bug' (2 found):"));
        assert!(text.contains("Fix bug in parser"));
        assert!(text.contains("Fix bug in lexer"));
        assert!(text.contains("Use a more specific search query"));

        let model = open_model(&data);
        assert!(model.tasks.values().all(|t| t.status == TaskStatus::Todo));
    }
}
