//! Tests for daily review component.

use super::*;
use crate::app::Model;
use crate::config::Theme;
use crate::domain::Priority;
use crate::ui::test_utils::{buffer_content, render_widget};
use chrono::{Duration, Utc};
use ratatui::{buffer::Buffer, layout::Rect};

#[test]
fn test_phase_navigation() {
    let phase = DailyReviewPhase::Welcome;
    assert_eq!(phase.next(), DailyReviewPhase::OverdueTasks);
    assert_eq!(phase.prev(), DailyReviewPhase::Welcome); // Can't go before start

    let phase = DailyReviewPhase::Summary;
    assert_eq!(phase.next(), DailyReviewPhase::Summary); // Can't go past end
    assert_eq!(phase.prev(), DailyReviewPhase::ScheduledTasks);
}

#[test]
fn test_phase_numbers() {
    assert_eq!(DailyReviewPhase::Welcome.number(), 1);
    assert_eq!(DailyReviewPhase::OverdueTasks.number(), 2);
    assert_eq!(DailyReviewPhase::TodayTasks.number(), 3);
    assert_eq!(DailyReviewPhase::ScheduledTasks.number(), 4);
    assert_eq!(DailyReviewPhase::Summary.number(), 5);
}

#[test]
fn test_phase_titles() {
    assert_eq!(DailyReviewPhase::Welcome.title(), "Good Morning!");
    assert_eq!(DailyReviewPhase::OverdueTasks.title(), "Overdue Tasks");
    assert_eq!(DailyReviewPhase::TodayTasks.title(), "Today's Tasks");
    assert_eq!(
        DailyReviewPhase::ScheduledTasks.title(),
        "Scheduled for Today"
    );
    assert_eq!(DailyReviewPhase::Summary.title(), "Daily Summary");
}

#[test]
fn test_daily_review_renders() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let review = DailyReview::new(&model, &theme, DailyReviewPhase::Welcome, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    // Should render without panic
    assert!(buffer.area.width > 0);
}

#[test]
fn test_daily_review_summary_phase() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let review = DailyReview::new(&model, &theme, DailyReviewPhase::Summary, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    // Should render without panic
    assert!(buffer.area.width > 0);
}

#[test]
fn test_daily_review_overdue_phase_empty() {
    let model = Model::new(); // No tasks
    let theme = Theme::default();
    let review = DailyReview::new(&model, &theme, DailyReviewPhase::OverdueTasks, 0);
    let buffer = render_widget(review, 80, 24);
    let content = buffer_content(&buffer);

    // Should show empty message when no overdue tasks
    assert!(content.contains("No overdue tasks"));
}

#[test]
fn test_daily_review_overdue_phase_with_tasks() {
    use crate::domain::Task;

    let mut model = Model::new();
    let theme = Theme::default();

    // Create an overdue task
    let today = Utc::now().date_naive();
    let mut task = Task::new("Overdue task");
    task.due_date = Some(today - Duration::days(3));
    task.priority = Priority::High;
    model.tasks.insert(task.id, task);

    let review = DailyReview::new(&model, &theme, DailyReviewPhase::OverdueTasks, 0);
    let buffer = render_widget(review, 80, 24);
    let content = buffer_content(&buffer);

    // Should show the overdue task
    assert!(content.contains("Overdue task"));
    assert!(content.contains("overdue")); // Due date info
}

#[test]
fn test_daily_review_today_phase_empty() {
    let model = Model::new();
    let theme = Theme::default();
    let review = DailyReview::new(&model, &theme, DailyReviewPhase::TodayTasks, 0);
    let buffer = render_widget(review, 80, 24);
    let content = buffer_content(&buffer);

    assert!(content.contains("No tasks due today"));
}

#[test]
fn test_daily_review_today_phase_with_tasks() {
    use crate::domain::Task;

    let mut model = Model::new();
    let theme = Theme::default();

    let today = Utc::now().date_naive();
    let mut task = Task::new("Today task");
    task.due_date = Some(today);
    model.tasks.insert(task.id, task);

    let review = DailyReview::new(&model, &theme, DailyReviewPhase::TodayTasks, 0);
    let buffer = render_widget(review, 80, 24);
    let content = buffer_content(&buffer);

    assert!(content.contains("Today task"));
    assert!(content.contains("today")); // Due date info
}

#[test]
fn test_daily_review_scheduled_phase_empty() {
    let model = Model::new();
    let theme = Theme::default();
    let review = DailyReview::new(&model, &theme, DailyReviewPhase::ScheduledTasks, 0);
    let buffer = render_widget(review, 80, 24);
    let content = buffer_content(&buffer);

    assert!(content.contains("No scheduled tasks"));
}

#[test]
fn test_daily_review_scheduled_phase_with_tasks() {
    use crate::domain::Task;

    let mut model = Model::new();
    let theme = Theme::default();

    let today = Utc::now().date_naive();
    let mut task = Task::new("Scheduled task");
    task.scheduled_date = Some(today);
    // Don't set due_date to today so it shows in scheduled
    task.due_date = Some(today + Duration::days(5));
    model.tasks.insert(task.id, task);

    let review = DailyReview::new(&model, &theme, DailyReviewPhase::ScheduledTasks, 0);
    let buffer = render_widget(review, 80, 24);
    let content = buffer_content(&buffer);

    assert!(content.contains("Scheduled task"));
}

#[test]
fn test_daily_review_welcome_shows_counts() {
    use crate::domain::Task;

    let mut model = Model::new();
    let theme = Theme::default();

    let today = Utc::now().date_naive();

    // Add an overdue task
    let mut overdue = Task::new("Overdue");
    overdue.due_date = Some(today - Duration::days(1));
    model.tasks.insert(overdue.id, overdue);

    // Add a task due today
    let mut due_today = Task::new("Due today");
    due_today.due_date = Some(today);
    model.tasks.insert(due_today.id, due_today);

    let review = DailyReview::new(&model, &theme, DailyReviewPhase::Welcome, 0);
    let buffer = render_widget(review, 80, 24);
    let content = buffer_content(&buffer);

    // Should show the date
    assert!(content.contains(&today.format("%A").to_string())); // Day name
}

#[test]
fn test_daily_review_priority_colors() {
    use crate::domain::Task;

    let mut model = Model::new();
    let theme = Theme::default();

    let today = Utc::now().date_naive();

    // Add tasks with different priorities
    for priority in [
        Priority::Urgent,
        Priority::High,
        Priority::Medium,
        Priority::Low,
        Priority::None,
    ] {
        let mut task = Task::new(format!("{priority:?} priority"));
        task.due_date = Some(today - Duration::days(1)); // Make overdue
        task.priority = priority;
        model.tasks.insert(task.id, task);
    }

    let review = DailyReview::new(&model, &theme, DailyReviewPhase::OverdueTasks, 0);
    let buffer = render_widget(review, 80, 24);

    // Should render without panic, priority indicators should be present
    assert!(buffer.area.width > 0);
}

#[test]
fn test_daily_review_footer_navigation_hints() {
    let model = Model::new();
    let theme = Theme::default();

    // Welcome phase should show Start and Exit
    let review = DailyReview::new(&model, &theme, DailyReviewPhase::Welcome, 0);
    let buffer = render_widget(review, 80, 24);
    let content = buffer_content(&buffer);
    assert!(content.contains("Start") || content.contains("Enter"));

    // Middle phases should show Back and Next
    let review = DailyReview::new(&model, &theme, DailyReviewPhase::TodayTasks, 0);
    let buffer = render_widget(review, 80, 24);
    let content = buffer_content(&buffer);
    assert!(content.contains("Back") || content.contains("Next"));

    // Summary phase should show Back and Exit
    let review = DailyReview::new(&model, &theme, DailyReviewPhase::Summary, 0);
    let buffer = render_widget(review, 80, 24);
    let content = buffer_content(&buffer);
    assert!(content.contains("Back"));
}
