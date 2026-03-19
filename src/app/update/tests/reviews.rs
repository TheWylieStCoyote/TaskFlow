//! Daily and weekly review tests.

use chrono::{Duration, Utc};

use crate::app::{update::update, Message, Model, UiMessage};
use crate::domain::Task;
use crate::ui::{DailyReviewPhase, WeeklyReviewPhase};

fn create_model_with_tasks_for_review() -> Model {
    let mut model = Model::new();
    let today = Utc::now().date_naive();
    let yesterday = today - Duration::days(1);
    let tomorrow = today + Duration::days(1);

    // Overdue task
    let overdue = Task::new("Overdue task").with_due_date(yesterday);
    model.tasks.insert(overdue.id, overdue);

    // Today task
    let today_task = Task::new("Today task").with_due_date(today);
    model.tasks.insert(today_task.id, today_task);

    // Scheduled task
    let mut scheduled = Task::new("Scheduled task");
    scheduled.scheduled_date = Some(today);
    model.tasks.insert(scheduled.id, scheduled);

    // Future task
    let future = Task::new("Future task").with_due_date(tomorrow);
    model.tasks.insert(future.id, future);

    model.refresh_visible_tasks();
    model
}

// Daily review tests
#[test]
fn test_show_daily_review() {
    let mut model = Model::new();
    model.daily_review.visible = false;
    model.daily_review.selected = 5;

    update(&mut model, Message::Ui(UiMessage::ShowDailyReview));

    assert!(model.daily_review.visible);
    assert_eq!(model.daily_review.phase, DailyReviewPhase::Welcome);
    assert_eq!(model.daily_review.selected, 0);
}

#[test]
fn test_hide_daily_review() {
    let mut model = Model::new();
    model.daily_review.visible = true;

    update(&mut model, Message::Ui(UiMessage::HideDailyReview));

    assert!(!model.daily_review.visible);
}

#[test]
fn test_daily_review_next() {
    let mut model = Model::new();
    model.daily_review.visible = true;
    model.daily_review.phase = DailyReviewPhase::Welcome;
    model.daily_review.selected = 3;

    update(&mut model, Message::Ui(UiMessage::DailyReviewNext));

    assert_ne!(model.daily_review.phase, DailyReviewPhase::Welcome);
    assert_eq!(model.daily_review.selected, 0);
}

#[test]
fn test_daily_review_prev() {
    let mut model = Model::new();
    model.daily_review.visible = true;
    model.daily_review.phase = DailyReviewPhase::OverdueTasks;
    model.daily_review.selected = 3;

    update(&mut model, Message::Ui(UiMessage::DailyReviewPrev));

    assert_eq!(model.daily_review.phase, DailyReviewPhase::Welcome);
    assert_eq!(model.daily_review.selected, 0);
}

#[test]
fn test_daily_review_up() {
    let mut model = Model::new();
    model.daily_review.selected = 2;

    update(&mut model, Message::Ui(UiMessage::DailyReviewUp));

    assert_eq!(model.daily_review.selected, 1);
}

#[test]
fn test_daily_review_up_at_zero() {
    let mut model = Model::new();
    model.daily_review.selected = 0;

    update(&mut model, Message::Ui(UiMessage::DailyReviewUp));

    assert_eq!(model.daily_review.selected, 0);
}

#[test]
fn test_daily_review_down_overdue_phase() {
    let mut model = create_model_with_tasks_for_review();
    model.daily_review.visible = true;
    model.daily_review.phase = DailyReviewPhase::OverdueTasks;
    model.daily_review.selected = 0;

    update(&mut model, Message::Ui(UiMessage::DailyReviewDown));

    // Selection should stay at 0 if only 1 overdue task
    assert!(model.daily_review.selected <= 1);
}

#[test]
fn test_daily_review_down_today_phase() {
    let mut model = create_model_with_tasks_for_review();
    model.daily_review.visible = true;
    model.daily_review.phase = DailyReviewPhase::TodayTasks;
    model.daily_review.selected = 0;

    update(&mut model, Message::Ui(UiMessage::DailyReviewDown));

    // Selection should stay at 0 if only 1 today task
    assert!(model.daily_review.selected <= 1);
}

#[test]
fn test_daily_review_complete() {
    let mut model = create_model_with_tasks_for_review();
    model.daily_review.visible = true;
    model.daily_review.phase = DailyReviewPhase::OverdueTasks;
    model.daily_review.selected = 0;

    update(&mut model, Message::Ui(UiMessage::DailyReviewComplete));

    // Status message should be set
    assert!(model.alerts.status_message.is_some());
}

// Weekly review tests
#[test]
fn test_show_weekly_review() {
    let mut model = Model::new();
    model.weekly_review.visible = false;
    model.weekly_review.selected = 5;

    update(&mut model, Message::Ui(UiMessage::ShowWeeklyReview));

    assert!(model.weekly_review.visible);
    assert_eq!(model.weekly_review.phase, WeeklyReviewPhase::Welcome);
    assert_eq!(model.weekly_review.selected, 0);
}

#[test]
fn test_hide_weekly_review() {
    let mut model = Model::new();
    model.weekly_review.visible = true;

    update(&mut model, Message::Ui(UiMessage::HideWeeklyReview));

    assert!(!model.weekly_review.visible);
}

#[test]
fn test_weekly_review_next() {
    let mut model = Model::new();
    model.weekly_review.visible = true;
    model.weekly_review.phase = WeeklyReviewPhase::Welcome;
    model.weekly_review.selected = 3;

    update(&mut model, Message::Ui(UiMessage::WeeklyReviewNext));

    assert_ne!(model.weekly_review.phase, WeeklyReviewPhase::Welcome);
    assert_eq!(model.weekly_review.selected, 0);
}

#[test]
fn test_weekly_review_prev() {
    let mut model = Model::new();
    model.weekly_review.visible = true;
    model.weekly_review.phase = WeeklyReviewPhase::CompletedTasks;
    model.weekly_review.selected = 3;

    update(&mut model, Message::Ui(UiMessage::WeeklyReviewPrev));

    assert_eq!(model.weekly_review.phase, WeeklyReviewPhase::Welcome);
    assert_eq!(model.weekly_review.selected, 0);
}

#[test]
fn test_weekly_review_up() {
    let mut model = Model::new();
    model.weekly_review.selected = 2;

    update(&mut model, Message::Ui(UiMessage::WeeklyReviewUp));

    assert_eq!(model.weekly_review.selected, 1);
}

#[test]
fn test_weekly_review_up_at_zero() {
    let mut model = Model::new();
    model.weekly_review.selected = 0;

    update(&mut model, Message::Ui(UiMessage::WeeklyReviewUp));

    assert_eq!(model.weekly_review.selected, 0);
}

#[test]
fn test_weekly_review_down_completed_phase() {
    let mut model = create_model_with_tasks_for_review();
    model.weekly_review.visible = true;
    model.weekly_review.phase = WeeklyReviewPhase::CompletedTasks;
    model.weekly_review.selected = 0;

    update(&mut model, Message::Ui(UiMessage::WeeklyReviewDown));

    // Selection should not change (no completed tasks)
    assert_eq!(model.weekly_review.selected, 0);
}

#[test]
fn test_weekly_review_down_overdue_phase() {
    let mut model = create_model_with_tasks_for_review();
    model.weekly_review.visible = true;
    model.weekly_review.phase = WeeklyReviewPhase::OverdueTasks;
    model.weekly_review.selected = 0;

    update(&mut model, Message::Ui(UiMessage::WeeklyReviewDown));

    // Should stay at 0 if only one overdue
    assert!(model.weekly_review.selected <= 1);
}

#[test]
fn test_weekly_review_down_upcoming_phase() {
    let mut model = create_model_with_tasks_for_review();
    model.weekly_review.visible = true;
    model.weekly_review.phase = WeeklyReviewPhase::UpcomingWeek;
    model.weekly_review.selected = 0;

    update(&mut model, Message::Ui(UiMessage::WeeklyReviewDown));

    // Selection depends on number of upcoming tasks
    assert!(model.weekly_review.selected <= 2);
}

// Evening review tests
use crate::ui::EveningReviewPhase;

#[test]
fn test_show_evening_review() {
    let mut model = Model::new();
    model.evening_review.visible = false;
    model.evening_review.selected = 5;

    update(&mut model, Message::Ui(UiMessage::ShowEveningReview));

    assert!(model.evening_review.visible);
    assert_eq!(model.evening_review.phase, EveningReviewPhase::Welcome);
    assert_eq!(model.evening_review.selected, 0);
}

#[test]
fn test_hide_evening_review() {
    let mut model = Model::new();
    model.evening_review.visible = true;

    update(&mut model, Message::Ui(UiMessage::HideEveningReview));

    assert!(!model.evening_review.visible);
}

#[test]
fn test_evening_review_next_advances_phase() {
    let mut model = Model::new();
    model.evening_review.visible = true;
    model.evening_review.phase = EveningReviewPhase::Welcome;
    model.evening_review.selected = 3;

    update(&mut model, Message::Ui(UiMessage::EveningReviewNext));

    assert_ne!(model.evening_review.phase, EveningReviewPhase::Welcome);
    assert_eq!(model.evening_review.selected, 0);
}

#[test]
fn test_evening_review_next_skips_time_review_without_entries() {
    let mut model = Model::new();
    model.evening_review.visible = true;
    // IncompleteTasks.next() == TimeReview
    model.evening_review.phase = EveningReviewPhase::TomorrowPreview;

    update(&mut model, Message::Ui(UiMessage::EveningReviewNext));

    // Should skip TimeReview and go to Summary when no time entries
    assert_eq!(model.evening_review.phase, EveningReviewPhase::Summary);
}

#[test]
fn test_evening_review_prev_goes_back() {
    let mut model = Model::new();
    model.evening_review.visible = true;
    model.evening_review.phase = EveningReviewPhase::CompletedToday;
    model.evening_review.selected = 2;

    update(&mut model, Message::Ui(UiMessage::EveningReviewPrev));

    assert_eq!(model.evening_review.phase, EveningReviewPhase::Welcome);
    assert_eq!(model.evening_review.selected, 0);
}

#[test]
fn test_evening_review_prev_skips_time_review_without_entries() {
    let mut model = Model::new();
    model.evening_review.visible = true;
    model.evening_review.phase = EveningReviewPhase::Summary;

    update(&mut model, Message::Ui(UiMessage::EveningReviewPrev));

    // Should skip back over TimeReview to TomorrowPreview when no time entries
    assert_eq!(
        model.evening_review.phase,
        EveningReviewPhase::TomorrowPreview
    );
}

#[test]
fn test_evening_review_up() {
    let mut model = Model::new();
    model.evening_review.selected = 2;

    update(&mut model, Message::Ui(UiMessage::EveningReviewUp));

    assert_eq!(model.evening_review.selected, 1);
}

#[test]
fn test_evening_review_up_at_zero() {
    let mut model = Model::new();
    model.evening_review.selected = 0;

    update(&mut model, Message::Ui(UiMessage::EveningReviewUp));

    assert_eq!(model.evening_review.selected, 0);
}

#[test]
fn test_evening_review_down_incomplete_tasks_phase() {
    let mut model = create_model_with_tasks_for_review();
    model.evening_review.visible = true;
    model.evening_review.phase = EveningReviewPhase::IncompleteTasks;
    model.evening_review.selected = 0;

    update(&mut model, Message::Ui(UiMessage::EveningReviewDown));

    // Should not go below count - 1
    assert!(model.evening_review.selected <= 2);
}

#[test]
fn test_evening_review_reschedule() {
    let mut model = create_model_with_tasks_for_review();
    model.evening_review.visible = true;
    model.evening_review.phase = EveningReviewPhase::IncompleteTasks;
    model.evening_review.selected = 0;

    update(&mut model, Message::Ui(UiMessage::EveningReviewReschedule));

    // Status message should be set
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_evening_review_snooze() {
    let mut model = create_model_with_tasks_for_review();
    model.evening_review.visible = true;
    model.evening_review.phase = EveningReviewPhase::IncompleteTasks;
    model.evening_review.selected = 0;

    update(&mut model, Message::Ui(UiMessage::EveningReviewSnooze));

    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_evening_review_complete() {
    let mut model = create_model_with_tasks_for_review();
    model.evening_review.visible = true;
    model.evening_review.phase = EveningReviewPhase::IncompleteTasks;
    model.evening_review.selected = 0;

    update(&mut model, Message::Ui(UiMessage::EveningReviewComplete));

    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_weekly_review_down_stale_projects_phase() {
    let mut model = Model::new();
    model.weekly_review.visible = true;
    model.weekly_review.phase = WeeklyReviewPhase::StaleProjects;
    model.weekly_review.selected = 0;

    update(&mut model, Message::Ui(UiMessage::WeeklyReviewDown));

    // No stale projects → selection stays at 0
    assert_eq!(model.weekly_review.selected, 0);
}

#[test]
fn test_daily_review_down_scheduled_phase() {
    let mut model = create_model_with_tasks_for_review();
    model.daily_review.visible = true;
    model.daily_review.phase = DailyReviewPhase::ScheduledTasks;
    model.daily_review.selected = 0;

    update(&mut model, Message::Ui(UiMessage::DailyReviewDown));

    // Should stay at 0 if only 1 scheduled task
    assert!(model.daily_review.selected <= 1);
}
