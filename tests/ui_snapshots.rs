//! UI component snapshot tests.
//!
//! These tests capture the rendered output of UI components and compare
//! against previously saved snapshots, making it easy to detect visual
//! regressions.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use taskflow::app::Model;
use taskflow::config::Theme;
use taskflow::domain::{Priority, Project, Task, TaskStatus};

/// Helper to extract text content from a ratatui Buffer.
/// Returns a string where each line represents a row in the buffer.
fn buffer_to_string(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut lines = Vec::new();

    for y in area.y..area.y + area.height {
        let mut line = String::new();
        for x in area.x..area.x + area.width {
            let cell = buffer.cell((x, y)).unwrap();
            line.push_str(cell.symbol());
        }
        // Trim trailing whitespace for cleaner snapshots
        lines.push(line.trim_end().to_string());
    }

    // Remove trailing empty lines
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

/// Create a test model with consistent sample data for snapshots.
fn create_test_model() -> Model {
    let mut model = Model::new();

    // Add a few projects
    let project1 = Project::new("Work Project");
    let project2 = Project::new("Personal");
    model.projects.insert(project1.id, project1.clone());
    model.projects.insert(project2.id, project2.clone());

    // Add tasks with various properties
    let task1 = Task::new("Fix critical bug")
        .with_priority(Priority::Urgent)
        .with_status(TaskStatus::InProgress)
        .with_project(project1.id)
        .with_tags(vec!["bug".to_string(), "urgent".to_string()]);

    let task2 = Task::new("Write documentation")
        .with_priority(Priority::Medium)
        .with_status(TaskStatus::Todo)
        .with_project(project1.id)
        .with_tags(vec!["docs".to_string()]);

    let task3 = Task::new("Review PR")
        .with_priority(Priority::High)
        .with_status(TaskStatus::Todo)
        .with_tags(vec!["code-review".to_string()]);

    let task4 = Task::new("Completed task")
        .with_priority(Priority::Low)
        .with_status(TaskStatus::Done)
        .with_project(project2.id);

    let task5 = Task::new("Blocked task")
        .with_priority(Priority::Medium)
        .with_status(TaskStatus::Blocked);

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.tasks.insert(task3.id, task3);
    model.tasks.insert(task4.id, task4);
    model.tasks.insert(task5.id, task5);

    model.refresh_visible_tasks();
    model
}

mod help_component {
    use super::*;
    use taskflow::config::Keybindings;
    use taskflow::ui::HelpPopup;

    #[test]
    fn test_help_popup_layout() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 80, 30);
        let mut buffer = Buffer::empty(area);
        let help = HelpPopup::new(&keybindings, &theme);
        help.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("help_popup_layout", output);
    }

    #[test]
    fn test_help_popup_small_area() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 40, 15);
        let mut buffer = Buffer::empty(area);
        let help = HelpPopup::new(&keybindings, &theme);
        help.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("help_popup_small", output);
    }
}

mod sidebar_component {
    use super::*;
    use taskflow::ui::Sidebar;

    #[test]
    fn test_sidebar_with_projects() {
        let model = create_test_model();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 25, 20);
        let mut buffer = Buffer::empty(area);
        let sidebar = Sidebar::new(&model, &theme);
        sidebar.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("sidebar_with_projects", output);
    }

    #[test]
    fn test_sidebar_empty_model() {
        let model = Model::new();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 25, 20);
        let mut buffer = Buffer::empty(area);
        let sidebar = Sidebar::new(&model, &theme);
        sidebar.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("sidebar_empty", output);
    }
}

mod kanban_component {
    use super::*;
    use taskflow::ui::Kanban;

    #[test]
    fn test_kanban_board() {
        let model = create_test_model();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 25);
        let mut buffer = Buffer::empty(area);
        let kanban = Kanban::new(&model, &theme);
        kanban.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("kanban_board", output);
    }

    #[test]
    fn test_kanban_empty() {
        let model = Model::new();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 25);
        let mut buffer = Buffer::empty(area);
        let kanban = Kanban::new(&model, &theme);
        kanban.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("kanban_empty", output);
    }

    #[test]
    fn test_kanban_narrow() {
        let model = create_test_model();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 60, 20);
        let mut buffer = Buffer::empty(area);
        let kanban = Kanban::new(&model, &theme);
        kanban.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("kanban_narrow", output);
    }
}

mod eisenhower_component {
    use super::*;
    use taskflow::ui::Eisenhower;

    #[test]
    fn test_eisenhower_matrix() {
        let model = create_test_model();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);
        let matrix = Eisenhower::new(&model, &theme);
        matrix.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("eisenhower_matrix", output);
    }

    #[test]
    fn test_eisenhower_empty() {
        let model = Model::new();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);
        let matrix = Eisenhower::new(&model, &theme);
        matrix.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("eisenhower_empty", output);
    }
}

mod burndown_component {
    use super::*;
    use taskflow::ui::Burndown;

    #[test]
    fn test_burndown_chart() {
        // Use sample data - note: output is time-sensitive, so we verify
        // that it renders successfully with key elements present
        let model = Model::new().with_sample_data();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let burndown = Burndown::new(&model, &theme);
        burndown.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        // Verify key elements are present
        assert!(output.contains("Burndown"), "should have title");
        assert!(output.contains("Progress"), "should have progress section");
        assert!(output.contains("Total tasks"), "should have total tasks");
        assert!(!output.is_empty(), "should produce output");
    }

    #[test]
    fn test_burndown_empty() {
        let model = Model::new();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let burndown = Burndown::new(&model, &theme);
        burndown.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("burndown_empty", output);
    }
}

mod heatmap_component {
    use super::*;
    use taskflow::ui::Heatmap;

    #[test]
    fn test_heatmap_with_data() {
        // Sample data has time-sensitive completion data
        let model = Model::new().with_sample_data();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 90, 20);
        let mut buffer = Buffer::empty(area);
        let heatmap = Heatmap::new(&model, &theme);
        heatmap.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        // Verify key heatmap elements are present
        assert!(output.contains("Heatmap"), "should have title");
        assert!(output.contains("streak"), "should show streak info");
        assert!(!output.is_empty(), "should produce output");
    }

    #[test]
    fn test_heatmap_empty() {
        let model = Model::new();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 90, 20);
        let mut buffer = Buffer::empty(area);
        let heatmap = Heatmap::new(&model, &theme);
        heatmap.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("heatmap_empty", output);
    }
}

mod focus_view_component {
    use super::*;
    use taskflow::ui::FocusView;

    #[test]
    fn test_focus_view_with_tasks() {
        let mut model = create_test_model();
        model.selected_index = 0;
        model.refresh_visible_tasks();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);
        let focus = FocusView::new(&model, &theme);
        focus.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("focus_view_with_tasks", output);
    }

    #[test]
    fn test_focus_view_empty() {
        let model = Model::new();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);
        let focus = FocusView::new(&model, &theme);
        focus.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("focus_view_empty", output);
    }
}

mod calendar_component {
    use super::*;
    use chrono::NaiveDate;
    use taskflow::ui::Calendar;

    #[test]
    fn test_calendar_view() {
        let mut model = create_test_model();
        // Add tasks with due dates for the calendar
        let mut task = Task::new("Task with due date")
            .with_priority(Priority::High)
            .with_status(TaskStatus::Todo);
        task.due_date = Some(NaiveDate::from_ymd_opt(2024, 12, 15).unwrap());
        model.tasks.insert(task.id, task);
        model.refresh_visible_tasks();

        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let calendar = Calendar::new(&model, &theme);
        calendar.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("calendar_view", output);
    }

    #[test]
    fn test_calendar_empty() {
        let model = Model::new();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let calendar = Calendar::new(&model, &theme);
        calendar.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("calendar_empty", output);
    }
}

mod network_component {
    use super::*;
    use taskflow::ui::Network;

    #[test]
    fn test_network_view() {
        let model = create_test_model();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);
        let network = Network::new(&model, &theme, 0);
        network.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("network_view", output);
    }
}

mod weekly_planner_component {
    use super::*;
    use taskflow::ui::WeeklyPlanner;

    #[test]
    fn test_weekly_planner() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 120, 30);
        let mut buffer = Buffer::empty(area);
        let planner = WeeklyPlanner::new(&model, &theme);
        planner.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("weekly_planner", output);
    }
}

mod forecast_component {
    use super::*;
    use taskflow::ui::Forecast;

    #[test]
    fn test_forecast_view() {
        // Sample data is time-sensitive, verify key elements
        let model = Model::new().with_sample_data();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 120, 30);
        let mut buffer = Buffer::empty(area);
        let forecast = Forecast::new(&model, &theme);
        forecast.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        // Verify key elements are present
        assert!(output.contains("Forecast"), "should have title");
        assert!(
            output.contains("Tasks Due"),
            "should have tasks due section"
        );
        assert!(output.contains("Summary"), "should have summary section");
        assert!(!output.is_empty(), "should produce output");
    }

    #[test]
    fn test_forecast_empty() {
        let model = Model::new();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 120, 30);
        let mut buffer = Buffer::empty(area);
        let forecast = Forecast::new(&model, &theme);
        forecast.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("forecast_empty", output);
    }
}

mod dashboard_component {
    use super::*;
    use taskflow::ui::Dashboard;

    #[test]
    fn test_dashboard_with_data() {
        // Sample data has random order and time-sensitive stats
        let model = Model::new().with_sample_data();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let dashboard = Dashboard::new(&model, &theme);
        dashboard.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        // Verify key dashboard elements are present
        assert!(
            output.contains("Priority") || output.contains("Status"),
            "should have priority or status section"
        );
        assert!(output.contains("Projects"), "should have projects section");
        assert!(!output.is_empty(), "should produce output");
    }

    #[test]
    fn test_dashboard_empty() {
        let model = Model::new();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let dashboard = Dashboard::new(&model, &theme);
        dashboard.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("dashboard_empty", output);
    }
}

// Regression tests for specific UI states
mod regression_tests {
    use super::*;
    use taskflow::ui::Sidebar;

    #[test]
    fn test_sidebar_with_many_projects() {
        let mut model = Model::new();

        // Add many projects to test scrolling behavior
        for i in 0..15 {
            let project = Project::new(format!("Project {}", i));
            model.projects.insert(project.id, project);
        }

        let theme = Theme::default();

        let area = Rect::new(0, 0, 25, 15); // Smaller than project count
        let mut buffer = Buffer::empty(area);
        let sidebar = Sidebar::new(&model, &theme);
        sidebar.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("sidebar_many_projects", output);
    }

    #[test]
    fn test_sidebar_with_long_project_names() {
        let mut model = Model::new();

        let project = Project::new("This is a very long project name that should be truncated");
        model.projects.insert(project.id, project);

        let theme = Theme::default();

        let area = Rect::new(0, 0, 25, 15);
        let mut buffer = Buffer::empty(area);
        let sidebar = Sidebar::new(&model, &theme);
        sidebar.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("sidebar_long_names", output);
    }
}

mod reports_component {
    use super::*;
    use taskflow::ui::{ReportPanel, ReportsView};

    fn create_model_with_estimates() -> Model {
        let mut model = Model::new().with_sample_data();

        // Add tasks with estimates for better reports
        for (i, task) in model.tasks.values_mut().enumerate() {
            task.estimated_minutes = Some((30 + i * 15) as u32);
            task.actual_minutes = ((30 + i * 10) as u32).min(task.estimated_minutes.unwrap() + 20);
        }

        model
    }

    #[test]
    fn test_reports_overview_panel() {
        let model = create_model_with_estimates();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let reports = ReportsView::new(&model, ReportPanel::Overview, &theme);
        reports.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        // Verify key overview elements
        assert!(
            output.contains("Total") || output.contains("Done"),
            "should have stats"
        );
        assert!(
            output.contains("Progress") || output.contains("Complete"),
            "should show progress"
        );
    }

    #[test]
    fn test_reports_overview_empty() {
        let model = Model::new();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let reports = ReportsView::new(&model, ReportPanel::Overview, &theme);
        reports.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("reports_overview_empty", output);
    }

    #[test]
    fn test_reports_velocity_panel() {
        let model = create_model_with_estimates();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let reports = ReportsView::new(&model, ReportPanel::Velocity, &theme);
        reports.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        // Velocity panel should show trend information
        assert!(
            output.contains("Velocity") || output.contains("Week") || output.contains("Tasks"),
            "should have velocity info"
        );
    }

    #[test]
    fn test_reports_velocity_empty() {
        let model = Model::new();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let reports = ReportsView::new(&model, ReportPanel::Velocity, &theme);
        reports.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("reports_velocity_empty", output);
    }

    #[test]
    fn test_reports_tags_panel() {
        let model = create_model_with_estimates();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let reports = ReportsView::new(&model, ReportPanel::Tags, &theme);
        reports.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        // Tags panel should show tag distribution
        assert!(
            output.contains("Tags") || output.contains("No tags") || output.contains('#'),
            "should have tags section"
        );
    }

    #[test]
    fn test_reports_tags_empty() {
        let model = Model::new();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let reports = ReportsView::new(&model, ReportPanel::Tags, &theme);
        reports.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("reports_tags_empty", output);
    }

    #[test]
    fn test_reports_time_panel() {
        let model = create_model_with_estimates();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let reports = ReportsView::new(&model, ReportPanel::Time, &theme);
        reports.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        // Time panel should show time tracking info
        assert!(
            output.contains("Time") || output.contains("Hours") || output.contains("Day"),
            "should have time info"
        );
    }

    #[test]
    fn test_reports_time_empty() {
        let model = Model::new();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let reports = ReportsView::new(&model, ReportPanel::Time, &theme);
        reports.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("reports_time_empty", output);
    }

    #[test]
    fn test_reports_focus_panel() {
        let mut model = create_model_with_estimates();
        // Add some pomodoro stats
        model.pomodoro_stats.record_cycle(25);
        model.pomodoro_stats.record_cycle(25);

        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let reports = ReportsView::new(&model, ReportPanel::Focus, &theme);
        reports.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        // Focus panel should show pomodoro/focus info
        assert!(
            output.contains("Focus") || output.contains("Pomodoro") || output.contains("cycle"),
            "should have focus info"
        );
    }

    #[test]
    fn test_reports_focus_empty() {
        let model = Model::new();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let reports = ReportsView::new(&model, ReportPanel::Focus, &theme);
        reports.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("reports_focus_empty", output);
    }

    #[test]
    fn test_reports_insights_panel() {
        let model = create_model_with_estimates();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let reports = ReportsView::new(&model, ReportPanel::Insights, &theme);
        reports.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        // Insights panel should show recommendations/insights
        assert!(
            output.contains("Insight") || output.contains("Tip") || output.contains("Streak"),
            "should have insights"
        );
    }

    #[test]
    fn test_reports_insights_empty() {
        let model = Model::new();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let reports = ReportsView::new(&model, ReportPanel::Insights, &theme);
        reports.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("reports_insights_empty", output);
    }

    #[test]
    fn test_reports_estimation_panel() {
        let model = create_model_with_estimates();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let reports = ReportsView::new(&model, ReportPanel::Estimation, &theme);
        reports.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        // Estimation panel should show accuracy info
        assert!(
            output.contains("Estimation") || output.contains("Accuracy") || output.contains('%'),
            "should have estimation info"
        );
    }

    #[test]
    fn test_reports_estimation_empty() {
        let model = Model::new();
        let theme = Theme::default();

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        let reports = ReportsView::new(&model, ReportPanel::Estimation, &theme);
        reports.render(area, &mut buffer);

        let output = buffer_to_string(&buffer);
        insta::assert_snapshot!("reports_estimation_empty", output);
    }
}
