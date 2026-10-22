//! Stats command - show quick productivity statistics.

use std::fmt::Write as _;

use chrono::{DateTime, Duration, NaiveDate, Utc};

use taskflow::app::Model;
use taskflow::domain::TaskStatus;

use crate::cli::Cli;
use crate::load_model_for_cli;
use taskflow::domain::{today, LocalDate};

/// Aggregated numbers shown by `taskflow stats`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Stats {
    pub total_tasks: usize,
    pub completed: usize,
    pub in_progress: usize,
    pub todo: usize,
    pub blocked: usize,
    pub due_today: usize,
    pub completed_today: usize,
    pub overdue: usize,
    pub completed_this_week: usize,
    pub time_today_mins: i64,
    pub time_week_mins: i64,
    pub active_habits: usize,
    pub habits_due_today: usize,
    pub habits_completed_today: usize,
}

/// Show quick productivity statistics
pub fn show_stats(cli: &Cli) -> anyhow::Result<()> {
    let model = load_model_for_cli(cli)?;
    let stats = compute_stats(&model, today(), Utc::now());
    print!("{}", render_stats(&stats));
    Ok(())
}

/// Compute the statistics for `model` relative to `today` and `now`.
pub(crate) fn compute_stats(model: &Model, today: NaiveDate, now: DateTime<Utc>) -> Stats {
    let week_ago = today - Duration::days(7);

    // Task counts by status
    let total_tasks = model.tasks.len();
    let completed = model
        .tasks
        .values()
        .filter(|t| t.status == TaskStatus::Done)
        .count();
    let in_progress = model
        .tasks
        .values()
        .filter(|t| t.status == TaskStatus::InProgress)
        .count();
    let todo = model
        .tasks
        .values()
        .filter(|t| t.status == TaskStatus::Todo)
        .count();
    let blocked = model
        .tasks
        .values()
        .filter(|t| t.status == TaskStatus::Blocked)
        .count();

    // Today's stats
    let due_today = model
        .tasks
        .values()
        .filter(|t| t.due_date == Some(today) && !t.status.is_complete())
        .count();
    let completed_today = model
        .tasks
        .values()
        .filter(|t| t.completed_at.is_some_and(|c| c.local_date() == today))
        .count();

    // Overdue count
    let overdue = model
        .tasks
        .values()
        .filter(|t| t.due_date.is_some_and(|d| d < today) && !t.status.is_complete())
        .count();

    // This week's completions
    let completed_this_week = model
        .tasks
        .values()
        .filter(|t| t.completed_at.is_some_and(|c| c.local_date() >= week_ago))
        .count();

    // Time tracked today
    let time_today_mins: i64 = model
        .time_entries
        .values()
        .filter(|e| e.started_at.local_date() == today)
        .map(|e| entry_minutes(e, now))
        .sum();

    // Time tracked this week
    let time_week_mins: i64 = model
        .time_entries
        .values()
        .filter(|e| e.started_at.local_date() >= week_ago)
        .map(|e| entry_minutes(e, now))
        .sum();

    // Habit streaks (if any)
    let active_habits = model.habits.len();
    let habits_due_today = model.habits.values().filter(|h| h.is_due_today()).count();
    let habits_completed_today = model
        .habits
        .values()
        .filter(|h| h.is_completed_on(today))
        .count();

    Stats {
        total_tasks,
        completed,
        in_progress,
        todo,
        blocked,
        due_today,
        completed_today,
        overdue,
        completed_this_week,
        time_today_mins,
        time_week_mins,
        active_habits,
        habits_due_today,
        habits_completed_today,
    }
}

/// Minutes for an entry: a manual/stopped duration as stored, or the time
/// elapsed up to `now` for a running timer.
fn entry_minutes(entry: &taskflow::domain::TimeEntry, now: DateTime<Utc>) -> i64 {
    if entry.is_running() && entry.duration_minutes.is_none() {
        (now - entry.started_at).num_minutes().max(0)
    } else {
        i64::from(entry.calculated_duration_minutes())
    }
}

/// Render the statistics report exactly as `taskflow stats` prints it.
pub(crate) fn render_stats(stats: &Stats) -> String {
    let mut out = String::new();

    // Display stats
    out.push_str("TaskFlow Statistics\n");
    let _ = writeln!(out, "{}", "=".repeat(40));
    out.push('\n');

    // Tasks overview
    out.push_str("Tasks Overview\n");
    let _ = writeln!(out, "{}", "-".repeat(40));
    let _ = writeln!(out, "  Total:        {}", stats.total_tasks);
    let _ = writeln!(
        out,
        "  Completed:    {} ({:.0}%)",
        stats.completed,
        if stats.total_tasks > 0 {
            stats.completed as f64 / stats.total_tasks as f64 * 100.0
        } else {
            0.0
        }
    );
    let _ = writeln!(out, "  In Progress:  {}", stats.in_progress);
    let _ = writeln!(out, "  Todo:         {}", stats.todo);
    let _ = writeln!(out, "  Blocked:      {}", stats.blocked);
    out.push('\n');

    // Today
    out.push_str("Today\n");
    let _ = writeln!(out, "{}", "-".repeat(40));
    let _ = writeln!(out, "  Due today:        {}", stats.due_today);
    let _ = writeln!(out, "  Completed today:  {}", stats.completed_today);
    if stats.overdue > 0 {
        let _ = writeln!(out, "  Overdue:          {} ⚠️", stats.overdue);
    }
    out.push('\n');

    // This week
    out.push_str("This Week\n");
    let _ = writeln!(out, "{}", "-".repeat(40));
    let _ = writeln!(out, "  Tasks completed:  {}", stats.completed_this_week);
    let _ = writeln!(
        out,
        "  Time tracked:     {}",
        format_duration(stats.time_week_mins)
    );
    let _ = writeln!(
        out,
        "  Time today:       {}",
        format_duration(stats.time_today_mins)
    );
    out.push('\n');

    // Habits (if any)
    if stats.active_habits > 0 {
        out.push_str("Habits\n");
        let _ = writeln!(out, "{}", "-".repeat(40));
        let _ = writeln!(out, "  Active habits:    {}", stats.active_habits);
        let _ = writeln!(out, "  Due today:        {}", stats.habits_due_today);
        let _ = writeln!(
            out,
            "  Completed today:  {}/{}",
            stats.habits_completed_today, stats.habits_due_today
        );
        out.push('\n');
    }

    // Quick summary line
    if stats.completed_today > 0 || stats.due_today > 0 {
        let _ = writeln!(
            out,
            "Summary: {} of {} tasks completed today",
            stats.completed_today,
            stats.due_today + stats.completed_today
        );
    }

    out
}

/// Format minutes as "Xh Ym" or "Xm"
pub(crate) fn format_duration(minutes: i64) -> String {
    taskflow::domain::duration::format_minutes_i64(minutes)
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use taskflow::app::Model;
    use taskflow::domain::{today, Habit, Task, TaskStatus, TimeEntry};

    use super::{compute_stats, format_duration, render_stats, show_stats, Stats};
    use crate::commands_tests::support::{
        add_task, data_file, local_noon, seed_model, temp_dir, yaml_cli,
    };

    /// A model with one task per counter, plus time entries and habits.
    fn fixture() -> Model {
        let today = today();
        let mut model = Model::new();
        let mut push = |task: Task| {
            model.tasks.insert(task.id, task);
        };

        push(Task::new("Due today").with_due_date(today));
        push(Task::new("Overdue").with_due_date(today - Duration::days(1)));
        push(Task::new("Started").with_status(TaskStatus::InProgress));
        push(Task::new("Blocked").with_status(TaskStatus::Blocked));
        push(
            Task::new("Done today")
                .with_status(TaskStatus::Done)
                .with_completed_at(local_noon(today)),
        );
        push(
            Task::new("Done this week")
                .with_status(TaskStatus::Done)
                .with_completed_at(local_noon(today - Duration::days(3))),
        );
        push(
            Task::new("Done long ago")
                .with_status(TaskStatus::Done)
                .with_completed_at(local_noon(today - Duration::days(10))),
        );
        // Complete, so it neither counts as due today nor as overdue; it was
        // closed long before this week.
        push(
            Task::new("Cancelled")
                .with_status(TaskStatus::Cancelled)
                .with_due_date(today)
                .with_completed_at(local_noon(today - Duration::days(20))),
        );

        let task_id = model.tasks.keys().next().copied().unwrap();
        let mut entry = |start, end| {
            let mut e = TimeEntry::start(task_id);
            e.started_at = start;
            e.ended_at = end;
            model.time_entries.insert(e.id, e);
        };
        let noon = local_noon(today);
        entry(noon, Some(noon + Duration::minutes(90)));
        entry(noon, None); // still running
        let three_days_ago = local_noon(today - Duration::days(3));
        entry(three_days_ago, Some(three_days_ago + Duration::minutes(30)));
        let ten_days_ago = local_noon(today - Duration::days(10));
        entry(ten_days_ago, Some(ten_days_ago + Duration::minutes(600)));

        let mut done_habit = Habit::new("Read");
        done_habit.check_in(today, true, None);
        model.habits.insert(done_habit.id, done_habit);
        let open_habit = Habit::new("Run");
        model.habits.insert(open_habit.id, open_habit);

        model
    }

    #[test]
    fn compute_stats_counts_every_bucket() {
        let today = today();
        let model = fixture();
        let now = local_noon(today) + Duration::minutes(45);

        let stats = compute_stats(&model, today, now);

        assert_eq!(
            stats,
            Stats {
                total_tasks: 8,
                completed: 3,
                in_progress: 1,
                todo: 2,
                blocked: 1,
                due_today: 1,
                completed_today: 1,
                overdue: 1,
                completed_this_week: 2,
                time_today_mins: 90 + 45,
                time_week_mins: 90 + 45 + 30,
                active_habits: 2,
                habits_due_today: 2,
                habits_completed_today: 1,
            }
        );
    }

    #[test]
    fn compute_stats_on_empty_model_is_all_zero() {
        let today = today();
        let stats = compute_stats(&Model::new(), today, local_noon(today));
        assert_eq!(stats, Stats::default());
    }

    #[test]
    fn render_full_report() {
        let stats = Stats {
            total_tasks: 8,
            completed: 3,
            in_progress: 1,
            todo: 2,
            blocked: 1,
            due_today: 1,
            completed_today: 1,
            overdue: 1,
            completed_this_week: 2,
            time_today_mins: 135,
            time_week_mins: 165,
            active_habits: 2,
            habits_due_today: 2,
            habits_completed_today: 1,
        };
        let out = render_stats(&stats);
        let lines: Vec<_> = out.lines().collect();
        assert_eq!(lines[0], "TaskFlow Statistics");
        assert_eq!(lines[1], "=".repeat(40));
        assert!(lines.contains(&"  Total:        8"));
        assert!(lines.contains(&"  Completed:    3 (38%)"));
        assert!(lines.contains(&"  In Progress:  1"));
        assert!(lines.contains(&"  Todo:         2"));
        assert!(lines.contains(&"  Blocked:      1"));
        assert!(lines.contains(&"  Due today:        1"));
        assert!(lines.contains(&"  Completed today:  1"));
        assert!(lines.contains(&"  Overdue:          1 ⚠️"));
        assert!(lines.contains(&"  Tasks completed:  2"));
        assert!(lines.contains(&"  Time tracked:     2h 45m"));
        assert!(lines.contains(&"  Time today:       2h 15m"));
        assert!(lines.contains(&"Habits"));
        assert!(lines.contains(&"  Active habits:    2"));
        assert!(lines.contains(&"  Completed today:  1/2"));
        assert_eq!(
            lines.last().copied(),
            Some("Summary: 1 of 2 tasks completed today")
        );
    }

    #[test]
    fn render_omits_optional_sections_when_empty() {
        let out = render_stats(&Stats::default());
        assert!(out.contains("  Completed:    0 (0%)"));
        assert!(!out.contains("Overdue:"));
        assert!(!out.contains("Habits"));
        assert!(!out.contains("Summary:"));
        assert!(out.contains("  Time tracked:     0m"));
    }

    #[test]
    fn render_summary_appears_when_only_due_today() {
        let stats = Stats {
            total_tasks: 1,
            todo: 1,
            due_today: 1,
            ..Stats::default()
        };
        let out = render_stats(&stats);
        assert!(out.contains("  Completed:    0 (0%)"));
        assert!(out.ends_with("Summary: 0 of 1 tasks completed today\n"));
    }

    #[test]
    fn format_duration_cases() {
        assert_eq!(format_duration(0), "0m");
        assert_eq!(format_duration(-5), "0m");
        assert_eq!(format_duration(45), "45m");
        assert_eq!(format_duration(60), "1h");
        assert_eq!(format_duration(135), "2h 15m");
    }

    #[test]
    fn show_stats_runs_against_the_data_file() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        seed_model(&data, |model| {
            add_task(model, Task::new("Open"));
            add_task(model, Task::new("Closed").with_status(TaskStatus::Done));
        });
        show_stats(&yaml_cli(&data)).expect("stats succeeds");
    }

    #[test]
    fn entry_minutes_honours_manual_duration_and_running_timers() {
        use super::entry_minutes;
        use chrono::{Duration, Utc};
        use taskflow::domain::{Task, TimeEntry};
        let task = Task::new("t");
        let now = Utc::now();

        let mut manual = TimeEntry::start(task.id);
        manual.started_at = now - Duration::hours(3);
        manual.ended_at = Some(now - Duration::hours(1));
        manual.duration_minutes = Some(25); // corrected by hand
        assert_eq!(entry_minutes(&manual, now), 25);

        let mut running = TimeEntry::start(task.id);
        running.started_at = now - Duration::minutes(12);
        assert_eq!(entry_minutes(&running, now), 12);
    }
}
