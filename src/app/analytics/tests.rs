//! Tests for the analytics module.

use super::*;
use crate::app::Model;
use crate::domain::analytics::ReportConfig;
use crate::domain::today;
use crate::domain::{Priority, Project, Task, TaskStatus};

fn create_test_model() -> Model {
    let mut model = Model::new();

    // Use dates that are safely within the date range regardless of timezone
    // Using 2-3 days ago ensures they're always within a 7-day window
    let base_date = chrono::Utc::now() - chrono::Duration::days(2);

    // Add some tasks with various states
    let mut task1 = Task::new("Task 1");
    task1.status = TaskStatus::Done;
    task1.completed_at = Some(base_date);
    task1.tags = vec!["work".to_string(), "urgent".to_string()];
    task1.actual_minutes = 60;
    model.tasks.insert(task1.id, task1);

    let mut task2 = Task::new("Task 2");
    task2.status = TaskStatus::Done;
    task2.completed_at = Some(base_date - chrono::Duration::days(1));
    task2.tags = vec!["work".to_string()];
    task2.priority = Priority::High;
    task2.actual_minutes = 30;
    model.tasks.insert(task2.id, task2);

    let mut task3 = Task::new("Task 3");
    task3.status = TaskStatus::InProgress;
    task3.priority = Priority::Medium;
    model.tasks.insert(task3.id, task3);

    let mut task4 = Task::new("Task 4");
    task4.status = TaskStatus::Todo;
    task4.priority = Priority::Low;
    task4.tags = vec!["personal".to_string()];
    model.tasks.insert(task4.id, task4);

    // Add a project
    let project = Project::new("Test Project");
    model.projects.insert(project.id, project);

    model
}

#[test]
fn test_analytics_engine_creation() {
    let model = Model::new();
    let _engine = AnalyticsEngine::new(&model);
}

#[test]
fn test_status_breakdown() {
    let model = create_test_model();
    let engine = AnalyticsEngine::new(&model);

    let breakdown = engine.compute_status_breakdown();
    assert_eq!(breakdown.done, 2);
    assert_eq!(breakdown.in_progress, 1);
    assert_eq!(breakdown.todo, 1);
    assert_eq!(breakdown.cancelled, 0);
    assert_eq!(breakdown.total(), 4);
}

#[test]
fn test_priority_breakdown() {
    let model = create_test_model();
    let engine = AnalyticsEngine::new(&model);

    let breakdown = engine.compute_priority_breakdown();
    assert_eq!(breakdown.high, 1);
    assert_eq!(breakdown.medium, 1);
    assert_eq!(breakdown.low, 1);
    assert_eq!(breakdown.none, 1);
    assert_eq!(breakdown.urgent, 0);
}

#[test]
fn test_tag_stats() {
    let model = create_test_model();
    let engine = AnalyticsEngine::new(&model);

    let stats = engine.compute_tag_stats();

    // "work" tag should be most common
    let work_stats = stats.iter().find(|s| s.tag == "work");
    assert!(work_stats.is_some());
    assert_eq!(work_stats.unwrap().count, 2);
    assert_eq!(work_stats.unwrap().completed, 2);

    // "personal" tag
    let personal_stats = stats.iter().find(|s| s.tag == "personal");
    assert!(personal_stats.is_some());
    assert_eq!(personal_stats.unwrap().count, 1);
    assert_eq!(personal_stats.unwrap().completed, 0);
}

#[test]
fn test_insights() {
    let model = create_test_model();
    let engine = AnalyticsEngine::new(&model);

    let insights = engine.compute_insights();
    assert_eq!(insights.total_completed, 2);
    assert!(insights.total_time_tracked >= 90); // At least 60 + 30 from actual_minutes
}

#[test]
fn test_completion_trend() {
    let model = create_test_model();
    let engine = AnalyticsEngine::new(&model);

    let start = today() - chrono::Duration::days(7);
    let end = today();

    let trend = engine.compute_completion_trend(start, end);

    // We have 2 completed tasks in create_test_model()
    assert!(!trend.completions_by_day.is_empty());
    assert_eq!(trend.total_completed(), 2);
}

#[test]
fn test_velocity_metrics() {
    let model = create_test_model();
    let engine = AnalyticsEngine::new(&model);

    let start = today() - chrono::Duration::days(30);
    let end = today();

    let velocity = engine.compute_velocity(start, end);

    // Should have weekly data with 2 completed tasks
    assert!(!velocity.weekly_velocity.is_empty());
    // Average should be positive since we have completions
    assert!(velocity.avg_weekly > 0.0);
    // Total completed across all weeks should be 2
    let total: u32 = velocity
        .weekly_velocity
        .iter()
        .map(|(_, count)| count)
        .sum();
    assert_eq!(total, 2);
}

#[test]
fn test_burn_charts() {
    let model = create_test_model();
    let engine = AnalyticsEngine::new(&model);

    let start = today() - chrono::Duration::days(7);
    let end = today();

    let charts = engine.compute_burn_charts(start, end);

    // Should have at least global + per-project charts
    assert!(!charts.is_empty());
    assert!(charts.iter().any(|c| c.project_name == "All Tasks"));
}

#[test]
fn test_time_analytics() {
    let model = create_test_model();
    let engine = AnalyticsEngine::new(&model);

    let start = today() - chrono::Duration::days(7);
    let end = today();

    let analytics = engine.compute_time_analytics(start, end);

    // Task1 has 60 minutes, Task2 has 30 minutes = 90 total
    assert!(analytics.total_minutes >= 90);
    // Should have time tracked by project (at least the None/"unassigned" project)
    assert!(!analytics.by_project.is_empty());
}

#[test]
fn test_generate_full_report() {
    let model = create_test_model();
    let engine = AnalyticsEngine::new(&model);

    let config = ReportConfig::last_n_days(30);
    let report = engine.generate_report(&config);

    // Verify all components are present
    assert_eq!(report.status_breakdown.total(), 4);
    assert!(!report.tag_stats.is_empty());
    assert!(!report.burn_charts.is_empty());
}

#[test]
fn test_empty_model_report() {
    let model = Model::new();
    let engine = AnalyticsEngine::new(&model);

    let config = ReportConfig::last_n_days(7);
    let report = engine.generate_report(&config);

    assert_eq!(report.status_breakdown.total(), 0);
    assert!(report.tag_stats.is_empty());
    assert!((report.velocity.avg_weekly - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_current_streak_calculation() {
    let mut model = Model::new();

    // Create tasks completed on consecutive days
    let today = chrono::Utc::now();

    let mut task1 = Task::new("Today");
    task1.status = TaskStatus::Done;
    task1.completed_at = Some(today);
    model.tasks.insert(task1.id, task1);

    let mut task2 = Task::new("Yesterday");
    task2.status = TaskStatus::Done;
    task2.completed_at = Some(today - chrono::Duration::days(1));
    model.tasks.insert(task2.id, task2);

    let mut task3 = Task::new("Day before");
    task3.status = TaskStatus::Done;
    task3.completed_at = Some(today - chrono::Duration::days(2));
    model.tasks.insert(task3.id, task3);

    let engine = AnalyticsEngine::new(&model);
    let insights = engine.compute_insights();

    // Should have exactly a 3-day streak (today, yesterday, day before)
    assert_eq!(insights.current_streak, 3);
    assert_eq!(insights.longest_streak, 3);
    assert_eq!(insights.total_completed, 3);
}

#[test]
fn time_analytics_counts_task_time_once_when_entries_exist() {
    use crate::domain::TimeEntry;

    let mut model = Model::new();
    let mut task = Task::new("Tracked");
    task.tags = vec!["deep".to_string(), "work".to_string()];
    task.actual_minutes = 60; // cached total, must not be added on top of the entry
    let task_id = task.id;
    model.tasks.insert(task_id, task);

    let started = chrono::Utc::now() - chrono::Duration::hours(2);
    let mut entry = TimeEntry::start(task_id);
    entry.started_at = started;
    entry.ended_at = Some(started + chrono::Duration::minutes(60));
    entry.duration_minutes = Some(60);
    model.time_entries.insert(entry.id, entry);

    let engine = AnalyticsEngine::new(&model);
    let analytics = engine.compute_time_analytics(today() - chrono::Duration::days(7), today());

    assert_eq!(analytics.total_minutes, 60);
    assert_eq!(analytics.entry_count, 1);
    assert_eq!(analytics.by_task.get(&task_id), Some(&60));
    assert_eq!(analytics.by_project.get(&None), Some(&60));
    assert_eq!(analytics.by_tag.get("deep"), Some(&60));
    assert_eq!(analytics.by_tag.get("work"), Some(&60));
    let local_day = started.with_timezone(&chrono::Local).date_naive();
    assert_eq!(analytics.by_day.get(&local_day), Some(&60));
    let local_hour = chrono::Timelike::hour(&started.with_timezone(&chrono::Local)) as usize;
    assert_eq!(analytics.by_hour[local_hour], 60);

    let insights = engine.compute_insights();
    assert_eq!(insights.total_time_tracked, 60);
}

#[test]
fn time_analytics_counts_legacy_totals_without_entries_exactly_once() {
    // The fixture has two completed tasks with actual_minutes (60 + 30) and no entries.
    let model = create_test_model();
    let engine = AnalyticsEngine::new(&model);

    let analytics = engine.compute_time_analytics(today() - chrono::Duration::days(7), today());
    assert_eq!(analytics.total_minutes, 90);
    assert_eq!(analytics.entry_count, 0);
    assert_eq!(analytics.active_days(), 2);
    assert_eq!(analytics.avg_minutes_per_active_day(), 45);

    assert_eq!(engine.compute_insights().total_time_tracked, 90);
}

#[test]
fn time_analytics_daily_series_is_zero_filled_and_sorted() {
    let model = create_test_model();
    let engine = AnalyticsEngine::new(&model);
    let start = today() - chrono::Duration::days(6);
    let analytics = engine.compute_time_analytics(start, today());

    let series = analytics.daily_series(start, today());
    assert_eq!(series.len(), 7);
    assert_eq!(series.first().map(|(d, _)| *d), Some(start));
    assert_eq!(series.last().map(|(d, _)| *d), Some(today()));
    assert_eq!(series.iter().map(|(_, m)| m).sum::<u32>(), 90);

    let projects = analytics.projects_by_minutes();
    assert_eq!(projects, vec![(None, 90)]);
    let tags = analytics.tags_by_minutes();
    assert_eq!(tags.first(), Some(&("work".to_string(), 90)));
}

#[test]
fn time_analytics_task_rows_carry_title_project_entries_and_share() {
    use crate::domain::TimeEntry;

    let mut model = Model::new();
    let project = Project::new("Alpha");
    let project_id = project.id;
    model.projects.insert(project_id, project);
    let mut big = Task::new("Big task");
    big.project_id = Some(project_id);
    let big_id = big.id;
    let small = Task::new("Small task");
    let small_id = small.id;
    model.tasks.insert(big_id, big);
    model.tasks.insert(small_id, small);

    let base = chrono::Utc::now() - chrono::Duration::hours(5);
    for (task_id, minutes, offset_h) in [(big_id, 60, 0), (big_id, 30, 1), (small_id, 10, 2)] {
        let mut e = TimeEntry::start(task_id);
        e.started_at = base + chrono::Duration::hours(offset_h);
        e.ended_at = Some(e.started_at + chrono::Duration::minutes(minutes));
        e.duration_minutes = Some(minutes as u32);
        model.time_entries.insert(e.id, e);
    }
    // An orphaned entry whose task no longer exists.
    let ghost_id = crate::domain::TaskId::new();
    let mut ghost = TimeEntry::start(ghost_id);
    ghost.started_at = base;
    ghost.ended_at = Some(base + chrono::Duration::minutes(5));
    ghost.duration_minutes = Some(5);
    model.time_entries.insert(ghost.id, ghost);

    let engine = AnalyticsEngine::new(&model);
    let analytics = engine.compute_time_analytics(today() - chrono::Duration::days(7), today());
    let rows = analytics.task_rows();

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].task_id, big_id);
    assert_eq!(rows[0].title, "Big task");
    assert_eq!(rows[0].project_id, Some(project_id));
    assert_eq!(rows[0].entries, 2);
    assert_eq!(rows[0].minutes, 90);
    assert!((rows[0].share - 90.0 / 105.0 * 100.0).abs() < 0.01);
    assert_eq!(rows[1].title, "Small task");
    assert_eq!(rows[1].entries, 1);
    assert_eq!(rows[2].title, "(deleted task)");
    assert_eq!(rows[2].project_id, None);
    assert_eq!(rows[2].minutes, 5);
    assert_eq!(analytics.task_info[&big_id].entries, 2);
}

#[test]
fn time_analytics_legacy_task_rows_have_no_entries() {
    let model = create_test_model();
    let engine = AnalyticsEngine::new(&model);
    let analytics = engine.compute_time_analytics(today() - chrono::Duration::days(7), today());
    let rows = analytics.task_rows();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.entries == 0));
    assert_eq!(rows[0].minutes, 60);
    assert_eq!(rows[0].title, "Task 1");
}
