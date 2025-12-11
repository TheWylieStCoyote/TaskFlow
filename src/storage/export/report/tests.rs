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
