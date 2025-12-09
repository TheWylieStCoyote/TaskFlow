//! Tests for the timeline widget.

use chrono::NaiveDate;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use super::*;
use crate::app::Model;
use crate::config::Theme;
use crate::domain::Task;

#[test]
fn test_timeline_renders_without_panic() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let timeline = Timeline::new(&model, &theme);

    let area = Rect::new(0, 0, 120, 30);
    let mut buffer = Buffer::empty(area);
    timeline.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_task_span_with_both_dates() {
    let mut task = Task::new("Test");
    task.scheduled_date = Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    task.due_date = Some(NaiveDate::from_ymd_opt(2024, 1, 5).unwrap());

    let (start, end) = Timeline::task_span(&task);
    assert_eq!(start, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    assert_eq!(end, NaiveDate::from_ymd_opt(2024, 1, 5).unwrap());
}

#[test]
fn test_task_span_milestone() {
    let mut task = Task::new("Milestone");
    task.due_date = Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());

    let (start, end) = Timeline::task_span(&task);
    assert_eq!(start, end);
}

#[test]
fn test_build_bar_string() {
    let model = Model::new();
    let theme = Theme::default();
    let timeline = Timeline::new(&model, &theme);

    assert_eq!(timeline.build_bar_string(5, false, false), "[===]");
    assert_eq!(timeline.build_bar_string(5, true, false), "<===]");
    assert_eq!(timeline.build_bar_string(5, false, true), "[===>");
    assert_eq!(timeline.build_bar_string(5, true, true), "<===>");
    assert_eq!(timeline.build_bar_string(1, false, false), "█");
}
