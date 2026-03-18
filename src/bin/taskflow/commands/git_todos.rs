//! Git TODO extraction command.

use std::collections::HashMap;
use std::path::Path;

use tracing::warn;

use taskflow::app::extract_git_location;
use taskflow::domain::git::scan_git_todos_with_patterns;
use taskflow::domain::{Priority, Task};

use crate::cli::Cli;
use crate::load_model_for_cli;

/// Extract TODO/FIXME comments from a git repository and create tasks.
pub fn extract_git_todos(
    cli: &Cli,
    repo: &Path,
    patterns: &[String],
    project_name: Option<&str>,
    extra_tags: Option<&[String]>,
    priority_str: &str,
    dry_run: bool,
) -> anyhow::Result<()> {
    // Parse priority
    let priority = match priority_str.to_lowercase().as_str() {
        "urgent" => Priority::Urgent,
        "high" => Priority::High,
        "medium" | "med" => Priority::Medium,
        "low" => Priority::Low,
        _ => Priority::None,
    };

    // Scan git repository for TODOs
    let todos = scan_git_todos_with_patterns(repo, patterns);

    if todos.is_empty() {
        println!("No TODO/FIXME comments found in {}", repo.display());
        return Ok(());
    }

    if dry_run {
        println!("Found {} TODO/FIXME comments (dry run):\n", todos.len());
        for todo in &todos {
            println!("  {} [{}:{}]", todo.title, todo.file, todo.line);
        }
        return Ok(());
    }

    // Load model with storage
    let mut model = load_model_for_cli(cli)?;

    // Find project ID if specified
    let project_id = project_name.and_then(|name| {
        let name_lower = name.to_lowercase();
        model
            .projects
            .values()
            .find(|p| p.name.to_lowercase().contains(&name_lower))
            .map(|p| p.id)
    });

    // Build lookup of existing git-todo tasks by their source location
    let mut existing_by_location: HashMap<String, taskflow::domain::TaskId> = HashMap::new();
    for task in model.tasks.values() {
        if let Some(ref desc) = task.description {
            if let Some((file, line)) = extract_git_location(desc) {
                existing_by_location.insert(format!("{file}:{line}"), task.id);
            }
        }
    }

    let mut created = 0;
    let mut updated = 0;

    for todo in &todos {
        let location_key = format!("{}:{}", todo.file, todo.line);
        let description = format!(
            "git:{}:{}\n\nFile: {}\nLine: {}\nPattern: {}\n\n{}",
            todo.file, todo.line, todo.file, todo.line, todo.pattern, todo.context
        );

        // Build tags
        let mut tags = vec!["git-todo".to_string(), todo.pattern.to_lowercase()];
        if let Some(extra) = extra_tags {
            tags.extend(extra.iter().cloned());
        }

        if let Some(&existing_id) = existing_by_location.get(&location_key) {
            // Update existing task
            if let Some(task) = model.tasks.get_mut(&existing_id) {
                task.title = todo.title.clone();
                task.description = Some(description);
                if task.project_id.is_none() {
                    task.project_id = project_id;
                }
                for tag in &tags {
                    if !task.tags.contains(tag) {
                        task.tags.push(tag.clone());
                    }
                }
                let task_clone = task.clone();
                model.sync_task(&task_clone);
                updated += 1;
            }
        } else {
            // Create new task
            let mut task = Task::new(&todo.title)
                .with_priority(priority)
                .with_description(description);

            if let Some(pid) = project_id {
                task = task.with_project(pid);
            }

            task.tags = tags;

            let task_id = task.id;
            model.tasks.insert(task_id, task.clone());
            model.sync_task(&task);
            created += 1;
        }
    }

    // Save to disk
    if let Err(e) = model.save() {
        warn!(error = %e, "Could not save git TODOs to disk");
        eprintln!("Warning: Could not save to disk: {e}");
    }

    // Print summary
    println!("✓ Extracted TODOs from {}", repo.display());
    println!("  Patterns: {}", patterns.join(", "));
    println!("  Created: {}", created);
    println!("  Updated: {}", updated);
    if let Some(name) = project_name {
        if project_id.is_some() {
            println!("  Project: @{}", name);
        } else {
            warn!(project = %name, "Referenced project not found for git TODOs");
            eprintln!("  Project: @{} (not found)", name);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use taskflow::app::extract_git_location;

    #[test]
    fn test_extract_git_location() {
        let desc = "git:src/auth.rs:42\n\nFile: src/auth.rs\nLine: 42";
        assert_eq!(
            extract_git_location(desc),
            Some(("src/auth.rs".to_string(), 42))
        );

        let desc = "No git marker here";
        assert_eq!(extract_git_location(desc), None);
    }
}
