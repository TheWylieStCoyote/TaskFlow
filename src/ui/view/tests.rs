//! Tests for the view module.

use super::{footer, layout, view};
use crate::app::{Model, ViewId};
use crate::config::Theme;
use crate::ui::components::{InputMode, InputTarget};
use ratatui::{backend::TestBackend, layout::Rect, Terminal};

// =========================================================================
// Helper functions
// =========================================================================

fn create_test_terminal(width: u16, height: u16) -> Terminal<TestBackend> {
    let backend = TestBackend::new(width, height);
    Terminal::new(backend).unwrap()
}

fn buffer_content(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
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

// =========================================================================
// get_view_hint tests
// =========================================================================

#[test]
fn test_get_view_hint_focus_mode() {
    let mut model = Model::new();
    model.focus_mode = true;

    let hint = footer::get_view_hint(&model);
    assert_eq!(hint, Some("[/]: chain | t: timer | f: exit"));
}

#[test]
fn test_get_view_hint_kanban() {
    let mut model = Model::new();
    model.current_view = ViewId::Kanban;

    let hint = footer::get_view_hint(&model);
    assert_eq!(hint, Some("h/l: columns | j/k: tasks"));
}

#[test]
fn test_get_view_hint_eisenhower() {
    let mut model = Model::new();
    model.current_view = ViewId::Eisenhower;

    let hint = footer::get_view_hint(&model);
    assert_eq!(hint, Some("h/l/j/k: quadrants"));
}

#[test]
fn test_get_view_hint_weekly_planner() {
    let mut model = Model::new();
    model.current_view = ViewId::WeeklyPlanner;

    let hint = footer::get_view_hint(&model);
    assert_eq!(hint, Some("h/l: days | j/k: tasks"));
}

#[test]
fn test_get_view_hint_timeline() {
    let mut model = Model::new();
    model.current_view = ViewId::Timeline;

    let hint = footer::get_view_hint(&model);
    assert_eq!(hint, Some("h/l: scroll | </>: zoom | t: today"));
}

#[test]
fn test_get_view_hint_network() {
    let mut model = Model::new();
    model.current_view = ViewId::Network;

    let hint = footer::get_view_hint(&model);
    assert_eq!(hint, Some("h/l/j/k: navigate"));
}

#[test]
fn test_get_view_hint_habits() {
    let mut model = Model::new();
    model.current_view = ViewId::Habits;

    let hint = footer::get_view_hint(&model);
    assert_eq!(hint, Some("n: new | Space: check-in"));
}

#[test]
fn test_get_view_hint_calendar() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;

    let hint = footer::get_view_hint(&model);
    assert_eq!(hint, Some("h/l: months | Enter: day tasks"));
}

#[test]
fn test_get_view_hint_view_only_returns_none() {
    let mut model = Model::new();

    model.current_view = ViewId::Heatmap;
    assert_eq!(footer::get_view_hint(&model), None);

    model.current_view = ViewId::Forecast;
    assert_eq!(footer::get_view_hint(&model), None);

    model.current_view = ViewId::Burndown;
    assert_eq!(footer::get_view_hint(&model), None);
}

#[test]
fn test_get_view_hint_task_list_returns_none() {
    let mut model = Model::new();
    model.current_view = ViewId::TaskList;

    let hint = footer::get_view_hint(&model);
    assert_eq!(hint, None);
}

// =========================================================================
// view function tests
// =========================================================================

#[test]
fn test_view_renders_without_panic() {
    let model = Model::new();
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_view_renders_header() {
    let model = Model::new();
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("TaskFlow"),
        "Header should contain TaskFlow"
    );
}

#[test]
fn test_view_renders_footer_task_count() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(content.contains("tasks"), "Footer should show task count");
}

#[test]
fn test_view_renders_help_popup() {
    let mut model = Model::new();
    model.show_help = true;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(100, 40);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("Help") || content.contains("Keybindings"),
        "Should show help popup"
    );
}

#[test]
fn test_view_renders_confirm_delete_dialog() {
    let mut model = Model::new().with_sample_data();
    model.show_confirm_delete = true;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("Delete") || content.contains("y/n"),
        "Should show delete confirmation"
    );
}

// =========================================================================
// render_header tests
// =========================================================================

#[test]
fn test_render_header_contains_title() {
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 3);

    terminal
        .draw(|frame| {
            let area = frame.area();
            layout::render_header(frame, area, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(content.contains("TaskFlow"));
    assert!(content.contains("Project Management"));
}

// =========================================================================
// render_footer tests
// =========================================================================

#[test]
fn test_render_footer_shows_task_count() {
    let model = Model::new();
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 1);

    terminal
        .draw(|frame| {
            let area = frame.area();
            footer::render_footer(&model, frame, area, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(content.contains("tasks"));
    assert!(content.contains("completed"));
}

#[test]
fn test_render_footer_shows_error_message() {
    let mut model = Model::new();
    model.alerts.error_message = Some("Test error message".to_string());
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 1);

    terminal
        .draw(|frame| {
            let area = frame.area();
            footer::render_footer(&model, frame, area, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("Test error message"),
        "Should show error message"
    );
}

#[test]
fn test_render_footer_shows_status_message() {
    let mut model = Model::new();
    model.alerts.status_message = Some("Status update".to_string());
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 1);

    terminal
        .draw(|frame| {
            let area = frame.area();
            footer::render_footer(&model, frame, area, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("Status update"),
        "Should show status message"
    );
}

#[test]
fn test_render_footer_shows_recording_indicator() {
    let mut model = Model::new();
    model.macro_state.start_recording(0); // Start recording in slot 0
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 1);

    terminal
        .draw(|frame| {
            let area = frame.area();
            footer::render_footer(&model, frame, area, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(content.contains("REC"), "Should show recording indicator");
}

#[test]
fn test_render_footer_shows_overdue_count() {
    let mut model = Model::new();
    model.footer_stats.overdue_count = 3;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 1);

    terminal
        .draw(|frame| {
            let area = frame.area();
            footer::render_footer(&model, frame, area, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(content.contains("overdue"), "Should show overdue count");
}

#[test]
fn test_render_footer_shows_due_today_count() {
    let mut model = Model::new();
    model.footer_stats.due_today_count = 5;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 1);

    terminal
        .draw(|frame| {
            let area = frame.area();
            footer::render_footer(&model, frame, area, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(content.contains("due today"), "Should show due today count");
}

#[test]
fn test_render_footer_shows_multi_select_mode() {
    let mut model = Model::new().with_sample_data();
    model.multi_select.mode = true;
    model.multi_select.selected.insert(model.visible_tasks[0]);
    let theme = Theme::default();
    let mut terminal = create_test_terminal(100, 1);

    terminal
        .draw(|frame| {
            let area = frame.area();
            footer::render_footer(&model, frame, area, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("MULTI-SELECT"),
        "Should show multi-select indicator"
    );
}

#[test]
fn test_render_footer_shows_view_hint() {
    let mut model = Model::new();
    model.current_view = ViewId::Kanban;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(100, 1);

    terminal
        .draw(|frame| {
            let area = frame.area();
            footer::render_footer(&model, frame, area, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("h/l") || content.contains("columns"),
        "Should show view-specific hint"
    );
}

#[test]
fn test_render_footer_shows_help_hint() {
    let model = Model::new();
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 1);

    terminal
        .draw(|frame| {
            let area = frame.area();
            footer::render_footer(&model, frame, area, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(content.contains("? help"), "Should show help hint");
}

#[test]
fn test_render_footer_shows_completed_visibility() {
    let mut model = Model::new();
    model.filtering.show_completed = true;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 1);

    terminal
        .draw(|frame| {
            let area = frame.area();
            footer::render_footer(&model, frame, area, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("showing all"),
        "Should show 'showing all' when completed visible"
    );
}

#[test]
fn test_render_footer_hiding_completed() {
    let mut model = Model::new();
    model.filtering.show_completed = false;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 1);

    terminal
        .draw(|frame| {
            let area = frame.area();
            footer::render_footer(&model, frame, area, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("hiding completed"),
        "Should show 'hiding completed' when filtered"
    );
}

// =========================================================================
// render_content tests
// =========================================================================

#[test]
fn test_render_content_focus_mode() {
    let mut model = Model::new().with_sample_data();
    model.focus_mode = true;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 20);

    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 80, 20);
            layout::render_content(&model, frame, area, &theme);
        })
        .unwrap();

    // Should render focus view without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_render_content_with_sidebar() {
    let mut model = Model::new().with_sample_data();
    model.show_sidebar = true;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 20);

    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 80, 20);
            layout::render_content(&model, frame, area, &theme);
        })
        .unwrap();

    // Should cache sidebar area
    assert!(model.layout_cache.sidebar_area().is_some());
}

#[test]
fn test_render_content_without_sidebar() {
    let mut model = Model::new();
    model.show_sidebar = false;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 20);

    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 80, 20);
            layout::render_content(&model, frame, area, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

// =========================================================================
// render_main_content tests for different views
// =========================================================================

#[test]
fn test_render_main_content_calendar() {
    let mut model = Model::new();
    model.current_view = ViewId::Calendar;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 20);

    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 80, 20);
            layout::render_main_content(&model, frame, area, &theme);
        })
        .unwrap();

    // Should cache calendar area
    assert!(model.layout_cache.calendar_area().is_some());
}

#[test]
fn test_render_main_content_dashboard() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Dashboard;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(100, 30);

    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 100, 30);
            layout::render_main_content(&model, frame, area, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_render_main_content_reports() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Reports;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(100, 30);

    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 100, 30);
            layout::render_main_content(&model, frame, area, &theme);
        })
        .unwrap();

    // Should cache reports tabs area
    assert!(model.layout_cache.reports_tabs_area().is_some());
}

#[test]
fn test_render_main_content_habits() {
    let mut model = Model::new();
    model.current_view = ViewId::Habits;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 20);

    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 80, 20);
            layout::render_main_content(&model, frame, area, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_render_main_content_kanban() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Kanban;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 20);

    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 80, 20);
            layout::render_main_content(&model, frame, area, &theme);
        })
        .unwrap();

    // Should cache kanban column areas
    assert!(model.layout_cache.kanban_column(0).is_some());
    assert!(model.layout_cache.kanban_column(3).is_some());
}

#[test]
fn test_render_main_content_eisenhower() {
    let mut model = Model::new();
    model.current_view = ViewId::Eisenhower;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 20);

    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 80, 20);
            layout::render_main_content(&model, frame, area, &theme);
        })
        .unwrap();

    // Should cache all four quadrants
    for i in 0..4 {
        assert!(
            model.layout_cache.eisenhower_quadrant(i).is_some(),
            "Quadrant {i} should be cached"
        );
    }
}

#[test]
fn test_render_main_content_weekly_planner() {
    let mut model = Model::new();
    model.current_view = ViewId::WeeklyPlanner;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 20);

    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 80, 20);
            layout::render_main_content(&model, frame, area, &theme);
        })
        .unwrap();

    // Should cache all seven day areas
    for i in 0..7 {
        assert!(
            model.layout_cache.weekly_planner_day(i).is_some(),
            "Day {i} should be cached"
        );
    }
}

#[test]
fn test_render_main_content_timeline() {
    let mut model = Model::new();
    model.current_view = ViewId::Timeline;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 20);

    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 80, 20);
            layout::render_main_content(&model, frame, area, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_render_main_content_heatmap() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Heatmap;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 20);

    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 80, 20);
            layout::render_main_content(&model, frame, area, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_render_main_content_forecast() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Forecast;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(120, 30);

    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 120, 30);
            layout::render_main_content(&model, frame, area, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_render_main_content_network() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Network;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 20);

    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 80, 20);
            layout::render_main_content(&model, frame, area, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_render_main_content_burndown() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Burndown;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 20);

    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 80, 20);
            layout::render_main_content(&model, frame, area, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_render_main_content_task_list_default() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::TaskList;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 20);

    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 80, 20);
            layout::render_main_content(&model, frame, area, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

// =========================================================================
// Popup rendering tests
// =========================================================================

#[test]
fn test_view_renders_input_dialog_for_new_task() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.target = InputTarget::Task;
    model.input.buffer = "New task title".to_string();
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("New Task") || content.contains("New task title"),
        "Should show new task input dialog"
    );
}

#[test]
fn test_view_renders_input_dialog_for_search() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.target = InputTarget::Search;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("Search"),
        "Should show search input dialog"
    );
}

#[test]
fn test_view_renders_quick_capture_dialog() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.target = InputTarget::QuickCapture;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(100, 30);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    // Quick capture has syntax hints
    let content = buffer_content(&terminal);
    assert!(
        content.contains("Capture") || content.contains("Quick"),
        "Should show quick capture dialog"
    );
}

#[test]
fn test_view_renders_template_picker() {
    let mut model = Model::new();
    model.template_picker.visible = true;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_view_renders_saved_filter_picker() {
    let mut model = Model::new();
    model.saved_filter_picker.visible = true;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_view_renders_keybindings_editor() {
    let mut model = Model::new();
    model.keybindings_editor.visible = true;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(100, 40);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_view_renders_time_log_editor() {
    let mut model = Model::new().with_sample_data();
    model.time_log.visible = true;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_view_renders_work_log_editor() {
    let mut model = Model::new().with_sample_data();
    model.work_log_editor.visible = true;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_view_renders_description_editor() {
    let mut model = Model::new();
    model.description_editor.visible = true;
    model.description_editor.buffer = vec!["Line 1".to_string(), "Line 2".to_string()];
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_view_renders_overdue_alert() {
    let mut model = Model::new().with_sample_data();
    model.alerts.show_overdue = true;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_view_renders_storage_error_alert() {
    let mut model = Model::new();
    model.alerts.show_storage_error = true;
    model.alerts.storage_error = Some("Failed to load data".to_string());
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("Failed") || content.contains("Error"),
        "Should show storage error"
    );
}

#[test]
fn test_view_renders_daily_review() {
    let mut model = Model::new().with_sample_data();
    model.daily_review.visible = true;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(100, 40);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_view_renders_weekly_review() {
    let mut model = Model::new().with_sample_data();
    model.weekly_review.visible = true;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(100, 40);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_view_renders_habit_analytics_popup() {
    let mut model = Model::new();
    model.habit_view.show_analytics = true;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    // Should render without panic
    assert!(terminal.backend().buffer().area.width > 0);
}

#[test]
fn test_view_renders_import_preview() {
    use crate::storage::ImportResult;

    let mut model = Model::new();
    model.import.show_preview = true;
    model.import.pending = Some(ImportResult {
        imported: vec![],
        imported_events: vec![],
        skipped: vec![],
        errors: vec![],
    });
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("Import") || content.contains("Preview"),
        "Should show import preview"
    );
}

// =========================================================================
// Input target title tests
// =========================================================================

#[test]
fn test_input_dialog_titles_for_various_targets() {
    let theme = Theme::default();
    let targets = vec![
        (InputTarget::Task, "New Task"),
        (InputTarget::Project, "New Project"),
        (InputTarget::Search, "Search"),
        (InputTarget::FilterByTag, "Filter by Tag"),
    ];

    for (target, expected_title) in targets {
        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = target;
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains(expected_title),
            "Should show title '{}' for target {:?}",
            expected_title,
            model.input.target
        );
    }
}

#[test]
fn test_input_dialog_edit_task_targets() {
    use crate::domain::TaskId;

    let theme = Theme::default();
    let task_id = TaskId::new();

    let targets = vec![
        (InputTarget::EditTask(task_id), "Edit Task"),
        (InputTarget::EditDueDate(task_id), "Due Date"),
        (InputTarget::EditScheduledDate(task_id), "Scheduled Date"),
        (InputTarget::EditTags(task_id), "Tags"),
        (InputTarget::EditDescription(task_id), "Description"),
        (InputTarget::EditEstimate(task_id), "Time Estimate"),
        (InputTarget::MoveToProject(task_id), "Move to Project"),
        (InputTarget::EditDependencies(task_id), "Blocked by"),
        (InputTarget::EditRecurrence(task_id), "Recurrence"),
        (InputTarget::LinkTask(task_id), "Link to next"),
        (InputTarget::SnoozeTask(task_id), "Snooze"),
    ];

    for (target, expected_substr) in targets {
        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = target.clone();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains(expected_substr),
            "Should show '{expected_substr}' for target {target:?}"
        );
    }
}

#[test]
fn test_input_dialog_subtask_target() {
    use crate::domain::TaskId;

    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.target = InputTarget::Subtask(TaskId::new());
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(content.contains("Subtask"), "Should show Subtask title");
}

#[test]
fn test_input_dialog_edit_project_target() {
    use crate::domain::ProjectId;

    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.target = InputTarget::EditProject(ProjectId::new());
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("Rename") || content.contains("Project"),
        "Should show rename project title"
    );
}

#[test]
fn test_input_dialog_bulk_targets() {
    let theme = Theme::default();

    // BulkMoveToProject
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.target = InputTarget::BulkMoveToProject;
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("Move Selected"),
        "Should show bulk move title"
    );

    // BulkSetStatus
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.target = InputTarget::BulkSetStatus;
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("Set Status"),
        "Should show bulk status title"
    );
}

#[test]
fn test_input_dialog_import_csv() {
    use crate::storage::ImportFormat;

    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.target = InputTarget::ImportFilePath(ImportFormat::Csv);
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(content.contains("CSV"), "Should show CSV import title");
}

#[test]
fn test_input_dialog_import_ics() {
    use crate::storage::ImportFormat;

    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.target = InputTarget::ImportFilePath(ImportFormat::Ics);
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(content.contains("ICS"), "Should show ICS import title");
}

#[test]
fn test_input_dialog_saved_filter_name() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.target = InputTarget::SavedFilterName;
    let theme = Theme::default();
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("Save Filter") || content.contains("Filter"),
        "Should show save filter title"
    );
}

#[test]
fn test_input_dialog_habit_targets() {
    use crate::domain::HabitId;

    let theme = Theme::default();

    // NewHabit
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.target = InputTarget::NewHabit;
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(content.contains("New Habit"), "Should show new habit title");

    // EditHabit
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.input.target = InputTarget::EditHabit(HabitId::new());
    let mut terminal = create_test_terminal(80, 24);

    terminal
        .draw(|frame| {
            view(&model, frame, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    assert!(
        content.contains("Edit Habit"),
        "Should show edit habit title"
    );
}

// =========================================================================
// Pomodoro footer tests
// =========================================================================

#[test]
fn test_render_footer_with_pomodoro_work_phase() {
    use crate::domain::{PomodoroSession, TaskId};

    let mut model = Model::new();
    // Create a pomodoro session directly
    let task_id = TaskId::new();
    model.pomodoro.session = Some(PomodoroSession::new(task_id, &model.pomodoro.config, 4));
    let theme = Theme::default();
    let mut terminal = create_test_terminal(120, 1);

    terminal
        .draw(|frame| {
            let area = frame.area();
            footer::render_footer(&model, frame, area, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    // Should show timer display with work phase icon and time
    assert!(
        content.contains("25:00") || content.contains("24:") || content.contains(':'),
        "Should show work timer"
    );
}

#[test]
fn test_render_footer_with_pomodoro_paused() {
    use crate::domain::{PomodoroSession, TaskId};

    let mut model = Model::new();
    let task_id = TaskId::new();
    let mut session = PomodoroSession::new(task_id, &model.pomodoro.config, 4);
    session.paused = true;
    model.pomodoro.session = Some(session);
    let theme = Theme::default();
    let mut terminal = create_test_terminal(120, 1);

    terminal
        .draw(|frame| {
            let area = frame.area();
            footer::render_footer(&model, frame, area, &theme);
        })
        .unwrap();

    // Should show pause indicator
    let content = buffer_content(&terminal);
    assert!(
        content.contains("⏸") || content.contains("[0/4]"),
        "Should show pause indicator or cycle count"
    );
}

#[test]
fn test_render_footer_with_pomodoro_cycles() {
    use crate::domain::{PomodoroSession, TaskId};

    let mut model = Model::new();
    let task_id = TaskId::new();
    let mut session = PomodoroSession::new(task_id, &model.pomodoro.config, 4);
    session.cycles_completed = 2;
    model.pomodoro.session = Some(session);
    let theme = Theme::default();
    let mut terminal = create_test_terminal(120, 1);

    terminal
        .draw(|frame| {
            let area = frame.area();
            footer::render_footer(&model, frame, area, &theme);
        })
        .unwrap();

    let content = buffer_content(&terminal);
    // Should show cycle progress like [2/4]
    assert!(content.contains("[2/4]"), "Should show cycle progress");
}

#[test]
fn test_render_footer_with_pomodoro_break_phase() {
    use crate::domain::{PomodoroPhase, PomodoroSession, TaskId};

    let mut model = Model::new();
    let task_id = TaskId::new();
    let mut session = PomodoroSession::new(task_id, &model.pomodoro.config, 4);
    session.phase = PomodoroPhase::ShortBreak;
    session.remaining_secs = 5 * 60; // 5 minutes
    model.pomodoro.session = Some(session);
    let theme = Theme::default();
    let mut terminal = create_test_terminal(120, 1);

    terminal
        .draw(|frame| {
            let area = frame.area();
            footer::render_footer(&model, frame, area, &theme);
        })
        .unwrap();

    // Should render break phase (different color)
    let content = buffer_content(&terminal);
    assert!(
        content.contains("5:00") || content.contains(':'),
        "Should show break timer"
    );
}
