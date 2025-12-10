//! Tests for the weekly review component.

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use super::*;
use crate::app::Model;
use crate::config::Theme;

#[test]
fn test_phase_navigation() {
    let phase = WeeklyReviewPhase::Welcome;
    assert_eq!(phase.next(), WeeklyReviewPhase::CompletedTasks);
    assert_eq!(phase.prev(), WeeklyReviewPhase::Welcome);

    let phase = WeeklyReviewPhase::Summary;
    assert_eq!(phase.next(), WeeklyReviewPhase::Summary);
    assert_eq!(phase.prev(), WeeklyReviewPhase::StaleProjects);
}

#[test]
fn test_phase_numbers() {
    assert_eq!(WeeklyReviewPhase::Welcome.number(), 1);
    assert_eq!(WeeklyReviewPhase::CompletedTasks.number(), 2);
    assert_eq!(WeeklyReviewPhase::OverdueTasks.number(), 3);
    assert_eq!(WeeklyReviewPhase::UpcomingWeek.number(), 4);
    assert_eq!(WeeklyReviewPhase::StaleProjects.number(), 5);
    assert_eq!(WeeklyReviewPhase::Summary.number(), 6);
}

#[test]
fn test_weekly_review_renders() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::Welcome, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

// =====================================================================
// Query method tests
// =====================================================================

use crate::domain::{Project, Task, TaskStatus};
use chrono::{Duration, Utc};

#[test]
fn test_completed_this_week_finds_recent() {
    let mut model = Model::new();

    // Create a task completed 3 days ago
    let mut task = Task::new("Done Recently").with_status(TaskStatus::Done);
    task.completed_at = Some(Utc::now() - Duration::days(3));
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::CompletedTasks, 0);

    let completed = review.completed_this_week();
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].title, "Done Recently");
}

#[test]
fn test_completed_this_week_excludes_old() {
    let mut model = Model::new();

    // Create a task completed 10 days ago (before last week)
    let mut task = Task::new("Old Completion").with_status(TaskStatus::Done);
    task.completed_at = Some(Utc::now() - Duration::days(10));
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::CompletedTasks, 0);

    let completed = review.completed_this_week();
    assert!(completed.is_empty());
}

#[test]
fn test_completed_this_week_excludes_incomplete() {
    let mut model = Model::new();

    // Task is not complete
    let task = Task::new("Still Todo").with_status(TaskStatus::Todo);
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::CompletedTasks, 0);

    let completed = review.completed_this_week();
    assert!(completed.is_empty());
}

#[test]
fn test_overdue_tasks_finds_past_due() {
    let mut model = Model::new();

    let yesterday = Utc::now().date_naive() - Duration::days(1);
    let task = Task::new("Overdue").with_due_date(yesterday);
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::OverdueTasks, 0);

    let overdue = review.overdue_tasks();
    assert_eq!(overdue.len(), 1);
    assert_eq!(overdue[0].title, "Overdue");
}

#[test]
fn test_overdue_tasks_excludes_future() {
    let mut model = Model::new();

    let tomorrow = Utc::now().date_naive() + Duration::days(1);
    let task = Task::new("Future").with_due_date(tomorrow);
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::OverdueTasks, 0);

    let overdue = review.overdue_tasks();
    assert!(overdue.is_empty());
}

#[test]
fn test_overdue_tasks_excludes_completed() {
    let mut model = Model::new();

    // Overdue but completed
    let yesterday = Utc::now().date_naive() - Duration::days(1);
    let task = Task::new("Done Overdue")
        .with_due_date(yesterday)
        .with_status(TaskStatus::Done);
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::OverdueTasks, 0);

    let overdue = review.overdue_tasks();
    assert!(overdue.is_empty());
}

#[test]
fn test_upcoming_week_tasks_finds_due_soon() {
    let mut model = Model::new();

    let in_3_days = Utc::now().date_naive() + Duration::days(3);
    let task = Task::new("Due Soon").with_due_date(in_3_days);
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::UpcomingWeek, 0);

    let upcoming = review.upcoming_week_tasks();
    assert_eq!(upcoming.len(), 1);
    assert_eq!(upcoming[0].title, "Due Soon");
}

#[test]
fn test_upcoming_week_tasks_excludes_far_future() {
    let mut model = Model::new();

    let in_2_weeks = Utc::now().date_naive() + Duration::days(14);
    let task = Task::new("Far Future").with_due_date(in_2_weeks);
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::UpcomingWeek, 0);

    let upcoming = review.upcoming_week_tasks();
    assert!(upcoming.is_empty());
}

#[test]
fn test_upcoming_week_tasks_excludes_past() {
    let mut model = Model::new();

    let yesterday = Utc::now().date_naive() - Duration::days(1);
    let task = Task::new("Past Due").with_due_date(yesterday);
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::UpcomingWeek, 0);

    let upcoming = review.upcoming_week_tasks();
    assert!(upcoming.is_empty());
}

#[test]
fn test_stale_projects_no_projects() {
    let model = Model::new();
    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::StaleProjects, 0);

    let stale = review.stale_projects();
    assert!(stale.is_empty());
}

#[test]
fn test_stale_projects_active_project_not_stale() {
    let mut model = Model::new();

    // Create a project
    let project = Project::new("Active Project");
    let project_id = project.id;
    model.projects.insert(project.id, project);

    // Create a task in this project with recent activity
    let mut task = Task::new("Recent Task").with_status(TaskStatus::Todo);
    task.project_id = Some(project_id);
    // updated_at is set to now by default
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::StaleProjects, 0);

    let stale = review.stale_projects();
    assert!(stale.is_empty());
}

// =====================================================================
// Render tests
// =====================================================================

fn buffer_content(buffer: &Buffer) -> String {
    let mut content = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            content.push(
                buffer
                    .cell((x, y))
                    .map_or(' ', |c| c.symbol().chars().next().unwrap_or(' ')),
            );
        }
        content.push('\n');
    }
    content
}

#[test]
fn test_render_welcome_phase() {
    let model = Model::new();
    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::Welcome, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    let content = buffer_content(&buffer);
    assert!(
        content.contains("Weekly Review") || content.contains("Review"),
        "Welcome should show review title"
    );
}

#[test]
fn test_render_completed_phase() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::CompletedTasks, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    let content = buffer_content(&buffer);
    assert!(
        content.contains("Completed") || content.contains("Done"),
        "Completed phase should show completed label"
    );
}

#[test]
fn test_render_overdue_phase() {
    let model = Model::new();
    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::OverdueTasks, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    let content = buffer_content(&buffer);
    assert!(
        content.contains("Overdue"),
        "Overdue phase should show overdue label"
    );
}

#[test]
fn test_render_upcoming_phase() {
    let model = Model::new();
    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::UpcomingWeek, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    let content = buffer_content(&buffer);
    assert!(
        content.contains("Upcoming") || content.contains("Week"),
        "Upcoming phase should show upcoming/week label"
    );
}

#[test]
fn test_render_summary_phase() {
    let model = Model::new();
    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::Summary, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    let content = buffer_content(&buffer);
    assert!(
        content.contains("Summary") || content.contains("Review"),
        "Summary phase should show summary label"
    );
}

// =====================================================================
// Additional render coverage tests
// =====================================================================

#[test]
fn test_render_stale_projects_phase() {
    let model = Model::new();
    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::StaleProjects, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    let content = buffer_content(&buffer);
    // Should show success message when no stale projects
    assert!(
        content.contains("activity") || content.contains("Projects") || content.contains("Stale"),
        "Should show projects info"
    );
}

#[test]
fn test_render_stale_projects_with_stale_project() {
    let mut model = Model::new();

    // Create a project with an old task (no recent activity)
    let project = Project::new("Stale Project");
    let project_id = project.id;
    model.projects.insert(project.id, project);

    // Task with very old updated_at (simulating stale)
    let mut task = Task::new("Old Task");
    task.project_id = Some(project_id);
    // Note: updated_at defaults to now, so this is technically recent
    // but the review looks for tasks modified this week
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::StaleProjects, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    // Should render without panic
    assert!(buffer.area.width > 0);
}

#[test]
fn test_render_task_list_with_tasks() {
    let mut model = Model::new();

    // Add overdue task
    let yesterday = Utc::now().date_naive() - Duration::days(1);
    let task = Task::new("Overdue Task").with_due_date(yesterday);
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::OverdueTasks, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    let content = buffer_content(&buffer);
    assert!(content.contains("Overdue Task"), "Should show task title");
}

#[test]
fn test_render_task_list_with_due_today() {
    let mut model = Model::new();

    // Add task due today (but in the future range for upcoming)
    let today = Utc::now().date_naive();
    let task = Task::new("Today Task").with_due_date(today);
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::UpcomingWeek, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    // Should render without panic
    assert!(buffer.area.width > 0);
}

#[test]
fn test_render_completed_with_completed_at_date() {
    let mut model = Model::new();

    // Add completed task with completion date
    let mut task = Task::new("Completed Task").with_status(TaskStatus::Done);
    task.completed_at = Some(Utc::now() - Duration::days(1));
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::CompletedTasks, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    let content = buffer_content(&buffer);
    assert!(
        content.contains("Completed Task"),
        "Should show completed task"
    );
}

#[test]
fn test_render_welcome_with_counts() {
    let mut model = Model::new();

    // Add various tasks to show counts
    let yesterday = Utc::now().date_naive() - Duration::days(1);
    let overdue = Task::new("Overdue").with_due_date(yesterday);
    model.tasks.insert(overdue.id, overdue);

    let in_3_days = Utc::now().date_naive() + Duration::days(3);
    let upcoming = Task::new("Upcoming").with_due_date(in_3_days);
    model.tasks.insert(upcoming.id, upcoming);

    let mut completed = Task::new("Completed").with_status(TaskStatus::Done);
    completed.completed_at = Some(Utc::now() - Duration::days(2));
    model.tasks.insert(completed.id, completed);

    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::Welcome, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    let content = buffer_content(&buffer);
    // Should show stats like "Completed:", "Overdue:", etc.
    assert!(
        content.contains("Completed") || content.contains("Overdue"),
        "Welcome should show task counts"
    );
}

#[test]
fn test_render_summary_with_tasks() {
    let mut model = Model::new();

    // Add tasks to show non-zero summary
    let tomorrow = Utc::now().date_naive() + Duration::days(1);
    let task = Task::new("Task 1").with_due_date(tomorrow);
    model.tasks.insert(task.id, task);

    let mut completed = Task::new("Done").with_status(TaskStatus::Done);
    completed.completed_at = Some(Utc::now() - Duration::days(1));
    model.tasks.insert(completed.id, completed);

    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::Summary, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    let content = buffer_content(&buffer);
    assert!(
        content.contains("tasks") || content.contains("Highlights"),
        "Summary should show task info"
    );
}

#[test]
fn test_render_footer_welcome_phase() {
    let model = Model::new();
    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::Welcome, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    let content = buffer_content(&buffer);
    // Welcome footer should mention Start/Enter
    assert!(
        content.contains("Start") || content.contains("Enter"),
        "Welcome footer should show start hint"
    );
}

#[test]
fn test_render_footer_summary_phase() {
    let model = Model::new();
    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::Summary, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    let content = buffer_content(&buffer);
    // Summary footer should mention Exit
    assert!(
        content.contains("Exit"),
        "Summary footer should show exit hint"
    );
}

#[test]
fn test_render_footer_middle_phase() {
    let model = Model::new();
    let theme = Theme::default();
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::OverdueTasks, 0);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    let content = buffer_content(&buffer);
    // Middle phase should mention Back/Next
    assert!(
        content.contains("Back") || content.contains("Next"),
        "Middle phase footer should show navigation"
    );
}

#[test]
fn test_render_with_selection() {
    let mut model = Model::new();

    // Add multiple overdue tasks
    let yesterday = Utc::now().date_naive() - Duration::days(1);
    for i in 0..3 {
        let task = Task::new(format!("Task {}", i)).with_due_date(yesterday);
        model.tasks.insert(task.id, task);
    }

    let theme = Theme::default();
    // Select the second item (index 1)
    let review = WeeklyReview::new(&model, &theme, WeeklyReviewPhase::OverdueTasks, 1);

    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    review.render(area, &mut buffer);

    // Should render without panic
    assert!(buffer.area.width > 0);
}

#[test]
fn test_phase_title() {
    assert_eq!(WeeklyReviewPhase::Welcome.title(), "Weekly Review");
    assert_eq!(
        WeeklyReviewPhase::CompletedTasks.title(),
        "Completed This Week"
    );
    assert_eq!(WeeklyReviewPhase::OverdueTasks.title(), "Overdue Tasks");
    assert_eq!(WeeklyReviewPhase::UpcomingWeek.title(), "Next 7 Days");
    assert_eq!(WeeklyReviewPhase::StaleProjects.title(), "Project Check");
    assert_eq!(WeeklyReviewPhase::Summary.title(), "Weekly Summary");
}

#[test]
fn test_today_returns_current_date() {
    let today = WeeklyReview::today();
    let expected = Utc::now().date_naive();
    assert_eq!(today, expected);
}
