//! Next command - show the next task to work on.

use std::fmt::Write as _;

use chrono::NaiveDate;

use taskflow::app::Model;
use taskflow::domain::{Priority, Task, TaskStatus};

use crate::cli::Cli;
use crate::load_model_for_cli;
use taskflow::domain::today;

/// Show the next task to work on.
///
/// Selection criteria (in order of importance):
/// 1. In-progress tasks (already started)
/// 2. Overdue tasks (highest priority first)
/// 3. Due today (highest priority first)
/// 4. Highest priority task with earliest due date
pub fn next_task(cli: &Cli) -> anyhow::Result<()> {
    let model = load_model_for_cli(cli)?;
    let today = today();

    match select_next_task(&model, today) {
        None => {
            println!("No actionable tasks found.");
            println!("All tasks are either complete, blocked, or snoozed.");
        }
        Some(next) => print!("{}", render_next_task(next, today)),
    }

    Ok(())
}

/// Pick the single best task to work on next, or `None` if nothing is actionable.
pub(crate) fn select_next_task(model: &Model, today: NaiveDate) -> Option<&Task> {
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

    // Priority scoring for sorting
    let priority_score = |p: &Priority| match p {
        Priority::Urgent => 0,
        Priority::High => 1,
        Priority::Medium => 2,
        Priority::Low => 3,
        Priority::None => 4,
    };

    // Find the best next task using scoring
    candidates.into_iter().min_by(|a, b| {
        // 1. In-progress tasks come first
        let a_in_progress = a.status == TaskStatus::InProgress;
        let b_in_progress = b.status == TaskStatus::InProgress;
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
}

/// Render the "Next Task" card exactly as `taskflow next` prints it.
pub(crate) fn render_next_task(next: &Task, today: NaiveDate) -> String {
    let mut out = String::new();

    // Display the next task
    out.push_str("Next Task:\n");
    let _ = writeln!(out, "{}", "=".repeat(50));
    out.push('\n');

    // Title with priority indicator
    let priority_str = match next.priority {
        Priority::Urgent => "[URGENT] ",
        Priority::High => "[HIGH] ",
        Priority::Medium | Priority::None => "",
        Priority::Low => "[low] ",
    };
    let _ = writeln!(out, "  {}{}", priority_str, next.title);

    // Status
    let status_display = match next.status {
        TaskStatus::InProgress => "In Progress",
        TaskStatus::Todo => "Todo",
        TaskStatus::Blocked => "Blocked",
        TaskStatus::Done => "Done",
        TaskStatus::Cancelled => "Cancelled",
    };
    let _ = writeln!(out, "  Status: {status_display}");

    // Due date
    if let Some(due) = next.due_date {
        let due_str = if due < today {
            format!("{} (OVERDUE)", due.format("%Y-%m-%d"))
        } else if due == today {
            "Today".to_string()
        } else {
            due.format("%Y-%m-%d").to_string()
        };
        let _ = writeln!(out, "  Due: {due_str}");
    }

    // Tags
    if !next.tags.is_empty() {
        let _ = writeln!(
            out,
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
            let _ = writeln!(out, "  Estimate: {hours}h {mins}m");
        } else {
            let _ = writeln!(out, "  Estimate: {mins}m");
        }
    }

    // Description (truncated)
    if let Some(ref desc) = next.description {
        if !desc.is_empty() {
            out.push('\n');
            let truncated = if desc.len() > 200 {
                format!("{}...", &desc[..200])
            } else {
                desc.clone()
            };
            let _ = writeln!(out, "  {}", truncated.replace('\n', "\n  "));
        }
    }

    out.push('\n');

    out
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Utc};

    use taskflow::app::Model;
    use taskflow::domain::{today, Priority, Task, TaskStatus};

    use super::{next_task, render_next_task, select_next_task};
    use crate::commands_tests::support::{add_task, data_file, seed_model, temp_dir, yaml_cli};

    const TODAY: NaiveDate = match NaiveDate::from_ymd_opt(2026, 10, 21) {
        Some(d) => d,
        None => panic!("valid date"),
    };

    fn days(n: i64) -> NaiveDate {
        TODAY + chrono::Duration::days(n)
    }

    fn model_of(tasks: Vec<Task>) -> Model {
        let mut model = Model::new();
        for task in tasks {
            model.tasks.insert(task.id, task);
        }
        model
    }

    fn next_title(model: &Model) -> Option<&str> {
        select_next_task(model, TODAY).map(|t| t.title.as_str())
    }

    #[test]
    fn empty_model_has_no_next_task() {
        assert_eq!(next_title(&Model::new()), None);
    }

    #[test]
    fn in_progress_beats_overdue() {
        let model = model_of(vec![
            Task::new("Overdue")
                .with_priority(Priority::Urgent)
                .with_due_date(days(-3)),
            Task::new("Started").with_status(TaskStatus::InProgress),
        ]);
        assert_eq!(next_title(&model), Some("Started"));
    }

    #[test]
    fn overdue_beats_due_today_regardless_of_priority() {
        let model = model_of(vec![
            Task::new("Due today")
                .with_priority(Priority::Urgent)
                .with_due_date(TODAY),
            Task::new("Overdue")
                .with_priority(Priority::Low)
                .with_due_date(days(-1)),
        ]);
        assert_eq!(next_title(&model), Some("Overdue"));
    }

    #[test]
    fn due_today_beats_future_regardless_of_priority() {
        let model = model_of(vec![
            Task::new("Future")
                .with_priority(Priority::Urgent)
                .with_due_date(days(2)),
            Task::new("Today")
                .with_priority(Priority::Low)
                .with_due_date(TODAY),
        ]);
        assert_eq!(next_title(&model), Some("Today"));
    }

    #[test]
    fn same_urgency_class_orders_by_priority_then_due_date() {
        let model = model_of(vec![
            Task::new("High without due").with_priority(Priority::High),
            Task::new("High due later")
                .with_priority(Priority::High)
                .with_due_date(days(5)),
            Task::new("High due soon")
                .with_priority(Priority::High)
                .with_due_date(days(2)),
            Task::new("Medium due tomorrow")
                .with_priority(Priority::Medium)
                .with_due_date(days(1)),
        ]);
        assert_eq!(next_title(&model), Some("High due soon"));
    }

    #[test]
    fn full_tie_breaks_on_creation_time() {
        let mut older = Task::new("Older");
        older.created_at = Utc.with_ymd_and_hms(2026, 1, 1, 8, 0, 0).unwrap();
        let mut newer = Task::new("Newer");
        newer.created_at = Utc.with_ymd_and_hms(2026, 1, 2, 8, 0, 0).unwrap();
        let model = model_of(vec![newer, older]);
        assert_eq!(next_title(&model), Some("Older"));
    }

    #[test]
    fn complete_blocked_and_snoozed_tasks_are_not_candidates() {
        let mut snoozed = Task::new("Snoozed").with_priority(Priority::Urgent);
        snoozed.snooze_until_date(today() + chrono::Duration::days(10));
        let mut blocked = Task::new("Blocked").with_priority(Priority::Urgent);
        blocked.dependencies.push(snoozed.id);
        let done = Task::new("Done").with_status(TaskStatus::Done);
        let cancelled = Task::new("Cancelled").with_status(TaskStatus::Cancelled);

        let model = model_of(vec![snoozed, blocked, done, cancelled]);
        assert_eq!(next_title(&model), None);
    }

    #[test]
    fn dependency_on_completed_task_does_not_block() {
        let done = Task::new("Done").with_status(TaskStatus::Done);
        let mut follow_up = Task::new("Follow up");
        follow_up.dependencies.push(done.id);
        let model = model_of(vec![done, follow_up]);
        assert_eq!(next_title(&model), Some("Follow up"));
    }

    #[test]
    fn render_full_card() {
        let mut task = Task::new("Ship release")
            .with_priority(Priority::Urgent)
            .with_due_date(days(-1))
            .with_tags(vec!["release".to_string(), "ops".to_string()])
            .with_estimated_minutes(135)
            .with_description("Line one\nLine two");
        task.status = TaskStatus::InProgress;

        let out = render_next_task(&task, TODAY);
        let expected = format!(
            "Next Task:\n{}\n\n  [URGENT] Ship release\n  Status: In Progress\n  Due: 2026-10-20 (OVERDUE)\n  Tags: #release #ops\n  Estimate: 2h 15m\n\n  Line one\n  Line two\n\n",
            "=".repeat(50)
        );
        assert_eq!(out, expected);
    }

    #[test]
    fn render_minimal_card_omits_optional_sections() {
        let task = Task::new("Tiny");
        let out = render_next_task(&task, TODAY);
        let lines: Vec<_> = out.lines().collect();
        assert_eq!(lines[0], "Next Task:");
        assert_eq!(lines[3], "  Tiny");
        assert_eq!(lines[4], "  Status: Todo");
        assert_eq!(lines.len(), 6, "no due, tags, estimate or description");
        assert!(!out.contains("Due:"));
        assert!(!out.contains("Tags:"));
        assert!(!out.contains("Estimate:"));
    }

    #[test]
    fn render_due_today_future_and_short_estimate() {
        let today_task = Task::new("T")
            .with_priority(Priority::High)
            .with_due_date(TODAY)
            .with_estimated_minutes(45);
        let out = render_next_task(&today_task, TODAY);
        assert!(out.contains("  [HIGH] T\n"));
        assert!(out.contains("  Due: Today\n"));
        assert!(out.contains("  Estimate: 45m\n"));

        let future_task = Task::new("F")
            .with_priority(Priority::Low)
            .with_due_date(days(9))
            .with_status(TaskStatus::Blocked);
        let out = render_next_task(&future_task, TODAY);
        assert!(out.contains("  [low] F\n"));
        assert!(out.contains("  Status: Blocked\n"));
        assert!(out.contains("  Due: 2026-10-30\n"));

        let medium = Task::new("M").with_priority(Priority::Medium);
        assert!(render_next_task(&medium, TODAY).contains("\n  M\n"));
    }

    #[test]
    fn render_truncates_long_description_and_skips_empty_one() {
        let long = "x".repeat(250);
        let task = Task::new("Long").with_description(long);
        let out = render_next_task(&task, TODAY);
        let expected_line = format!("  {}...", "x".repeat(200));
        assert!(out.lines().any(|l| l == expected_line));

        let empty = Task::new("Empty").with_description("");
        let out = render_next_task(&empty, TODAY);
        assert_eq!(out.lines().count(), 6, "an empty description adds no lines");
    }

    #[test]
    fn next_task_command_runs_against_the_data_file() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        let cli = yaml_cli(&data);

        // Empty store: the "nothing actionable" branch.
        seed_model(&data, |_| {});
        next_task(&cli).expect("empty store");

        // A candidate exists: the card branch.
        seed_model(&data, |model| {
            add_task(model, Task::new("Candidate").with_priority(Priority::High));
        });
        next_task(&cli).expect("with candidate");
    }
}
