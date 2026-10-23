//! Tests for the task list widget.

use super::*;
use crate::app::Model;
use crate::config::Theme;
use crate::domain::{Priority, Task, TaskStatus};
use crate::ui::test_utils::{buffer_content, render_widget};

#[test]
fn test_task_list_renders_title() {
    let model = Model::new();
    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 60, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Tasks"),
        "Task list title should be visible"
    );
}

#[test]
fn test_task_list_renders_empty_list() {
    let model = Model::new();
    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 60, 20);

    // Should render without panic
    let _ = buffer_content(&buffer);
}

#[test]
fn test_task_list_renders_task_titles() {
    let mut model = Model::new();
    let task = Task::new("Test Task Title");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.visible_tasks.push(task_id);

    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 60, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Test Task Title"),
        "Task title should be visible"
    );
}

#[test]
fn test_task_list_renders_priority_indicator() {
    let mut model = Model::new();
    let task = Task::new("Urgent Task").with_priority(Priority::Urgent);
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.visible_tasks.push(task_id);

    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 60, 20);
    let content = buffer_content(&buffer);

    // Urgent tasks show "!!!!"
    assert!(
        content.contains("!!!!") || content.contains("Urgent"),
        "Priority indicator should be visible"
    );
}

#[test]
fn test_task_list_renders_completed_task() {
    let mut model = Model::new();
    let task = Task::new("Completed Task").with_status(TaskStatus::Done);
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.visible_tasks.push(task_id);
    model.filtering.show_completed = true;

    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 60, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("Completed Task"),
        "Completed task title should be visible"
    );
}

#[test]
fn test_task_list_renders_status_symbol() {
    let mut model = Model::new();
    let task = Task::new("In Progress Task").with_status(TaskStatus::InProgress);
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.visible_tasks.push(task_id);

    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 60, 20);
    let content = buffer_content(&buffer);

    // Should have status symbol (varies by status)
    assert!(
        content.contains("In Progress Task"),
        "Task with status should be visible"
    );
}

#[test]
fn test_task_list_renders_tags() {
    let mut model = Model::new();
    let task = Task::new("Tagged Task").with_tags(vec!["rust".into(), "test".into()]);
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.visible_tasks.push(task_id);

    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 80, 20);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("#rust") || content.contains("#test"),
        "Tags should be visible"
    );
}

#[test]
fn test_task_list_renders_due_date() {
    let mut model = Model::new();
    let today = today();
    let task = Task::new("Due Task").with_due_date(today);
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.visible_tasks.push(task_id);

    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 80, 20);
    let content = buffer_content(&buffer);

    // Due date shown in format [MM/DD]
    assert!(content.contains('['), "Due date bracket should be visible");
}

#[test]
fn test_task_list_renders_with_sample_data() {
    let model = Model::new().with_sample_data();
    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 80, 30);
    let content = buffer_content(&buffer);

    // Sample data contains various tasks
    assert!(
        content.contains("Tasks"),
        "Task list title should be visible"
    );
}

#[test]
fn test_task_list_grouped_view() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Projects;
    model.refresh_visible_tasks();

    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 80, 30);
    let content = buffer_content(&buffer);

    assert!(
        content.contains("by Project"),
        "Grouped title should be visible"
    );
}

#[test]
fn test_task_list_description_indicator() {
    let mut model = Model::new();
    let task = Task::new("Task with Notes").with_description("Some important notes".to_string());
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.visible_tasks.push(task_id);

    let theme = Theme::default();
    let task_list = TaskList::new(&model, &theme);
    let buffer = render_widget(task_list, 80, 20);
    let content = buffer_content(&buffer);

    // [+] indicator for tasks with descriptions
    assert!(
        content.contains("[+]"),
        "Description indicator should be visible"
    );
}

// =====================================================================
// Item-level tests for task_to_list_item and project_header_to_list_item
// =====================================================================

use super::item::{project_header_to_list_item, task_to_list_item, TaskItemContext};
use crate::domain::today;

/// Helper to extract text from a ListItem by rendering to buffer
fn list_item_text(item: ratatui::widgets::ListItem<'_>) -> String {
    use ratatui::widgets::List;
    let list = List::new(vec![item]);
    let area = Rect::new(0, 0, 200, 1);
    let mut buffer = Buffer::empty(area);
    ratatui::prelude::Widget::render(list, area, &mut buffer);

    let mut content = String::new();
    for x in 0..buffer.area.width {
        content.push(
            buffer
                .cell((x, 0))
                .map_or(' ', |c| c.symbol().chars().next().unwrap_or(' ')),
        );
    }
    content.trim_end().to_string()
}

fn default_context<'a>(task: &'a Task, theme: &'a Theme) -> TaskItemContext<'a> {
    TaskItemContext {
        task,
        is_selected: false,
        is_tracking: false,
        time_spent: 0,
        nesting_depth: 0,
        is_multi_selected: false,
        has_dependencies: false,
        is_recurring: false,
        has_chain: false,
        subtask_progress: (0, 0),
        theme,
        git_branch: None,
        is_collapsed: false,
        is_editing: false,
        edit_buffer: None,
        edit_cursor: 0,
    }
}

#[test]
fn test_item_priority_urgent() {
    let theme = Theme::default();
    let task = Task::new("Test").with_priority(Priority::Urgent);
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(text.contains("!!!!"), "Urgent priority should show !!!!");
}

#[test]
fn test_item_priority_high() {
    let theme = Theme::default();
    let task = Task::new("Test").with_priority(Priority::High);
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(text.contains("!!!"), "High priority should show !!!");
}

#[test]
fn test_item_priority_medium() {
    let theme = Theme::default();
    let task = Task::new("Test").with_priority(Priority::Medium);
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(text.contains("!!"), "Medium priority should show !!");
}

#[test]
fn test_item_priority_low() {
    let theme = Theme::default();
    let task = Task::new("Test").with_priority(Priority::Low);
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(text.contains('!'), "Low priority should show !");
}

#[test]
fn test_item_priority_none() {
    let theme = Theme::default();
    let task = Task::new("Test").with_priority(Priority::None);
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    // No exclamation marks for no priority - check before "Test" appears
    let title_pos = text.find("Test").unwrap_or(text.len());
    let priority_section = &text[..title_pos];
    assert!(
        !priority_section.contains('!'),
        "No priority should not show !"
    );
}

#[test]
fn test_item_status_done() {
    let theme = Theme::default();
    let task = Task::new("Test").with_status(TaskStatus::Done);
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(
        text.contains(TaskStatus::Done.symbol()),
        "Done status symbol should be visible"
    );
}

#[test]
fn test_item_status_in_progress() {
    let theme = Theme::default();
    let task = Task::new("Test").with_status(TaskStatus::InProgress);
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(
        text.contains(TaskStatus::InProgress.symbol()),
        "InProgress status symbol should be visible"
    );
}

#[test]
fn test_item_status_blocked() {
    let theme = Theme::default();
    let task = Task::new("Test").with_status(TaskStatus::Blocked);
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(
        text.contains(TaskStatus::Blocked.symbol()),
        "Blocked status symbol should be visible"
    );
}

#[test]
fn test_item_status_cancelled() {
    let theme = Theme::default();
    let task = Task::new("Test").with_status(TaskStatus::Cancelled);
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(
        text.contains(TaskStatus::Cancelled.symbol()),
        "Cancelled status symbol should be visible"
    );
}

#[test]
fn test_item_overdue_task() {
    use chrono::Duration;

    let theme = Theme::default();
    let yesterday = today() - Duration::days(1);
    let task = Task::new("Overdue Task").with_due_date(yesterday);
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    // Overdue tasks show warning prefix
    assert!(
        text.contains('⚠'),
        "Overdue task should show warning symbol"
    );
}

#[test]
fn test_item_due_today() {
    let theme = Theme::default();
    let today = today();
    let task = Task::new("Due Today").with_due_date(today);
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    // Due today shows ! prefix
    assert!(
        text.contains("! Due Today"),
        "Due today task should show ! prefix"
    );
}

#[test]
fn test_item_future_due_date() {
    use chrono::Duration;

    let theme = Theme::default();
    let tomorrow = today() + Duration::days(1);
    let task = Task::new("Future Task").with_due_date(tomorrow);
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    // Future due date should be shown in [MM/DD] format
    let expected_date = tomorrow.format("%m/%d").to_string();
    assert!(
        text.contains(&expected_date),
        "Future due date should be visible"
    );
}

#[test]
fn test_item_time_tracking_active() {
    let theme = Theme::default();
    let task = Task::new("Tracking");
    let mut ctx = default_context(&task, &theme);
    ctx.is_tracking = true;
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    // Active tracking shows ● indicator
    assert!(
        text.contains('●'),
        "Active tracking should show ● indicator"
    );
}

#[test]
fn test_item_time_tracking_inactive() {
    let theme = Theme::default();
    let task = Task::new("Not Tracking");
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    // No tracking indicator when not tracking (● only from multi-select)
    // Title should be visible without tracking indicator
    assert!(text.contains("Not Tracking"));
}

#[test]
fn test_item_time_spent_minutes() {
    let theme = Theme::default();
    let task = Task::new("Worked Task");
    let mut ctx = default_context(&task, &theme);
    ctx.time_spent = 45;
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(text.contains("(45m)"), "Time spent in minutes should show");
}

#[test]
fn test_item_time_spent_hours_minutes() {
    let theme = Theme::default();
    let task = Task::new("Long Task");
    let mut ctx = default_context(&task, &theme);
    ctx.time_spent = 90; // 1h 30m
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(
        text.contains("(1h 30m)"),
        "Time spent should show hours and minutes"
    );
}

#[test]
fn test_item_time_spent_zero() {
    let theme = Theme::default();
    let task = Task::new("No Time");
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    // No time indicator when time_spent is 0
    assert!(
        !text.contains("(0m)"),
        "Zero time spent should not show indicator"
    );
}

#[test]
fn test_item_tags_single() {
    let theme = Theme::default();
    let task = Task::new("Tagged").with_tags(vec!["rust".into()]);
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(text.contains("#rust"), "Single tag should be visible");
}

#[test]
fn test_item_tags_multiple() {
    let theme = Theme::default();
    let task = Task::new("Multi Tag").with_tags(vec!["rust".into(), "test".into(), "ui".into()]);
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(text.contains("#rust"), "First tag should be visible");
    assert!(text.contains("#test"), "Second tag should be visible");
    assert!(text.contains("#ui"), "Third tag should be visible");
}

#[test]
fn test_item_tags_empty() {
    let theme = Theme::default();
    let task = Task::new("No Tags");
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(!text.contains('#'), "No tags should not show # symbol");
}

#[test]
fn test_item_description_indicator() {
    let theme = Theme::default();
    let task = Task::new("Has Notes").with_description("Important details".to_string());
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(
        text.contains("[+]"),
        "Description indicator [+] should show"
    );
}

#[test]
fn test_item_no_description() {
    let theme = Theme::default();
    let task = Task::new("No Notes");
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(!text.contains("[+]"), "No description should not show [+]");
}

#[test]
fn test_item_dependency_indicator() {
    let theme = Theme::default();
    let task = Task::new("Blocked Task");
    let mut ctx = default_context(&task, &theme);
    ctx.has_dependencies = true;
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(text.contains("[B]"), "Dependency indicator [B] should show");
}

#[test]
fn test_item_recurrence_indicator() {
    let theme = Theme::default();
    let task = Task::new("Recurring");
    let mut ctx = default_context(&task, &theme);
    ctx.is_recurring = true;
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(text.contains('↻'), "Recurrence indicator ↻ should show");
}

#[test]
fn test_item_chain_indicator() {
    let theme = Theme::default();
    let task = Task::new("Chained");
    let mut ctx = default_context(&task, &theme);
    ctx.has_chain = true;
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(text.contains('→'), "Chain indicator → should show");
}

#[test]
fn test_item_subtask_progress() {
    let theme = Theme::default();
    let task = Task::new("Parent Task");
    let mut ctx = default_context(&task, &theme);
    ctx.subtask_progress = (3, 5); // 3 of 5 done = 60%
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    // Feature 7: progress bar format is [████░░] completed/total
    assert!(text.contains("3/5"), "Subtask progress should show 3/5");
}

#[test]
fn test_item_subtask_progress_complete() {
    let theme = Theme::default();
    let task = Task::new("Parent Task");
    let mut ctx = default_context(&task, &theme);
    ctx.subtask_progress = (5, 5); // All done = 100%
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    // Feature 7: progress bar format is [████████] completed/total
    assert!(text.contains("5/5"), "Subtask progress should show 5/5");
}

#[test]
fn test_item_subtask_progress_none() {
    let theme = Theme::default();
    let task = Task::new("No Subtasks");
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(!text.contains('%'), "No subtasks should not show %");
}

#[test]
fn test_item_nesting_depth_one() {
    let theme = Theme::default();
    let task = Task::new("Subtask");
    let mut ctx = default_context(&task, &theme);
    ctx.nesting_depth = 1;
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(text.contains("└─"), "Subtask should show branch character");
}

#[test]
fn test_item_nesting_depth_two() {
    let theme = Theme::default();
    let task = Task::new("Deep Subtask");
    let mut ctx = default_context(&task, &theme);
    ctx.nesting_depth = 2;
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    // Should have indentation + branch
    assert!(
        text.contains("└─"),
        "Nested subtask should show branch character"
    );
    assert!(
        text.contains("  └─"),
        "Depth 2 should have extra indentation"
    );
}

#[test]
fn test_item_multi_selected() {
    let theme = Theme::default();
    let task = Task::new("Selected");
    let mut ctx = default_context(&task, &theme);
    ctx.is_multi_selected = true;
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(text.starts_with("● "), "Multi-selected should start with ●");
}

#[test]
fn test_project_header_basic() {
    let theme = Theme::default();
    let item = project_header_to_list_item("My Project", 10, &theme);
    let text = list_item_text(item);

    assert!(
        text.contains("My Project"),
        "Project name should be visible"
    );
    assert!(text.contains("(10)"), "Task count should be visible");
}

#[test]
fn test_project_header_zero_tasks() {
    let theme = Theme::default();
    let item = project_header_to_list_item("Empty Project", 0, &theme);
    let text = list_item_text(item);

    assert!(text.contains("(0)"), "Zero task count should show");
}

#[test]
fn test_project_header_decorations() {
    let theme = Theme::default();
    let item = project_header_to_list_item("Test", 5, &theme);
    let text = list_item_text(item);

    // Headers have ── decorations
    assert!(text.contains("──"), "Header should have decorations");
}

#[test]
fn test_item_git_branch_indicator() {
    let theme = Theme::default();
    let task = Task::new("Git Task");
    let mut ctx = default_context(&task, &theme);
    ctx.git_branch = Some("feature/login");
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(
        text.contains("⎇ feature/login"),
        "Git branch should be visible"
    );
}

#[test]
fn test_item_git_branch_truncated() {
    let theme = Theme::default();
    let task = Task::new("Git Task");
    let mut ctx = default_context(&task, &theme);
    ctx.git_branch = Some("feature/very-long-branch-name-that-should-truncate");
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    // Should truncate to 17 chars + "..."
    assert!(text.contains("⎇"), "Git branch icon should be visible");
    assert!(text.contains("..."), "Long branch should be truncated");
}

#[test]
fn test_item_no_git_branch() {
    let theme = Theme::default();
    let task = Task::new("No Git");
    let ctx = default_context(&task, &theme);
    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(!text.contains('⎇'), "No git branch should not show ⎇ icon");
}

// ============================================================================
// Subtask collapse indicator
// ============================================================================

#[test]
fn test_item_collapse_indicator_follows_subtask_state() {
    let theme = Theme::default();
    let task = Task::new("Parent Task");

    let mut ctx = default_context(&task, &theme);
    ctx.subtask_progress = (1, 3);
    let expanded = list_item_text(task_to_list_item(&ctx));
    assert!(
        expanded.contains("▼ "),
        "expanded parent shows ▼: {expanded}"
    );
    assert!(!expanded.contains('▶'));

    ctx.is_collapsed = true;
    let collapsed = list_item_text(task_to_list_item(&ctx));
    assert!(
        collapsed.contains("▶ "),
        "collapsed parent shows ▶: {collapsed}"
    );
    assert!(!collapsed.contains('▼'));

    // A task without subtasks never shows an indicator, collapsed or not
    ctx.subtask_progress = (0, 0);
    let leaf = list_item_text(task_to_list_item(&ctx));
    assert!(!leaf.contains('▶') && !leaf.contains('▼'));
}

#[test]
fn test_task_list_marks_collapsed_parent_from_model_state() {
    let mut model = Model::new();
    let parent = Task::new("Parent Task");
    let child = Task::new("Hidden Child").with_parent(parent.id);
    let parent_id = parent.id;
    model.tasks.insert(parent.id, parent);
    model.tasks.insert(child.id, child);
    model.collapsed_tasks.insert(parent_id);
    model.refresh_visible_tasks();

    let theme = Theme::default();
    let content = buffer_content(&render_widget(TaskList::new(&model, &theme), 60, 10));

    assert!(
        content.contains("▶"),
        "collapsed marker rendered: {content}"
    );
    assert!(content.contains("Parent Task"));
    assert!(!content.contains("Hidden Child"));
}

// ============================================================================
// Inline title editing
// ============================================================================

#[test]
fn test_item_inline_edit_shows_buffer_with_cursor_instead_of_title() {
    let theme = Theme::default();
    let task = Task::new("Old title");
    let mut ctx = default_context(&task, &theme);
    ctx.is_editing = true;
    ctx.edit_buffer = Some("New títle");
    ctx.edit_cursor = 4; // on the 'í' (char index, not byte index)

    let item = task_to_list_item(&ctx);
    let text = list_item_text(item);

    assert!(
        text.contains("New títle"),
        "buffer replaces the title: {text}"
    );
    assert!(!text.contains("Old title"));
}

/// Renders one item and returns the (symbol, background) of every cell in the row.
fn item_cells(item: ratatui::widgets::ListItem<'_>) -> Vec<(String, ratatui::style::Color)> {
    use ratatui::widgets::List;
    let area = Rect::new(0, 0, 80, 1);
    let mut buffer = Buffer::empty(area);
    ratatui::prelude::Widget::render(List::new(vec![item]), area, &mut buffer);
    (0..area.width)
        .filter_map(|x| buffer.cell((x, 0)))
        .map(|c| (c.symbol().to_string(), c.bg))
        .collect()
}

#[test]
fn test_item_inline_edit_block_cursor_highlights_the_cursor_cell() {
    use ratatui::style::Color;
    let theme = Theme::default();
    let task = Task::new("Old title");
    let mut ctx = default_context(&task, &theme);
    ctx.is_editing = true;
    ctx.edit_buffer = Some("abc");

    // Cursor on 'b': only that cell carries the block background
    ctx.edit_cursor = 1;
    let cells = item_cells(task_to_list_item(&ctx));
    let bg_of = |sym: &str| cells.iter().find(|(s, _)| s == sym).map(|(_, bg)| *bg);
    assert_ne!(bg_of("b"), Some(Color::Reset), "cursor cell is a block");
    assert_eq!(bg_of("a"), Some(Color::Reset));
    assert_eq!(bg_of("c"), Some(Color::Reset));

    // Cursor past the end: a highlighted blank follows the text
    ctx.edit_cursor = 3;
    let cells = item_cells(task_to_list_item(&ctx));
    let c_pos = cells.iter().position(|(s, _)| s == "c").unwrap();
    assert_eq!(cells[c_pos + 1].0, " ");
    assert_ne!(cells[c_pos + 1].1, Color::Reset, "trailing block cursor");
}

#[test]
fn test_item_editing_flag_without_buffer_falls_back_to_title() {
    let theme = Theme::default();
    let task = Task::new("Kept title");
    let mut ctx = default_context(&task, &theme);
    ctx.is_editing = true;
    ctx.edit_buffer = None;

    let text = list_item_text(task_to_list_item(&ctx));
    assert!(text.contains("Kept title"));
}

#[test]
fn test_task_list_renders_inline_edit_from_model_input() {
    use crate::ui::components::{InputMode, InputTarget};

    let mut model = Model::new();
    let edited = Task::new("Original title");
    let other = Task::new("Untouched task");
    let edited_id = edited.id;
    model.tasks.insert(edited.id, edited);
    model.tasks.insert(other.id, other);
    model.refresh_visible_tasks();
    model.input.mode = InputMode::Editing;
    model.input.target = InputTarget::EditTask(edited_id);
    model.input.buffer = "Renamed inline".to_string();
    model.input.cursor = model.input.buffer.chars().count();

    let theme = Theme::default();
    let content = buffer_content(&render_widget(TaskList::new(&model, &theme), 60, 10));

    assert!(content.contains("Renamed inline"), "{content}");
    assert!(!content.contains("Original title"));
    assert!(content.contains("Untouched task"), "other rows unchanged");

    // In a view without the task list the row is not edited inline
    model.current_view = crate::app::ViewId::Kanban;
    let content = buffer_content(&render_widget(TaskList::new(&model, &theme), 60, 10));
    assert!(content.contains("Original title"));
}
