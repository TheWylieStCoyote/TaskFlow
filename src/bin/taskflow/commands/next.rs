//! Next command - show the next task to work on.

use chrono::Utc;

use taskflow::domain::{Priority, Task};

use crate::cli::Cli;
use crate::load_model_for_cli;

/// Show the next task to work on.
///
/// Selection criteria (in order of importance):
/// 1. In-progress tasks (already started)
/// 2. Overdue tasks (highest priority first)
/// 3. Due today (highest priority first)
/// 4. Highest priority task with earliest due date
pub fn next_task(cli: &Cli) -> anyhow::Result<()> {
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

    // Filter to actionable incomplete tasks
    let candidates: Vec<_> = model
        .tasks
        .values()
        .filter(|t| {
            // Must be incomplete
            if t.status.is_complete() {
                return false;
            }
            // Must not be blocked by dependencies
            if is_blocked(t) {
                return false;
            }
            // Must not be snoozed
            if t.is_snoozed() {
                return false;
            }
            true
        })
        .collect();

    if candidates.is_empty() {
        println!("No actionable tasks found.");
        println!("All tasks are either complete, blocked, or snoozed.");
        return Ok(());
    }

    // Priority scoring for sorting
    let priority_score = |p: &Priority| match p {
        Priority::Urgent => 0,
        Priority::High => 1,
        Priority::Medium => 2,
        Priority::Low => 3,
        Priority::None => 4,
    };

    // Find the best next task using scoring
    let next = candidates
        .into_iter()
        .min_by(|a, b| {
            // 1. In-progress tasks come first
            let a_in_progress = a.status == taskflow::domain::TaskStatus::InProgress;
            let b_in_progress = b.status == taskflow::domain::TaskStatus::InProgress;
            if a_in_progress != b_in_progress {
                return b_in_progress.cmp(&a_in_progress);
            }

            // 2. Overdue tasks come next (by priority within overdue)
            let a_overdue = a.due_date.is_some_and(|d| d < today);
            let b_overdue = b.due_date.is_some_and(|d| d < today);
            if a_overdue != b_overdue {
                return b_overdue.cmp(&a_overdue);
            }

            // 3. Due today comes next
            let a_today = a.due_date == Some(today);
            let b_today = b.due_date == Some(today);
            if a_today != b_today {
                return b_today.cmp(&a_today);
            }

            // 4. Within same urgency class, sort by priority then due date
            let cmp = priority_score(&a.priority).cmp(&priority_score(&b.priority));
            if cmp != std::cmp::Ordering::Equal {
                return cmp;
            }

            // 5. Earlier due date wins
            match (&a.due_date, &b.due_date) {
                (Some(da), Some(db)) => da.cmp(db),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.created_at.cmp(&b.created_at),
            }
        })
        .expect("candidates is non-empty");

    // Display the next task
    println!("Next Task:");
    println!("{}", "=".repeat(50));
    println!();

    // Title with priority indicator
    let priority_str = match next.priority {
        Priority::Urgent => "[URGENT] ",
        Priority::High => "[HIGH] ",
        Priority::Medium => "",
        Priority::Low => "[low] ",
        Priority::None => "",
    };
    println!("  {}{}", priority_str, next.title);

    // Status
    let status_display = match next.status {
        taskflow::domain::TaskStatus::InProgress => "In Progress",
        taskflow::domain::TaskStatus::Todo => "Todo",
        taskflow::domain::TaskStatus::Blocked => "Blocked",
        taskflow::domain::TaskStatus::Done => "Done",
        taskflow::domain::TaskStatus::Cancelled => "Cancelled",
    };
    println!("  Status: {}", status_display);

    // Due date
    if let Some(due) = next.due_date {
        let due_str = if due < today {
            format!("{} (OVERDUE)", due.format("%Y-%m-%d"))
        } else if due == today {
            "Today".to_string()
        } else {
            due.format("%Y-%m-%d").to_string()
        };
        println!("  Due: {}", due_str);
    }

    // Tags
    if !next.tags.is_empty() {
        println!(
            "  Tags: {}",
            next.tags
                .iter()
                .map(|t| format!("#{t}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }

    // Estimate
    if let Some(est) = next.estimated_minutes {
        let hours = est / 60;
        let mins = est % 60;
        if hours > 0 {
            println!("  Estimate: {}h {}m", hours, mins);
        } else {
            println!("  Estimate: {}m", mins);
        }
    }

    // Description (truncated)
    if let Some(ref desc) = next.description {
        if !desc.is_empty() {
            println!();
            let truncated = if desc.len() > 200 {
                format!("{}...", &desc[..200])
            } else {
                desc.clone()
            };
            println!("  {}", truncated.replace('\n', "\n  "));
        }
    }

    println!();

    Ok(())
}
