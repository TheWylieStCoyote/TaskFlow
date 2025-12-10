//! Analytics report export functionality (Markdown and HTML).

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
    write_html_header(writer)?;

    // Title
    writeln!(writer, "<h1>📊 TaskFlow Analytics Report</h1>")?;
    writeln!(
        writer,
        "<p><strong>Period:</strong> {} to {}</p>",
        report.config.start_date, report.config.end_date
    )?;

    // Overview card
    write_overview_card(writer, report)?;

    // Status breakdown card
    write_status_card(writer, report)?;

    // Priority breakdown card
    write_priority_card(writer, report)?;

    // Velocity card
    write_velocity_card(writer, report)?;

    // Productivity insights card
    write_insights_card(writer, report)?;

    // Tag statistics card
    if !report.tag_stats.is_empty() {
        write_tags_card(writer, report)?;
    }

    // Project progress card
    if !report.burn_charts.is_empty() {
        write_projects_card(writer, report)?;
    }

    // Footer
    write_html_footer(writer)?;

    Ok(())
}

fn write_html_header<W: Write>(writer: &mut W) -> std::io::Result<()> {
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
    Ok(())
}

fn write_html_footer<W: Write>(writer: &mut W) -> std::io::Result<()> {
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

fn write_overview_card<W: Write>(writer: &mut W, report: &AnalyticsReport) -> std::io::Result<()> {
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
    Ok(())
}

fn write_status_card<W: Write>(writer: &mut W, report: &AnalyticsReport) -> std::io::Result<()> {
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
    Ok(())
}

fn write_priority_card<W: Write>(writer: &mut W, report: &AnalyticsReport) -> std::io::Result<()> {
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
    Ok(())
}

fn write_velocity_card<W: Write>(writer: &mut W, report: &AnalyticsReport) -> std::io::Result<()> {
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
    Ok(())
}

fn write_insights_card<W: Write>(writer: &mut W, report: &AnalyticsReport) -> std::io::Result<()> {
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
    Ok(())
}

fn write_tags_card<W: Write>(writer: &mut W, report: &AnalyticsReport) -> std::io::Result<()> {
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
    Ok(())
}

fn write_projects_card<W: Write>(writer: &mut W, report: &AnalyticsReport) -> std::io::Result<()> {
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
            "    <div class=\"progress-fill\" style=\"width: {pct:.1}%\"></div>"
        )?;
        writeln!(writer, "  </div>")?;
    }
    writeln!(writer, "</div>")?;
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
    use crate::domain::analytics::{
        BurnChart, CompletionTrend, PriorityBreakdown, ProductivityInsights, ReportConfig,
        StatusBreakdown, TagStats, TimeAnalytics, TimeSeriesPoint, VelocityMetrics,
    };
    use chrono::{NaiveDate, Weekday};

    /// Create a sample analytics report for testing
    fn sample_report() -> AnalyticsReport {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();

        AnalyticsReport {
            config: ReportConfig::custom(start, end),
            completion_trend: CompletionTrend {
                completions_by_day: vec![
                    TimeSeriesPoint::new(start, 5.0),
                    TimeSeriesPoint::new(end, 8.0),
                ],
                creations_by_day: vec![
                    TimeSeriesPoint::new(start, 10.0),
                    TimeSeriesPoint::new(end, 5.0),
                ],
                completion_rate_over_time: vec![
                    TimeSeriesPoint::new(start, 0.5),
                    TimeSeriesPoint::new(end, 0.8),
                ],
            },
            velocity: VelocityMetrics {
                weekly_velocity: vec![
                    (NaiveDate::from_ymd_opt(2025, 1, 6).unwrap(), 10),
                    (NaiveDate::from_ymd_opt(2025, 1, 13).unwrap(), 15),
                ],
                monthly_velocity: vec![],
                avg_weekly: 12.5,
                trend: 2.5, // Improving
            },
            burn_charts: vec![BurnChart {
                project_name: "Test Project".to_string(),
                project_id: None,
                scope_line: vec![TimeSeriesPoint::new(start, 100.0)],
                completed_line: vec![TimeSeriesPoint::new(start, 75.0)],
                ideal_line: None,
            }],
            time_analytics: TimeAnalytics {
                by_project: std::collections::HashMap::default(),
                by_day_of_week: [10, 20, 30, 40, 50, 5, 5],
                by_hour: {
                    let mut arr = [0; 24];
                    arr[14] = 60;
                    arr[9] = 30;
                    arr
                },
                total_minutes: 160,
            },
            insights: ProductivityInsights {
                best_day: Some(Weekday::Fri),
                peak_hour: Some(14),
                current_streak: 5,
                longest_streak: 10,
                avg_tasks_per_day: 2.5,
                total_completed: 75,
                total_time_tracked: 480, // 8 hours
            },
            status_breakdown: StatusBreakdown {
                todo: 10,
                in_progress: 5,
                blocked: 2,
                done: 30,
                cancelled: 3,
            },
            priority_breakdown: PriorityBreakdown {
                none: 15,
                low: 10,
                medium: 10,
                high: 10,
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

    /// Create an empty analytics report for edge case testing
    fn empty_report() -> AnalyticsReport {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();

        AnalyticsReport {
            config: ReportConfig::custom(start, end),
            completion_trend: CompletionTrend::default(),
            velocity: VelocityMetrics::default(),
            burn_charts: vec![],
            time_analytics: TimeAnalytics::default(),
            insights: ProductivityInsights::default(),
            status_breakdown: StatusBreakdown::default(),
            priority_breakdown: PriorityBreakdown::default(),
            tag_stats: vec![],
        }
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("hello"), "hello");
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(escape_html("it's"), "it&#39;s");
    }

    // ==================== Markdown Report Tests ====================

    #[test]
    fn test_markdown_report_structure() {
        let report = sample_report();
        let output = export_report_to_markdown_string(&report).unwrap();

        // Check for main sections
        assert!(output.contains("# TaskFlow Analytics Report"));
        assert!(output.contains("## Overview"));
        assert!(output.contains("## Status Breakdown"));
        assert!(output.contains("## Priority Breakdown"));
        assert!(output.contains("## Velocity"));
        assert!(output.contains("## Productivity Insights"));
        assert!(output.contains("## Top Tags"));
        assert!(output.contains("## Project Progress"));
        assert!(output.contains("*Generated by TaskFlow*"));
    }

    #[test]
    fn test_markdown_report_date_range() {
        let report = sample_report();
        let output = export_report_to_markdown_string(&report).unwrap();

        // Check date range is included
        assert!(output.contains("**Period:** 2025-01-01 to 2025-01-31"));
    }

    #[test]
    fn test_markdown_report_metrics() {
        let report = sample_report();
        let output = export_report_to_markdown_string(&report).unwrap();

        // Check metrics values
        assert!(output.contains("| Total Tasks | 50 |")); // 10+5+2+30+3 = 50
        assert!(output.contains("| Completion Rate | 60.0% |")); // 30/50 = 60%
        assert!(output.contains("| Tasks Completed | 13 |")); // 5+8 = 13
        assert!(output.contains("| Tasks Created | 15 |")); // 10+5 = 15
    }

    #[test]
    fn test_markdown_status_breakdown() {
        let report = sample_report();
        let output = export_report_to_markdown_string(&report).unwrap();

        assert!(output.contains("| To Do | 10 |"));
        assert!(output.contains("| In Progress | 5 |"));
        assert!(output.contains("| Blocked | 2 |"));
        assert!(output.contains("| Done | 30 |"));
        assert!(output.contains("| Cancelled | 3 |"));
    }

    #[test]
    fn test_markdown_priority_breakdown() {
        let report = sample_report();
        let output = export_report_to_markdown_string(&report).unwrap();

        assert!(output.contains("| Urgent | 5 |"));
        assert!(output.contains("| High | 10 |"));
        assert!(output.contains("| Medium | 10 |"));
        assert!(output.contains("| Low | 10 |"));
        assert!(output.contains("| None | 15 |"));
    }

    #[test]
    fn test_markdown_velocity_improving_trend() {
        let report = sample_report();
        let output = export_report_to_markdown_string(&report).unwrap();

        // trend is 2.5 (positive) so should show improving
        assert!(output.contains("📈 Improving"));
        assert!(output.contains("**Average Weekly Velocity:** 12.5 tasks/week"));
    }

    #[test]
    fn test_markdown_velocity_declining_trend() {
        let mut report = sample_report();
        report.velocity.trend = -1.5;
        let output = export_report_to_markdown_string(&report).unwrap();

        assert!(output.contains("📉 Declining"));
    }

    #[test]
    fn test_markdown_velocity_stable_trend() {
        let mut report = sample_report();
        report.velocity.trend = 0.0;
        let output = export_report_to_markdown_string(&report).unwrap();

        assert!(output.contains("➡️ Stable"));
    }

    #[test]
    fn test_markdown_productivity_insights() {
        let report = sample_report();
        let output = export_report_to_markdown_string(&report).unwrap();

        assert!(output.contains("**Current Streak:** 5 days"));
        assert!(output.contains("**Longest Streak:** 10 days"));
        assert!(output.contains("**Most Productive Day:** Fri"));
        assert!(output.contains("**Peak Productivity Hour:** 14:00"));
        assert!(output.contains("**Total Time Tracked:** 8h 0m"));
    }

    #[test]
    fn test_markdown_tag_stats() {
        let report = sample_report();
        let output = export_report_to_markdown_string(&report).unwrap();

        assert!(output.contains("| work | 20 | 15 | 75% |"));
        assert!(output.contains("| personal | 10 | 8 | 80% |"));
    }

    #[test]
    fn test_markdown_project_progress() {
        let report = sample_report();
        let output = export_report_to_markdown_string(&report).unwrap();

        assert!(output.contains("| Test Project | 25 | 75.0% |"));
    }

    #[test]
    fn test_markdown_empty_report() {
        let report = empty_report();
        let output = export_report_to_markdown_string(&report).unwrap();

        // Should still have structure with zero values
        assert!(output.contains("# TaskFlow Analytics Report"));
        assert!(output.contains("| Total Tasks | 0 |"));
        assert!(output.contains("| Completion Rate | 0.0% |"));

        // Optional sections should not appear when empty
        assert!(!output.contains("## Top Tags"));
        assert!(!output.contains("## Project Progress"));
    }

    #[test]
    fn test_markdown_best_streak_trophy() {
        let mut report = sample_report();
        report.insights.current_streak = 15;
        report.insights.longest_streak = 10;
        let output = export_report_to_markdown_string(&report).unwrap();

        // When current streak >= longest, show trophy
        assert!(output.contains("**Current Streak:** 15 days 🏆"));
    }

    // ==================== HTML Report Tests ====================

    #[test]
    fn test_html_report_structure() {
        let report = sample_report();
        let output = export_report_to_html_string(&report).unwrap();

        // Check HTML structure
        assert!(output.contains("<!DOCTYPE html>"));
        assert!(output.contains("<html lang=\"en\">"));
        assert!(output.contains("<head>"));
        assert!(output.contains("<body>"));
        assert!(output.contains("</html>"));
        assert!(output.contains("<title>TaskFlow Analytics Report</title>"));
    }

    #[test]
    fn test_html_report_css_embedded() {
        let report = sample_report();
        let output = export_report_to_html_string(&report).unwrap();

        // Check CSS is embedded
        assert!(output.contains("<style>"));
        assert!(output.contains("body {"));
        assert!(output.contains(".card {"));
        assert!(output.contains(".metric {"));
        assert!(output.contains(".progress-bar {"));
        assert!(output.contains(".progress-fill {"));
    }

    #[test]
    fn test_html_report_all_cards() {
        let report = sample_report();
        let output = export_report_to_html_string(&report).unwrap();

        // Each section should be in a card div
        assert!(output.contains("<div class=\"card\">"));
        assert!(output.contains("<h2>Overview</h2>"));
        assert!(output.contains("<h2>Status Breakdown</h2>"));
        assert!(output.contains("<h2>Priority Breakdown</h2>"));
        assert!(output.contains("<h2>Velocity</h2>"));
        assert!(output.contains("<h2>Productivity Insights</h2>"));
        assert!(output.contains("<h2>Top Tags</h2>"));
        assert!(output.contains("<h2>Project Progress</h2>"));
    }

    #[test]
    fn test_html_report_metrics() {
        let report = sample_report();
        let output = export_report_to_html_string(&report).unwrap();

        // Check metric values are in spans
        assert!(output.contains("Total Tasks: <span class=\"metric-value\">50</span>"));
        assert!(output.contains("Completion Rate: <span class=\"metric-value\">60.0%</span>"));
    }

    #[test]
    fn test_html_report_status_emojis() {
        let report = sample_report();
        let output = export_report_to_html_string(&report).unwrap();

        assert!(output.contains("📋 To Do"));
        assert!(output.contains("🔄 In Progress"));
        assert!(output.contains("🚫 Blocked"));
        assert!(output.contains("✅ Done"));
        assert!(output.contains("❌ Cancelled"));
    }

    #[test]
    fn test_html_report_priority_emojis() {
        let report = sample_report();
        let output = export_report_to_html_string(&report).unwrap();

        assert!(output.contains("🔴 Urgent"));
        assert!(output.contains("🟠 High"));
        assert!(output.contains("🟡 Medium"));
        assert!(output.contains("🟢 Low"));
        assert!(output.contains("⚪ None"));
    }

    #[test]
    fn test_html_progress_bars() {
        let report = sample_report();
        let output = export_report_to_html_string(&report).unwrap();

        // Progress bar should have width based on completion percentage
        assert!(output.contains("<div class=\"progress-bar\">"));
        assert!(output.contains("<div class=\"progress-fill\" style=\"width: 75.0%\"></div>"));
    }

    #[test]
    fn test_html_trend_classes() {
        // Test improving trend
        let mut report = sample_report();
        report.velocity.trend = 2.5;
        let output = export_report_to_html_string(&report).unwrap();
        assert!(output.contains("class=\"trend-up\""));
        assert!(output.contains("📈"));

        // Test declining trend
        report.velocity.trend = -2.5;
        let output = export_report_to_html_string(&report).unwrap();
        assert!(output.contains("class=\"trend-down\""));
        assert!(output.contains("📉"));

        // Test stable trend
        report.velocity.trend = 0.0;
        let output = export_report_to_html_string(&report).unwrap();
        assert!(output.contains("class=\"trend-stable\""));
        assert!(output.contains("➡️"));
    }

    #[test]
    fn test_html_empty_report() {
        let report = empty_report();
        let output = export_report_to_html_string(&report).unwrap();

        // Should still have valid HTML structure
        assert!(output.contains("<!DOCTYPE html>"));
        assert!(output.contains("</html>"));

        // Optional sections should not appear
        assert!(!output.contains("<h2>Top Tags</h2>"));
        assert!(!output.contains("<h2>Project Progress</h2>"));
    }

    #[test]
    fn test_html_escaping_in_tags() {
        let mut report = sample_report();
        report.tag_stats = vec![TagStats {
            tag: "<script>alert('xss')</script>".to_string(),
            count: 5,
            completed: 3,
        }];
        let output = export_report_to_html_string(&report).unwrap();

        // Should be escaped
        assert!(output.contains("&lt;script&gt;"));
        assert!(!output.contains("<script>alert"));
    }

    #[test]
    fn test_html_escaping_in_project_names() {
        let mut report = sample_report();
        report.burn_charts = vec![BurnChart {
            project_name: "Project <Test> & \"More\"".to_string(),
            project_id: None,
            scope_line: vec![TimeSeriesPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                10.0,
            )],
            completed_line: vec![TimeSeriesPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                5.0,
            )],
            ideal_line: None,
        }];
        let output = export_report_to_html_string(&report).unwrap();

        // Should be escaped
        assert!(output.contains("Project &lt;Test&gt; &amp; &quot;More&quot;"));
    }

    #[test]
    fn test_html_footer() {
        let report = sample_report();
        let output = export_report_to_html_string(&report).unwrap();

        assert!(output.contains("<footer"));
        assert!(output.contains("Generated by TaskFlow"));
        assert!(output.contains("</footer>"));
    }
}
