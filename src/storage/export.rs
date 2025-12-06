use std::collections::HashMap;
use std::io::Write;

use crate::domain::analytics::AnalyticsReport;
use crate::domain::{Priority, Task, TaskId, TaskStatus};

/// Export format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Ics,
    Dot,
    Mermaid,
}

impl ExportFormat {
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "ics" | "ical" | "icalendar" => Some(Self::Ics),
            "dot" | "graphviz" => Some(Self::Dot),
            "mermaid" | "md" => Some(Self::Mermaid),
            _ => None,
        }
    }

    #[must_use]
    pub const fn file_extension(&self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Ics => "ics",
            Self::Dot => "dot",
            Self::Mermaid => "md",
        }
    }
}

/// Exports tasks to CSV format.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if writing fails.
pub fn export_to_csv<W: Write>(tasks: &[Task], writer: &mut W) -> std::io::Result<()> {
    // Write header
    writeln!(
        writer,
        "ID,Title,Status,Priority,Due Date,Tags,Project ID,Description,Created,Completed"
    )?;

    for task in tasks {
        let id = task.id.0.to_string();
        let title = escape_csv(&task.title);
        let status = task.status.as_str();
        let priority = task.priority.as_str();
        let due_date = task.due_date.map(|d| d.to_string()).unwrap_or_default();
        let tags = task.tags.join(";");
        let project_id = task
            .project_id
            .as_ref()
            .map(|p| p.0.to_string())
            .unwrap_or_default();
        let description = task
            .description
            .as_ref()
            .map(|d| escape_csv(d))
            .unwrap_or_default();
        let created = task.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
        let completed = task
            .completed_at
            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();

        writeln!(
            writer,
            "{id},{title},{status},{priority},{due_date},{tags},{project_id},{description},{created},{completed}"
        )?;
    }

    Ok(())
}

/// Escape a string for CSV (wrap in quotes if needed, escape internal quotes)
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Exports tasks to ICS (iCalendar) format.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if writing fails.
pub fn export_to_ics<W: Write>(tasks: &[Task], writer: &mut W) -> std::io::Result<()> {
    // Write calendar header
    writeln!(writer, "BEGIN:VCALENDAR")?;
    writeln!(writer, "VERSION:2.0")?;
    writeln!(writer, "PRODID:-//TaskFlow//TaskFlow TUI//EN")?;
    writeln!(writer, "CALSCALE:GREGORIAN")?;
    writeln!(writer, "METHOD:PUBLISH")?;

    for task in tasks {
        // Export as VTODO (task) component
        writeln!(writer, "BEGIN:VTODO")?;

        // UID (unique identifier)
        let uid = task.id.0;
        writeln!(writer, "UID:{uid}")?;

        // DTSTAMP (timestamp)
        let dtstamp = task.created_at.format("%Y%m%dT%H%M%SZ");
        writeln!(writer, "DTSTAMP:{dtstamp}")?;

        // CREATED
        writeln!(writer, "CREATED:{dtstamp}")?;

        // LAST-MODIFIED
        let last_modified = task.updated_at.format("%Y%m%dT%H%M%SZ");
        writeln!(writer, "LAST-MODIFIED:{last_modified}")?;

        // SUMMARY (title)
        writeln!(writer, "SUMMARY:{}", escape_ics(&task.title))?;

        // DESCRIPTION
        if let Some(ref desc) = task.description {
            writeln!(writer, "DESCRIPTION:{}", escape_ics(desc))?;
        }

        // DUE date
        if let Some(due) = task.due_date {
            writeln!(writer, "DUE;VALUE=DATE:{}", due.format("%Y%m%d"))?;
        }

        // STATUS
        let ics_status = match task.status {
            TaskStatus::Todo => "NEEDS-ACTION",
            TaskStatus::InProgress => "IN-PROCESS",
            TaskStatus::Blocked => "NEEDS-ACTION",
            TaskStatus::Done => "COMPLETED",
            TaskStatus::Cancelled => "CANCELLED",
        };
        writeln!(writer, "STATUS:{ics_status}")?;

        // PRIORITY (1-9 in ICS, 1 is highest)
        let ics_priority = match task.priority {
            Priority::Urgent => 1,
            Priority::High => 3,
            Priority::Medium => 5,
            Priority::Low => 7,
            Priority::None => 9,
        };
        writeln!(writer, "PRIORITY:{ics_priority}")?;

        // COMPLETED timestamp
        if let Some(completed) = task.completed_at {
            writeln!(writer, "COMPLETED:{}", completed.format("%Y%m%dT%H%M%SZ"))?;
        }

        // PERCENT-COMPLETE
        let percent = match task.status {
            TaskStatus::Todo => 0,
            TaskStatus::InProgress => 50,
            TaskStatus::Blocked => 25,
            TaskStatus::Done => 100,
            TaskStatus::Cancelled => 100,
        };
        writeln!(writer, "PERCENT-COMPLETE:{percent}")?;

        // CATEGORIES (tags)
        if !task.tags.is_empty() {
            writeln!(writer, "CATEGORIES:{}", task.tags.join(","))?;
        }

        writeln!(writer, "END:VTODO")?;
    }

    writeln!(writer, "END:VCALENDAR")?;
    Ok(())
}

/// Escape special characters for ICS format
fn escape_ics(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

/// Exports task chains/dependencies to DOT (Graphviz) format.
///
/// This creates a directed graph showing:
/// - Task nodes with color-coded status (green=done, yellow=in progress, red=blocked)
/// - Chain edges (blue) from `next_task_id` relationships
/// - Dependency edges (red dashed) from `dependencies` relationships
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if writing fails.
pub fn export_to_dot<W: Write>(
    tasks: &HashMap<TaskId, Task>,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(writer, "digraph TaskChains {{")?;
    writeln!(writer, "    rankdir=LR;")?;
    writeln!(
        writer,
        "    node [shape=box, style=filled, fontname=\"Arial\"];"
    )?;
    writeln!(writer)?;

    // Write all nodes
    writeln!(writer, "    // Nodes")?;
    for task in tasks.values() {
        let node_id = format!("task_{}", task.id.0.to_string().replace('-', "_"));
        let label = escape_dot(&task.title);
        let fill_color = match task.status {
            TaskStatus::Done => "\"#90EE90\"",       // Light green
            TaskStatus::Cancelled => "\"#D3D3D3\"",  // Light gray
            TaskStatus::InProgress => "\"#FFD700\"", // Gold
            TaskStatus::Blocked => "\"#FFB6C1\"",    // Light pink
            TaskStatus::Todo => "\"#FFFFFF\"",       // White
        };
        writeln!(
            writer,
            "    {node_id} [label=\"{label}\" fillcolor={fill_color}];"
        )?;
    }

    writeln!(writer)?;

    // Write chain edges (next_task_id)
    writeln!(writer, "    // Chain edges (next_task_id)")?;
    for task in tasks.values() {
        if let Some(ref next_id) = task.next_task_id {
            if tasks.contains_key(next_id) {
                let from_id = format!("task_{}", task.id.0.to_string().replace('-', "_"));
                let to_id = format!("task_{}", next_id.0.to_string().replace('-', "_"));
                writeln!(
                    writer,
                    "    {from_id} -> {to_id} [color=\"blue\" label=\"chain\"];"
                )?;
            }
        }
    }

    writeln!(writer)?;

    // Write dependency edges
    writeln!(writer, "    // Dependency edges (blocks)")?;
    for task in tasks.values() {
        for dep_id in &task.dependencies {
            if tasks.contains_key(dep_id) {
                let from_id = format!("task_{}", dep_id.0.to_string().replace('-', "_"));
                let to_id = format!("task_{}", task.id.0.to_string().replace('-', "_"));
                writeln!(
                    writer,
                    "    {from_id} -> {to_id} [color=\"red\" style=\"dashed\" label=\"blocks\"];"
                )?;
            }
        }
    }

    writeln!(writer, "}}")?;
    Ok(())
}

/// Escape special characters for DOT format
fn escape_dot(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Exports task chains/dependencies to Mermaid format.
///
/// This creates a flowchart showing:
/// - Task nodes with styled classes for status
/// - Chain edges (solid arrows) from `next_task_id` relationships
/// - Dependency edges (dotted arrows with "blocks" label) from `dependencies` relationships
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if writing fails.
pub fn export_to_mermaid<W: Write>(
    tasks: &HashMap<TaskId, Task>,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(writer, "```mermaid")?;
    writeln!(writer, "flowchart LR")?;
    writeln!(writer)?;

    // Define style classes
    writeln!(writer, "    %% Style definitions")?;
    writeln!(writer, "    classDef done fill:#90EE90,stroke:#228B22")?;
    writeln!(writer, "    classDef cancelled fill:#D3D3D3,stroke:#696969")?;
    writeln!(
        writer,
        "    classDef inprogress fill:#FFD700,stroke:#DAA520"
    )?;
    writeln!(writer, "    classDef blocked fill:#FFB6C1,stroke:#DC143C")?;
    writeln!(writer, "    classDef todo fill:#FFFFFF,stroke:#333333")?;
    writeln!(writer)?;

    // Write nodes with shorter IDs for readability
    let id_map: HashMap<TaskId, String> = tasks
        .keys()
        .enumerate()
        .map(|(i, id)| (id.clone(), format!("T{i}")))
        .collect();

    writeln!(writer, "    %% Nodes")?;
    for task in tasks.values() {
        let short_id = id_map.get(&task.id).unwrap();
        let label = escape_mermaid(&task.title);
        let class = match task.status {
            TaskStatus::Done => "done",
            TaskStatus::Cancelled => "cancelled",
            TaskStatus::InProgress => "inprogress",
            TaskStatus::Blocked => "blocked",
            TaskStatus::Todo => "todo",
        };
        writeln!(writer, "    {short_id}[\"{label}\"]:::{class}")?;
    }

    writeln!(writer)?;

    // Write chain edges (next_task_id)
    writeln!(writer, "    %% Chain edges (next task in sequence)")?;
    for task in tasks.values() {
        if let Some(ref next_id) = task.next_task_id {
            if let (Some(from), Some(to)) = (id_map.get(&task.id), id_map.get(next_id)) {
                writeln!(writer, "    {from} --> {to}")?;
            }
        }
    }

    writeln!(writer)?;

    // Write dependency edges
    writeln!(writer, "    %% Dependency edges (blocks)")?;
    for task in tasks.values() {
        for dep_id in &task.dependencies {
            if let (Some(from), Some(to)) = (id_map.get(dep_id), id_map.get(&task.id)) {
                writeln!(writer, "    {from} -.->|blocks| {to}")?;
            }
        }
    }

    writeln!(writer, "```")?;
    Ok(())
}

/// Escape special characters for Mermaid format
fn escape_mermaid(s: &str) -> String {
    s.replace('"', "'")
        .replace('[', "(")
        .replace(']', ")")
        .replace('\n', " ")
}

/// Exports tasks to a string in the specified format.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if formatting fails.
pub fn export_to_string(tasks: &[Task], format: ExportFormat) -> std::io::Result<String> {
    let mut buffer = Vec::new();
    match format {
        ExportFormat::Csv => export_to_csv(tasks, &mut buffer)?,
        ExportFormat::Ics => export_to_ics(tasks, &mut buffer)?,
        ExportFormat::Dot | ExportFormat::Mermaid => {
            // These formats need the full task map for chain/dependency lookups
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Use export_chains_to_string for DOT/Mermaid formats",
            ));
        }
    }
    String::from_utf8(buffer).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Exports task chains to a string in DOT or Mermaid format.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if formatting fails.
pub fn export_chains_to_string(
    tasks: &HashMap<TaskId, Task>,
    format: ExportFormat,
) -> std::io::Result<String> {
    let mut buffer = Vec::new();
    match format {
        ExportFormat::Dot => export_to_dot(tasks, &mut buffer)?,
        ExportFormat::Mermaid => export_to_mermaid(tasks, &mut buffer)?,
        ExportFormat::Csv | ExportFormat::Ics => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Use export_to_string for CSV/ICS formats",
            ));
        }
    }
    String::from_utf8(buffer).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Exports an analytics report to Markdown format.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if writing fails.
pub fn export_report_to_markdown<W: Write>(
    report: &AnalyticsReport,
    writer: &mut W,
) -> std::io::Result<()> {
    // Title and date range
    writeln!(writer, "# TaskFlow Analytics Report")?;
    writeln!(writer)?;
    writeln!(
        writer,
        "**Period:** {} to {}",
        report.config.start_date, report.config.end_date
    )?;
    writeln!(writer)?;

    // Overview section
    writeln!(writer, "## Overview")?;
    writeln!(writer)?;
    writeln!(writer, "| Metric | Value |")?;
    writeln!(writer, "|--------|-------|")?;
    writeln!(
        writer,
        "| Total Tasks | {} |",
        report.status_breakdown.total()
    )?;
    writeln!(
        writer,
        "| Completion Rate | {:.1}% |",
        report.status_breakdown.completion_rate() * 100.0
    )?;
    writeln!(
        writer,
        "| Tasks Completed | {} |",
        report.completion_trend.total_completed()
    )?;
    writeln!(
        writer,
        "| Tasks Created | {} |",
        report.completion_trend.total_created()
    )?;
    writeln!(
        writer,
        "| Avg Tasks/Day | {:.1} |",
        report.insights.avg_tasks_per_day
    )?;
    writeln!(writer)?;

    // Status breakdown
    writeln!(writer, "## Status Breakdown")?;
    writeln!(writer)?;
    writeln!(writer, "| Status | Count |")?;
    writeln!(writer, "|--------|-------|")?;
    writeln!(writer, "| To Do | {} |", report.status_breakdown.todo)?;
    writeln!(
        writer,
        "| In Progress | {} |",
        report.status_breakdown.in_progress
    )?;
    writeln!(writer, "| Blocked | {} |", report.status_breakdown.blocked)?;
    writeln!(writer, "| Done | {} |", report.status_breakdown.done)?;
    writeln!(
        writer,
        "| Cancelled | {} |",
        report.status_breakdown.cancelled
    )?;
    writeln!(writer)?;

    // Priority breakdown
    writeln!(writer, "## Priority Breakdown")?;
    writeln!(writer)?;
    writeln!(writer, "| Priority | Count |")?;
    writeln!(writer, "|----------|-------|")?;
    writeln!(writer, "| Urgent | {} |", report.priority_breakdown.urgent)?;
    writeln!(writer, "| High | {} |", report.priority_breakdown.high)?;
    writeln!(writer, "| Medium | {} |", report.priority_breakdown.medium)?;
    writeln!(writer, "| Low | {} |", report.priority_breakdown.low)?;
    writeln!(writer, "| None | {} |", report.priority_breakdown.none)?;
    writeln!(writer)?;

    // Velocity metrics
    writeln!(writer, "## Velocity")?;
    writeln!(writer)?;
    writeln!(
        writer,
        "- **Average Weekly Velocity:** {:.1} tasks/week",
        report.velocity.avg_weekly
    )?;
    let trend_indicator = if report.velocity.is_improving() {
        "📈 Improving"
    } else if report.velocity.trend < 0.0 {
        "📉 Declining"
    } else {
        "➡️ Stable"
    };
    writeln!(
        writer,
        "- **Trend:** {} ({:+.1})",
        trend_indicator, report.velocity.trend
    )?;
    if let Some((date, count)) = report.velocity.best_week() {
        writeln!(
            writer,
            "- **Best Week:** {} ({} tasks)",
            date.format("%Y-%m-%d"),
            count
        )?;
    }
    writeln!(writer)?;

    // Productivity insights
    writeln!(writer, "## Productivity Insights")?;
    writeln!(writer)?;
    writeln!(
        writer,
        "- **Current Streak:** {} days{}",
        report.insights.current_streak,
        if report.insights.is_best_streak() {
            " 🏆"
        } else {
            ""
        }
    )?;
    writeln!(
        writer,
        "- **Longest Streak:** {} days",
        report.insights.longest_streak
    )?;
    if let Some(day) = report.insights.best_day {
        writeln!(writer, "- **Most Productive Day:** {day}")?;
    }
    if let Some(hour) = report.insights.peak_hour {
        writeln!(writer, "- **Peak Productivity Hour:** {hour}:00")?;
    }
    if report.insights.total_time_tracked > 0 {
        let hours = report.insights.total_time_tracked / 60;
        let mins = report.insights.total_time_tracked % 60;
        writeln!(writer, "- **Total Time Tracked:** {hours}h {mins}m")?;
    }
    writeln!(writer)?;

    // Tag statistics
    if !report.tag_stats.is_empty() {
        writeln!(writer, "## Top Tags")?;
        writeln!(writer)?;
        writeln!(writer, "| Tag | Count | Completed | Rate |")?;
        writeln!(writer, "|-----|-------|-----------|------|")?;
        for stat in report.tag_stats.iter().take(10) {
            writeln!(
                writer,
                "| {} | {} | {} | {:.0}% |",
                stat.tag,
                stat.count,
                stat.completed,
                stat.completion_rate() * 100.0
            )?;
        }
        writeln!(writer)?;
    }

    // Burndown charts summary
    if !report.burn_charts.is_empty() {
        writeln!(writer, "## Project Progress")?;
        writeln!(writer)?;
        writeln!(writer, "| Project | Remaining | Completion |")?;
        writeln!(writer, "|---------|-----------|------------|")?;
        for chart in &report.burn_charts {
            writeln!(
                writer,
                "| {} | {:.0} | {:.1}% |",
                chart.project_name,
                chart.remaining_work(),
                chart.completion_percentage()
            )?;
        }
        writeln!(writer)?;
    }

    // Footer
    writeln!(writer, "---")?;
    writeln!(writer, "*Generated by TaskFlow*")?;

    Ok(())
}

/// Exports an analytics report to HTML format.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if writing fails.
pub fn export_report_to_html<W: Write>(
    report: &AnalyticsReport,
    writer: &mut W,
) -> std::io::Result<()> {
    // HTML header with embedded styles
    writeln!(writer, "<!DOCTYPE html>")?;
    writeln!(writer, "<html lang=\"en\">")?;
    writeln!(writer, "<head>")?;
    writeln!(writer, "  <meta charset=\"UTF-8\">")?;
    writeln!(
        writer,
        "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">"
    )?;
    writeln!(writer, "  <title>TaskFlow Analytics Report</title>")?;
    writeln!(writer, "  <style>")?;
    writeln!(
        writer,
        "    body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 900px; margin: 0 auto; padding: 20px; background: #f5f5f5; }}"
    )?;
    writeln!(
        writer,
        "    .card {{ background: white; border-radius: 8px; padding: 20px; margin-bottom: 20px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}"
    )?;
    writeln!(
        writer,
        "    h1 {{ color: #333; border-bottom: 2px solid #4CAF50; padding-bottom: 10px; }}"
    )?;
    writeln!(writer, "    h2 {{ color: #555; margin-top: 0; }}")?;
    writeln!(
        writer,
        "    table {{ width: 100%; border-collapse: collapse; margin: 10px 0; }}"
    )?;
    writeln!(
        writer,
        "    th, td {{ text-align: left; padding: 12px; border-bottom: 1px solid #ddd; }}"
    )?;
    writeln!(
        writer,
        "    th {{ background: #f8f8f8; font-weight: 600; }}"
    )?;
    writeln!(writer, "    tr:hover {{ background: #f5f5f5; }}")?;
    writeln!(
        writer,
        "    .metric {{ display: inline-block; background: #e8f5e9; padding: 8px 16px; border-radius: 20px; margin: 5px; }}"
    )?;
    writeln!(
        writer,
        "    .metric-value {{ font-weight: bold; color: #2e7d32; }}"
    )?;
    writeln!(
        writer,
        "    .trend-up {{ color: #4CAF50; }} .trend-down {{ color: #f44336; }} .trend-stable {{ color: #9e9e9e; }}"
    )?;
    writeln!(
        writer,
        "    .progress-bar {{ background: #e0e0e0; border-radius: 10px; overflow: hidden; height: 20px; }}"
    )?;
    writeln!(
        writer,
        "    .progress-fill {{ background: linear-gradient(90deg, #4CAF50, #8BC34A); height: 100%; transition: width 0.3s; }}"
    )?;
    writeln!(writer, "  </style>")?;
    writeln!(writer, "</head>")?;
    writeln!(writer, "<body>")?;

    // Title
    writeln!(writer, "<h1>📊 TaskFlow Analytics Report</h1>")?;
    writeln!(
        writer,
        "<p><strong>Period:</strong> {} to {}</p>",
        report.config.start_date, report.config.end_date
    )?;

    // Overview card
    writeln!(writer, "<div class=\"card\">")?;
    writeln!(writer, "  <h2>Overview</h2>")?;
    writeln!(
        writer,
        "  <span class=\"metric\">Total Tasks: <span class=\"metric-value\">{}</span></span>",
        report.status_breakdown.total()
    )?;
    writeln!(
        writer,
        "  <span class=\"metric\">Completion Rate: <span class=\"metric-value\">{:.1}%</span></span>",
        report.status_breakdown.completion_rate() * 100.0
    )?;
    writeln!(
        writer,
        "  <span class=\"metric\">Completed: <span class=\"metric-value\">{}</span></span>",
        report.completion_trend.total_completed()
    )?;
    writeln!(
        writer,
        "  <span class=\"metric\">Created: <span class=\"metric-value\">{}</span></span>",
        report.completion_trend.total_created()
    )?;
    writeln!(writer, "</div>")?;

    // Status breakdown card
    writeln!(writer, "<div class=\"card\">")?;
    writeln!(writer, "  <h2>Status Breakdown</h2>")?;
    writeln!(writer, "  <table>")?;
    writeln!(writer, "    <tr><th>Status</th><th>Count</th></tr>")?;
    writeln!(
        writer,
        "    <tr><td>📋 To Do</td><td>{}</td></tr>",
        report.status_breakdown.todo
    )?;
    writeln!(
        writer,
        "    <tr><td>🔄 In Progress</td><td>{}</td></tr>",
        report.status_breakdown.in_progress
    )?;
    writeln!(
        writer,
        "    <tr><td>🚫 Blocked</td><td>{}</td></tr>",
        report.status_breakdown.blocked
    )?;
    writeln!(
        writer,
        "    <tr><td>✅ Done</td><td>{}</td></tr>",
        report.status_breakdown.done
    )?;
    writeln!(
        writer,
        "    <tr><td>❌ Cancelled</td><td>{}</td></tr>",
        report.status_breakdown.cancelled
    )?;
    writeln!(writer, "  </table>")?;
    writeln!(writer, "</div>")?;

    // Priority breakdown card
    writeln!(writer, "<div class=\"card\">")?;
    writeln!(writer, "  <h2>Priority Breakdown</h2>")?;
    writeln!(writer, "  <table>")?;
    writeln!(writer, "    <tr><th>Priority</th><th>Count</th></tr>")?;
    writeln!(
        writer,
        "    <tr><td>🔴 Urgent</td><td>{}</td></tr>",
        report.priority_breakdown.urgent
    )?;
    writeln!(
        writer,
        "    <tr><td>🟠 High</td><td>{}</td></tr>",
        report.priority_breakdown.high
    )?;
    writeln!(
        writer,
        "    <tr><td>🟡 Medium</td><td>{}</td></tr>",
        report.priority_breakdown.medium
    )?;
    writeln!(
        writer,
        "    <tr><td>🟢 Low</td><td>{}</td></tr>",
        report.priority_breakdown.low
    )?;
    writeln!(
        writer,
        "    <tr><td>⚪ None</td><td>{}</td></tr>",
        report.priority_breakdown.none
    )?;
    writeln!(writer, "  </table>")?;
    writeln!(writer, "</div>")?;

    // Velocity card
    writeln!(writer, "<div class=\"card\">")?;
    writeln!(writer, "  <h2>Velocity</h2>")?;
    writeln!(
        writer,
        "  <p><strong>Average Weekly:</strong> {:.1} tasks/week</p>",
        report.velocity.avg_weekly
    )?;
    let (trend_class, trend_icon) = if report.velocity.is_improving() {
        ("trend-up", "📈")
    } else if report.velocity.trend < 0.0 {
        ("trend-down", "📉")
    } else {
        ("trend-stable", "➡️")
    };
    writeln!(
        writer,
        "  <p><strong>Trend:</strong> <span class=\"{}\">{} {:+.1}</span></p>",
        trend_class, trend_icon, report.velocity.trend
    )?;
    if let Some((date, count)) = report.velocity.best_week() {
        writeln!(
            writer,
            "  <p><strong>Best Week:</strong> {} ({} tasks)</p>",
            date.format("%Y-%m-%d"),
            count
        )?;
    }
    writeln!(writer, "</div>")?;

    // Productivity insights card
    writeln!(writer, "<div class=\"card\">")?;
    writeln!(writer, "  <h2>Productivity Insights</h2>")?;
    writeln!(
        writer,
        "  <p><strong>Current Streak:</strong> {} days{}</p>",
        report.insights.current_streak,
        if report.insights.is_best_streak() {
            " 🏆"
        } else {
            ""
        }
    )?;
    writeln!(
        writer,
        "  <p><strong>Longest Streak:</strong> {} days</p>",
        report.insights.longest_streak
    )?;
    if let Some(day) = report.insights.best_day {
        writeln!(
            writer,
            "  <p><strong>Most Productive Day:</strong> {day}</p>"
        )?;
    }
    if let Some(hour) = report.insights.peak_hour {
        writeln!(writer, "  <p><strong>Peak Hour:</strong> {hour}:00</p>")?;
    }
    if report.insights.total_time_tracked > 0 {
        let hours = report.insights.total_time_tracked / 60;
        let mins = report.insights.total_time_tracked % 60;
        writeln!(
            writer,
            "  <p><strong>Total Time Tracked:</strong> {hours}h {mins}m</p>"
        )?;
    }
    writeln!(writer, "</div>")?;

    // Tag statistics card
    if !report.tag_stats.is_empty() {
        writeln!(writer, "<div class=\"card\">")?;
        writeln!(writer, "  <h2>Top Tags</h2>")?;
        writeln!(writer, "  <table>")?;
        writeln!(
            writer,
            "    <tr><th>Tag</th><th>Count</th><th>Completed</th><th>Rate</th></tr>"
        )?;
        for stat in report.tag_stats.iter().take(10) {
            writeln!(
                writer,
                "    <tr><td>#{}</td><td>{}</td><td>{}</td><td>{:.0}%</td></tr>",
                escape_html(&stat.tag),
                stat.count,
                stat.completed,
                stat.completion_rate() * 100.0
            )?;
        }
        writeln!(writer, "  </table>")?;
        writeln!(writer, "</div>")?;
    }

    // Project progress card
    if !report.burn_charts.is_empty() {
        writeln!(writer, "<div class=\"card\">")?;
        writeln!(writer, "  <h2>Project Progress</h2>")?;
        for chart in &report.burn_charts {
            let pct = chart.completion_percentage();
            writeln!(
                writer,
                "  <p><strong>{}</strong> - {:.0} remaining ({:.1}% complete)</p>",
                escape_html(&chart.project_name),
                chart.remaining_work(),
                pct
            )?;
            writeln!(writer, "  <div class=\"progress-bar\">")?;
            writeln!(
                writer,
                "    <div class=\"progress-fill\" style=\"width: {:.1}%\"></div>",
                pct
            )?;
            writeln!(writer, "  </div>")?;
        }
        writeln!(writer, "</div>")?;
    }

    // Footer
    writeln!(
        writer,
        "<footer style=\"text-align: center; color: #888; margin-top: 40px;\">"
    )?;
    writeln!(writer, "  <p>Generated by TaskFlow</p>")?;
    writeln!(writer, "</footer>")?;

    writeln!(writer, "</body>")?;
    writeln!(writer, "</html>")?;

    Ok(())
}

/// Escape special characters for HTML
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Exports an analytics report to a string in Markdown format.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if formatting fails.
pub fn export_report_to_markdown_string(report: &AnalyticsReport) -> std::io::Result<String> {
    let mut buffer = Vec::new();
    export_report_to_markdown(report, &mut buffer)?;
    String::from_utf8(buffer).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Exports an analytics report to a string in HTML format.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if formatting fails.
pub fn export_report_to_html_string(report: &AnalyticsReport) -> std::io::Result<String> {
    let mut buffer = Vec::new();
    export_report_to_html(report, &mut buffer)?;
    String::from_utf8(buffer).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Utc};

    fn create_test_task(title: &str) -> Task {
        Task::new(title)
    }

    #[test]
    fn test_export_format_parse() {
        assert_eq!(ExportFormat::parse("csv"), Some(ExportFormat::Csv));
        assert_eq!(ExportFormat::parse("CSV"), Some(ExportFormat::Csv));
        assert_eq!(ExportFormat::parse("ics"), Some(ExportFormat::Ics));
        assert_eq!(ExportFormat::parse("ical"), Some(ExportFormat::Ics));
        assert_eq!(ExportFormat::parse("unknown"), None);
    }

    #[test]
    fn test_export_format_extension() {
        assert_eq!(ExportFormat::Csv.file_extension(), "csv");
        assert_eq!(ExportFormat::Ics.file_extension(), "ics");
    }

    #[test]
    fn test_escape_csv_simple() {
        assert_eq!(escape_csv("hello"), "hello");
    }

    #[test]
    fn test_escape_csv_with_comma() {
        assert_eq!(escape_csv("hello, world"), "\"hello, world\"");
    }

    #[test]
    fn test_escape_csv_with_quotes() {
        assert_eq!(escape_csv("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn test_escape_csv_with_newline() {
        assert_eq!(escape_csv("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn test_escape_ics() {
        assert_eq!(escape_ics("hello"), "hello");
        assert_eq!(escape_ics("a;b"), "a\\;b");
        assert_eq!(escape_ics("a,b"), "a\\,b");
        assert_eq!(escape_ics("a\nb"), "a\\nb");
    }

    #[test]
    fn test_export_csv_basic() {
        let tasks = vec![create_test_task("Test Task")];
        let result = export_to_string(&tasks, ExportFormat::Csv).unwrap();

        assert!(result.starts_with("ID,Title,Status,Priority"));
        assert!(result.contains("Test Task"));
        assert!(result.contains("todo"));
        assert!(result.contains("none"));
    }

    #[test]
    fn test_export_csv_with_due_date() {
        let mut task = create_test_task("Task with date");
        task.due_date = Some(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());

        let tasks = vec![task];
        let result = export_to_string(&tasks, ExportFormat::Csv).unwrap();

        assert!(result.contains("2025-06-15"));
    }

    #[test]
    fn test_export_csv_with_tags() {
        let mut task = create_test_task("Tagged task");
        task.tags = vec!["rust".to_string(), "tui".to_string()];

        let tasks = vec![task];
        let result = export_to_string(&tasks, ExportFormat::Csv).unwrap();

        assert!(result.contains("rust;tui"));
    }

    #[test]
    fn test_export_ics_basic() {
        let tasks = vec![create_test_task("ICS Test")];
        let result = export_to_string(&tasks, ExportFormat::Ics).unwrap();

        assert!(result.starts_with("BEGIN:VCALENDAR"));
        assert!(result.contains("VERSION:2.0"));
        assert!(result.contains("BEGIN:VTODO"));
        assert!(result.contains("SUMMARY:ICS Test"));
        assert!(result.contains("STATUS:NEEDS-ACTION"));
        assert!(result.contains("END:VTODO"));
        assert!(result.ends_with("END:VCALENDAR\n"));
    }

    #[test]
    fn test_export_ics_with_priority() {
        let task = create_test_task("Urgent task").with_priority(Priority::Urgent);
        let tasks = vec![task];
        let result = export_to_string(&tasks, ExportFormat::Ics).unwrap();

        assert!(result.contains("PRIORITY:1"));
    }

    #[test]
    fn test_export_ics_completed() {
        let mut task = create_test_task("Completed task");
        task.status = TaskStatus::Done;
        task.completed_at = Some(Utc::now());

        let tasks = vec![task];
        let result = export_to_string(&tasks, ExportFormat::Ics).unwrap();

        assert!(result.contains("STATUS:COMPLETED"));
        assert!(result.contains("PERCENT-COMPLETE:100"));
        assert!(result.contains("COMPLETED:"));
    }

    #[test]
    fn test_export_ics_with_due_date() {
        let mut task = create_test_task("Due task");
        task.due_date = Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap());

        let tasks = vec![task];
        let result = export_to_string(&tasks, ExportFormat::Ics).unwrap();

        assert!(result.contains("DUE;VALUE=DATE:20251225"));
    }

    #[test]
    fn test_export_ics_with_tags() {
        let mut task = create_test_task("Tagged task");
        task.tags = vec!["work".to_string(), "urgent".to_string()];

        let tasks = vec![task];
        let result = export_to_string(&tasks, ExportFormat::Ics).unwrap();

        assert!(result.contains("CATEGORIES:work,urgent"));
    }

    #[test]
    fn test_export_empty_tasks() {
        let tasks: Vec<Task> = vec![];

        let csv_result = export_to_string(&tasks, ExportFormat::Csv).unwrap();
        assert!(csv_result.starts_with("ID,Title,Status")); // Header only

        let ics_result = export_to_string(&tasks, ExportFormat::Ics).unwrap();
        assert!(ics_result.contains("BEGIN:VCALENDAR"));
        assert!(ics_result.contains("END:VCALENDAR"));
        assert!(!ics_result.contains("BEGIN:VTODO")); // No tasks
    }

    #[test]
    fn test_export_format_parse_dot() {
        assert_eq!(ExportFormat::parse("dot"), Some(ExportFormat::Dot));
        assert_eq!(ExportFormat::parse("graphviz"), Some(ExportFormat::Dot));
    }

    #[test]
    fn test_export_format_parse_mermaid() {
        assert_eq!(ExportFormat::parse("mermaid"), Some(ExportFormat::Mermaid));
        assert_eq!(ExportFormat::parse("md"), Some(ExportFormat::Mermaid));
    }

    #[test]
    fn test_escape_dot() {
        assert_eq!(escape_dot("hello"), "hello");
        assert_eq!(escape_dot("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_dot("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn test_escape_mermaid() {
        assert_eq!(escape_mermaid("hello"), "hello");
        assert_eq!(escape_mermaid("say \"hi\""), "say 'hi'");
        assert_eq!(escape_mermaid("array[0]"), "array(0)");
    }

    #[test]
    fn test_export_dot_basic() {
        let mut tasks = HashMap::new();
        let task = create_test_task("Test Task");
        tasks.insert(task.id.clone(), task);

        let result = export_chains_to_string(&tasks, ExportFormat::Dot).unwrap();

        assert!(result.starts_with("digraph TaskChains {"));
        assert!(result.contains("rankdir=LR"));
        assert!(result.contains("Test Task"));
        assert!(result.ends_with("}\n"));
    }

    #[test]
    fn test_export_dot_with_chain() {
        let mut tasks = HashMap::new();

        let task1 = create_test_task("First Task");
        let task2 = create_test_task("Second Task");
        let task1_id = task1.id.clone();

        // Create chain: task1 -> task2
        let mut task1_modified = task1.clone();
        task1_modified.next_task_id = Some(task2.id.clone());

        tasks.insert(task1_id.clone(), task1_modified);
        tasks.insert(task2.id.clone(), task2);

        let result = export_chains_to_string(&tasks, ExportFormat::Dot).unwrap();

        assert!(result.contains("Chain edges"));
        assert!(result.contains("[color=\"blue\" label=\"chain\"]"));
    }

    #[test]
    fn test_export_dot_with_dependency() {
        let mut tasks = HashMap::new();

        let task1 = create_test_task("Dependency");
        let task1_id = task1.id.clone();

        let mut task2 = create_test_task("Dependent Task");
        task2.dependencies.push(task1_id.clone());

        tasks.insert(task1_id, task1);
        tasks.insert(task2.id.clone(), task2);

        let result = export_chains_to_string(&tasks, ExportFormat::Dot).unwrap();

        assert!(result.contains("Dependency edges"));
        assert!(result.contains("[color=\"red\" style=\"dashed\" label=\"blocks\"]"));
    }

    #[test]
    fn test_export_dot_status_colors() {
        let mut tasks = HashMap::new();

        let done_task = create_test_task("Done Task").with_status(TaskStatus::Done);
        tasks.insert(done_task.id.clone(), done_task);

        let result = export_chains_to_string(&tasks, ExportFormat::Dot).unwrap();

        assert!(result.contains("#90EE90")); // Light green for done
    }

    #[test]
    fn test_export_mermaid_basic() {
        let mut tasks = HashMap::new();
        let task = create_test_task("Test Task");
        tasks.insert(task.id.clone(), task);

        let result = export_chains_to_string(&tasks, ExportFormat::Mermaid).unwrap();

        assert!(result.starts_with("```mermaid"));
        assert!(result.contains("flowchart LR"));
        assert!(result.contains("Test Task"));
        assert!(result.ends_with("```\n"));
    }

    #[test]
    fn test_export_mermaid_with_chain() {
        let mut tasks = HashMap::new();

        let task1 = create_test_task("First Task");
        let task2 = create_test_task("Second Task");

        let mut task1_modified = task1.clone();
        task1_modified.next_task_id = Some(task2.id.clone());

        tasks.insert(task1.id.clone(), task1_modified);
        tasks.insert(task2.id.clone(), task2);

        let result = export_chains_to_string(&tasks, ExportFormat::Mermaid).unwrap();

        assert!(result.contains("Chain edges"));
        assert!(result.contains("-->")); // Chain arrow
    }

    #[test]
    fn test_export_mermaid_with_dependency() {
        let mut tasks = HashMap::new();

        let task1 = create_test_task("Dependency");
        let task1_id = task1.id.clone();

        let mut task2 = create_test_task("Dependent Task");
        task2.dependencies.push(task1_id.clone());

        tasks.insert(task1_id, task1);
        tasks.insert(task2.id.clone(), task2);

        let result = export_chains_to_string(&tasks, ExportFormat::Mermaid).unwrap();

        assert!(result.contains("Dependency edges"));
        assert!(result.contains("-.->|blocks|")); // Dependency arrow with label
    }

    #[test]
    fn test_export_mermaid_style_classes() {
        let mut tasks = HashMap::new();
        let done_task = create_test_task("Done Task").with_status(TaskStatus::Done);
        tasks.insert(done_task.id.clone(), done_task);

        let result = export_chains_to_string(&tasks, ExportFormat::Mermaid).unwrap();

        assert!(result.contains("classDef done"));
        assert!(result.contains(":::done")); // Node uses the class
    }

    #[test]
    fn test_export_chains_empty() {
        let tasks: HashMap<TaskId, Task> = HashMap::new();

        let dot_result = export_chains_to_string(&tasks, ExportFormat::Dot).unwrap();
        assert!(dot_result.contains("digraph TaskChains"));

        let mermaid_result = export_chains_to_string(&tasks, ExportFormat::Mermaid).unwrap();
        assert!(mermaid_result.contains("flowchart LR"));
    }

    // Helper to create a test analytics report
    fn create_test_report() -> AnalyticsReport {
        use crate::domain::analytics::{
            BurnChart, CompletionTrend, PriorityBreakdown, ProductivityInsights, ReportConfig,
            StatusBreakdown, TagStats, TimeAnalytics, TimeSeriesPoint, VelocityMetrics,
        };
        use chrono::Weekday;

        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();

        AnalyticsReport {
            config: ReportConfig::custom(start, end),
            completion_trend: CompletionTrend {
                completions_by_day: vec![
                    TimeSeriesPoint::new(start, 5.0),
                    TimeSeriesPoint::new(end, 3.0),
                ],
                creations_by_day: vec![
                    TimeSeriesPoint::new(start, 10.0),
                    TimeSeriesPoint::new(end, 5.0),
                ],
                completion_rate_over_time: vec![
                    TimeSeriesPoint::new(start, 0.5),
                    TimeSeriesPoint::new(end, 0.6),
                ],
            },
            velocity: VelocityMetrics {
                weekly_velocity: vec![
                    (NaiveDate::from_ymd_opt(2025, 1, 6).unwrap(), 5),
                    (NaiveDate::from_ymd_opt(2025, 1, 13).unwrap(), 10),
                ],
                monthly_velocity: vec![(start, 15)],
                avg_weekly: 7.5,
                trend: 0.5,
            },
            burn_charts: vec![BurnChart {
                project_name: "Test Project".to_string(),
                project_id: None,
                scope_line: vec![TimeSeriesPoint::new(start, 20.0)],
                completed_line: vec![TimeSeriesPoint::new(start, 15.0)],
                ideal_line: None,
            }],
            time_analytics: TimeAnalytics {
                by_project: std::collections::HashMap::new(),
                by_day_of_week: [0, 60, 120, 90, 150, 30, 0],
                by_hour: [0; 24],
                total_minutes: 450,
            },
            insights: ProductivityInsights {
                best_day: Some(Weekday::Fri),
                peak_hour: Some(14),
                current_streak: 5,
                longest_streak: 10,
                avg_tasks_per_day: 2.5,
                total_completed: 50,
                total_time_tracked: 450,
            },
            status_breakdown: StatusBreakdown {
                todo: 10,
                in_progress: 5,
                blocked: 2,
                done: 30,
                cancelled: 3,
            },
            priority_breakdown: PriorityBreakdown {
                none: 10,
                low: 15,
                medium: 12,
                high: 8,
                urgent: 5,
            },
            tag_stats: vec![
                TagStats {
                    tag: "work".to_string(),
                    count: 20,
                    completed: 15,
                },
                TagStats {
                    tag: "personal".to_string(),
                    count: 10,
                    completed: 8,
                },
            ],
        }
    }

    #[test]
    fn test_export_report_markdown_basic() {
        let report = create_test_report();
        let result = export_report_to_markdown_string(&report).unwrap();

        // Check header
        assert!(result.contains("# TaskFlow Analytics Report"));
        assert!(result.contains("**Period:** 2025-01-01 to 2025-01-31"));

        // Check overview section
        assert!(result.contains("## Overview"));
        assert!(result.contains("| Total Tasks | 50 |"));
        assert!(result.contains("| Completion Rate | 60.0% |"));

        // Check status breakdown
        assert!(result.contains("## Status Breakdown"));
        assert!(result.contains("| To Do | 10 |"));
        assert!(result.contains("| Done | 30 |"));

        // Check priority breakdown
        assert!(result.contains("## Priority Breakdown"));
        assert!(result.contains("| Urgent | 5 |"));
        assert!(result.contains("| High | 8 |"));

        // Check velocity
        assert!(result.contains("## Velocity"));
        assert!(result.contains("7.5 tasks/week"));
        assert!(result.contains("Improving"));

        // Check productivity insights
        assert!(result.contains("## Productivity Insights"));
        assert!(result.contains("Current Streak:** 5 days"));
        assert!(result.contains("Longest Streak:** 10 days"));
        assert!(result.contains("Most Productive Day:** Fri"));
        assert!(result.contains("Peak Productivity Hour:** 14:00"));

        // Check tag statistics
        assert!(result.contains("## Top Tags"));
        assert!(result.contains("| work | 20 | 15 |"));

        // Check project progress
        assert!(result.contains("## Project Progress"));
        assert!(result.contains("| Test Project |"));

        // Check footer
        assert!(result.contains("*Generated by TaskFlow*"));
    }

    #[test]
    fn test_export_report_html_basic() {
        let report = create_test_report();
        let result = export_report_to_html_string(&report).unwrap();

        // Check HTML structure
        assert!(result.contains("<!DOCTYPE html>"));
        assert!(result.contains("<html lang=\"en\">"));
        assert!(result.contains("<title>TaskFlow Analytics Report</title>"));
        assert!(result.contains("</html>"));

        // Check content
        assert!(result.contains("TaskFlow Analytics Report"));
        assert!(result.contains("2025-01-01 to 2025-01-31"));

        // Check cards
        assert!(result.contains("<div class=\"card\">"));
        assert!(result.contains("<h2>Overview</h2>"));
        assert!(result.contains("<h2>Status Breakdown</h2>"));
        assert!(result.contains("<h2>Priority Breakdown</h2>"));
        assert!(result.contains("<h2>Velocity</h2>"));
        assert!(result.contains("<h2>Productivity Insights</h2>"));
        assert!(result.contains("<h2>Top Tags</h2>"));
        assert!(result.contains("<h2>Project Progress</h2>"));

        // Check styled elements
        assert!(result.contains("<style>"));
        assert!(result.contains("class=\"metric\""));
        assert!(result.contains("class=\"progress-bar\""));
        assert!(result.contains("class=\"progress-fill\""));

        // Check footer
        assert!(result.contains("Generated by TaskFlow"));
    }

    #[test]
    fn test_export_report_html_escapes_special_chars() {
        assert_eq!(escape_html("hello"), "hello");
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("\"quote\""), "&quot;quote&quot;");
        assert_eq!(escape_html("it's"), "it&#39;s");
    }

    #[test]
    fn test_export_report_markdown_empty_sections() {
        use crate::domain::analytics::{
            CompletionTrend, PriorityBreakdown, ProductivityInsights, ReportConfig,
            StatusBreakdown, TimeAnalytics, VelocityMetrics,
        };

        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();

        // Create a minimal report with empty optional sections
        let report = AnalyticsReport {
            config: ReportConfig::custom(start, end),
            completion_trend: CompletionTrend::default(),
            velocity: VelocityMetrics::default(),
            burn_charts: vec![], // Empty
            time_analytics: TimeAnalytics::default(),
            insights: ProductivityInsights::default(),
            status_breakdown: StatusBreakdown::default(),
            priority_breakdown: PriorityBreakdown::default(),
            tag_stats: vec![], // Empty
        };

        let result = export_report_to_markdown_string(&report).unwrap();

        // Should still have basic structure
        assert!(result.contains("# TaskFlow Analytics Report"));
        assert!(result.contains("## Overview"));
        assert!(result.contains("## Status Breakdown"));

        // Should NOT have empty sections
        assert!(!result.contains("## Top Tags"));
        assert!(!result.contains("## Project Progress"));
    }

    #[test]
    fn test_export_report_markdown_time_tracked() {
        use crate::domain::analytics::{
            CompletionTrend, PriorityBreakdown, ProductivityInsights, ReportConfig,
            StatusBreakdown, TimeAnalytics, VelocityMetrics,
        };

        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();

        let report = AnalyticsReport {
            config: ReportConfig::custom(start, end),
            completion_trend: CompletionTrend::default(),
            velocity: VelocityMetrics::default(),
            burn_charts: vec![],
            time_analytics: TimeAnalytics::default(),
            insights: ProductivityInsights {
                total_time_tracked: 125, // 2h 5m
                ..Default::default()
            },
            status_breakdown: StatusBreakdown::default(),
            priority_breakdown: PriorityBreakdown::default(),
            tag_stats: vec![],
        };

        let result = export_report_to_markdown_string(&report).unwrap();
        assert!(result.contains("**Total Time Tracked:** 2h 5m"));
    }
}
