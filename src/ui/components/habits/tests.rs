//! Tests for habit tracking view component.

use super::*;
use ratatui::{buffer::Buffer, layout::Rect};

use crate::app::Model;
use crate::config::Theme;
use chrono::Utc;

#[test]
fn test_habits_view_renders_without_panic() {
    let model = Model::new();
    let theme = Theme::default();
    let view = HabitsView::new(&model, &theme);

    let area = Rect::new(0, 0, 100, 30);
    let mut buffer = Buffer::empty(area);
    view.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_habits_view_with_habits() {
    let mut model = Model::new();
    let habit = crate::domain::Habit::new("Exercise".to_string());
    model.habits.insert(habit.id, habit);
    model.refresh_visible_habits();

    let theme = Theme::default();
    let view = HabitsView::new(&model, &theme);

    let area = Rect::new(0, 0, 100, 30);
    let mut buffer = Buffer::empty(area);
    view.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_habits_view_shows_archived_title() {
    let mut model = Model::new();
    model.habit_view.show_archived = true;

    let theme = Theme::default();
    let view = HabitsView::new(&model, &theme);

    let area = Rect::new(0, 0, 100, 30);
    let mut buffer = Buffer::empty(area);
    view.render(area, &mut buffer);

    // Check that "archived" appears in the buffer
    let content: String = (0..buffer.area.width)
        .filter_map(|x| buffer.cell((x, 0)).map(ratatui::buffer::Cell::symbol))
        .collect();
    assert!(content.contains("archived"));
}

#[test]
fn test_habits_view_with_archived_habit() {
    let mut model = Model::new();
    let mut habit = crate::domain::Habit::new("Old Habit".to_string());
    habit.archived = true;
    model.habits.insert(habit.id, habit);
    model.habit_view.show_archived = true;
    model.refresh_visible_habits();

    let theme = Theme::default();
    let view = HabitsView::new(&model, &theme);

    let area = Rect::new(0, 0, 100, 30);
    let mut buffer = Buffer::empty(area);
    view.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_habits_view_with_streak() {
    let mut model = Model::new();
    let mut habit = crate::domain::Habit::new("Daily Reading".to_string());
    // Add check-ins for last 7 days to build a streak
    let today = Utc::now().date_naive();
    for i in 0..7 {
        let date = today - chrono::TimeDelta::days(i);
        habit.check_in(date, true, None);
    }
    model.habits.insert(habit.id, habit);
    model.refresh_visible_habits();

    let theme = Theme::default();
    let view = HabitsView::new(&model, &theme);

    let area = Rect::new(0, 0, 100, 30);
    let mut buffer = Buffer::empty(area);
    view.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_habits_view_completion_rate_color_success() {
    let model = Model::new();
    let theme = Theme::default();
    let view = HabitsView::new(&model, &theme);

    let color = view.completion_rate_color(0.9);
    assert_eq!(color, theme.colors.success.to_color());
}

#[test]
fn test_habits_view_completion_rate_color_accent() {
    let model = Model::new();
    let theme = Theme::default();
    let view = HabitsView::new(&model, &theme);

    let color = view.completion_rate_color(0.6);
    assert_eq!(color, theme.colors.accent.to_color());
}

#[test]
fn test_habits_view_completion_rate_color_warning() {
    let model = Model::new();
    let theme = Theme::default();
    let view = HabitsView::new(&model, &theme);

    let color = view.completion_rate_color(0.35);
    assert_eq!(color, theme.colors.warning.to_color());
}

#[test]
fn test_habits_view_completion_rate_color_danger() {
    let model = Model::new();
    let theme = Theme::default();
    let view = HabitsView::new(&model, &theme);

    let color = view.completion_rate_color(0.1);
    assert_eq!(color, theme.colors.danger.to_color());
}

#[test]
fn test_habit_analytics_popup_no_selection() {
    let model = Model::new();
    let theme = Theme::default();
    let popup = HabitAnalyticsPopup::new(&model, &theme);

    let area = Rect::new(0, 0, 60, 20);
    let mut buffer = Buffer::empty(area);
    popup.render(area, &mut buffer);

    // Should render without panic even with no selection
    assert!(buffer.area.width > 0);
}

#[test]
fn test_habit_analytics_popup_with_selection() {
    let mut model = Model::new();
    let mut habit = crate::domain::Habit::new("Meditation".to_string());
    habit.description = Some("10 minutes daily".to_string());
    let habit_id = habit.id;
    model.habits.insert(habit_id, habit);
    model.refresh_visible_habits();
    model.habit_view.selected = 0;

    let theme = Theme::default();
    let popup = HabitAnalyticsPopup::new(&model, &theme);

    let area = Rect::new(0, 0, 60, 20);
    let mut buffer = Buffer::empty(area);
    popup.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_habit_analytics_popup_with_trend_data() {
    let mut model = Model::new();
    let mut habit = crate::domain::Habit::new("Exercise".to_string());
    // Add enough check-ins to generate trend data
    let today = Utc::now().date_naive();
    for i in 0..30 {
        let date = today - chrono::TimeDelta::days(i);
        // Check in more frequently in recent days to create improving trend
        if i < 15 || i % 2 == 0 {
            habit.check_in(date, true, None);
        }
    }
    model.habits.insert(habit.id, habit);
    model.refresh_visible_habits();
    model.habit_view.selected = 0;

    let theme = Theme::default();
    let popup = HabitAnalyticsPopup::new(&model, &theme);

    let area = Rect::new(0, 0, 60, 20);
    let mut buffer = Buffer::empty(area);
    popup.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_habits_view_weekly_habit() {
    let mut model = Model::new();
    let habit = crate::domain::Habit::new("Gym".to_string()).with_frequency(
        crate::domain::HabitFrequency::Weekly {
            days: vec![
                chrono::Weekday::Mon,
                chrono::Weekday::Wed,
                chrono::Weekday::Fri,
            ],
        },
    );
    model.habits.insert(habit.id, habit);
    model.refresh_visible_habits();

    let theme = Theme::default();
    let view = HabitsView::new(&model, &theme);

    let area = Rect::new(0, 0, 100, 30);
    let mut buffer = Buffer::empty(area);
    view.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_habits_view_every_n_days_habit() {
    let mut model = Model::new();
    let habit = crate::domain::Habit::new("Deep Clean".to_string())
        .with_frequency(crate::domain::HabitFrequency::EveryNDays { n: 3 });
    model.habits.insert(habit.id, habit);
    model.refresh_visible_habits();

    let theme = Theme::default();
    let view = HabitsView::new(&model, &theme);

    let area = Rect::new(0, 0, 100, 30);
    let mut buffer = Buffer::empty(area);
    view.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}

#[test]
fn test_habits_view_narrow_area() {
    let mut model = Model::new();
    let habit = crate::domain::Habit::new("Test".to_string());
    model.habits.insert(habit.id, habit);
    model.refresh_visible_habits();

    let theme = Theme::default();
    let view = HabitsView::new(&model, &theme);

    // Very narrow area
    let area = Rect::new(0, 0, 40, 10);
    let mut buffer = Buffer::empty(area);
    view.render(area, &mut buffer);

    assert!(buffer.area.width > 0);
}
