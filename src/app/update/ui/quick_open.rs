//! Quick-open fuzzy task picker message handlers.

use crate::app::{FocusPane, Model, UiMessage, ViewId};
use crate::domain::TaskId;
use crate::ui::{get_quick_open_result_count, get_quick_open_task_id};

/// Handle quick-open UI messages.
pub fn handle_ui_quick_open(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ShowQuickOpen => {
            model.quick_open.visible = true;
            model.quick_open.query.clear();
            model.quick_open.cursor = 0;
            model.quick_open.selected = 0;
        }
        UiMessage::HideQuickOpen => {
            model.quick_open.visible = false;
        }
        UiMessage::QuickOpenInput(ch) => {
            model.quick_open.query.insert(model.quick_open.cursor, ch);
            model.quick_open.cursor += ch.len_utf8();
            model.quick_open.selected = 0;
        }
        UiMessage::QuickOpenBackspace => {
            if model.quick_open.cursor > 0 {
                let new_cursor = model.quick_open.query[..model.quick_open.cursor]
                    .char_indices()
                    .last()
                    .map_or(0, |(i, _)| i);
                model
                    .quick_open
                    .query
                    .drain(new_cursor..model.quick_open.cursor);
                model.quick_open.cursor = new_cursor;
                model.quick_open.selected = 0;
            }
        }
        UiMessage::QuickOpenUp => {
            if model.quick_open.selected > 0 {
                model.quick_open.selected -= 1;
            }
        }
        UiMessage::QuickOpenDown => {
            let count = get_quick_open_result_count(model);
            if model.quick_open.selected + 1 < count {
                model.quick_open.selected += 1;
            }
        }
        UiMessage::QuickOpenSelect => {
            let target = get_quick_open_task_id(model);
            model.quick_open.visible = false;
            if let Some(task_id) = target {
                jump_to_task(model, task_id);
            }
        }
        _ => {}
    }
}

/// Move the task-list selection to `task_id`, switching views if needed.
///
/// The picker searches every task, so the chosen one may be hidden by the
/// current view, project selection or the completed-tasks toggle. If it is
/// not in `visible_tasks`, fall back to the full task list (or the Snoozed
/// view for a snoozed task), reveal completed tasks when the target is done,
/// and look again. Search, tag and priority filters are left alone; if they
/// still hide the task a status message says so.
fn jump_to_task(model: &mut Model, task_id: TaskId) {
    if select_visible(model, task_id) {
        return;
    }

    let Some(task) = model.tasks.get(&task_id) else {
        return;
    };
    let view = if task.is_snoozed() {
        ViewId::Snoozed
    } else {
        ViewId::TaskList
    };
    if task.status.is_complete() {
        model.filtering.show_completed = true;
    }

    model.current_view = view;
    model.selected_project = None;
    model.selected_index = 0;
    *model.task_list_state.borrow_mut() = ratatui::widgets::ListState::default();
    model.refresh_visible_tasks();

    if !select_visible(model, task_id) {
        model.alerts.status_message =
            Some("Task is hidden by the active search or filters".to_string());
        model.alerts.status_message_set_at = Some(std::time::Instant::now());
    }
}

/// Select `task_id` if it is currently visible; also focus the task list.
fn select_visible(model: &mut Model, task_id: TaskId) -> bool {
    let Some(pos) = model.visible_tasks.iter().position(|id| *id == task_id) else {
        return false;
    };
    model.selected_index = pos;
    model.focus_pane = FocusPane::TaskList;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::update::ui::handle_ui;
    use crate::domain::{today, ProjectId, Task, TaskStatus};
    use chrono::Duration;

    fn add_task(model: &mut Model, title: &str) -> TaskId {
        let task = Task::new(title);
        let id = task.id;
        model.tasks.insert(id, task);
        id
    }

    /// A model holding `titles` (ids returned in the same order) with the
    /// task list refreshed and the picker open.
    fn model_with_tasks(titles: &[&str]) -> (Model, Vec<TaskId>) {
        let mut model = Model::new();
        let ids = titles.iter().map(|t| add_task(&mut model, t)).collect();
        model.refresh_visible_tasks();
        handle_ui_quick_open(&mut model, UiMessage::ShowQuickOpen);
        (model, ids)
    }

    fn type_query(model: &mut Model, text: &str) {
        for ch in text.chars() {
            handle_ui_quick_open(model, UiMessage::QuickOpenInput(ch));
        }
    }

    #[test]
    fn show_resets_query_cursor_and_selection() {
        let mut model = Model::new();
        model.quick_open.query = "stale".to_string();
        model.quick_open.cursor = 5;
        model.quick_open.selected = 3;
        handle_ui_quick_open(&mut model, UiMessage::ShowQuickOpen);
        assert!(model.quick_open.visible);
        assert!(model.quick_open.query.is_empty());
        assert_eq!(model.quick_open.cursor, 0);
        assert_eq!(model.quick_open.selected, 0);
    }

    #[test]
    fn hide_only_clears_visibility() {
        let (mut model, _) = model_with_tasks(&["Alpha"]);
        type_query(&mut model, "al");
        handle_ui_quick_open(&mut model, UiMessage::HideQuickOpen);
        assert!(!model.quick_open.visible);
        assert_eq!(model.quick_open.query, "al");
    }

    #[test]
    fn input_inserts_at_cursor_and_resets_selection() {
        let (mut model, _) = model_with_tasks(&["Alpha", "Beta", "Gamma"]);
        handle_ui_quick_open(&mut model, UiMessage::QuickOpenDown);
        assert_eq!(model.quick_open.selected, 1);
        type_query(&mut model, "aé");
        assert_eq!(model.quick_open.query, "aé");
        assert_eq!(model.quick_open.cursor, "aé".len());
        assert_eq!(model.quick_open.selected, 0);
    }

    #[test]
    fn backspace_removes_whole_multibyte_char() {
        let (mut model, _) = model_with_tasks(&["Alpha"]);
        type_query(&mut model, "aé");
        handle_ui_quick_open(&mut model, UiMessage::QuickOpenBackspace);
        assert_eq!(model.quick_open.query, "a");
        assert_eq!(model.quick_open.cursor, 1);
        handle_ui_quick_open(&mut model, UiMessage::QuickOpenBackspace);
        assert!(model.quick_open.query.is_empty());
        assert_eq!(model.quick_open.cursor, 0);
    }

    #[test]
    fn backspace_on_empty_query_is_a_noop() {
        let (mut model, _) = model_with_tasks(&["Alpha"]);
        handle_ui_quick_open(&mut model, UiMessage::QuickOpenBackspace);
        assert!(model.quick_open.query.is_empty());
        assert_eq!(model.quick_open.cursor, 0);
    }

    #[test]
    fn up_stops_at_first_result() {
        let (mut model, _) = model_with_tasks(&["Alpha", "Beta"]);
        handle_ui_quick_open(&mut model, UiMessage::QuickOpenUp);
        assert_eq!(model.quick_open.selected, 0);
        handle_ui_quick_open(&mut model, UiMessage::QuickOpenDown);
        handle_ui_quick_open(&mut model, UiMessage::QuickOpenUp);
        assert_eq!(model.quick_open.selected, 0);
    }

    #[test]
    fn down_clamps_to_result_count() {
        // Titles share no letters, so the fuzzy fallback cannot match across them.
        let (mut model, _) = model_with_tasks(&["Alpha", "Bistro", "Quorum"]);
        for _ in 0..5 {
            handle_ui_quick_open(&mut model, UiMessage::QuickOpenDown);
        }
        assert_eq!(model.quick_open.selected, 2);

        // Narrowing the query shrinks the list and the cursor follows.
        type_query(&mut model, "alpha");
        assert_eq!(get_quick_open_result_count(&model), 1);
        handle_ui_quick_open(&mut model, UiMessage::QuickOpenDown);
        assert_eq!(model.quick_open.selected, 0);
    }

    #[test]
    fn down_with_no_tasks_stays_at_zero() {
        let (mut model, _) = model_with_tasks(&[]);
        handle_ui_quick_open(&mut model, UiMessage::QuickOpenDown);
        assert_eq!(model.quick_open.selected, 0);
        assert_eq!(get_quick_open_result_count(&model), 0);
    }

    #[test]
    fn result_count_and_task_id_follow_query_and_selection() {
        let (mut model, ids) = model_with_tasks(&["Bistro", "Alpha", "Quorum"]);
        assert_eq!(get_quick_open_result_count(&model), 3);
        // Empty query: all tasks, sorted by title.
        assert_eq!(get_quick_open_task_id(&model), Some(ids[1]));
        model.quick_open.selected = 1;
        assert_eq!(get_quick_open_task_id(&model), Some(ids[0]));

        type_query(&mut model, "quo");
        assert_eq!(get_quick_open_result_count(&model), 1);
        assert_eq!(get_quick_open_task_id(&model), Some(ids[2]));

        handle_ui_quick_open(&mut model, UiMessage::ShowQuickOpen);
        type_query(&mut model, "zzzz");
        assert_eq!(get_quick_open_result_count(&model), 0);
        assert_eq!(get_quick_open_task_id(&model), None);
    }

    #[test]
    fn select_jumps_to_visible_task_and_hides_picker() {
        let (mut model, ids) = model_with_tasks(&["Alpha", "Beta", "Gamma"]);
        model.focus_pane = FocusPane::Sidebar;
        type_query(&mut model, "gam");
        handle_ui_quick_open(&mut model, UiMessage::QuickOpenSelect);
        assert!(!model.quick_open.visible);
        assert_eq!(model.selected_task_id(), Some(ids[2]));
        assert_eq!(model.focus_pane, FocusPane::TaskList);
        assert_eq!(model.current_view, ViewId::TaskList);
    }

    #[test]
    fn select_with_no_match_just_hides() {
        let (mut model, _) = model_with_tasks(&["Alpha", "Beta"]);
        model.selected_index = 1;
        type_query(&mut model, "zzqx");
        handle_ui_quick_open(&mut model, UiMessage::QuickOpenSelect);
        assert!(!model.quick_open.visible);
        assert_eq!(model.selected_index, 1);
    }

    #[test]
    fn select_reveals_completed_task() {
        let (mut model, ids) = model_with_tasks(&["Alpha", "Beta"]);
        model.tasks.get_mut(&ids[1]).unwrap().status = TaskStatus::Done;
        model.refresh_visible_tasks();
        assert!(!model.visible_tasks.contains(&ids[1]));

        type_query(&mut model, "beta");
        handle_ui_quick_open(&mut model, UiMessage::QuickOpenSelect);
        assert!(model.filtering.show_completed);
        assert_eq!(model.selected_task_id(), Some(ids[1]));
    }

    #[test]
    fn select_switches_from_a_view_that_hides_the_task() {
        let (mut model, ids) = model_with_tasks(&["Alpha", "Beta"]);
        model.current_view = ViewId::Today;
        model.refresh_visible_tasks();
        assert!(!model.visible_tasks.contains(&ids[0]));

        type_query(&mut model, "alpha");
        handle_ui_quick_open(&mut model, UiMessage::QuickOpenSelect);
        assert_eq!(model.current_view, ViewId::TaskList);
        assert_eq!(model.selected_task_id(), Some(ids[0]));
        assert!(model.alerts.status_message.is_none());
    }

    #[test]
    fn select_clears_project_filter() {
        let (mut model, ids) = model_with_tasks(&["Alpha"]);
        model.selected_project = Some(ProjectId::new());
        model.refresh_visible_tasks();
        assert!(model.visible_tasks.is_empty());

        handle_ui_quick_open(&mut model, UiMessage::QuickOpenSelect);
        assert_eq!(model.selected_project, None);
        assert_eq!(model.selected_task_id(), Some(ids[0]));
    }

    #[test]
    fn select_switches_to_snoozed_view_for_snoozed_task() {
        let (mut model, ids) = model_with_tasks(&["Alpha", "Beta"]);
        model
            .tasks
            .get_mut(&ids[1])
            .unwrap()
            .snooze_until_date(today() + Duration::days(3));
        model.refresh_visible_tasks();
        assert!(!model.visible_tasks.contains(&ids[1]));

        type_query(&mut model, "beta");
        handle_ui_quick_open(&mut model, UiMessage::QuickOpenSelect);
        assert_eq!(model.current_view, ViewId::Snoozed);
        assert_eq!(model.selected_task_id(), Some(ids[1]));
    }

    #[test]
    fn select_reports_task_hidden_by_search_filter() {
        let (mut model, ids) = model_with_tasks(&["Alpha", "Gamma"]);
        model.filtering.filter.search_text = Some("alpha".to_string());
        model.refresh_visible_tasks();
        assert!(!model.visible_tasks.contains(&ids[1]));

        type_query(&mut model, "gam");
        handle_ui_quick_open(&mut model, UiMessage::QuickOpenSelect);
        assert!(!model.quick_open.visible);
        assert_ne!(model.selected_task_id(), Some(ids[1]));
        assert!(model.alerts.status_message.is_some());
        assert!(model.alerts.status_message_set_at.is_some());
    }

    #[test]
    fn handle_ui_routes_quick_open_messages() {
        let mut model = Model::new();
        handle_ui(&mut model, UiMessage::ShowQuickOpen);
        assert!(model.quick_open.visible);
        handle_ui(&mut model, UiMessage::QuickOpenInput('x'));
        assert_eq!(model.quick_open.query, "x");
        handle_ui(&mut model, UiMessage::QuickOpenSelect);
        assert!(!model.quick_open.visible);
    }
}
