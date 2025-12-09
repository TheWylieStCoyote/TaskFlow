//! Dashboard component tests

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::app::Model;
use crate::config::Theme;
use crate::domain::{Task, TaskStatus};

use super::stats::{format_duration, DashboardStats};
use super::Dashboard;

/// Helper to render a widget into a buffer
fn render_widget<W: Widget>(widget: W, width: u16, height: u16) -> Buffer {
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    widget.render(area, &mut buffer);
    buffer
}

/// Extract text content from buffer
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
fn test_dashboard_renders_completion_panel() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Completion"),
        "Completion panel should be visible"
    );
}

#[test]
fn test_dashboard_renders_time_tracking_panel() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Time Tracking"),
        "Time Tracking panel should be visible"
    );
}

#[test]
fn test_dashboard_renders_projects_panel() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Projects"),
        "Projects panel should be visible"
    );
}

#[test]
fn test_dashboard_renders_status_distribution_panel() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Status Distribution"),
        "Status Distribution panel should be visible"
    );
}

#[test]
fn test_dashboard_renders_this_week_panel() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("This Week"),
        "This Week panel should be visible"
    );
}

#[test]
fn test_dashboard_shows_overall_completion_rate() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Overall"),
        "Overall completion rate should be visible"
    );
}

#[test]
fn test_dashboard_shows_overdue_count() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Overdue"),
        "Overdue count should be visible"
    );
}

#[test]
fn test_dashboard_shows_tracking_status() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    // Should show Tracking: with either Active or Idle
    assert!(
        content.contains("Tracking"),
        "Tracking status should be visible"
    );
}

#[test]
fn test_dashboard_shows_status_types() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    // Status distribution should show task statuses
    assert!(
        content.contains("Todo") || content.contains("Done"),
        "Status types should be visible"
    );
}

#[test]
fn test_dashboard_shows_no_projects_when_empty() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 25);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("No projects"),
        "Should show 'No projects' when empty"
    );
}

#[test]
fn test_dashboard_with_sample_data() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);

    // Should render without panic
    let _ = buffer_content(&buffer);
}

#[test]
fn test_dashboard_completion_rate_calculation() {
    let mut model = Model::new();

    // Add 4 tasks, 2 done, 2 not done
    let task1 = Task::new("Task 1").with_status(TaskStatus::Done);
    let task2 = Task::new("Task 2").with_status(TaskStatus::Done);
    let task3 = Task::new("Task 3").with_status(TaskStatus::Todo);
    let task4 = Task::new("Task 4").with_status(TaskStatus::Todo);

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.tasks.insert(task3.id, task3);
    model.tasks.insert(task4.id, task4);

    let stats = DashboardStats::new(&model);

    // Completion rate should be 50%
    assert!((stats.completion_rate() - 50.0).abs() < 0.1);
}

#[test]
fn test_dashboard_completion_rate_empty() {
    let model = Model::new();
    let stats = DashboardStats::new(&model);

    // No tasks = 0% completion
    assert!(stats.completion_rate().abs() < 0.01);
}

#[test]
fn test_dashboard_status_counts() {
    let mut model = Model::new();

    let task1 = Task::new("Task 1").with_status(TaskStatus::Todo);
    let task2 = Task::new("Task 2").with_status(TaskStatus::InProgress);
    let task3 = Task::new("Task 3").with_status(TaskStatus::Done);

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.tasks.insert(task3.id, task3);

    let stats = DashboardStats::new(&model);

    let (todo, in_progress, blocked, done, cancelled) = stats.status_counts();
    assert_eq!(todo, 1);
    assert_eq!(in_progress, 1);
    assert_eq!(blocked, 0);
    assert_eq!(done, 1);
    assert_eq!(cancelled, 0);
}

#[test]
fn test_dashboard_format_duration() {
    assert_eq!(format_duration(30), "30m");
    assert_eq!(format_duration(60), "1h 0m");
    assert_eq!(format_duration(90), "1h 30m");
    assert_eq!(format_duration(125), "2h 5m");
}

#[test]
fn test_dashboard_renders_focus_sessions_panel() {
    let model = Model::new();
    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Focus Sessions"),
        "Focus Sessions panel should be visible"
    );
}

#[test]
fn test_dashboard_shows_focus_stats() {
    let mut model = Model::new();
    // Record a pomodoro cycle
    model.pomodoro_stats.record_cycle(25);

    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    // Should show the stats
    assert!(
        content.contains("Today") || content.contains("Streak"),
        "Focus stats should be visible"
    );
}

#[test]
fn test_dashboard_completion_rate_all_done() {
    let mut model = Model::new();

    let task1 = Task::new("Task 1").with_status(TaskStatus::Done);
    let task2 = Task::new("Task 2").with_status(TaskStatus::Done);
    let task3 = Task::new("Task 3").with_status(TaskStatus::Cancelled); // Also complete

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.tasks.insert(task3.id, task3);

    let stats = DashboardStats::new(&model);

    // All tasks complete = 100%
    assert!((stats.completion_rate() - 100.0).abs() < 0.01);
}

#[test]
fn test_dashboard_completion_rate_none_done() {
    let mut model = Model::new();

    let task1 = Task::new("Task 1").with_status(TaskStatus::Todo);
    let task2 = Task::new("Task 2").with_status(TaskStatus::InProgress);
    let task3 = Task::new("Task 3").with_status(TaskStatus::Blocked);

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.tasks.insert(task3.id, task3);

    let stats = DashboardStats::new(&model);

    // No tasks complete = 0%
    assert!(stats.completion_rate().abs() < 0.01);
}

#[test]
fn test_dashboard_completion_by_priority_high() {
    use crate::domain::Priority;

    let mut model = Model::new();

    let task1 = Task::new("High 1")
        .with_priority(Priority::High)
        .with_status(TaskStatus::Done);
    let task2 = Task::new("High 2")
        .with_priority(Priority::High)
        .with_status(TaskStatus::Todo);
    let task3 = Task::new("Low 1")
        .with_priority(Priority::Low)
        .with_status(TaskStatus::Done);

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.tasks.insert(task3.id, task3);

    let stats = DashboardStats::new(&model);

    // High priority: 1 done out of 2
    let (completed, total) = stats.completion_by_priority(Priority::High);
    assert_eq!(completed, 1);
    assert_eq!(total, 2);

    // Low priority: 1 done out of 1
    let (completed, total) = stats.completion_by_priority(Priority::Low);
    assert_eq!(completed, 1);
    assert_eq!(total, 1);

    // Medium priority: 0 out of 0
    let (completed, total) = stats.completion_by_priority(Priority::Medium);
    assert_eq!(completed, 0);
    assert_eq!(total, 0);
}

#[test]
fn test_dashboard_completion_by_priority_all_levels() {
    use crate::domain::Priority;

    let mut model = Model::new();

    // Add one task of each priority, all done
    for (i, priority) in [
        Priority::None,
        Priority::Low,
        Priority::Medium,
        Priority::High,
        Priority::Urgent,
    ]
    .into_iter()
    .enumerate()
    {
        let task = Task::new(format!("Task {i}"))
            .with_priority(priority)
            .with_status(TaskStatus::Done);
        model.tasks.insert(task.id, task);
    }

    let stats = DashboardStats::new(&model);

    // Each priority should have 1/1
    for priority in [
        Priority::None,
        Priority::Low,
        Priority::Medium,
        Priority::High,
        Priority::Urgent,
    ] {
        let (completed, total) = stats.completion_by_priority(priority);
        assert_eq!(completed, 1, "Priority {priority:?} completed count");
        assert_eq!(total, 1, "Priority {priority:?} total count");
    }
}

#[test]
fn test_dashboard_overdue_count_with_overdue_tasks() {
    use chrono::{Duration, Utc};

    let mut model = Model::new();

    let yesterday = Utc::now().date_naive() - Duration::days(1);
    let tomorrow = Utc::now().date_naive() + Duration::days(1);

    // Overdue task (due yesterday, not done)
    let task1 = Task::new("Overdue").with_due_date(yesterday);
    // Not overdue (due tomorrow)
    let task2 = Task::new("Future").with_due_date(tomorrow);
    // Overdue but done (should not count)
    let task3 = Task::new("Done overdue")
        .with_due_date(yesterday)
        .with_status(TaskStatus::Done);

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);
    model.tasks.insert(task3.id, task3);

    let stats = DashboardStats::new(&model);
    assert_eq!(stats.overdue_count(), 1);
}

#[test]
fn test_dashboard_overdue_count_no_overdue() {
    use chrono::{Duration, Utc};

    let mut model = Model::new();

    let tomorrow = Utc::now().date_naive() + Duration::days(1);

    let task1 = Task::new("Future").with_due_date(tomorrow);
    let task2 = Task::new("No due date");

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);

    let stats = DashboardStats::new(&model);
    assert_eq!(stats.overdue_count(), 0);
}

#[test]
fn test_dashboard_total_time_tracked_with_entries() {
    use crate::domain::{TimeEntry, TimeEntryId};
    use chrono::{Duration, Utc};

    let mut model = Model::new();

    let task = Task::new("Tracked task");
    let task_id = task.id;
    model.tasks.insert(task.id, task);

    // Add time entries totaling 90 minutes
    let start1 = Utc::now() - Duration::hours(2);
    let entry1 = TimeEntry {
        id: TimeEntryId::new(),
        task_id,
        description: None,
        started_at: start1,
        ended_at: Some(start1 + Duration::minutes(30)),
        duration_minutes: Some(30),
    };

    let start2 = Utc::now() - Duration::hours(1);
    let entry2 = TimeEntry {
        id: TimeEntryId::new(),
        task_id,
        description: None,
        started_at: start2,
        ended_at: Some(start2 + Duration::minutes(60)),
        duration_minutes: Some(60),
    };

    model.time_entries.insert(entry1.id, entry1);
    model.time_entries.insert(entry2.id, entry2);

    let stats = DashboardStats::new(&model);
    assert_eq!(stats.total_time_tracked(), 90);
}

#[test]
fn test_dashboard_total_time_tracked_empty() {
    let model = Model::new();
    let stats = DashboardStats::new(&model);
    assert_eq!(stats.total_time_tracked(), 0);
}

#[test]
fn test_dashboard_estimation_stats_mixed() {
    let mut model = Model::new();

    // Task with estimate, over budget (estimated 30, actual 45)
    let mut task1 = Task::new("Over budget");
    task1.estimated_minutes = Some(30);
    task1.actual_minutes = 45;
    model.tasks.insert(task1.id, task1);

    // Task with estimate, under budget (estimated 60, actual 40)
    let mut task2 = Task::new("Under budget");
    task2.estimated_minutes = Some(60);
    task2.actual_minutes = 40;
    model.tasks.insert(task2.id, task2);

    // Task with estimate, on target (estimated 20, actual 20)
    let mut task3 = Task::new("On target");
    task3.estimated_minutes = Some(20);
    task3.actual_minutes = 20;
    model.tasks.insert(task3.id, task3);

    // Task without estimate (should be ignored)
    let task4 = Task::new("No estimate");
    model.tasks.insert(task4.id, task4);

    let stats = DashboardStats::new(&model);
    let (total_estimated, total_actual, over, under, on_target, avg_accuracy) =
        stats.estimation_stats();

    assert_eq!(total_estimated, 110); // 30 + 60 + 20
    assert_eq!(total_actual, 105); // 45 + 40 + 20
    assert_eq!(over, 1);
    assert_eq!(under, 1);
    assert_eq!(on_target, 1);
    assert!(avg_accuracy.is_some());
}

#[test]
fn test_dashboard_estimation_stats_no_estimates() {
    let mut model = Model::new();

    let task1 = Task::new("No estimate 1");
    let task2 = Task::new("No estimate 2");

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);

    let stats = DashboardStats::new(&model);
    let (total_estimated, total_actual, over, under, on_target, avg_accuracy) =
        stats.estimation_stats();

    assert_eq!(total_estimated, 0);
    assert_eq!(total_actual, 0);
    assert_eq!(over, 0);
    assert_eq!(under, 0);
    assert_eq!(on_target, 0);
    assert!(avg_accuracy.is_none());
}

#[test]
fn test_dashboard_estimation_stats_all_over() {
    let mut model = Model::new();

    // All tasks over budget
    for i in 1..=3 {
        let mut task = Task::new(format!("Task {i}"));
        task.estimated_minutes = Some(10);
        task.actual_minutes = 20; // Double the estimate
        model.tasks.insert(task.id, task);
    }

    let stats = DashboardStats::new(&model);
    let (_, _, over, under, on_target, _) = stats.estimation_stats();

    assert_eq!(over, 3);
    assert_eq!(under, 0);
    assert_eq!(on_target, 0);
}

#[test]
fn test_dashboard_status_counts_all_types() {
    let mut model = Model::new();

    // Add tasks with each status type
    let statuses = [
        TaskStatus::Todo,
        TaskStatus::InProgress,
        TaskStatus::Blocked,
        TaskStatus::Done,
        TaskStatus::Cancelled,
    ];

    for (i, status) in statuses.into_iter().enumerate() {
        let task = Task::new(format!("Task {i}")).with_status(status);
        model.tasks.insert(task.id, task);
    }

    let stats = DashboardStats::new(&model);
    let (todo, in_progress, blocked, done, cancelled) = stats.status_counts();

    assert_eq!(todo, 1);
    assert_eq!(in_progress, 1);
    assert_eq!(blocked, 1);
    assert_eq!(done, 1);
    assert_eq!(cancelled, 1);
}

#[test]
fn test_dashboard_format_duration_zero() {
    assert_eq!(format_duration(0), "0m");
}

#[test]
fn test_dashboard_format_duration_hours_only() {
    assert_eq!(format_duration(120), "2h 0m");
    assert_eq!(format_duration(180), "3h 0m");
}

// =====================================================================
// Panel content tests - verify conditional formatting and threshold logic
// =====================================================================

use crate::domain::Project;

#[test]
fn test_panel_completion_high_rate() {
    let mut model = Model::new();

    // 4 out of 5 done = 80% (should be green/success threshold)
    for i in 0..5 {
        let status = if i < 4 {
            TaskStatus::Done
        } else {
            TaskStatus::Todo
        };
        let task = Task::new(format!("Task {i}")).with_status(status);
        model.tasks.insert(task.id, task);
    }

    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    assert!(content.contains("80%"), "Should show 80% completion rate");
}

#[test]
fn test_panel_completion_medium_rate() {
    let mut model = Model::new();

    // 3 out of 5 done = 60% (warning threshold 50-75%)
    for i in 0..5 {
        let status = if i < 3 {
            TaskStatus::Done
        } else {
            TaskStatus::Todo
        };
        let task = Task::new(format!("Task {i}")).with_status(status);
        model.tasks.insert(task.id, task);
    }

    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    assert!(content.contains("60%"), "Should show 60% completion rate");
}

#[test]
fn test_panel_completion_low_rate() {
    let mut model = Model::new();

    // 1 out of 5 done = 20% (danger threshold <50%)
    for i in 0..5 {
        let status = if i < 1 {
            TaskStatus::Done
        } else {
            TaskStatus::Todo
        };
        let task = Task::new(format!("Task {i}")).with_status(status);
        model.tasks.insert(task.id, task);
    }

    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    assert!(content.contains("20%"), "Should show 20% completion rate");
}

#[test]
fn test_panel_overdue_shows_count() {
    use chrono::{Duration, Utc};

    let mut model = Model::new();

    let yesterday = Utc::now().date_naive() - Duration::days(1);
    let task1 = Task::new("Overdue 1").with_due_date(yesterday);
    let task2 = Task::new("Overdue 2").with_due_date(yesterday);

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);

    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    // Overdue count should show 2
    assert!(
        content.contains("Overdue") && content.contains('2'),
        "Should show 2 overdue tasks"
    );
}

#[test]
fn test_panel_time_tracking_idle() {
    let model = Model::new();

    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Idle"),
        "Should show Idle when not tracking"
    );
}

#[test]
fn test_panel_time_tracking_active() {
    use crate::domain::TimeEntry;

    let mut model = Model::new();

    // Add a task and start tracking
    let task = Task::new("Tracking Task");
    let task_id = task.id;
    model.tasks.insert(task.id, task);

    // Create time entry and store it, then set as active
    let entry = TimeEntry::start(task_id);
    let entry_id = entry.id;
    model.time_entries.insert(entry_id, entry);
    model.active_time_entry = Some(entry_id);

    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Active"),
        "Should show Active when tracking"
    );
}

#[test]
fn test_panel_projects_empty() {
    let model = Model::new();

    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("No projects"),
        "Should show 'No projects' message"
    );
}

#[test]
fn test_panel_projects_with_data() {
    let mut model = Model::new();

    // Add a project with tasks
    let project = Project::new("Test Project");
    let project_id = project.id;
    model.projects.insert(project.id, project);

    // Add tasks to the project (2/3 done)
    for i in 0..3 {
        let status = if i < 2 {
            TaskStatus::Done
        } else {
            TaskStatus::Todo
        };
        let mut task = Task::new(format!("Task {i}")).with_status(status);
        task.project_id = Some(project_id);
        model.tasks.insert(task.id, task);
    }

    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    assert!(content.contains("Test Project"), "Should show project name");
    assert!(
        content.contains("67%"),
        "Should show 67% project completion"
    );
}

#[test]
fn test_panel_status_distribution_all_types() {
    let mut model = Model::new();

    // Add tasks with different statuses
    for (i, status) in [
        TaskStatus::Todo,
        TaskStatus::InProgress,
        TaskStatus::Blocked,
        TaskStatus::Done,
        TaskStatus::Cancelled,
    ]
    .into_iter()
    .enumerate()
    {
        let task = Task::new(format!("Task {i}")).with_status(status);
        model.tasks.insert(task.id, task);
    }

    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    assert!(content.contains("Todo"), "Should show Todo status");
    assert!(
        content.contains("In Progress"),
        "Should show In Progress status"
    );
    assert!(content.contains("Blocked"), "Should show Blocked status");
    assert!(content.contains("Done"), "Should show Done status");
    assert!(
        content.contains("Cancelled"),
        "Should show Cancelled status"
    );
}

#[test]
fn test_panel_estimation_with_data() {
    let mut model = Model::new();

    // Add task with estimation and actual time
    let mut task = Task::new("Estimated Task");
    task.estimated_minutes = Some(60);
    task.actual_minutes = 75; // Over by 15 minutes
    model.tasks.insert(task.id, task);

    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Estimation") || content.contains("Accuracy"),
        "Should show estimation panel"
    );
}

#[test]
fn test_panel_estimation_no_data() {
    let model = Model::new();

    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    // N/A shown when no estimates
    assert!(
        content.contains("N/A") || content.contains('0'),
        "Should show N/A or 0 when no estimates"
    );
}

#[test]
fn test_panel_focus_sessions_idle() {
    let model = Model::new();

    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Focus Sessions"),
        "Should show Focus Sessions panel"
    );
    assert!(
        content.contains("Idle") || content.contains('○'),
        "Should show idle indicator when no active session"
    );
}

#[test]
fn test_panel_focus_sessions_with_stats() {
    let mut model = Model::new();

    // Record some pomodoro cycles
    model.pomodoro_stats.record_cycle(25);
    model.pomodoro_stats.record_cycle(25);

    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Focus Sessions"),
        "Should show Focus Sessions panel"
    );
    // Should show cycle count (today might be 2 if run today)
    assert!(
        content.contains("🍅") || content.contains("cycles"),
        "Should show cycle indicator"
    );
}

#[test]
fn test_panel_activity_this_week() {
    let model = Model::new();

    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("This Week") || content.contains("Created"),
        "Should show This Week panel"
    );
}

#[test]
fn test_panel_priority_completion_urgent() {
    use crate::domain::Priority;

    let mut model = Model::new();

    // Add urgent tasks (1 done, 2 total)
    let task1 = Task::new("Urgent 1")
        .with_priority(Priority::Urgent)
        .with_status(TaskStatus::Done);
    let task2 = Task::new("Urgent 2")
        .with_priority(Priority::Urgent)
        .with_status(TaskStatus::Todo);

    model.tasks.insert(task1.id, task1);
    model.tasks.insert(task2.id, task2);

    let theme = Theme::default();
    let dashboard = Dashboard::new(&model, &theme);
    let buffer = render_widget(dashboard, 80, 30);
    let content = buffer_content(&buffer);

    // Should show Urgent completion stats
    assert!(content.contains("Urgent"), "Should show Urgent priority");
    assert!(
        content.contains("1/2") || content.contains("50%"),
        "Should show urgent completion stats"
    );
}
