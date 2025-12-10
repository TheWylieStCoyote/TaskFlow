//! Mark task done command.

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
