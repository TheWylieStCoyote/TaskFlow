//! Quick add task command.

use tracing::warn;

use taskflow::app::quick_add::parse_quick_add;
use taskflow::domain::Task;

use crate::cli::Cli;
use crate::load_model_for_cli;

/// Quick add a task from the command line
pub fn quick_add_task(cli: &Cli, task_words: &[String]) -> anyhow::Result<()> {
    // Join all words into a single task string
    let task_input = task_words.join(" ");
    if task_input.trim().is_empty() {
        warn!("Empty task description provided");
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
        warn!(error = %e, "Task added but could not save to disk");
        eprintln!("Warning: Task added to session but could not save to disk: {e}");
        eprintln!("  The task will be available until you close the TUI.");
        eprintln!("  Check disk space and file permissions, then try again.");
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
            warn!(project = %project_name, "Referenced project not found");
            eprintln!("  Project: @{project_name} (not found - create it in the TUI with 'P')");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use taskflow::domain::{Priority, Project};

    use super::quick_add_task;
    use crate::commands_tests::support::{
        add_project, assert_exit_one, data_file, open_model, run_exit_case, seed_model, temp_dir,
        yaml_cli,
    };

    fn words(items: &[&str]) -> Vec<String> {
        items.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn adds_a_plain_task_and_persists_it() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        seed_model(&data, |_| {});

        quick_add_task(&yaml_cli(&data), &words(&["Buy", "milk"])).expect("add succeeds");

        let model = open_model(&data);
        assert_eq!(model.tasks.len(), 1);
        let task = model.tasks.values().next().unwrap();
        assert_eq!(task.title, "Buy milk");
        assert!(task.tags.is_empty());
        assert!(task.project_id.is_none());
    }

    #[test]
    fn applies_quick_add_metadata_and_resolves_project() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        let mut backend_id = None;
        seed_model(&data, |model| {
            backend_id = Some(add_project(model, Project::new("Backend API")));
        });

        quick_add_task(
            &yaml_cli(&data),
            &words(&[
                "Fix",
                "login",
                "#bug",
                "#auth",
                "!high",
                "due:2026-12-01",
                "sched:2026-11-28",
                "@backend",
            ]),
        )
        .expect("add succeeds");

        let model = open_model(&data);
        let task = model.tasks.values().next().unwrap();
        assert_eq!(task.title, "Fix login");
        assert_eq!(task.tags, vec!["bug".to_string(), "auth".to_string()]);
        assert_eq!(task.priority, Priority::High);
        assert_eq!(task.due_date, NaiveDate::from_ymd_opt(2026, 12, 1));
        assert_eq!(task.scheduled_date, NaiveDate::from_ymd_opt(2026, 11, 28));
        assert_eq!(task.project_id, backend_id);
    }

    #[test]
    fn unknown_project_leaves_task_unassigned() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        seed_model(&data, |model| {
            add_project(model, Project::new("Backend"));
        });

        quick_add_task(&yaml_cli(&data), &words(&["Call", "mum", "@nowhere"]))
            .expect("add succeeds even when the project is missing");

        let model = open_model(&data);
        let task = model.tasks.values().next().unwrap();
        assert_eq!(task.title, "Call mum");
        assert!(task.project_id.is_none());
    }

    #[test]
    fn empty_description_exits_with_status_one() {
        let dir = temp_dir();
        let data = data_file(dir.path());

        let output = run_exit_case("add_empty", &data, None);
        let text = assert_exit_one(&output);
        assert!(text.contains("Error: Task description cannot be empty"));
        assert!(text.contains("Usage: taskflow add <task description>"));
        assert!(!data.exists(), "nothing is written before validation");
    }
}
