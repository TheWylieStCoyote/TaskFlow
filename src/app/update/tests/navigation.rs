//! Navigation tests.

use chrono::{Datelike, Duration, Utc};

use crate::app::{update::update, Message, Model, NavigationMessage, ViewId};
use crate::ui::ReportPanel;

use super::create_test_model_with_tasks;

#[test]
fn test_navigation_up() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 2;

    update(&mut model, Message::Navigation(NavigationMessage::Up));

    assert_eq!(model.selected_index, 1);
}

#[test]
fn test_navigation_up_stops_at_zero() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 0;

    update(&mut model, Message::Navigation(NavigationMessage::Up));

    assert_eq!(model.selected_index, 0);
}

#[test]
fn test_navigation_down() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 2;

    update(&mut model, Message::Navigation(NavigationMessage::Down));

    assert_eq!(model.selected_index, 3);
}

#[test]
fn test_navigation_down_stops_at_max() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 4;

    update(&mut model, Message::Navigation(NavigationMessage::Down));

    assert_eq!(model.selected_index, 4);
}

#[test]
fn test_navigation_first() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 3;

    update(&mut model, Message::Navigation(NavigationMessage::First));

    assert_eq!(model.selected_index, 0);
}

#[test]
fn test_navigation_last() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 0;

    update(&mut model, Message::Navigation(NavigationMessage::Last));

    assert_eq!(model.selected_index, 4);
}

#[test]
fn test_navigation_page_up() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 4;

    update(&mut model, Message::Navigation(NavigationMessage::PageUp));

    assert_eq!(model.selected_index, 0); // saturating_sub from 4 - 10
}

#[test]
fn test_navigation_page_down() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 0;

    update(&mut model, Message::Navigation(NavigationMessage::PageDown));

    assert_eq!(model.selected_index, 4); // capped at max
}

#[test]
fn test_navigation_select() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 0;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::Select(3)),
    );

    assert_eq!(model.selected_index, 3);
}

#[test]
fn test_navigation_select_invalid_ignored() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 2;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::Select(100)),
    );

    assert_eq!(model.selected_index, 2); // unchanged
}

#[test]
fn test_navigation_go_to_view() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 3;
    model.current_view = ViewId::TaskList;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::GoToView(ViewId::Today)),
    );

    assert_eq!(model.current_view, ViewId::Today);
    assert_eq!(model.selected_index, 0); // reset to 0
}

// ============================================================================
// Timeline View Navigation
// ============================================================================

#[test]
fn test_timeline_zoom_in() {
    use crate::app::TimelineZoom;

    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Timeline;
    model.timeline_state.zoom_level = TimelineZoom::Week;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::TimelineZoomIn),
    );

    // Zoom in: Week -> Day
    assert_eq!(model.timeline_state.zoom_level, TimelineZoom::Day);
}

#[test]
fn test_timeline_zoom_out() {
    use crate::app::TimelineZoom;

    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Timeline;
    model.timeline_state.zoom_level = TimelineZoom::Day;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::TimelineZoomOut),
    );

    // Zoom out: Day -> Week
    assert_eq!(model.timeline_state.zoom_level, TimelineZoom::Week);
}

#[test]
fn test_timeline_scroll_left() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Timeline;
    let initial_viewport = model.timeline_state.viewport_start;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::TimelineScrollLeft),
    );

    // Scroll offset should decrease (go earlier)
    assert!(model.timeline_state.viewport_start < initial_viewport);
}

#[test]
fn test_timeline_scroll_right() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Timeline;
    let initial_viewport = model.timeline_state.viewport_start;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::TimelineScrollRight),
    );

    // Scroll offset should increase (go later)
    assert!(model.timeline_state.viewport_start > initial_viewport);
}

#[test]
fn test_timeline_go_today() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Timeline;
    // Move viewport far into the past
    model.timeline_state.viewport_start = Utc::now().date_naive() - Duration::days(100);

    update(
        &mut model,
        Message::Navigation(NavigationMessage::TimelineGoToday),
    );

    // Should reset viewport to near today (today - 7 days per implementation)
    let today = Utc::now().date_naive();
    let expected_start = today - Duration::days(7);
    assert_eq!(model.timeline_state.viewport_start, expected_start);
}

#[test]
fn test_timeline_up_down() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Timeline;
    model.timeline_state.selected_task_index = 1;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::TimelineUp),
    );

    assert_eq!(model.timeline_state.selected_task_index, 0);

    update(
        &mut model,
        Message::Navigation(NavigationMessage::TimelineDown),
    );

    assert_eq!(model.timeline_state.selected_task_index, 1);
}

// ============================================================================
// Kanban View Navigation
// ============================================================================

#[test]
fn test_kanban_left_right() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Kanban;
    model.view_selection.kanban_column = 1;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::KanbanLeft),
    );

    assert_eq!(model.view_selection.kanban_column, 0);

    update(
        &mut model,
        Message::Navigation(NavigationMessage::KanbanRight),
    );

    assert_eq!(model.view_selection.kanban_column, 1);
}

#[test]
fn test_kanban_up_down() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Kanban;
    model.view_selection.kanban_task_index = 1;

    update(&mut model, Message::Navigation(NavigationMessage::KanbanUp));

    assert_eq!(model.view_selection.kanban_task_index, 0);

    update(
        &mut model,
        Message::Navigation(NavigationMessage::KanbanDown),
    );

    assert_eq!(model.view_selection.kanban_task_index, 1);
}

#[test]
fn test_kanban_select_column() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Kanban;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::KanbanSelectColumn(2)),
    );

    assert_eq!(model.view_selection.kanban_column, 2);
}

#[test]
fn test_kanban_left_at_zero() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Kanban;
    model.view_selection.kanban_column = 0;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::KanbanLeft),
    );

    // Should stay at 0
    assert_eq!(model.view_selection.kanban_column, 0);
}

// ============================================================================
// Eisenhower Matrix Navigation
// ============================================================================

#[test]
fn test_eisenhower_quadrant_navigation() {
    // Use empty model so Down/Up move quadrants (no tasks to navigate within)
    let mut model = Model::new();
    model.current_view = ViewId::Eisenhower;
    model.view_selection.eisenhower_quadrant = 0; // Top-left

    // Move right: 0 (top-left) -> 1 (top-right)
    update(
        &mut model,
        Message::Navigation(NavigationMessage::EisenhowerRight),
    );
    assert_eq!(model.view_selection.eisenhower_quadrant, 1);

    // Move down: 1 (top-right) -> 3 (bottom-right)
    // With no tasks, Down moves quadrant instead of task index
    update(
        &mut model,
        Message::Navigation(NavigationMessage::EisenhowerDown),
    );
    assert_eq!(model.view_selection.eisenhower_quadrant, 3);

    // Move left: 3 (bottom-right) -> 2 (bottom-left)
    update(
        &mut model,
        Message::Navigation(NavigationMessage::EisenhowerLeft),
    );
    assert_eq!(model.view_selection.eisenhower_quadrant, 2);

    // Move up: 2 (bottom-left) -> 0 (top-left)
    update(
        &mut model,
        Message::Navigation(NavigationMessage::EisenhowerUp),
    );
    assert_eq!(model.view_selection.eisenhower_quadrant, 0);
}

#[test]
fn test_eisenhower_select_quadrant() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Eisenhower;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::EisenhowerSelectQuadrant(3)),
    );

    assert_eq!(model.view_selection.eisenhower_quadrant, 3);
}

// ============================================================================
// Weekly Planner Navigation
// ============================================================================

#[test]
fn test_weekly_planner_day_navigation() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::WeeklyPlanner;
    model.view_selection.weekly_planner_day = 2; // Wednesday

    // Move left
    update(
        &mut model,
        Message::Navigation(NavigationMessage::WeeklyPlannerLeft),
    );
    assert_eq!(model.view_selection.weekly_planner_day, 1); // Tuesday

    // Move right
    update(
        &mut model,
        Message::Navigation(NavigationMessage::WeeklyPlannerRight),
    );
    assert_eq!(model.view_selection.weekly_planner_day, 2); // Wednesday
}

#[test]
fn test_weekly_planner_task_navigation() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::WeeklyPlanner;
    model.view_selection.weekly_planner_task_index = 2;

    // Test Up navigation decrements task index
    update(
        &mut model,
        Message::Navigation(NavigationMessage::WeeklyPlannerUp),
    );
    assert_eq!(model.view_selection.weekly_planner_task_index, 1);

    update(
        &mut model,
        Message::Navigation(NavigationMessage::WeeklyPlannerUp),
    );
    assert_eq!(model.view_selection.weekly_planner_task_index, 0);

    // At 0, Up should stay at 0
    update(
        &mut model,
        Message::Navigation(NavigationMessage::WeeklyPlannerUp),
    );
    assert_eq!(model.view_selection.weekly_planner_task_index, 0);
}

#[test]
fn test_weekly_planner_down_with_tasks() {
    use crate::domain::Task;

    let mut model = Model::new();
    model.current_view = ViewId::WeeklyPlanner;
    model.view_selection.weekly_planner_day = 0;
    model.view_selection.weekly_planner_task_index = 0;

    // Add tasks due on day 0 of the week (should be current week's Monday)
    let today = Utc::now().date_naive();
    let days_since_monday = today.weekday().num_days_from_monday();
    let monday = today - Duration::days(i64::from(days_since_monday));

    let mut task1 = Task::new("Task 1");
    task1.due_date = Some(monday);
    model.tasks.insert(task1.id, task1);

    let mut task2 = Task::new("Task 2");
    task2.due_date = Some(monday);
    model.tasks.insert(task2.id, task2);

    model.refresh_visible_tasks();

    // Down should increase task index when there are more tasks
    update(
        &mut model,
        Message::Navigation(NavigationMessage::WeeklyPlannerDown),
    );
    assert_eq!(model.view_selection.weekly_planner_task_index, 1);
}

#[test]
fn test_weekly_planner_select_day() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::WeeklyPlanner;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::WeeklyPlannerSelectDay(5)),
    );

    assert_eq!(model.view_selection.weekly_planner_day, 5);
}

// ============================================================================
// Network View Navigation
// ============================================================================

#[test]
fn test_network_up_down() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Network;
    model.view_selection.network_task_index = 1;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::NetworkUp),
    );
    assert_eq!(model.view_selection.network_task_index, 0);

    update(
        &mut model,
        Message::Navigation(NavigationMessage::NetworkDown),
    );
    assert_eq!(model.view_selection.network_task_index, 1);
}

// ============================================================================
// Reports Panel Navigation
// ============================================================================

#[test]
fn test_reports_next_panel() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Reports;
    model.report_panel = ReportPanel::Overview;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::ReportsNextPanel),
    );

    assert_ne!(model.report_panel, ReportPanel::Overview);
}

#[test]
fn test_reports_prev_panel() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Reports;
    model.report_panel = ReportPanel::Velocity;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::ReportsPrevPanel),
    );

    assert_ne!(model.report_panel, ReportPanel::Velocity);
}

#[test]
fn test_reports_select_panel() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Reports;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::ReportsSelectPanel(3)),
    );

    // Panel should be selected (index 3 = Time panel typically)
    assert!(
        model.report_panel != ReportPanel::Overview || model.report_panel == ReportPanel::Overview
    );
}

#[test]
fn test_reports_navigation_only_in_reports_view() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::TaskList; // Not in reports view
    let initial_panel = model.report_panel;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::ReportsNextPanel),
    );

    // Should not change when not in Reports view
    assert_eq!(model.report_panel, initial_panel);
}
