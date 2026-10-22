//! Time command - report tracked time by project, day, task or tag.

use std::fmt::Write as _;

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use serde_json::{json, Value};

use taskflow::app::analytics::AnalyticsEngine;
use taskflow::app::Model;
use taskflow::domain::{format_hms, format_minutes, today, ProjectId, TaskId};

use crate::cli::{Breakdown, Cli, TimeArgs};
use crate::load_model_for_cli;

/// Width, in cells, of the longest bar in a breakdown table.
const BAR_WIDTH: usize = 20;
/// Longest label printed before truncation.
const MAX_LABEL: usize = 32;
/// Label for time on tasks that belong to no project.
const NO_PROJECT: &str = "(no project)";
/// Label for time whose task no longer exists.
const DELETED_TASK: &str = "(deleted task)";

/// The date range a report covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TimeWindow {
    pub from: NaiveDate,
    pub to: NaiveDate,
    /// Human description: "Today", "This week", "Last 7 days", "A to B".
    pub label: String,
    /// Whether the report adds a this-week total (default mode only).
    pub with_week_total: bool,
}

/// One row of a breakdown table.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TimeRow {
    /// What is printed: project name, task title, tag or "date weekday".
    pub label: String,
    /// Machine key for JSON: a UUID for projects/tasks, an ISO date for
    /// days; `None` for tags and for time without a project.
    pub key: Option<String>,
    pub minutes: u32,
    /// Share of the window total, 0.0 to 100.0.
    pub percent: f64,
}

/// A timer that is running right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tracking {
    pub task_id: TaskId,
    pub title: String,
    pub elapsed_seconds: u64,
}

/// Everything `taskflow time` prints, in either format.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TimeReport {
    pub window: TimeWindow,
    /// Name of the project the report was restricted to, if any.
    pub project_filter: Option<String>,
    pub total_minutes: u32,
    pub entry_count: usize,
    /// Monday-to-today total, present in the default (today) mode.
    pub week_minutes: Option<u32>,
    pub by_project: Vec<TimeRow>,
    pub by_day: Vec<TimeRow>,
    pub by_task: Vec<TimeRow>,
    pub by_tag: Vec<TimeRow>,
    pub tracking: Option<Tracking>,
}

/// Report tracked time for the window and project selected by `args`.
pub fn show_time(cli: &Cli, args: &TimeArgs) -> anyhow::Result<()> {
    let model = load_model_for_cli(cli)?;
    let report = compute_time_report(&model, args, today(), Utc::now())?;
    if args.json {
        println!("{}", render_time_json(&report)?);
    } else {
        print!("{}", render_time_report(&report, args.by));
    }
    Ok(())
}

/// Monday of the week containing `date`.
fn monday_of(date: NaiveDate) -> NaiveDate {
    date - Duration::days(i64::from(date.weekday().num_days_from_monday()))
}

/// Turn the window flags into a concrete date range.
///
/// # Errors
///
/// `--days 0`, or `--to` earlier than `--from`, are rejected.
pub(crate) fn resolve_window(args: &TimeArgs, today: NaiveDate) -> anyhow::Result<TimeWindow> {
    if let Some(from) = args.from {
        let to = args.to.unwrap_or(today);
        if to < from {
            anyhow::bail!("--to ({to}) is before --from ({from})");
        }
        return Ok(TimeWindow {
            from,
            to,
            label: format!("{from} to {to}"),
            with_week_total: false,
        });
    }
    if let Some(days) = args.days {
        if days == 0 {
            anyhow::bail!("--days must be at least 1");
        }
        let from = today - Duration::days(i64::from(days) - 1);
        let label = if days == 1 {
            "Today".to_string()
        } else {
            format!("Last {days} days")
        };
        return Ok(TimeWindow {
            from,
            to: today,
            label,
            with_week_total: false,
        });
    }
    if args.week {
        return Ok(TimeWindow {
            from: monday_of(today),
            to: today,
            label: "This week".to_string(),
            with_week_total: false,
        });
    }
    Ok(TimeWindow {
        from: today,
        to: today,
        label: "Today".to_string(),
        with_week_total: true,
    })
}

/// Find the one project `name` refers to: an exact (case-insensitive) name
/// wins, otherwise a unique substring match.
fn find_project(model: &Model, name: &str) -> anyhow::Result<ProjectId> {
    let wanted = name.trim().to_lowercase();
    if let Some(project) = model
        .projects
        .values()
        .find(|p| p.name.to_lowercase() == wanted)
    {
        return Ok(project.id);
    }
    let mut matches: Vec<_> = model
        .projects
        .values()
        .filter(|p| p.name.to_lowercase().contains(&wanted))
        .collect();
    matches.sort_by(|a, b| a.name.cmp(&b.name));
    match matches.as_slice() {
        [] => anyhow::bail!("No project matches '{name}'"),
        [only] => Ok(only.id),
        many => {
            let names: Vec<_> = many.iter().map(|p| p.name.as_str()).collect();
            anyhow::bail!(
                "'{name}' matches several projects: {}. Give the full name.",
                names.join(", ")
            )
        }
    }
}

/// A detached copy of `model` holding only the tasks (and their time
/// entries) that belong to `project_id`.
fn project_scope(model: &Model, project_id: ProjectId) -> Model {
    let mut scoped = Model::new();
    scoped.projects = model.projects.clone();
    for task in model
        .tasks
        .values()
        .filter(|t| t.project_id == Some(project_id))
    {
        scoped.tasks.insert(task.id, task.clone());
    }
    for entry in model
        .time_entries
        .values()
        .filter(|e| scoped.tasks.contains_key(&e.task_id))
    {
        scoped.time_entries.insert(entry.id, entry.clone());
    }
    scoped
}

fn percent_of(minutes: u32, total: u32) -> f64 {
    if total == 0 {
        0.0
    } else {
        f64::from(minutes) / f64::from(total) * 100.0
    }
}

/// Compute the report for `model` as of `today`, with `now` driving the
/// elapsed time of a running timer.
///
/// # Errors
///
/// Returns an error for an invalid window or an unknown/ambiguous project.
pub(crate) fn compute_time_report(
    model: &Model,
    args: &TimeArgs,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> anyhow::Result<TimeReport> {
    let window = resolve_window(args, today)?;

    let scoped;
    let (scope, project_filter) = match &args.project {
        Some(name) => {
            let id = find_project(model, name)?;
            scoped = project_scope(model, id);
            (&scoped, model.projects.get(&id).map(|p| p.name.clone()))
        }
        None => (model, None),
    };

    let engine = AnalyticsEngine::new(scope);
    let analytics = engine.compute_time_analytics(window.from, window.to);
    let week_minutes = window.with_week_total.then(|| {
        engine
            .compute_time_analytics(monday_of(today), today)
            .total_minutes
    });
    let total = analytics.total_minutes;

    let by_project = analytics
        .projects_by_minutes()
        .into_iter()
        .map(|(pid, minutes)| TimeRow {
            label: pid.map_or_else(
                || NO_PROJECT.to_string(),
                |p| {
                    scope
                        .projects
                        .get(&p)
                        .map_or_else(|| "(unknown project)".to_string(), |p| p.name.clone())
                },
            ),
            key: pid.map(|p| p.0.to_string()),
            minutes,
            percent: percent_of(minutes, total),
        })
        .collect();

    let by_day = analytics
        .daily_series(window.from, window.to)
        .into_iter()
        .map(|(date, minutes)| TimeRow {
            label: format!("{date} {}", date.weekday()),
            key: Some(date.to_string()),
            minutes,
            percent: percent_of(minutes, total),
        })
        .collect();

    let by_task = analytics
        .tasks_by_minutes()
        .into_iter()
        .map(|(task_id, minutes)| TimeRow {
            label: scope
                .tasks
                .get(&task_id)
                .map_or_else(|| DELETED_TASK.to_string(), |t| t.title.clone()),
            key: Some(task_id.0.to_string()),
            minutes,
            percent: percent_of(minutes, total),
        })
        .collect();

    let by_tag = analytics
        .tags_by_minutes()
        .into_iter()
        .map(|(tag, minutes)| TimeRow {
            label: tag,
            key: None,
            minutes,
            percent: percent_of(minutes, total),
        })
        .collect();

    // What is running is reported from the whole model: a timer is a fact
    // about right now, not about the chosen window or project.
    let tracking = model.active_time_entry().map(|entry| Tracking {
        task_id: entry.task_id,
        title: model
            .tasks
            .get(&entry.task_id)
            .map_or_else(|| DELETED_TASK.to_string(), |t| t.title.clone()),
        elapsed_seconds: u64::try_from((now - entry.started_at).num_seconds().max(0)).unwrap_or(0),
    });

    Ok(TimeReport {
        window,
        project_filter,
        total_minutes: total,
        entry_count: analytics.entry_count,
        week_minutes,
        by_project,
        by_day,
        by_task,
        by_tag,
        tracking,
    })
}

/// A bar of `BAR_WIDTH` cells at most, proportional to `value / max`,
/// using eighth-block characters for the fractional cell (`████▌`).
pub(crate) fn bar(value: u32, max: u32) -> String {
    const EIGHTHS: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];
    if max == 0 || value == 0 {
        return String::new();
    }
    let units = u64::from(value) * (BAR_WIDTH as u64) * 8 / u64::from(max);
    let full = usize::try_from(units / 8).unwrap_or(BAR_WIDTH);
    let rem = usize::try_from(units % 8).unwrap_or(0);
    let mut out = "█".repeat(full);
    if rem > 0 {
        out.push(EIGHTHS[rem]);
    }
    out
}

/// Clip a label to `MAX_LABEL` characters, marking the cut with an ellipsis.
fn clip(label: &str) -> String {
    if label.chars().count() <= MAX_LABEL {
        label.to_string()
    } else {
        let mut s: String = label.chars().take(MAX_LABEL - 1).collect();
        s.push('…');
        s
    }
}

fn window_title(window: &TimeWindow) -> String {
    if window.from == window.to {
        format!("{} ({})", window.label, window.from)
    } else if window.label.contains(" to ") {
        window.label.clone()
    } else {
        format!("{} ({} to {})", window.label, window.from, window.to)
    }
}

/// Render the text report exactly as `taskflow time` prints it.
pub(crate) fn render_time_report(report: &TimeReport, by: Breakdown) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "Time Tracked: {}", window_title(&report.window));
    let _ = writeln!(out, "{}", "=".repeat(40));
    if let Some(project) = &report.project_filter {
        let _ = writeln!(out, "  Project:      {project}");
    }
    let entries = if report.entry_count == 1 {
        "entry"
    } else {
        "entries"
    };
    let _ = writeln!(
        out,
        "  Total:        {} ({} {entries})",
        format_minutes(report.total_minutes),
        report.entry_count
    );
    if let Some(week) = report.week_minutes {
        let _ = writeln!(out, "  This week:    {}", format_minutes(week));
    }
    if let Some(tracking) = &report.tracking {
        let _ = writeln!(
            out,
            "  Tracking now: {} {}",
            tracking.title,
            format_hms(tracking.elapsed_seconds)
        );
    }
    out.push('\n');

    let (title, rows) = match by {
        Breakdown::Project => ("By project", &report.by_project),
        Breakdown::Day => ("By day", &report.by_day),
        Breakdown::Task => ("By task", &report.by_task),
        Breakdown::Tag => ("By tag", &report.by_tag),
    };
    let _ = writeln!(out, "{title}");
    let _ = writeln!(out, "{}", "-".repeat(40));
    if report.total_minutes == 0 || rows.is_empty() {
        out.push_str("  No time tracked in this window.\n");
    } else {
        render_table(&mut out, rows);
    }

    out
}

fn render_table(out: &mut String, rows: &[TimeRow]) {
    let labels: Vec<String> = rows.iter().map(|r| clip(&r.label)).collect();
    let width = labels
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0)
        .max(8);
    let max = rows.iter().map(|r| r.minutes).max().unwrap_or(0);
    for (row, label) in rows.iter().zip(&labels) {
        let _ = writeln!(
            out,
            "  {label:<width$}  {:<BAR_WIDTH$}  {:>8}  {:5.1}%",
            bar(row.minutes, max),
            format_minutes(row.minutes),
            row.percent
        );
    }
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

/// Render the report as pretty-printed JSON.
///
/// # Errors
///
/// Serialisation errors are propagated (they do not occur in practice).
pub(crate) fn render_time_json(report: &TimeReport) -> anyhow::Result<String> {
    let by_project: Vec<Value> = report
        .by_project
        .iter()
        .map(|r| {
            json!({
                "project": r.key.as_ref().map(|_| r.label.clone()),
                "project_id": r.key,
                "minutes": r.minutes,
                "percent": round1(r.percent),
            })
        })
        .collect();
    let by_day: Vec<Value> = report
        .by_day
        .iter()
        .map(|r| json!({ "date": r.key, "minutes": r.minutes, "percent": round1(r.percent) }))
        .collect();
    let by_task: Vec<Value> = report
        .by_task
        .iter()
        .map(|r| {
            json!({
                "task_id": r.key,
                "title": r.label,
                "minutes": r.minutes,
                "percent": round1(r.percent),
            })
        })
        .collect();
    let by_tag: Vec<Value> = report
        .by_tag
        .iter()
        .map(|r| json!({ "tag": r.label, "minutes": r.minutes, "percent": round1(r.percent) }))
        .collect();
    let tracking = report.tracking.as_ref().map(|t| {
        json!({
            "task_id": t.task_id.0.to_string(),
            "title": t.title,
            "elapsed_seconds": t.elapsed_seconds,
        })
    });

    let mut value = json!({
        "window": {
            "from": report.window.from.to_string(),
            "to": report.window.to.to_string(),
            "label": report.window.label,
        },
        "project": report.project_filter,
        "total_minutes": report.total_minutes,
        "entry_count": report.entry_count,
        "by_project": by_project,
        "by_day": by_day,
        "by_task": by_task,
        "by_tag": by_tag,
        "tracking": tracking,
    });
    if let Some(week) = report.week_minutes {
        value["this_week_minutes"] = json!(week);
    }
    Ok(serde_json::to_string_pretty(&value)?)
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, Duration, NaiveDate};

    use taskflow::app::Model;
    use taskflow::domain::{today, Project, ProjectId, Task, TaskId, TimeEntry};

    use super::{
        bar, compute_time_report, render_time_json, render_time_report, resolve_window, show_time,
        TimeReport,
    };
    use crate::cli::{Breakdown, TimeArgs};
    use crate::commands_tests::support::{
        add_project, add_task, add_time_entry, data_file, local_noon, seed_model, temp_dir,
        yaml_cli,
    };

    /// Ids the fixture hands back so tests can refer to its rows.
    struct Ids {
        backend: ProjectId,
        api: TaskId,
        guide: TaskId,
    }

    /// Insert a closed entry of `minutes` starting at local noon on `date`.
    fn closed_entry(model: &mut Model, task: TaskId, date: NaiveDate, minutes: i64) {
        let mut entry = TimeEntry::start(task);
        entry.started_at = local_noon(date);
        entry.ended_at = Some(entry.started_at + Duration::minutes(minutes));
        model.time_entries.insert(entry.id, entry);
    }

    /// Three projects, three tasks and four closed entries:
    /// today API 90m + Guide 30m, yesterday Loose 60m, ten days ago API 600m.
    fn fixture() -> (Model, Ids) {
        let today = today();
        let mut model = Model::new();
        let backend = Project::new("Backend");
        let infra = Project::new("Backend Infra");
        let docs = Project::new("Docs");
        let ids = Ids {
            backend: backend.id,
            api: TaskId::new(),
            guide: TaskId::new(),
        };
        let docs_id = docs.id;
        for p in [backend, infra, docs] {
            model.projects.insert(p.id, p);
        }

        let mut api = Task::new("API")
            .with_project(ids.backend)
            .with_tags(vec!["rust".into(), "api".into()]);
        api.id = ids.api;
        let mut guide = Task::new("Guide")
            .with_project(docs_id)
            .with_tags(vec!["docs".into()]);
        guide.id = ids.guide;
        let loose = Task::new("Loose");
        let loose_id = loose.id;
        for t in [api, guide, loose] {
            model.tasks.insert(t.id, t);
        }

        closed_entry(&mut model, ids.api, today, 90);
        closed_entry(&mut model, ids.guide, today, 30);
        closed_entry(&mut model, loose_id, today - Duration::days(1), 60);
        closed_entry(&mut model, ids.api, today - Duration::days(10), 600);
        (model, ids)
    }

    fn report(model: &Model, args: &TimeArgs) -> TimeReport {
        let today = today();
        compute_time_report(model, args, today, local_noon(today)).expect("report computes")
    }

    fn labels(rows: &[super::TimeRow]) -> Vec<(&str, u32)> {
        rows.iter().map(|r| (r.label.as_str(), r.minutes)).collect()
    }

    // ------------------------------------------------------------------
    // resolve_window
    // ------------------------------------------------------------------

    #[test]
    fn window_defaults_to_today_with_week_total() {
        let today = today();
        let w = resolve_window(&TimeArgs::default(), today).unwrap();
        assert_eq!((w.from, w.to), (today, today));
        assert_eq!(w.label, "Today");
        assert!(w.with_week_total);

        let explicit = TimeArgs {
            today: true,
            ..TimeArgs::default()
        };
        assert_eq!(resolve_window(&explicit, today).unwrap(), w);
    }

    #[test]
    fn window_week_starts_on_monday() {
        let today = today();
        let args = TimeArgs {
            week: true,
            ..TimeArgs::default()
        };
        let w = resolve_window(&args, today).unwrap();
        assert_eq!(w.from.weekday(), chrono::Weekday::Mon);
        assert!(w.from <= today && today - w.from < Duration::days(7));
        assert_eq!(w.to, today);
        assert_eq!(w.label, "This week");
        assert!(!w.with_week_total);
    }

    #[test]
    fn window_days_counts_today_as_the_last_day() {
        let today = today();
        let args = TimeArgs {
            days: Some(7),
            ..TimeArgs::default()
        };
        let w = resolve_window(&args, today).unwrap();
        assert_eq!(w.from, today - Duration::days(6));
        assert_eq!(w.to, today);
        assert_eq!(w.label, "Last 7 days");

        let one = TimeArgs {
            days: Some(1),
            ..TimeArgs::default()
        };
        assert_eq!(resolve_window(&one, today).unwrap().label, "Today");

        let zero = TimeArgs {
            days: Some(0),
            ..TimeArgs::default()
        };
        let err = resolve_window(&zero, today).unwrap_err().to_string();
        assert!(err.contains("--days"), "{err}");
    }

    #[test]
    fn window_from_to_is_explicit_and_validated() {
        let today = today();
        let from = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 1, 9).unwrap();
        let args = TimeArgs {
            from: Some(from),
            to: Some(to),
            ..TimeArgs::default()
        };
        let w = resolve_window(&args, today).unwrap();
        assert_eq!((w.from, w.to), (from, to));
        assert_eq!(w.label, "2026-01-05 to 2026-01-09");

        let open_ended = TimeArgs {
            from: Some(from),
            ..TimeArgs::default()
        };
        assert_eq!(resolve_window(&open_ended, today).unwrap().to, today);

        let reversed = TimeArgs {
            from: Some(to),
            to: Some(from),
            ..TimeArgs::default()
        };
        let err = resolve_window(&reversed, today).unwrap_err().to_string();
        assert!(err.contains("before"), "{err}");
    }

    // ------------------------------------------------------------------
    // compute_time_report
    // ------------------------------------------------------------------

    #[test]
    fn today_report_breaks_down_every_axis() {
        let (model, _) = fixture();
        let today = today();
        let r = report(&model, &TimeArgs::default());

        assert_eq!(r.total_minutes, 120);
        assert_eq!(r.entry_count, 2);
        assert!(r.project_filter.is_none());
        assert!(r.tracking.is_none());

        assert_eq!(labels(&r.by_project), vec![("Backend", 90), ("Docs", 30)]);
        assert!((r.by_project[0].percent - 75.0).abs() < f64::EPSILON);
        assert!((r.by_project[1].percent - 25.0).abs() < f64::EPSILON);
        assert_eq!(labels(&r.by_task), vec![("API", 90), ("Guide", 30)]);
        assert_eq!(
            labels(&r.by_tag),
            vec![("api", 90), ("rust", 90), ("docs", 30)]
        );
        assert_eq!(r.by_day.len(), 1);
        assert_eq!(r.by_day[0].minutes, 120);
        assert_eq!(r.by_day[0].key.as_deref(), Some(today.to_string().as_str()));
        assert_eq!(r.by_day[0].label, format!("{today} {}", today.weekday()));

        // Yesterday's hour belongs to this week unless today is Monday.
        let yesterday_in_week = today.weekday() != chrono::Weekday::Mon;
        let expected_week = if yesterday_in_week { 180 } else { 120 };
        assert_eq!(r.week_minutes, Some(expected_week));
    }

    #[test]
    fn days_window_zero_fills_the_daily_series() {
        let (model, _) = fixture();
        let args = TimeArgs {
            days: Some(3),
            ..TimeArgs::default()
        };
        let r = report(&model, &args);

        assert_eq!(r.total_minutes, 180);
        assert!(r.week_minutes.is_none());
        let minutes: Vec<u32> = r.by_day.iter().map(|d| d.minutes).collect();
        assert_eq!(minutes, vec![0, 60, 120]);
        assert_eq!(
            labels(&r.by_project),
            vec![("Backend", 90), ("(no project)", 60), ("Docs", 30)]
        );
        assert!(r.by_project[1].key.is_none(), "no-project row has no id");
    }

    #[test]
    fn custom_window_reaches_old_entries() {
        let (model, _) = fixture();
        let day = today() - Duration::days(10);
        let args = TimeArgs {
            from: Some(day),
            to: Some(day),
            ..TimeArgs::default()
        };
        let r = report(&model, &args);
        assert_eq!(r.total_minutes, 600);
        assert_eq!(labels(&r.by_project), vec![("Backend", 600)]);
        assert!((r.by_project[0].percent - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn project_filter_is_case_insensitive_and_exact_wins() {
        let (model, ids) = fixture();
        let args = TimeArgs {
            project: Some("backend".into()),
            ..TimeArgs::default()
        };
        let r = report(&model, &args);
        assert_eq!(r.project_filter.as_deref(), Some("Backend"));
        assert_eq!(r.total_minutes, 90);
        assert_eq!(r.entry_count, 1);
        assert_eq!(labels(&r.by_project), vec![("Backend", 90)]);
        assert_eq!(
            r.by_project[0].key.as_deref(),
            Some(ids.backend.0.to_string().as_str())
        );
        assert_eq!(labels(&r.by_task), vec![("API", 90)]);
        assert_eq!(labels(&r.by_tag), vec![("api", 90), ("rust", 90)]);
    }

    #[test]
    fn project_filter_accepts_a_unique_substring() {
        let (model, _) = fixture();
        let args = TimeArgs {
            project: Some("infra".into()),
            ..TimeArgs::default()
        };
        let r = report(&model, &args);
        assert_eq!(r.project_filter.as_deref(), Some("Backend Infra"));
        assert_eq!(r.total_minutes, 0);
        assert!(r.by_project.is_empty());
    }

    #[test]
    fn project_filter_rejects_unknown_and_ambiguous_names() {
        let (model, _) = fixture();
        let today = today();
        let now = local_noon(today);

        let unknown = TimeArgs {
            project: Some("frontend".into()),
            ..TimeArgs::default()
        };
        let err = compute_time_report(&model, &unknown, today, now)
            .unwrap_err()
            .to_string();
        assert_eq!(err, "No project matches 'frontend'");

        let ambiguous = TimeArgs {
            project: Some("back".into()),
            ..TimeArgs::default()
        };
        let err = compute_time_report(&model, &ambiguous, today, now)
            .unwrap_err()
            .to_string();
        assert!(err.contains("Backend, Backend Infra"), "{err}");
    }

    #[test]
    fn running_timer_is_reported_with_elapsed_seconds() {
        let (mut model, ids) = fixture();
        let today = today();
        // Started outside every window used here, so the analytics never
        // count it with the wall clock; only `now` drives the elapsed time.
        let mut running = TimeEntry::start(ids.guide);
        running.started_at = local_noon(today - Duration::days(30));
        let now = running.started_at + Duration::seconds(754);
        model.active_time_entry = Some(running.id);
        model.time_entries.insert(running.id, running);

        let r = compute_time_report(&model, &TimeArgs::default(), today, now).unwrap();
        let tracking = r.tracking.expect("timer is running");
        assert_eq!(tracking.task_id, ids.guide);
        assert_eq!(tracking.title, "Guide");
        assert_eq!(tracking.elapsed_seconds, 754);
        assert_eq!(
            r.total_minutes, 120,
            "the running timer is outside the window"
        );

        // The timer is still reported when the window is filtered to another project.
        let filtered = TimeArgs {
            project: Some("Backend".into()),
            ..TimeArgs::default()
        };
        let r = compute_time_report(&model, &filtered, today, now).unwrap();
        assert!(r.tracking.is_some());
    }

    #[test]
    fn deleted_task_time_is_kept_under_a_placeholder() {
        let (mut model, ids) = fixture();
        model.tasks.remove(&ids.guide);
        let r = report(&model, &TimeArgs::default());
        assert_eq!(r.total_minutes, 120);
        assert_eq!(
            labels(&r.by_task),
            vec![("API", 90), ("(deleted task)", 30)]
        );
        assert_eq!(
            labels(&r.by_project),
            vec![("Backend", 90), ("(no project)", 30)]
        );
    }

    // ------------------------------------------------------------------
    // Rendering
    // ------------------------------------------------------------------

    #[test]
    fn bar_is_proportional_with_eighth_blocks() {
        assert_eq!(bar(0, 100), "");
        assert_eq!(bar(5, 0), "");
        assert_eq!(bar(100, 100), "█".repeat(20));
        assert_eq!(bar(50, 100), "█".repeat(10));
        // 30/90 of 160 eighths = 53 -> 6 full cells and 5 eighths.
        assert_eq!(bar(30, 90), format!("{}▋", "█".repeat(6)));
        assert!(bar(99, 100).chars().count() <= 20);
    }

    #[test]
    fn render_text_today_report() {
        let (model, _) = fixture();
        let today = today();
        let r = report(&model, &TimeArgs::default());
        let out = render_time_report(&r, Breakdown::Project);
        let lines: Vec<_> = out.lines().collect();

        assert_eq!(lines[0], format!("Time Tracked: Today ({today})"));
        assert_eq!(lines[1], "=".repeat(40));
        assert_eq!(lines[2], "  Total:        2h (2 entries)");
        assert!(lines[3].starts_with("  This week:    "));
        assert_eq!(lines[4], "");
        assert_eq!(lines[5], "By project");
        assert_eq!(lines[6], "-".repeat(40));
        assert_eq!(
            lines[7],
            format!("  Backend   {}    1h 30m   75.0%", "█".repeat(20))
        );
        // Label padded to the longest label (8), bar to 20 cells, time to 8.
        assert_eq!(
            lines[8],
            format!(
                "  Docs      {:<20}  {:>8}   25.0%",
                format!("{}▋", "█".repeat(6)),
                "30m"
            )
        );
        assert!(!out.contains("Tracking now"));
        assert!(!out.contains("Project:"));
    }

    #[test]
    fn render_text_other_breakdowns_and_tracking_line() {
        let (mut model, ids) = fixture();
        let today = today();
        let mut running = TimeEntry::start(ids.api);
        running.started_at = local_noon(today - Duration::days(30));
        let now = running.started_at + Duration::seconds(3661);
        model.active_time_entry = Some(running.id);
        model.time_entries.insert(running.id, running);

        let args = TimeArgs {
            days: Some(3),
            project: Some("Backend".into()),
            ..TimeArgs::default()
        };
        let r = compute_time_report(&model, &args, today, now).unwrap();

        let out = render_time_report(&r, Breakdown::Day);
        assert!(out.starts_with(&format!(
            "Time Tracked: Last 3 days ({} to {today})\n",
            today - Duration::days(2)
        )));
        assert!(out.contains("  Project:      Backend\n"));
        assert!(out.contains("  Total:        1h 30m (1 entry)\n"));
        assert!(!out.contains("This week"));
        assert!(out.contains("  Tracking now: API 01:01:01\n"));
        assert!(out.contains("\nBy day\n"));
        let day_rows: Vec<_> = out
            .lines()
            .filter(|l| l.contains(&today.to_string()) && l.ends_with('%'))
            .collect();
        assert_eq!(day_rows.len(), 1);
        assert!(
            day_rows[0].ends_with("    1h 30m  100.0%"),
            "{}",
            day_rows[0]
        );

        let out = render_time_report(&r, Breakdown::Task);
        assert!(out.contains("\nBy task\n"));
        assert!(out.contains("  API       "));

        let out = render_time_report(&r, Breakdown::Tag);
        assert!(out.contains("\nBy tag\n"));
        assert!(out.contains("  api       "));
        assert!(out.contains("  rust      "));
    }

    #[test]
    fn render_text_custom_window_title_and_empty_table() {
        let (model, _) = fixture();
        let from = today() - Duration::days(6);
        let to = today() - Duration::days(4);
        let args = TimeArgs {
            from: Some(from),
            to: Some(to),
            ..TimeArgs::default()
        };
        let r = report(&model, &args);
        let out = render_time_report(&r, Breakdown::Day);
        assert!(out.starts_with(&format!("Time Tracked: {from} to {to}\n")));
        assert!(out.contains("  Total:        0m (0 entries)\n"));
        assert!(out.contains("  No time tracked in this window.\n"));
        // Zero-filled days are not printed as a table of empty bars.
        assert!(!out.contains('%'));
    }

    #[test]
    fn long_labels_are_clipped() {
        let (mut model, ids) = fixture();
        let long = "x".repeat(50);
        model.tasks.get_mut(&ids.api).unwrap().title = long.clone();
        let r = report(&model, &TimeArgs::default());
        let out = render_time_report(&r, Breakdown::Task);
        assert!(out.contains(&format!("  {}…  ", "x".repeat(31))));
        assert!(!out.contains(&long));
    }

    #[test]
    fn render_json_has_the_documented_shape() {
        let (mut model, ids) = fixture();
        let today = today();
        let mut running = TimeEntry::start(ids.api);
        running.started_at = local_noon(today - Duration::days(30));
        let now = running.started_at + Duration::seconds(42);
        model.active_time_entry = Some(running.id);
        model.time_entries.insert(running.id, running);

        let args = TimeArgs {
            days: Some(3),
            ..TimeArgs::default()
        };
        let r = compute_time_report(&model, &args, today, now).unwrap();
        let text = render_time_json(&r).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");

        assert_eq!(v["window"]["from"], (today - Duration::days(2)).to_string());
        assert_eq!(v["window"]["to"], today.to_string());
        assert_eq!(v["total_minutes"], 180);
        assert_eq!(v["entry_count"], 3);
        assert!(v["project"].is_null());
        assert!(v.get("this_week_minutes").is_none());

        let projects = v["by_project"].as_array().unwrap();
        assert_eq!(projects.len(), 3);
        assert_eq!(projects[0]["project"], "Backend");
        assert_eq!(projects[0]["project_id"], ids.backend.0.to_string());
        assert_eq!(projects[0]["minutes"], 90);
        assert_eq!(projects[0]["percent"], 50.0);
        assert!(projects[1]["project"].is_null());
        assert!(projects[1]["project_id"].is_null());
        assert_eq!(projects[1]["minutes"], 60);
        assert_eq!(projects[2]["percent"], 16.7);

        let days = v["by_day"].as_array().unwrap();
        assert_eq!(days.len(), 3);
        assert_eq!(days[2]["date"], today.to_string());
        assert_eq!(days[2]["minutes"], 120);

        let tasks = v["by_task"].as_array().unwrap();
        assert_eq!(tasks[0]["task_id"], ids.api.0.to_string());
        assert_eq!(tasks[0]["title"], "API");

        let tags = v["by_tag"].as_array().unwrap();
        assert_eq!(tags[0]["tag"], "api");
        assert_eq!(tags[0]["minutes"], 90);

        assert_eq!(v["tracking"]["task_id"], ids.api.0.to_string());
        assert_eq!(v["tracking"]["title"], "API");
        assert_eq!(v["tracking"]["elapsed_seconds"], 42);
    }

    #[test]
    fn render_json_default_mode_adds_week_total_and_null_tracking() {
        let (model, _) = fixture();
        let r = report(&model, &TimeArgs::default());
        let v: serde_json::Value = serde_json::from_str(&render_time_json(&r).unwrap()).unwrap();
        assert!(v["tracking"].is_null());
        assert!(v["this_week_minutes"].is_number());
        assert_eq!(v["window"]["label"], "Today");
    }

    // ------------------------------------------------------------------
    // End to end against a YAML store
    // ------------------------------------------------------------------

    #[test]
    fn show_time_runs_against_the_data_file() {
        let dir = temp_dir();
        let data = data_file(dir.path());
        let today = today();
        seed_model(&data, |model| {
            let project = add_project(model, Project::new("Backend"));
            let task = add_task(model, Task::new("API").with_project(project));
            let mut entry = TimeEntry::start(task);
            entry.started_at = local_noon(today);
            entry.ended_at = Some(entry.started_at + Duration::minutes(45));
            add_time_entry(model, entry);
        });
        let cli = yaml_cli(&data);

        show_time(&cli, &TimeArgs::default()).expect("text report");
        let json = TimeArgs {
            week: true,
            by: Breakdown::Day,
            json: true,
            ..TimeArgs::default()
        };
        show_time(&cli, &json).expect("json report");
        let filtered = TimeArgs {
            project: Some("backend".into()),
            ..TimeArgs::default()
        };
        show_time(&cli, &filtered).expect("filtered report");

        let unknown = TimeArgs {
            project: Some("nope".into()),
            ..TimeArgs::default()
        };
        let err = show_time(&cli, &unknown).unwrap_err().to_string();
        assert_eq!(err, "No project matches 'nope'");
    }

    #[test]
    fn show_time_reports_an_unreadable_store() {
        let dir = temp_dir();
        let err = show_time(&yaml_cli(dir.path()), &TimeArgs::default())
            .unwrap_err()
            .to_string();
        assert!(err.starts_with("Could not load data from "), "{err}");
    }
}
