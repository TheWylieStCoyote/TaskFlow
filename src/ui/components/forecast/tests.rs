//! Tests for forecast view component.

use super::*;
use crate::app::Model;
use crate::config::Theme;
use crate::domain::Task;
use chrono::Weekday;

#[test]
fn test_week_start_calculation() {
    // Test that week_start returns Monday
    let wednesday = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap(); // A Wednesday
    let monday = Forecast::week_start(wednesday);
    assert_eq!(monday.weekday(), Weekday::Mon);
    assert_eq!(monday, NaiveDate::from_ymd_opt(2024, 1, 8).unwrap());
}

#[test]
fn test_week_start_already_monday() {
    let monday = NaiveDate::from_ymd_opt(2024, 1, 8).unwrap();
    let result = Forecast::week_start(monday);
    assert_eq!(result, monday);
}

#[test]
fn test_week_start_sunday() {
    let sunday = NaiveDate::from_ymd_opt(2024, 1, 14).unwrap();
    let monday = Forecast::week_start(sunday);
    assert_eq!(monday.weekday(), Weekday::Mon);
    assert_eq!(monday, NaiveDate::from_ymd_opt(2024, 1, 8).unwrap());
}

#[test]
fn test_forecast_renders_without_panic() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let forecast = Forecast::new(&model, &theme);

    let area = Rect::new(0, 0, 120, 30);
    let mut buffer = Buffer::empty(area);
    forecast.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_forecast_small_area_does_not_panic() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let forecast = Forecast::new(&model, &theme);

    // Very small area - should early return without panic
    let area = Rect::new(0, 0, 30, 10);
    let mut buffer = Buffer::empty(area);
    forecast.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_daily_capacity_constant() {
    // Ensure daily capacity is 8 hours as documented
    assert_eq!(DAILY_CAPACITY_HOURS, 8);
}

#[test]
fn test_forecast_weeks_constant() {
    // Verify we forecast 8 weeks ahead
    assert_eq!(FORECAST_WEEKS, 8);
}

#[test]
fn test_weekly_workload_empty_model() {
    let model = Model::new();
    let theme = Theme::default();
    let forecast = Forecast::new(&model, &theme);
    let workload = forecast.get_weekly_workload();

    // Should have 8 weeks
    assert_eq!(workload.len(), FORECAST_WEEKS);
    // All weeks should have 0 tasks
    for (_, count, mins) in &workload {
        assert_eq!(*count, 0);
        assert_eq!(*mins, 0);
    }
}

#[test]
fn test_daily_workload_empty_model() {
    let model = Model::new();
    let theme = Theme::default();
    let forecast = Forecast::new(&model, &theme);
    let workload = forecast.get_daily_workload();

    // Should have 7 days
    assert_eq!(workload.len(), 7);
    // All days should have 0 tasks
    for (_, count, mins) in &workload {
        assert_eq!(*count, 0);
        assert_eq!(*mins, 0);
    }
}

#[test]
fn test_weekly_workload_with_tasks() {
    let mut model = Model::new();
    let today = Local::now().date_naive();

    // Add task due today
    let mut task = Task::new("Task due today");
    task.due_date = Some(today);
    task.estimated_minutes = Some(60);
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let forecast = Forecast::new(&model, &theme);
    let workload = forecast.get_weekly_workload();

    // First week should have 1 task with 60 minutes
    assert_eq!(workload[0].1, 1);
    assert_eq!(workload[0].2, 60);
}

#[test]
fn test_daily_workload_with_tasks() {
    let mut model = Model::new();
    let today = Local::now().date_naive();

    // Add task due today
    let mut task = Task::new("Task due today");
    task.due_date = Some(today);
    task.estimated_minutes = Some(120);
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let forecast = Forecast::new(&model, &theme);
    let workload = forecast.get_daily_workload();

    // First day (today) should have 1 task with 120 minutes
    assert_eq!(workload[0].1, 1);
    assert_eq!(workload[0].2, 120);
}

#[test]
fn test_default_estimate_for_tasks_without_estimate() {
    let mut model = Model::new();
    let today = Local::now().date_naive();

    // Add task without estimate
    let mut task = Task::new("Task without estimate");
    task.due_date = Some(today);
    task.estimated_minutes = None; // No estimate
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let forecast = Forecast::new(&model, &theme);
    let workload = forecast.get_daily_workload();

    // Should use default 30 minutes
    assert_eq!(workload[0].1, 1);
    assert_eq!(workload[0].2, 30);
}
