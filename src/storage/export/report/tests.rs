//! Tests for analytics report export functionality.

use super::html::escape_html;
use super::{export_report_to_html_string, export_report_to_markdown_string};
use crate::domain::analytics::AnalyticsReport;
use crate::domain::analytics::{
    BurnChart, CompletionTrend, EstimationAnalytics, PriorityBreakdown, ProductivityInsights,
    ReportConfig, StatusBreakdown, TagStats, TimeAnalytics, TimeSeriesPoint, VelocityMetrics,
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
        estimation_analytics: EstimationAnalytics::default(),
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
        estimation_analytics: EstimationAnalytics::default(),
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

// ==================== Additional branch coverage ====================

fn many_tags(n: usize) -> Vec<TagStats> {
    (0..n)
        .map(|i| TagStats {
            tag: format!("tag{i}"),
            count: 10,
            completed: 5,
        })
        .collect()
}

fn position(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("{needle:?} not found in output"))
}

// ---- HTML ----

#[test]
fn test_html_header_metadata_and_order() {
    let output = export_report_to_html_string(&sample_report()).unwrap();

    assert!(output.starts_with("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n"));
    assert!(output.contains("  <meta charset=\"UTF-8\">\n"));
    assert!(output.contains(
        "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n"
    ));
    assert!(output.contains("    h1 {"));
    assert!(output.contains("    th, td {"));
    assert!(output.contains(".trend-up {"));
    assert_eq!(output.matches("<html lang=\"en\">").count(), 1);
    assert_eq!(output.matches("</head>").count(), 1);

    // The header is emitted before the title, and the title before the first card.
    assert!(position(&output, "</style>") < position(&output, "<h1>"));
    assert!(position(&output, "<body>") < position(&output, "<h1>"));
    assert!(position(&output, "<h1>") < position(&output, "<div class=\"card\">"));
    assert!(output.contains("<p><strong>Period:</strong> 2025-01-01 to 2025-01-31</p>"));
    assert!(output.trim_end().ends_with("</html>"));
}

#[test]
fn test_html_card_order() {
    let output = export_report_to_html_string(&sample_report()).unwrap();
    let headings = [
        "<h2>Overview</h2>",
        "<h2>Status Breakdown</h2>",
        "<h2>Priority Breakdown</h2>",
        "<h2>Velocity</h2>",
        "<h2>Productivity Insights</h2>",
        "<h2>Top Tags</h2>",
        "<h2>Project Progress</h2>",
        "<footer",
    ];
    let positions: Vec<usize> = headings.iter().map(|h| position(&output, h)).collect();
    assert!(
        positions.windows(2).all(|w| w[0] < w[1]),
        "cards out of order: {positions:?}"
    );
}

#[test]
fn test_html_status_card_rows() {
    let output = export_report_to_html_string(&sample_report()).unwrap();

    assert!(output.contains("    <tr><th>Status</th><th>Count</th></tr>\n"));
    assert!(output.contains("    <tr><td>📋 To Do</td><td>10</td></tr>\n"));
    assert!(output.contains("    <tr><td>🔄 In Progress</td><td>5</td></tr>\n"));
    assert!(output.contains("    <tr><td>🚫 Blocked</td><td>2</td></tr>\n"));
    assert!(output.contains("    <tr><td>✅ Done</td><td>30</td></tr>\n"));
    assert!(output.contains("    <tr><td>❌ Cancelled</td><td>3</td></tr>\n"));
}

#[test]
fn test_html_status_and_priority_cards_with_zero_counts() {
    let output = export_report_to_html_string(&empty_report()).unwrap();

    assert!(output.contains("<h2>Status Breakdown</h2>"));
    assert!(output.contains("<tr><td>📋 To Do</td><td>0</td></tr>"));
    assert!(output.contains("<tr><td>🔄 In Progress</td><td>0</td></tr>"));
    assert!(output.contains("<tr><td>🚫 Blocked</td><td>0</td></tr>"));
    assert!(output.contains("<tr><td>✅ Done</td><td>0</td></tr>"));
    assert!(output.contains("<tr><td>❌ Cancelled</td><td>0</td></tr>"));

    assert!(output.contains("<h2>Priority Breakdown</h2>"));
    assert!(output.contains("<tr><td>🔴 Urgent</td><td>0</td></tr>"));
    assert!(output.contains("<tr><td>🟠 High</td><td>0</td></tr>"));
    assert!(output.contains("<tr><td>🟡 Medium</td><td>0</td></tr>"));
    assert!(output.contains("<tr><td>🟢 Low</td><td>0</td></tr>"));
    assert!(output.contains("<tr><td>⚪ None</td><td>0</td></tr>"));

    assert!(output.contains("Total Tasks: <span class=\"metric-value\">0</span>"));
    assert!(output.contains("Completion Rate: <span class=\"metric-value\">0.0%</span>"));
    assert!(output.contains("Completed: <span class=\"metric-value\">0</span>"));
    assert!(output.contains("Created: <span class=\"metric-value\">0</span>"));
}

#[test]
fn test_html_priority_card_rows() {
    let output = export_report_to_html_string(&sample_report()).unwrap();

    assert!(output.contains("    <tr><th>Priority</th><th>Count</th></tr>\n"));
    assert!(output.contains("    <tr><td>🔴 Urgent</td><td>5</td></tr>\n"));
    assert!(output.contains("    <tr><td>🟠 High</td><td>10</td></tr>\n"));
    assert!(output.contains("    <tr><td>🟡 Medium</td><td>10</td></tr>\n"));
    assert!(output.contains("    <tr><td>🟢 Low</td><td>10</td></tr>\n"));
    assert!(output.contains("    <tr><td>⚪ None</td><td>15</td></tr>\n"));
}

#[test]
fn test_html_priority_card_single_bucket() {
    let mut report = empty_report();
    report.priority_breakdown.urgent = 7;
    let output = export_report_to_html_string(&report).unwrap();

    assert!(output.contains("<tr><td>🔴 Urgent</td><td>7</td></tr>"));
    assert!(output.contains("<tr><td>🟠 High</td><td>0</td></tr>"));
    assert!(output.contains("<tr><td>⚪ None</td><td>0</td></tr>"));
}

#[test]
fn test_html_overview_metrics_from_trend() {
    let output = export_report_to_html_string(&sample_report()).unwrap();
    assert!(output.contains("Completed: <span class=\"metric-value\">13</span>"));
    assert!(output.contains("Created: <span class=\"metric-value\">15</span>"));
}

#[test]
fn test_html_insights_card_all_fields() {
    let output = export_report_to_html_string(&sample_report()).unwrap();

    // 5 < 10 so no trophy
    assert!(output.contains("  <p><strong>Current Streak:</strong> 5 days</p>\n"));
    assert!(output.contains("  <p><strong>Longest Streak:</strong> 10 days</p>\n"));
    assert!(output.contains("  <p><strong>Most Productive Day:</strong> Fri</p>\n"));
    assert!(output.contains("  <p><strong>Peak Hour:</strong> 14:00</p>\n"));
    assert!(output.contains("  <p><strong>Total Time Tracked:</strong> 8h 0m</p>\n"));
    assert!(!output.contains("🏆"));
}

#[test]
fn test_html_insights_card_omits_optional_lines() {
    let output = export_report_to_html_string(&empty_report()).unwrap();

    assert!(output.contains("<h2>Productivity Insights</h2>"));
    assert!(output.contains("  <p><strong>Current Streak:</strong> 0 days</p>\n"));
    assert!(output.contains("  <p><strong>Longest Streak:</strong> 0 days</p>\n"));
    assert!(!output.contains("Most Productive Day"));
    assert!(!output.contains("Peak Hour"));
    assert!(!output.contains("Total Time Tracked"));
    assert!(!output.contains("🏆"));
}

#[test]
fn test_html_insights_trophy_when_current_streak_is_best() {
    let mut report = sample_report();
    report.insights.current_streak = 15;
    report.insights.longest_streak = 10;
    let output = export_report_to_html_string(&report).unwrap();
    assert!(output.contains("<strong>Current Streak:</strong> 15 days 🏆</p>"));

    // Equal streaks also count as the best streak.
    report.insights.current_streak = 10;
    let output = export_report_to_html_string(&report).unwrap();
    assert!(output.contains("<strong>Current Streak:</strong> 10 days 🏆</p>"));
}

#[test]
fn test_html_insights_time_tracked_with_minutes() {
    let mut report = empty_report();
    report.insights.total_time_tracked = 95;
    report.insights.peak_hour = Some(0);
    report.insights.best_day = Some(Weekday::Mon);
    let output = export_report_to_html_string(&report).unwrap();

    assert!(output.contains("<strong>Total Time Tracked:</strong> 1h 35m</p>"));
    assert!(output.contains("<strong>Peak Hour:</strong> 0:00</p>"));
    assert!(output.contains("<strong>Most Productive Day:</strong> Mon</p>"));
}

#[test]
fn test_html_velocity_best_week_present_and_absent() {
    let output = export_report_to_html_string(&sample_report()).unwrap();
    assert!(output.contains("<strong>Average Weekly:</strong> 12.5 tasks/week</p>"));
    assert!(output.contains("<span class=\"trend-up\">📈 +2.5</span>"));
    assert!(output.contains("<strong>Best Week:</strong> 2025-01-13 (15 tasks)</p>"));

    let output = export_report_to_html_string(&empty_report()).unwrap();
    assert!(output.contains("<strong>Average Weekly:</strong> 0.0 tasks/week</p>"));
    assert!(output.contains("<span class=\"trend-stable\">➡️ +0.0</span>"));
    assert!(!output.contains("Best Week"));
}

#[test]
fn test_html_tags_card_rows_and_limit() {
    let output = export_report_to_html_string(&sample_report()).unwrap();
    assert!(
        output.contains("    <tr><th>Tag</th><th>Count</th><th>Completed</th><th>Rate</th></tr>\n")
    );
    assert!(output.contains("    <tr><td>#work</td><td>20</td><td>15</td><td>75%</td></tr>\n"));
    assert!(output.contains("    <tr><td>#personal</td><td>10</td><td>8</td><td>80%</td></tr>\n"));

    let mut report = sample_report();
    report.tag_stats = many_tags(12);
    let output = export_report_to_html_string(&report).unwrap();
    assert!(output.contains("<td>#tag0</td>"));
    assert!(output.contains("<td>#tag9</td>"));
    assert!(!output.contains("<td>#tag10</td>"));
    assert!(!output.contains("<td>#tag11</td>"));
    assert_eq!(output.matches("<td>#tag").count(), 10);
}

#[test]
fn test_html_project_card_text_and_bar() {
    let output = export_report_to_html_string(&sample_report()).unwrap();
    assert!(
        output.contains("  <p><strong>Test Project</strong> - 25 remaining (75.0% complete)</p>\n")
    );
    assert_eq!(output.matches("<div class=\"progress-bar\">").count(), 1);

    let mut report = sample_report();
    report.burn_charts.push(BurnChart {
        project_name: "Second".to_string(),
        project_id: None,
        scope_line: vec![TimeSeriesPoint::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            10.0,
        )],
        completed_line: vec![TimeSeriesPoint::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            2.0,
        )],
        ideal_line: None,
    });
    let output = export_report_to_html_string(&report).unwrap();
    assert_eq!(output.matches("<div class=\"progress-bar\">").count(), 2);
    assert!(output.contains("<strong>Second</strong> - 8 remaining (20.0% complete)"));
    assert!(output.contains("style=\"width: 20.0%\""));
}

#[test]
fn test_html_export_to_writer_matches_string_export() {
    let report = sample_report();
    let mut buffer = Vec::new();
    super::export_report_to_html(&report, &mut buffer).unwrap();
    assert_eq!(
        String::from_utf8(buffer).unwrap(),
        export_report_to_html_string(&report).unwrap()
    );
}

// ---- Markdown ----

#[test]
fn test_markdown_overview_table() {
    let output = export_report_to_markdown_string(&sample_report()).unwrap();
    assert!(output.contains("| Metric | Value |\n|--------|-------|\n| Total Tasks | 50 |\n"));
    assert!(output.contains("| Avg Tasks/Day | 2.5 |"));

    let output = export_report_to_markdown_string(&empty_report()).unwrap();
    assert!(output.contains("| Tasks Completed | 0 |"));
    assert!(output.contains("| Tasks Created | 0 |"));
    assert!(output.contains("| Avg Tasks/Day | 0.0 |"));
}

#[test]
fn test_markdown_section_order() {
    let output = export_report_to_markdown_string(&sample_report()).unwrap();
    let headings = [
        "# TaskFlow Analytics Report",
        "**Period:**",
        "## Overview",
        "## Status Breakdown",
        "## Priority Breakdown",
        "## Velocity",
        "## Productivity Insights",
        "## Top Tags",
        "## Project Progress",
        "---\n*Generated by TaskFlow*",
    ];
    let positions: Vec<usize> = headings.iter().map(|h| position(&output, h)).collect();
    assert!(
        positions.windows(2).all(|w| w[0] < w[1]),
        "sections out of order: {positions:?}"
    );
    assert!(output.trim_end().ends_with("*Generated by TaskFlow*"));
}

#[test]
fn test_markdown_zero_status_and_priority_rows() {
    let output = export_report_to_markdown_string(&empty_report()).unwrap();
    assert!(output.contains("| To Do | 0 |"));
    assert!(output.contains("| In Progress | 0 |"));
    assert!(output.contains("| Blocked | 0 |"));
    assert!(output.contains("| Done | 0 |"));
    assert!(output.contains("| Cancelled | 0 |"));
    assert!(output.contains("| Urgent | 0 |"));
    assert!(output.contains("| High | 0 |"));
    assert!(output.contains("| Medium | 0 |"));
    assert!(output.contains("| Low | 0 |"));
    assert!(output.contains("| None | 0 |"));
}

#[test]
fn test_markdown_velocity_best_week_present_and_absent() {
    let output = export_report_to_markdown_string(&sample_report()).unwrap();
    assert!(output.contains("- **Trend:** 📈 Improving (+2.5)"));
    assert!(output.contains("- **Best Week:** 2025-01-13 (15 tasks)"));

    let output = export_report_to_markdown_string(&empty_report()).unwrap();
    assert!(output.contains("- **Average Weekly Velocity:** 0.0 tasks/week"));
    assert!(output.contains("- **Trend:** ➡️ Stable (+0.0)"));
    assert!(!output.contains("Best Week"));

    let mut report = sample_report();
    report.velocity.trend = -1.5;
    let output = export_report_to_markdown_string(&report).unwrap();
    assert!(output.contains("- **Trend:** 📉 Declining (-1.5)"));
}

#[test]
fn test_markdown_insights_omit_optional_lines() {
    let output = export_report_to_markdown_string(&empty_report()).unwrap();
    assert!(output.contains("- **Current Streak:** 0 days\n"));
    assert!(output.contains("- **Longest Streak:** 0 days\n"));
    assert!(!output.contains("🏆"));
    assert!(!output.contains("Most Productive Day"));
    assert!(!output.contains("Peak Productivity Hour"));
    assert!(!output.contains("Total Time Tracked"));
}

#[test]
fn test_markdown_insights_time_tracked_with_minutes_and_equal_streaks() {
    let mut report = empty_report();
    report.insights.total_time_tracked = 125;
    report.insights.current_streak = 3;
    report.insights.longest_streak = 3;
    report.insights.peak_hour = Some(23);
    report.insights.best_day = Some(Weekday::Sun);
    let output = export_report_to_markdown_string(&report).unwrap();

    assert!(output.contains("- **Total Time Tracked:** 2h 5m"));
    assert!(output.contains("- **Current Streak:** 3 days 🏆"));
    assert!(output.contains("- **Longest Streak:** 3 days"));
    assert!(output.contains("- **Peak Productivity Hour:** 23:00"));
    assert!(output.contains("- **Most Productive Day:** Sun"));
}

#[test]
fn test_markdown_no_trophy_when_streak_is_zero() {
    // current_streak 0 == longest_streak 0, but a zero streak is never "best".
    let output = export_report_to_markdown_string(&empty_report()).unwrap();
    assert!(!output.contains("🏆"));
    let output = export_report_to_html_string(&empty_report()).unwrap();
    assert!(!output.contains("🏆"));
}

#[test]
fn test_markdown_tags_limited_to_ten() {
    let mut report = sample_report();
    report.tag_stats = many_tags(12);
    let output = export_report_to_markdown_string(&report).unwrap();

    assert!(
        output.contains("| Tag | Count | Completed | Rate |\n|-----|-------|-----------|------|\n")
    );
    assert!(output.contains("| tag0 | 10 | 5 | 50% |"));
    assert!(output.contains("| tag9 | 10 | 5 | 50% |"));
    assert!(!output.contains("| tag10 |"));
    assert!(!output.contains("| tag11 |"));
    assert_eq!(output.matches("| tag").count(), 10);
}

#[test]
fn test_markdown_project_progress_multiple_rows() {
    let mut report = sample_report();
    report.burn_charts.push(BurnChart {
        project_name: "Second".to_string(),
        project_id: None,
        scope_line: vec![TimeSeriesPoint::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            10.0,
        )],
        completed_line: vec![TimeSeriesPoint::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            2.0,
        )],
        ideal_line: None,
    });
    let output = export_report_to_markdown_string(&report).unwrap();

    assert!(output
        .contains("| Project | Remaining | Completion |\n|---------|-----------|------------|\n"));
    assert!(output.contains("| Test Project | 25 | 75.0% |"));
    assert!(output.contains("| Second | 8 | 20.0% |"));
    // Markdown output is not HTML-escaped.
    report.burn_charts[0].project_name = "A & B".to_string();
    let output = export_report_to_markdown_string(&report).unwrap();
    assert!(output.contains("| A & B | 25 | 75.0% |"));
}

#[test]
fn test_markdown_export_to_writer_matches_string_export() {
    let report = sample_report();
    let mut buffer = Vec::new();
    super::export_report_to_markdown(&report, &mut buffer).unwrap();
    assert_eq!(
        String::from_utf8(buffer).unwrap(),
        export_report_to_markdown_string(&report).unwrap()
    );
}
