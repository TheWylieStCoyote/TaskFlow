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
