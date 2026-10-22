//! Today command - list today's tasks.

use std::fmt::Write as _;

use chrono::NaiveDate;

use taskflow::app::Model;
use taskflow::domain::{Priority, Task, TaskStatus};

use crate::cli::Cli;
use crate::load_model_for_cli;
use taskflow::domain::today;

/// List tasks due today (shortcut for `list --view today`)
pub fn today_tasks(cli: &Cli, show_completed: bool) -> anyhow::Result<()> {
    let model = load_model_for_cli(cli)?;
    let tasks = select_today_tasks(&model, today(), show_completed);
    print!("{}", render_today_tasks(&tasks));
    Ok(())
}

/// Tasks due on `today`, sorted by priority (urgent first) then title.
pub(crate) fn select_today_tasks(
    model: &Model,
    today: NaiveDate,
    show_completed: bool,
) -> Vec<&Task> {
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

    tasks
}

/// Render today's task list exactly as `taskflow today` prints it.
pub(crate) fn render_today_tasks(tasks: &[&Task]) -> String {
    let mut out = String::new();

    if tasks.is_empty() {
        out.push_str("No tasks due today.\n");
        return out;
    }

    let _ = writeln!(out, "Today's Tasks ({} total)", tasks.len());
    let _ = writeln!(out, "{}", "-".repeat(50));

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

        let _ = writeln!(
            out,
            "{} {} {}{}",
            status_icon, priority_icon, task.title, tags_str
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use taskflow::app::Model;
    use taskflow::domain::{today, Priority, Task, TaskStatus};

    use super::{render_today_tasks, select_today_tasks, today_tasks};
    use crate::commands_tests::support::{add_task, data_file, seed_model, temp_dir, yaml_cli};

    const TODAY: NaiveDate = match NaiveDate::from_ymd_opt(2026, 10, 21) {
        Some(d) => d,
        None => panic!("valid date"),
    };

    fn fixture() -> Model {
        let mut model = Model::new();
        let tasks = vec![
            Task::new("zebra")
                .with_priority(Priority::Medium)
                .with_due_date(TODAY),
            Task::new("Apple")
                .with_priority(Priority::Medium)
                .with_due_date(TODAY),
            Task::new("urgent one")
                .with_priority(Priority::Urgent)
                .with_due_date(TODAY)
                .with_tags(vec!["ops".to_string()]),
            Task::new("low one")
                .with_priority(Priority::Low)
                .with_due_date(TODAY),
            Task::new("finished")
                .with_status(TaskStatus::Done)
                .with_due_date(TODAY),
            Task::new("tomorrow").with_due_date(TODAY + chrono::Duration::days(1)),
            Task::new("undated"),
        ];
        for task in tasks {
            model.tasks.insert(task.id, task);
        }
        model
    }

    fn titles<'a>(tasks: &[&'a Task]) -> Vec<&'a str> {
        tasks.iter().map(|t| t.title.as_str()).collect()
    }

    #[test]
    fn selects_only_tasks_due_today_sorted_by_priority_then_title() {
        let model = fixture();
        let tasks = select_today_tasks(&model, TODAY, false);
        assert_eq!(
            titles(&tasks),
            ["urgent one", "Apple", "zebra", "low one"],
            "title ordering is case-insensitive"
        );
    }

    #[test]
    fn show_completed_includes_finished_tasks() {
        let model = fixture();
        let tasks = select_today_tasks(&model, TODAY, true);
        assert_eq!(tasks.len(), 5);
        assert_eq!(tasks.last().unwrap().title, "finished");
    }

    #[test]
    fn no_tasks_on_a_different_day() {
        let model = fixture();
        let tasks = select_today_tasks(&model, TODAY + chrono::Duration::days(7), true);
        assert!(tasks.is_empty());
    }

    #[test]
    fn render_empty() {
        assert_eq!(render_today_tasks(&[]), "No tasks due today.\n");
    }

    #[test]
    fn render_rows() {
        let model = fixture();
        let tasks = select_today_tasks(&model, TODAY, true);
        let out = render_today_tasks(&tasks);
        let lines: Vec<_> = out.lines().collect();
        assert_eq!(lines[0], "Today's Tasks (5 total)");
        assert_eq!(lines[1], "-".repeat(50));
        assert_eq!(lines[2], "○ ‼️ urgent one #ops");
        assert_eq!(lines[3], "○ • Apple");
        assert_eq!(lines[4], "○ • zebra");
        assert_eq!(lines[5], "○ · low one");
        assert_eq!(lines[6], "✓   finished");
    }

    #[test]
    fn today_command_runs_against_the_data_file() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        let cli = yaml_cli(&data);

        seed_model(&data, |_| {});
        today_tasks(&cli, false).expect("empty store");

        seed_model(&data, |model| {
            add_task(model, Task::new("Due now").with_due_date(today()));
        });
        today_tasks(&cli, true).expect("with task");
    }
}
