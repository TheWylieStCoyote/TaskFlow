//! Reports component tests

use super::*;
use crate::app::Model;
use crate::config::Theme;
use crate::ui::test_utils::{buffer_content, render_widget};

#[test]
fn test_report_panel_navigation() {
    let panel = ReportPanel::Overview;
    assert_eq!(panel.next(), ReportPanel::Velocity);
    assert_eq!(panel.prev(), ReportPanel::Estimation);
}

#[test]
fn test_report_panel_cycle() {
    let mut panel = ReportPanel::Overview;
    for _ in 0..ReportPanel::COUNT {
        panel = panel.next();
    }
    assert_eq!(panel, ReportPanel::Overview);
}

#[test]
fn test_report_panel_index() {
    assert_eq!(ReportPanel::Overview.index(), 0);
    assert_eq!(ReportPanel::Velocity.index(), 1);
    assert_eq!(ReportPanel::Tags.index(), 2);
    assert_eq!(ReportPanel::Time.index(), 3);
    assert_eq!(ReportPanel::Tasks.index(), 4);
    assert_eq!(ReportPanel::Focus.index(), 5);
    assert_eq!(ReportPanel::Insights.index(), 6);
    assert_eq!(ReportPanel::Estimation.index(), 7);
    for idx in 0..ReportPanel::COUNT {
        assert_eq!(
            ReportPanel::from_index(idx).map(ReportPanel::index),
            Some(idx)
        );
    }
    assert_eq!(ReportPanel::from_index(ReportPanel::COUNT), None);
}

#[test]
fn test_report_panel_names() {
    let names = ReportPanel::names();
    assert_eq!(names.len(), 8);
    assert_eq!(names[0], "Overview");
    assert_eq!(names[4], "Tasks");
    assert_eq!(names[5], "Focus");
    assert_eq!(names[7], "Estimation");
}

#[test]
fn test_format_duration_minutes_only() {
    assert_eq!(format_duration(45), "45m");
    assert_eq!(format_duration(0), "0m");
}

#[test]
fn test_format_duration_hours_minutes() {
    assert_eq!(format_duration(90), "1h 30m");
    assert_eq!(format_duration(120), "2h");
    assert_eq!(format_duration(135), "2h 15m");
}

// ==================== Rendering Tests ====================

#[test]
fn test_reports_view_renders_title() {
    let model = Model::new();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Overview, &theme);
    let buffer = render_widget(view, 80, 30);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Reports"),
        "Reports title should be visible"
    );
}

#[test]
fn test_reports_view_renders_tabs() {
    let model = Model::new();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Overview, &theme);
    let buffer = render_widget(view, 100, 30);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Overview"),
        "Overview tab should be visible"
    );
    assert!(
        content.contains("Velocity"),
        "Velocity tab should be visible"
    );
    assert!(content.contains("Tags"), "Tags tab should be visible");
}

#[test]
fn test_reports_view_overview_panel() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Overview, &theme);
    let buffer = render_widget(view, 100, 40);
    let content = buffer_content(&buffer);

    // Overview shows stat boxes
    assert!(
        content.contains("Total") || content.contains("Done"),
        "Overview should show statistics"
    );
}

#[test]
fn test_reports_view_velocity_panel() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Velocity, &theme);
    let buffer = render_widget(view, 100, 40);
    let content = buffer_content(&buffer);

    // Velocity panel renders
    assert!(
        content.contains("Velocity") || content.contains("Tasks"),
        "Velocity panel should render"
    );
}

#[test]
fn test_reports_view_tags_panel() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Tags, &theme);
    let buffer = render_widget(view, 100, 40);
    let content = buffer_content(&buffer);

    // Tags panel shows tag statistics
    assert!(
        content.contains("Tag") || content.contains("unique"),
        "Tags panel should render"
    );
}

#[test]
fn test_reports_view_time_panel() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Time, &theme);
    let buffer = render_widget(view, 100, 40);
    let content = buffer_content(&buffer);

    // Time panel renders
    assert!(
        content.contains("Time") || content.contains("Hours"),
        "Time panel should render"
    );
}

#[test]
fn test_reports_view_focus_panel() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Focus, &theme);
    let buffer = render_widget(view, 100, 40);
    let content = buffer_content(&buffer);

    // Focus panel renders (may show various messages)
    assert!(
        content.contains("Focus") || content.contains("task") || content.contains("Tasks"),
        "Focus panel should render"
    );
}

#[test]
fn test_reports_view_insights_panel() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Insights, &theme);
    let buffer = render_widget(view, 100, 40);
    let content = buffer_content(&buffer);

    // Insights panel renders
    assert!(
        content.contains("Insight") || content.contains("Task"),
        "Insights panel should render"
    );
}

#[test]
fn test_reports_view_estimation_panel() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Estimation, &theme);
    let buffer = render_widget(view, 100, 40);
    let content = buffer_content(&buffer);

    // Estimation panel renders
    assert!(
        content.contains("Estimation") || content.contains("Accuracy") || content.contains("Task"),
        "Estimation panel should render"
    );
}

#[test]
fn test_reports_view_empty_model() {
    let model = Model::new();
    let theme = Theme::default();

    // Test all panels with empty model - should not panic
    for panel in [
        ReportPanel::Overview,
        ReportPanel::Velocity,
        ReportPanel::Tags,
        ReportPanel::Time,
        ReportPanel::Focus,
        ReportPanel::Insights,
        ReportPanel::Estimation,
    ] {
        let view = ReportsView::new(&model, panel, &theme);
        let _ = render_widget(view, 80, 30);
    }
}

#[test]
fn test_reports_view_small_area() {
    let model = Model::new();
    let theme = Theme::default();
    let view = ReportsView::new(&model, ReportPanel::Overview, &theme);

    // Very small area should not panic
    let _ = render_widget(view, 10, 5);
}

// ==================== Time Panel ====================

mod time_panel {
    use chrono::{Duration, Local, NaiveDate, TimeZone, Utc};

    use super::*;
    use crate::app::TimeWindow;
    use crate::domain::{today, Project, Task, TaskId, TimeEntry};

    /// A finished entry that started at `hour`:00 local time on `date`.
    fn entry_on(task_id: TaskId, date: NaiveDate, hour: u32, minutes: u32) -> TimeEntry {
        let naive = date.and_hms_opt(hour, 0, 0).expect("valid time");
        let started = Local
            .from_local_datetime(&naive)
            .earliest()
            .expect("valid local time")
            .with_timezone(&Utc);
        let mut entry = TimeEntry::start(task_id);
        entry.started_at = started;
        entry.ended_at = Some(started + Duration::minutes(i64::from(minutes)));
        entry.duration_minutes = Some(minutes);
        entry
    }

    /// Two tasks (one in a project), three entries: 45m today at noon,
    /// 2h three days ago at 09:00, 30m twenty days ago at 15:00.
    fn time_model() -> (Model, TaskId) {
        let mut model = Model::new();
        let project = Project::new("Alpha Project");
        let project_id = project.id;
        model.projects.insert(project_id, project);

        let mut deep = Task::new("Deep work session");
        deep.project_id = Some(project_id);
        let deep_id = deep.id;
        model.tasks.insert(deep_id, deep);
        let misc = Task::new("Inbox triage");
        let misc_id = misc.id;
        model.tasks.insert(misc_id, misc);

        let today = today();
        model.add_time_entry(entry_on(deep_id, today, 12, 45));
        model.add_time_entry(entry_on(deep_id, today - Duration::days(3), 9, 120));
        model.add_time_entry(entry_on(misc_id, today - Duration::days(20), 15, 30));
        (model, deep_id)
    }

    fn render_time(model: &Model, width: u16, height: u16) -> String {
        let theme = Theme::default();
        let view = ReportsView::new(model, ReportPanel::Time, &theme);
        buffer_content(&render_widget(view, width, height))
    }

    #[test]
    fn header_shows_window_and_hint_for_every_window() {
        let (mut model, _) = time_model();
        for (window, label) in [
            (TimeWindow::Days7, "Time (last 7 days)"),
            (TimeWindow::Days30, "Time (last 30 days)"),
            (TimeWindow::Days90, "Time (last 90 days)"),
        ] {
            model.reports_time_window = window;
            let content = render_time(&model, 100, 40);
            assert!(content.contains(label), "missing {label:?} in:\n{content}");
            assert!(
                content.contains("[w] window"),
                "missing hint in:\n{content}"
            );
        }
    }

    #[test]
    fn window_cycles_change_the_rendered_title() {
        let (mut model, _) = time_model();
        assert!(render_time(&model, 100, 40).contains("last 30 days"));
        model.reports_time_window = model.reports_time_window.next();
        assert!(render_time(&model, 100, 40).contains("last 90 days"));
        model.reports_time_window = model.reports_time_window.next();
        assert!(render_time(&model, 100, 40).contains("last 7 days"));
    }

    #[test]
    fn all_sections_render_with_data_in_30_day_window() {
        let (model, _) = time_model();
        let content = render_time(&model, 100, 40);

        // Stat boxes: 45 + 120 + 30 minutes in the window, 45 today, 3 active days
        for expected in [
            "Total",
            "3h 15m",
            "Today",
            "45m",
            "This Week",
            "Avg/active day",
            "1h 5m",
        ] {
            assert!(
                content.contains(expected),
                "missing {expected:?} in:\n{content}"
            );
        }
        assert!(content.contains("Tracking"));
        assert!(content.contains("idle"), "no timer is running");

        // By Project: name or "No project", hours with one decimal and share
        assert!(content.contains("By Project"));
        assert!(content.contains("Alpha Project"));
        assert!(content.contains("No project"));
        assert!(content.contains("0.5h  15%"), "30m of 195m:\n{content}");
        assert!(content.contains("85%"), "165m of 195m:\n{content}");

        // Daily sparkline names the busiest day
        let busiest = (today() - Duration::days(3)).format("%b %d").to_string();
        assert!(content.contains("Daily"));
        assert!(
            content.contains(&format!("max 2h on {busiest}")),
            "missing sparkline caption in:\n{content}"
        );

        // By Hour names the peak local hour; Top Tasks lists both tasks
        assert!(content.contains("By Hour (peak 09:00)"), "{content}");
        assert!(content.contains("Top Tasks"));
        assert!(content.contains("Deep work session"));
        assert!(content.contains("2h 45m"), "deep work total");
        assert!(content.contains("Inbox triage"));

        // Tall enough for the weekday chart too
        assert!(content.contains("Minutes by Day of Week"));
        assert!(!content.contains("No time tracked"));
    }

    #[test]
    fn seven_day_window_shows_a_bar_per_day_with_weekday_labels() {
        let (mut model, _) = time_model();
        model.reports_time_window = TimeWindow::Days7;
        let content = render_time(&model, 100, 40);

        let today = today();
        assert!(content.contains(&today.format("%a %d").to_string()));
        assert!(content.contains(&(today - Duration::days(3)).format("%a %d").to_string()));
        assert!(
            !content.contains("max "),
            "7-day view uses bars, not a sparkline"
        );
        // Window total excludes the 20-day-old entry
        assert!(content.contains("2h 45m"), "{content}");
        // The 20-day-old "Inbox triage" entry is outside the window
        assert!(!content.contains("Inbox triage"), "{content}");
    }

    #[test]
    fn long_windows_show_sparkline_with_first_and_last_dates() {
        let (mut model, _) = time_model();
        model.reports_time_window = TimeWindow::Days90;
        let content = render_time(&model, 100, 40);

        let today = today();
        // 90 dates ending today: the first label is 89 days back.
        assert!(content.contains(&(today - Duration::days(89)).format("%b %d").to_string()));
        assert!(content.contains(&today.format("%b %d").to_string()));
        assert!(content.contains("max 2h on "));
        assert!(
            content.contains("Inbox triage"),
            "20-day-old entry is inside 90 days"
        );
    }

    #[test]
    fn empty_model_shows_placeholders_without_panicking() {
        let model = Model::new();
        let content = render_time(&model, 100, 40);

        assert!(content.contains("Time (last 30 days)"));
        assert!(content.contains("No time tracked in this window"));
        assert!(content.contains("idle"));
        assert!(content.contains("By Hour "), "{content}");
        assert!(!content.contains("peak"), "no peak hour without data");
        assert!(content.contains("Top Tasks"));
        assert!(content.contains("Daily"));
        // Minutes by Day of Week still renders its (all-zero) bars
        assert!(content.contains("Minutes by Day of Week"));
    }

    #[test]
    fn tracking_box_shows_running_task_and_clock() {
        let (mut model, deep_id) = time_model();
        model.start_time_tracking(deep_id);
        let content = render_time(&model, 100, 40);

        assert!(!content.contains("idle"));
        assert!(content.contains("00:0"), "live HH:MM:SS clock:\n{content}");
        assert!(
            content.contains("Deep work"),
            "running task title:\n{content}"
        );
    }

    #[test]
    fn uses_cached_report_when_populated() {
        let (mut model, _) = time_model();
        model.ensure_report_cache_populated();
        for window in [TimeWindow::Days7, TimeWindow::Days30, TimeWindow::Days90] {
            model.reports_time_window = window;
            assert!(model.report_cache.report_for_window(window).is_some());
            let content = render_time(&model, 100, 40);
            assert!(content.contains("Alpha Project"), "{window:?}:\n{content}");
            assert!(content.contains("45m"), "today's entry in {window:?}");
        }
    }

    #[test]
    fn degrades_gracefully_at_small_sizes() {
        let (mut model, deep_id) = time_model();
        model.start_time_tracking(deep_id);

        // 80x24: stat boxes, projects/daily and hour/tasks fit; weekday chart is dropped
        let content = render_time(&model, 80, 24);
        assert!(content.contains("By Project"), "{content}");
        assert!(content.contains("Daily"));
        assert!(content.contains("By Hour"));
        assert!(content.contains("Top Tasks"));
        assert!(!content.contains("Day of Week"), "{content}");

        // 40x12: only the header and a compact text stats block fit
        let content = render_time(&model, 40, 12);
        assert!(content.contains("Time (last"), "{content}");
        assert!(content.contains("Total"), "{content}");
        assert!(!content.contains("By Project"));

        // Nothing panics at any size for any window
        for window in [TimeWindow::Days7, TimeWindow::Days30, TimeWindow::Days90] {
            model.reports_time_window = window;
            for (w, h) in [(80, 24), (40, 12), (30, 8), (10, 5), (3, 3), (120, 60)] {
                let _ = render_time(&model, w, h);
            }
        }
    }
}

mod tasks_panel {
    use super::*;
    use crate::app::TimeWindow;
    use crate::domain::{today, Project, Task, TaskId, TimeEntry};
    use chrono::{Duration, Local, NaiveDate, TimeZone};

    fn entry_on(task_id: TaskId, date: NaiveDate, hour: u32, minutes: u32) -> TimeEntry {
        let local = date.and_hms_opt(hour, 0, 0).unwrap();
        let start = Local
            .from_local_datetime(&local)
            .single()
            .unwrap()
            .with_timezone(&chrono::Utc);
        let mut entry = TimeEntry::start(task_id);
        entry.started_at = start;
        entry.ended_at = Some(start + Duration::minutes(i64::from(minutes)));
        entry.duration_minutes = Some(minutes);
        entry
    }

    fn tasks_model(n_tasks: usize) -> Model {
        let mut model = Model::new();
        let project = Project::new("Alpha Project");
        let project_id = project.id;
        model.projects.insert(project_id, project);
        let today = today();
        for i in 0..n_tasks {
            let mut task = Task::new(format!("Task number {i:02}"));
            if i % 2 == 0 {
                task.project_id = Some(project_id);
            }
            let id = task.id;
            model.tasks.insert(id, task);
            // Distinct minutes so the order is unambiguous: task 0 has the most.
            model.add_time_entry(entry_on(id, today, 9, 10 * (n_tasks - i) as u32));
            if i == 0 {
                model.add_time_entry(entry_on(id, today - Duration::days(1), 10, 5));
            }
        }
        model.report_panel = ReportPanel::Tasks;
        model
    }

    fn render(model: &Model, width: u16, height: u16) -> String {
        let theme = Theme::default();
        let view = ReportsView::new(model, ReportPanel::Tasks, &theme);
        buffer_content(&render_widget(view, width, height))
    }

    #[test]
    fn header_and_columns_render_with_the_window_label() {
        let mut model = tasks_model(3);
        model.reports_time_window = TimeWindow::Days7;
        let content = render(&model, 100, 30);
        assert!(content.contains("Tasks (last 7 days)"), "{content}");
        assert!(content.contains("[w] window"), "{content}");
        assert!(content.contains("3 tasks"), "{content}");
        assert!(content.contains("Task"), "{content}");
        assert!(content.contains("Project"), "{content}");
        assert!(content.contains("Entries"), "{content}");
        assert!(content.contains("Share"), "{content}");
    }

    #[test]
    fn rows_are_sorted_by_time_with_project_entries_and_share() {
        let model = tasks_model(3);
        let content = render(&model, 100, 30);
        let first = content.find("Task number 00").expect("row 0");
        let second = content.find("Task number 01").expect("row 1");
        let third = content.find("Task number 02").expect("row 2");
        assert!(first < second && second < third, "{content}");
        // Task 0: 30m + 5m over two entries, in Alpha Project. Total 65m.
        let row0 = content
            .lines()
            .find(|l| l.contains("Task number 00"))
            .unwrap();
        assert!(row0.contains("Alpha Project"), "{row0}");
        assert!(row0.contains("35m"), "{row0}");
        assert!(
            row0.contains(" 2 ") || row0.ends_with('2') || row0.contains("2 "),
            "{row0}"
        );
        assert!(row0.contains("54%"), "{row0}");
        let row1 = content
            .lines()
            .find(|l| l.contains("Task number 01"))
            .unwrap();
        assert!(row1.contains("(no project)"), "{row1}");
        assert!(content.contains("Total 1h 5m"), "{content}");
        assert!(content.contains("showing 1–3 of 3"), "{content}");
    }

    #[test]
    fn selection_scrolls_to_keep_the_selected_row_visible() {
        let mut model = tasks_model(40);
        model.reports_tasks_selected = 39;
        let content = render(&model, 100, 20);
        assert!(content.contains("Task number 39"), "{content}");
        assert!(!content.contains("Task number 00"), "{content}");
        assert!(content.contains("of 40"), "{content}");

        model.reports_tasks_selected = 0;
        let content = render(&model, 100, 20);
        assert!(content.contains("Task number 00"), "{content}");
        assert!(content.contains("showing 1–"), "{content}");
    }

    #[test]
    fn narrow_tables_drop_the_project_column_and_never_panic() {
        let model = tasks_model(5);
        let content = render(&model, 55, 20);
        assert!(!content.contains("Project"), "{content}");
        assert!(content.contains("Task number 00"), "{content}");
        for (w, h) in [(30, 8), (22, 12), (10, 5), (3, 3), (120, 60)] {
            let _ = render(&model, w, h);
        }
    }

    #[test]
    fn empty_window_shows_a_note() {
        let mut model = Model::new();
        model.report_panel = ReportPanel::Tasks;
        let content = render(&model, 80, 20);
        assert!(content.contains("0 tasks"), "{content}");
        assert!(content.contains("No time tracked"), "{content}");
    }
}
