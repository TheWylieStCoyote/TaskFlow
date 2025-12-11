//! Today command - list today's tasks.

use chrono::Utc;

use taskflow::domain::{Priority, TaskStatus};

use crate::cli::Cli;
use crate::load_model_for_cli;

/// List tasks due today (shortcut for `list --view today`)
pub fn today_tasks(cli: &Cli, show_completed: bool) -> anyhow::Result<()> {
    let model = load_model_for_cli(cli)?;
    let today = Utc::now().date_naive();

    // Filter tasks due today
    let mut tasks: Vec<_> = model
        .tasks
        .values()
        .filter(|t| {
            // Filter by due date = today
            if t.due_date != Some(today) {
                return false;
            }
            // Filter completed unless requested
            if !show_completed && t.status.is_complete() {
                return false;
            }
            true
        })
        .collect();

    // Sort by priority (urgent first), then by title
    let priority_order = |p: &Priority| match p {
        Priority::Urgent => 0,
        Priority::High => 1,
        Priority::Medium => 2,
        Priority::Low => 3,
        Priority::None => 4,
    };

    tasks.sort_by(|a, b| {
        priority_order(&a.priority)
            .cmp(&priority_order(&b.priority))
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });

    if tasks.is_empty() {
        println!("No tasks due today.");
        return Ok(());
    }

    println!("Today's Tasks ({} total)", tasks.len());
    println!("{}", "-".repeat(50));

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
            "{} {} {}{}",
            status_icon, priority_icon, task.title, tags_str
        );
    }

    Ok(())
}
