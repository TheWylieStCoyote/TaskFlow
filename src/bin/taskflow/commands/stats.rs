//! Stats command - show quick productivity statistics.

use chrono::{Duration, Utc};

use taskflow::domain::TaskStatus;

use crate::cli::Cli;
use crate::load_model_for_cli;

/// Show quick productivity statistics
pub fn show_stats(cli: &Cli) -> anyhow::Result<()> {
    let model = load_model_for_cli(cli)?;
    let today = Utc::now().date_naive();
    let now = Utc::now();
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
        .filter(|t| t.completed_at.is_some_and(|c| c.date_naive() == today))
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
        .filter(|t| t.completed_at.is_some_and(|c| c.date_naive() >= week_ago))
        .count();

    // Time tracked today
    let time_today_mins: i64 = model
        .time_entries
        .values()
        .filter(|e| e.started_at.date_naive() == today)
        .map(|e| {
            if let Some(ended) = e.ended_at {
                (ended - e.started_at).num_minutes()
            } else {
                // Running timer
                (now - e.started_at).num_minutes()
            }
        })
        .sum();

    // Time tracked this week
    let time_week_mins: i64 = model
        .time_entries
        .values()
        .filter(|e| e.started_at.date_naive() >= week_ago)
        .map(|e| {
            if let Some(ended) = e.ended_at {
                (ended - e.started_at).num_minutes()
            } else {
                (now - e.started_at).num_minutes()
            }
        })
        .sum();

    // Habit streaks (if any)
    let active_habits = model.habits.len();
    let habits_due_today = model.habits.values().filter(|h| h.is_due_today()).count();
    let habits_completed_today = model
        .habits
        .values()
        .filter(|h| h.is_completed_on(today))
        .count();

    // Display stats
    println!("TaskFlow Statistics");
    println!("{}", "=".repeat(40));
    println!();

    // Tasks overview
    println!("Tasks Overview");
    println!("{}", "-".repeat(40));
    println!("  Total:        {}", total_tasks);
    println!(
        "  Completed:    {} ({:.0}%)",
        completed,
        if total_tasks > 0 {
            completed as f64 / total_tasks as f64 * 100.0
        } else {
            0.0
        }
    );
    println!("  In Progress:  {}", in_progress);
    println!("  Todo:         {}", todo);
    println!("  Blocked:      {}", blocked);
    println!();

    // Today
    println!("Today");
    println!("{}", "-".repeat(40));
    println!("  Due today:        {}", due_today);
    println!("  Completed today:  {}", completed_today);
    if overdue > 0 {
        println!("  Overdue:          {} ⚠️", overdue);
    }
    println!();

    // This week
    println!("This Week");
    println!("{}", "-".repeat(40));
    println!("  Tasks completed:  {}", completed_this_week);
    println!("  Time tracked:     {}", format_duration(time_week_mins));
    println!("  Time today:       {}", format_duration(time_today_mins));
    println!();

    // Habits (if any)
    if active_habits > 0 {
        println!("Habits");
        println!("{}", "-".repeat(40));
        println!("  Active habits:    {}", active_habits);
        println!("  Due today:        {}", habits_due_today);
        println!(
            "  Completed today:  {}/{}",
            habits_completed_today, habits_due_today
        );
        println!();
    }

    // Quick summary line
    if completed_today > 0 || due_today > 0 {
        println!(
            "Summary: {} of {} tasks completed today",
            completed_today,
            due_today + completed_today
        );
    }

    Ok(())
}

/// Format minutes as "Xh Ym" or "Xm"
fn format_duration(minutes: i64) -> String {
    if minutes <= 0 {
        return "0m".to_string();
    }
    let hours = minutes / 60;
    let mins = minutes % 60;
    if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}
