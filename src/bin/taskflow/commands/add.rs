//! Quick add task command.

use taskflow::app::quick_add::parse_quick_add;
use taskflow::domain::Task;

use crate::cli::Cli;
use crate::load_model_for_cli;

/// Quick add a task from the command line
pub fn quick_add_task(cli: &Cli, task_words: &[String]) -> anyhow::Result<()> {
    // Join all words into a single task string
    let task_input = task_words.join(" ");
    if task_input.trim().is_empty() {
        eprintln!("Error: Task description cannot be empty");
        eprintln!("Usage: taskflow add <task description>");
        eprintln!("Example: taskflow add \"Buy milk #shopping !high due:tomorrow\"");
        std::process::exit(1);
    }

    // Load model with storage (fail fast on error)
    let mut model = load_model_for_cli(cli)?;

    // Parse the quick add syntax
    let parsed = parse_quick_add(&task_input);

    // Create the task
    let mut task = Task::new(&parsed.title);

    // Apply parsed metadata
    if let Some(priority) = parsed.priority {
        task = task.with_priority(priority);
    }
    if let Some(due_date) = parsed.due_date {
        task = task.with_due_date(due_date);
    }
    if let Some(sched_date) = parsed.scheduled_date {
        task.scheduled_date = Some(sched_date);
    }
    for tag in &parsed.tags {
        task.tags.push(tag.clone());
    }

    // Find project by name if specified
    if let Some(ref project_name) = parsed.project_name {
        let project_name_lower = project_name.to_lowercase();
        for project in model.projects.values() {
            if project.name.to_lowercase().contains(&project_name_lower) {
                task.project_id = Some(project.id);
                break;
            }
        }
    }

    // Add task and save
    let task_title = task.title.clone();
    let task_id = task.id;
    model.tasks.insert(task_id, task.clone());

    // Sync to storage
    model.sync_task(&task);
    if let Err(e) = model.save() {
        eprintln!("Warning: Could not save task: {e}");
    }

    // Print confirmation
    println!("✓ Added: {}", task_title);
    if !parsed.tags.is_empty() {
        println!(
            "  Tags: {}",
            parsed
                .tags
                .iter()
                .map(|t| format!("#{t}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    if let Some(priority) = parsed.priority {
        println!("  Priority: {:?}", priority);
    }
    if let Some(due) = parsed.due_date {
        println!("  Due: {}", due.format("%Y-%m-%d"));
    }
    if let Some(ref project_name) = parsed.project_name {
        if task.project_id.is_some() {
            println!("  Project: @{project_name}");
        } else {
            println!("  Project: @{project_name} (not found)");
        }
    }

    Ok(())
}
