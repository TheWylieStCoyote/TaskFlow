//! Markdown format export for analytics reports.

use std::io::Write;

use crate::domain::analytics::AnalyticsReport;

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
