//! Markdown format export for analytics reports.

use std::io::Write;

use crate::domain::analytics::AnalyticsReport;
use crate::domain::format_minutes;

use super::time_section::{day_rows, project_rows, text_bar};

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
    writeln!(writer)?;

    // Time tracking
    write_time_tracking(writer, report)?;

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

/// The "Time Tracking" section: totals, a per-project table and a per-day
/// bar chart over the report window. Omitted when nothing was tracked.
fn write_time_tracking<W: Write>(writer: &mut W, report: &AnalyticsReport) -> std::io::Result<()> {
    let time = &report.time_analytics;
    if time.total_minutes == 0 {
        return Ok(());
    }

    writeln!(writer, "## Time Tracking")?;
    writeln!(writer)?;
    let active_days = time.active_days();
    writeln!(
        writer,
        "- **Total:** {} over {} active {} ({} per active day)",
        format_minutes(time.total_minutes),
        active_days,
        if active_days == 1 { "day" } else { "days" },
        format_minutes(time.avg_minutes_per_active_day())
    )?;
    if let Some(hour) = time.peak_hour() {
        writeln!(writer, "- **Peak Hour:** {hour}:00")?;
    }
    if let Some(day) = time.most_productive_day() {
        writeln!(writer, "- **Best Weekday:** {day}")?;
    }
    writeln!(writer)?;

    writeln!(writer, "| Project | Time | Share |")?;
    writeln!(writer, "|---------|------|-------|")?;
    for (name, minutes, share) in project_rows(report) {
        writeln!(
            writer,
            "| {name} | {} | {share:.1}% |",
            format_minutes(minutes)
        )?;
    }
    writeln!(writer)?;

    let days = day_rows(report);
    let max = days.iter().map(|(_, m)| *m).max().unwrap_or(0);
    writeln!(writer, "| Day | Time | |")?;
    writeln!(writer, "|-----|------|---|")?;
    for (date, minutes) in days {
        writeln!(
            writer,
            "| {date} | {} | {} |",
            format_minutes(minutes),
            text_bar(minutes, max)
        )?;
    }
    writeln!(writer)?;

    Ok(())
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
