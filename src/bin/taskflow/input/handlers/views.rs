//! View-specific input handlers.

use crossterm::event::{self, KeyCode};

use taskflow::app::{Message, Model, NavigationMessage, UiMessage};

/// Handle calendar view input
pub fn handle_calendar_view(key: event::KeyEvent, model: &mut Model) -> Option<Message> {
    // Esc returns focus to sidebar
    if key.code == KeyCode::Esc {
        return Some(Message::Navigation(NavigationMessage::FocusSidebar));
    }

    // Tab toggles focus between calendar grid and task list
    if key.code == KeyCode::Tab {
        return Some(if model.calendar_state.focus_task_list {
            Message::Navigation(NavigationMessage::CalendarFocusGrid)
        } else {
            Message::Navigation(NavigationMessage::CalendarFocusTaskList)
        });
    }

    // s schedules the selected task on the focused day (plain s only, so
    // Alt+s keeps editing the time block and S the scheduled date)
    if key.code == KeyCode::Char('s') && key.modifiers == event::KeyModifiers::NONE {
        return Some(Message::Ui(UiMessage::ScheduleTaskToDay));
    }

    if model.calendar_state.focus_task_list {
        // When focused on task list
        match key.code {
            // h goes back to calendar grid
            KeyCode::Char('h') | KeyCode::Left => {
                return Some(Message::Navigation(NavigationMessage::CalendarFocusGrid));
            }
            // Enter opens task detail view (focus mode)
            KeyCode::Enter => {
                return Some(Message::Ui(UiMessage::ToggleFocusMode));
            }
            _ => {}
        }
    } else {
        // When focused on calendar grid, navigate days
        match key.code {
            KeyCode::Left => return Some(Message::Ui(UiMessage::CalendarPrevDay)),
            KeyCode::Right => return Some(Message::Ui(UiMessage::CalendarNextDay)),
            KeyCode::Char('h') => return Some(Message::Ui(UiMessage::CalendarPrevDay)),
            KeyCode::Char('l') | KeyCode::Enter => {
                // l/Enter moves to task list if there are tasks, otherwise next day (for l only)
                if !model.tasks_for_selected_day().is_empty() {
                    return Some(Message::Navigation(
                        NavigationMessage::CalendarFocusTaskList,
                    ));
                }
                // Enter stays on grid if no tasks, l advances to next day
                if key.code == KeyCode::Char('l') {
                    return Some(Message::Ui(UiMessage::CalendarNextDay));
                }
            }
            _ => {}
        }
    }

    None
}

/// Handle habits view input
/// Handle Duplicates view keys.
///
/// These are view-scoped so that `D` and `M` keep their global meanings
/// (edit due date / reschedule to next Monday) in every other view.
pub fn handle_duplicates_view(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('D') => Some(Message::Ui(UiMessage::DismissDuplicate)),
        KeyCode::Char('M') => Some(Message::Ui(UiMessage::MergeDuplicates)),
        KeyCode::Char('r') => Some(Message::Ui(UiMessage::RefreshDuplicates)),
        _ => None,
    }
}

pub fn handle_habits_view(key: event::KeyEvent, model: &Model) -> Option<Message> {
    match key.code {
        // Return focus to sidebar
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::FocusSidebar)),
        // Navigation
        KeyCode::Up | KeyCode::Char('k') => Some(Message::Ui(UiMessage::HabitUp)),
        KeyCode::Down | KeyCode::Char('j') => Some(Message::Ui(UiMessage::HabitDown)),
        // Create new habit
        KeyCode::Char('n') => Some(Message::Ui(UiMessage::StartCreateHabit)),
        // Edit selected habit
        KeyCode::Char('e') => {
            if let Some(&habit_id) = model.visible_habits.get(model.habit_view.selected) {
                Some(Message::Ui(UiMessage::StartEditHabit(habit_id)))
            } else {
                None
            }
        }
        // Delete selected habit
        KeyCode::Char('d') => Some(Message::Ui(UiMessage::HabitDelete)),
        // Toggle today's check-in
        KeyCode::Char(' ') | KeyCode::Char('x') => Some(Message::Ui(UiMessage::HabitToggleToday)),
        // Show analytics
        KeyCode::Char('a') => Some(Message::Ui(UiMessage::ShowHabitAnalytics)),
        // Archive habit
        KeyCode::Char('A') => Some(Message::Ui(UiMessage::HabitArchive)),
        // Toggle showing archived habits
        KeyCode::Char('H') => Some(Message::Ui(UiMessage::HabitToggleShowArchived)),
        _ => None,
    }
}

/// Handle goals view input
pub fn handle_goals_view(key: event::KeyEvent, model: &Model) -> Option<Message> {
    use taskflow::app::GoalMessage;

    match key.code {
        // Return focus to sidebar
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::FocusSidebar)),
        // Navigation
        KeyCode::Up | KeyCode::Char('k') => Some(Message::Goal(GoalMessage::NavigateUp)),
        KeyCode::Down | KeyCode::Char('j') => Some(Message::Goal(GoalMessage::NavigateDown)),
        // Create new goal
        KeyCode::Char('n') => Some(Message::Ui(UiMessage::StartCreateGoal)),
        // Create new key result (under selected goal)
        KeyCode::Char('N') => Some(Message::Ui(UiMessage::StartCreateKeyResult)),
        // Expand/collapse goal OR navigate into key results
        KeyCode::Enter | KeyCode::Char('l') => Some(Message::Goal(GoalMessage::NavigateInto)),
        // Navigate back (collapse)
        KeyCode::Char('h') => Some(Message::Goal(GoalMessage::NavigateBack)),
        // Edit selected goal
        KeyCode::Char('e') => {
            if let Some(&goal_id) = model.visible_goals.get(model.goal_view.selected_goal) {
                Some(Message::Ui(UiMessage::StartEditGoal(goal_id)))
            } else {
                None
            }
        }
        // Delete selected goal
        KeyCode::Char('d') => {
            if let Some(&goal_id) = model.visible_goals.get(model.goal_view.selected_goal) {
                Some(Message::Goal(GoalMessage::Delete(goal_id)))
            } else {
                None
            }
        }
        // Toggle show archived
        KeyCode::Char('a') => Some(Message::Goal(GoalMessage::ToggleArchived)),
        _ => None,
    }
}

/// Handle timeline view input
pub fn handle_timeline_view(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        // Exit timeline view
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::FocusSidebar)),
        // Scroll time axis
        KeyCode::Char('h') | KeyCode::Left => {
            Some(Message::Navigation(NavigationMessage::TimelineScrollLeft))
        }
        KeyCode::Char('l') | KeyCode::Right => {
            Some(Message::Navigation(NavigationMessage::TimelineScrollRight))
        }
        // Navigate tasks
        KeyCode::Up | KeyCode::Char('k') => {
            Some(Message::Navigation(NavigationMessage::TimelineUp))
        }
        KeyCode::Down | KeyCode::Char('j') => {
            Some(Message::Navigation(NavigationMessage::TimelineDown))
        }
        // Zoom controls
        KeyCode::Char('<') | KeyCode::Char(',') => {
            Some(Message::Navigation(NavigationMessage::TimelineZoomOut))
        }
        KeyCode::Char('>') | KeyCode::Char('.') => {
            Some(Message::Navigation(NavigationMessage::TimelineZoomIn))
        }
        // Jump to today
        KeyCode::Char('t') => Some(Message::Navigation(NavigationMessage::TimelineGoToday)),
        // Toggle dependency lines
        KeyCode::Char('d') => Some(Message::Ui(UiMessage::TimelineToggleDependencies)),
        // View task details (focus mode)
        KeyCode::Enter => Some(Message::Ui(UiMessage::TimelineViewSelected)),
        _ => None,
    }
}

/// Handle kanban view input
pub fn handle_kanban_view(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::FocusSidebar)),
        KeyCode::Char('h') | KeyCode::Left => {
            Some(Message::Navigation(NavigationMessage::KanbanLeft))
        }
        KeyCode::Char('l') | KeyCode::Right => {
            Some(Message::Navigation(NavigationMessage::KanbanRight))
        }
        KeyCode::Char('k') | KeyCode::Up => Some(Message::Navigation(NavigationMessage::KanbanUp)),
        KeyCode::Char('j') | KeyCode::Down => {
            Some(Message::Navigation(NavigationMessage::KanbanDown))
        }
        KeyCode::Enter => Some(Message::Ui(UiMessage::KanbanViewSelected)),
        _ => None,
    }
}

/// Handle eisenhower view input
pub fn handle_eisenhower_view(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::FocusSidebar)),
        KeyCode::Char('h') | KeyCode::Left => {
            Some(Message::Navigation(NavigationMessage::EisenhowerLeft))
        }
        KeyCode::Char('l') | KeyCode::Right => {
            Some(Message::Navigation(NavigationMessage::EisenhowerRight))
        }
        KeyCode::Char('k') | KeyCode::Up => {
            Some(Message::Navigation(NavigationMessage::EisenhowerUp))
        }
        KeyCode::Char('j') | KeyCode::Down => {
            Some(Message::Navigation(NavigationMessage::EisenhowerDown))
        }
        KeyCode::Enter => Some(Message::Ui(UiMessage::EisenhowerViewSelected)),
        _ => None,
    }
}

/// Handle weekly planner view input
pub fn handle_weekly_planner_view(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::FocusSidebar)),
        KeyCode::Char('h') | KeyCode::Left => {
            Some(Message::Navigation(NavigationMessage::WeeklyPlannerLeft))
        }
        KeyCode::Char('l') | KeyCode::Right => {
            Some(Message::Navigation(NavigationMessage::WeeklyPlannerRight))
        }
        KeyCode::Char('k') | KeyCode::Up => {
            Some(Message::Navigation(NavigationMessage::WeeklyPlannerUp))
        }
        KeyCode::Char('j') | KeyCode::Down => {
            Some(Message::Navigation(NavigationMessage::WeeklyPlannerDown))
        }
        KeyCode::Enter => Some(Message::Ui(UiMessage::WeeklyPlannerViewSelected)),
        _ => None,
    }
}

/// Handle reports view input
pub fn handle_reports_view(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::FocusSidebar)),
        KeyCode::Tab => Some(Message::Navigation(NavigationMessage::ReportsNextPanel)),
        KeyCode::BackTab => Some(Message::Navigation(NavigationMessage::ReportsPrevPanel)),
        KeyCode::Char('w') => Some(Message::Ui(UiMessage::ReportsCycleTimeWindow)),
        // Tasks panel table (no-ops on the other panels)
        KeyCode::Char('j') | KeyCode::Down => Some(Message::Ui(UiMessage::ReportsTasksMove(1))),
        KeyCode::Char('k') | KeyCode::Up => Some(Message::Ui(UiMessage::ReportsTasksMove(-1))),
        KeyCode::PageDown => Some(Message::Ui(UiMessage::ReportsTasksMove(10))),
        KeyCode::PageUp => Some(Message::Ui(UiMessage::ReportsTasksMove(-10))),
        KeyCode::Char('d') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            Some(Message::Ui(UiMessage::ReportsTasksMove(10)))
        }
        KeyCode::Char('u') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            Some(Message::Ui(UiMessage::ReportsTasksMove(-10)))
        }
        KeyCode::Char('g') => Some(Message::Ui(UiMessage::ReportsTasksMove(i32::MIN))),
        KeyCode::Char('G') => Some(Message::Ui(UiMessage::ReportsTasksMove(i32::MAX))),
        _ => None,
    }
}

/// Handle network view input
pub fn handle_network_view(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::FocusSidebar)),
        // h/l as aliases for k/j to match other views
        KeyCode::Char('k') | KeyCode::Up | KeyCode::Char('h') | KeyCode::Left => {
            Some(Message::Navigation(NavigationMessage::NetworkUp))
        }
        KeyCode::Char('j') | KeyCode::Down | KeyCode::Char('l') | KeyCode::Right => {
            Some(Message::Navigation(NavigationMessage::NetworkDown))
        }
        KeyCode::Enter => Some(Message::Ui(UiMessage::NetworkViewSelected)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use crossterm::event::{KeyEvent, KeyModifiers};
    use taskflow::app::GoalMessage;
    use taskflow::domain::{Goal, GoalId, Habit, HabitId, Task};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ch(c: char) -> KeyEvent {
        key(KeyCode::Char(c))
    }

    /// Calendar model pinned to March 2024 with day 15 selected so the
    /// expectations never depend on the wall clock.
    fn calendar_model(with_task_on_selected_day: bool) -> Model {
        let mut model = Model::new();
        model.calendar_state.year = 2024;
        model.calendar_state.month = 3;
        model.calendar_state.selected_day = Some(15);
        model.calendar_state.focus_task_list = false;
        if with_task_on_selected_day {
            let mut task = Task::new("Due mid-March");
            task.due_date = NaiveDate::from_ymd_opt(2024, 3, 15);
            model.tasks.insert(task.id, task);
            model.rebuild_caches();
            assert_eq!(model.tasks_for_selected_day().len(), 1);
        }
        model
    }

    fn habits_model() -> (Model, HabitId) {
        let mut model = Model::new();
        let habit = Habit::new("Stretch");
        let id = habit.id;
        model.habits.insert(id, habit);
        model.refresh_visible_habits();
        assert_eq!(model.visible_habits, vec![id]);
        (model, id)
    }

    fn goals_model() -> (Model, GoalId) {
        let mut model = Model::new();
        let goal = Goal::new("Ship v1");
        let id = goal.id;
        model.goals.insert(id, goal);
        model.refresh_visible_goals();
        assert_eq!(model.visible_goals, vec![id]);
        (model, id)
    }

    // ========================================================================
    // Calendar
    // ========================================================================

    #[test]
    fn calendar_esc_returns_focus_to_sidebar() {
        let mut model = calendar_model(false);
        assert!(matches!(
            handle_calendar_view(key(KeyCode::Esc), &mut model),
            Some(Message::Navigation(NavigationMessage::FocusSidebar))
        ));
    }

    #[test]
    fn calendar_tab_toggles_between_grid_and_task_list() {
        let mut model = calendar_model(false);
        assert!(matches!(
            handle_calendar_view(key(KeyCode::Tab), &mut model),
            Some(Message::Navigation(
                NavigationMessage::CalendarFocusTaskList
            ))
        ));
        model.calendar_state.focus_task_list = true;
        assert!(matches!(
            handle_calendar_view(key(KeyCode::Tab), &mut model),
            Some(Message::Navigation(NavigationMessage::CalendarFocusGrid))
        ));
    }

    #[test]
    fn calendar_task_list_focus_keys() {
        let mut model = calendar_model(false);
        model.calendar_state.focus_task_list = true;
        for k in [ch('h'), key(KeyCode::Left)] {
            assert!(matches!(
                handle_calendar_view(k, &mut model),
                Some(Message::Navigation(NavigationMessage::CalendarFocusGrid))
            ));
        }
        assert!(matches!(
            handle_calendar_view(key(KeyCode::Enter), &mut model),
            Some(Message::Ui(UiMessage::ToggleFocusMode))
        ));
        // j/k and others fall through to the global keybindings
        assert!(handle_calendar_view(ch('j'), &mut model).is_none());
        assert!(handle_calendar_view(key(KeyCode::Right), &mut model).is_none());
    }

    #[test]
    fn calendar_grid_navigates_days() {
        let mut model = calendar_model(false);
        assert!(matches!(
            handle_calendar_view(key(KeyCode::Left), &mut model),
            Some(Message::Ui(UiMessage::CalendarPrevDay))
        ));
        assert!(matches!(
            handle_calendar_view(ch('h'), &mut model),
            Some(Message::Ui(UiMessage::CalendarPrevDay))
        ));
        assert!(matches!(
            handle_calendar_view(key(KeyCode::Right), &mut model),
            Some(Message::Ui(UiMessage::CalendarNextDay))
        ));
        assert!(handle_calendar_view(ch('q'), &mut model).is_none());
    }

    #[test]
    fn calendar_grid_l_and_enter_without_tasks() {
        let mut model = calendar_model(false);
        // l advances a day when the selected day has no tasks
        assert!(matches!(
            handle_calendar_view(ch('l'), &mut model),
            Some(Message::Ui(UiMessage::CalendarNextDay))
        ));
        // Enter stays put
        assert!(handle_calendar_view(key(KeyCode::Enter), &mut model).is_none());
    }

    #[test]
    fn calendar_grid_l_and_enter_with_tasks_focus_task_list() {
        let mut model = calendar_model(true);
        for k in [ch('l'), key(KeyCode::Enter)] {
            assert!(matches!(
                handle_calendar_view(k, &mut model),
                Some(Message::Navigation(
                    NavigationMessage::CalendarFocusTaskList
                ))
            ));
        }
    }

    // ========================================================================
    // Duplicates
    // ========================================================================

    #[test]
    fn duplicates_view_keys() {
        assert!(matches!(
            handle_duplicates_view(ch('D')),
            Some(Message::Ui(UiMessage::DismissDuplicate))
        ));
        assert!(matches!(
            handle_duplicates_view(ch('M')),
            Some(Message::Ui(UiMessage::MergeDuplicates))
        ));
        assert!(matches!(
            handle_duplicates_view(ch('r')),
            Some(Message::Ui(UiMessage::RefreshDuplicates))
        ));
        assert!(handle_duplicates_view(ch('d')).is_none());
        assert!(handle_duplicates_view(key(KeyCode::Esc)).is_none());
    }

    // ========================================================================
    // Habits
    // ========================================================================

    #[test]
    fn habits_view_navigation_and_actions() {
        let (model, _) = habits_model();
        assert!(matches!(
            handle_habits_view(key(KeyCode::Esc), &model),
            Some(Message::Navigation(NavigationMessage::FocusSidebar))
        ));
        for k in [key(KeyCode::Up), ch('k')] {
            assert!(matches!(
                handle_habits_view(k, &model),
                Some(Message::Ui(UiMessage::HabitUp))
            ));
        }
        for k in [key(KeyCode::Down), ch('j')] {
            assert!(matches!(
                handle_habits_view(k, &model),
                Some(Message::Ui(UiMessage::HabitDown))
            ));
        }
        assert!(matches!(
            handle_habits_view(ch('n'), &model),
            Some(Message::Ui(UiMessage::StartCreateHabit))
        ));
        assert!(matches!(
            handle_habits_view(ch('d'), &model),
            Some(Message::Ui(UiMessage::HabitDelete))
        ));
        for k in [ch(' '), ch('x')] {
            assert!(matches!(
                handle_habits_view(k, &model),
                Some(Message::Ui(UiMessage::HabitToggleToday))
            ));
        }
        assert!(matches!(
            handle_habits_view(ch('a'), &model),
            Some(Message::Ui(UiMessage::ShowHabitAnalytics))
        ));
        assert!(matches!(
            handle_habits_view(ch('A'), &model),
            Some(Message::Ui(UiMessage::HabitArchive))
        ));
        assert!(matches!(
            handle_habits_view(ch('H'), &model),
            Some(Message::Ui(UiMessage::HabitToggleShowArchived))
        ));
        assert!(handle_habits_view(ch('q'), &model).is_none());
        assert!(handle_habits_view(key(KeyCode::Enter), &model).is_none());
    }

    #[test]
    fn habits_view_edit_uses_selected_habit_id() {
        let (model, id) = habits_model();
        assert!(matches!(
            handle_habits_view(ch('e'), &model),
            Some(Message::Ui(UiMessage::StartEditHabit(edit_id))) if edit_id == id
        ));
    }

    #[test]
    fn habits_view_edit_without_habits_is_ignored() {
        let model = Model::new();
        assert!(model.visible_habits.is_empty());
        assert!(handle_habits_view(ch('e'), &model).is_none());
    }

    // ========================================================================
    // Goals
    // ========================================================================

    #[test]
    fn goals_view_navigation_and_actions() {
        let (model, _) = goals_model();
        assert!(matches!(
            handle_goals_view(key(KeyCode::Esc), &model),
            Some(Message::Navigation(NavigationMessage::FocusSidebar))
        ));
        for k in [key(KeyCode::Up), ch('k')] {
            assert!(matches!(
                handle_goals_view(k, &model),
                Some(Message::Goal(GoalMessage::NavigateUp))
            ));
        }
        for k in [key(KeyCode::Down), ch('j')] {
            assert!(matches!(
                handle_goals_view(k, &model),
                Some(Message::Goal(GoalMessage::NavigateDown))
            ));
        }
        assert!(matches!(
            handle_goals_view(ch('n'), &model),
            Some(Message::Ui(UiMessage::StartCreateGoal))
        ));
        assert!(matches!(
            handle_goals_view(ch('N'), &model),
            Some(Message::Ui(UiMessage::StartCreateKeyResult))
        ));
        for k in [key(KeyCode::Enter), ch('l')] {
            assert!(matches!(
                handle_goals_view(k, &model),
                Some(Message::Goal(GoalMessage::NavigateInto))
            ));
        }
        assert!(matches!(
            handle_goals_view(ch('h'), &model),
            Some(Message::Goal(GoalMessage::NavigateBack))
        ));
        assert!(matches!(
            handle_goals_view(ch('a'), &model),
            Some(Message::Goal(GoalMessage::ToggleArchived))
        ));
        assert!(handle_goals_view(ch('q'), &model).is_none());
        assert!(handle_goals_view(key(KeyCode::Tab), &model).is_none());
    }

    #[test]
    fn goals_view_edit_and_delete_use_selected_goal_id() {
        let (model, id) = goals_model();
        assert!(matches!(
            handle_goals_view(ch('e'), &model),
            Some(Message::Ui(UiMessage::StartEditGoal(edit_id))) if edit_id == id
        ));
        assert!(matches!(
            handle_goals_view(ch('d'), &model),
            Some(Message::Goal(GoalMessage::Delete(del_id))) if del_id == id
        ));
    }

    #[test]
    fn goals_view_edit_and_delete_without_goals_are_ignored() {
        let model = Model::new();
        assert!(model.visible_goals.is_empty());
        assert!(handle_goals_view(ch('e'), &model).is_none());
        assert!(handle_goals_view(ch('d'), &model).is_none());
    }

    // ========================================================================
    // Timeline
    // ========================================================================

    #[test]
    fn timeline_view_keys() {
        assert!(matches!(
            handle_timeline_view(key(KeyCode::Esc)),
            Some(Message::Navigation(NavigationMessage::FocusSidebar))
        ));
        for k in [ch('h'), key(KeyCode::Left)] {
            assert!(matches!(
                handle_timeline_view(k),
                Some(Message::Navigation(NavigationMessage::TimelineScrollLeft))
            ));
        }
        for k in [ch('l'), key(KeyCode::Right)] {
            assert!(matches!(
                handle_timeline_view(k),
                Some(Message::Navigation(NavigationMessage::TimelineScrollRight))
            ));
        }
        for k in [ch('k'), key(KeyCode::Up)] {
            assert!(matches!(
                handle_timeline_view(k),
                Some(Message::Navigation(NavigationMessage::TimelineUp))
            ));
        }
        for k in [ch('j'), key(KeyCode::Down)] {
            assert!(matches!(
                handle_timeline_view(k),
                Some(Message::Navigation(NavigationMessage::TimelineDown))
            ));
        }
        for k in [ch('<'), ch(',')] {
            assert!(matches!(
                handle_timeline_view(k),
                Some(Message::Navigation(NavigationMessage::TimelineZoomOut))
            ));
        }
        for k in [ch('>'), ch('.')] {
            assert!(matches!(
                handle_timeline_view(k),
                Some(Message::Navigation(NavigationMessage::TimelineZoomIn))
            ));
        }
        assert!(matches!(
            handle_timeline_view(ch('t')),
            Some(Message::Navigation(NavigationMessage::TimelineGoToday))
        ));
        assert!(matches!(
            handle_timeline_view(ch('d')),
            Some(Message::Ui(UiMessage::TimelineToggleDependencies))
        ));
        assert!(matches!(
            handle_timeline_view(key(KeyCode::Enter)),
            Some(Message::Ui(UiMessage::TimelineViewSelected))
        ));
        assert!(handle_timeline_view(ch('q')).is_none());
        assert!(handle_timeline_view(key(KeyCode::Tab)).is_none());
    }

    // ========================================================================
    // Kanban / Eisenhower / Weekly planner share the same shape
    // ========================================================================

    #[test]
    fn kanban_view_keys() {
        assert!(matches!(
            handle_kanban_view(key(KeyCode::Esc)),
            Some(Message::Navigation(NavigationMessage::FocusSidebar))
        ));
        for k in [ch('h'), key(KeyCode::Left)] {
            assert!(matches!(
                handle_kanban_view(k),
                Some(Message::Navigation(NavigationMessage::KanbanLeft))
            ));
        }
        for k in [ch('l'), key(KeyCode::Right)] {
            assert!(matches!(
                handle_kanban_view(k),
                Some(Message::Navigation(NavigationMessage::KanbanRight))
            ));
        }
        for k in [ch('k'), key(KeyCode::Up)] {
            assert!(matches!(
                handle_kanban_view(k),
                Some(Message::Navigation(NavigationMessage::KanbanUp))
            ));
        }
        for k in [ch('j'), key(KeyCode::Down)] {
            assert!(matches!(
                handle_kanban_view(k),
                Some(Message::Navigation(NavigationMessage::KanbanDown))
            ));
        }
        assert!(matches!(
            handle_kanban_view(key(KeyCode::Enter)),
            Some(Message::Ui(UiMessage::KanbanViewSelected))
        ));
        assert!(handle_kanban_view(ch('x')).is_none());
    }

    #[test]
    fn eisenhower_view_keys() {
        assert!(matches!(
            handle_eisenhower_view(key(KeyCode::Esc)),
            Some(Message::Navigation(NavigationMessage::FocusSidebar))
        ));
        for k in [ch('h'), key(KeyCode::Left)] {
            assert!(matches!(
                handle_eisenhower_view(k),
                Some(Message::Navigation(NavigationMessage::EisenhowerLeft))
            ));
        }
        for k in [ch('l'), key(KeyCode::Right)] {
            assert!(matches!(
                handle_eisenhower_view(k),
                Some(Message::Navigation(NavigationMessage::EisenhowerRight))
            ));
        }
        for k in [ch('k'), key(KeyCode::Up)] {
            assert!(matches!(
                handle_eisenhower_view(k),
                Some(Message::Navigation(NavigationMessage::EisenhowerUp))
            ));
        }
        for k in [ch('j'), key(KeyCode::Down)] {
            assert!(matches!(
                handle_eisenhower_view(k),
                Some(Message::Navigation(NavigationMessage::EisenhowerDown))
            ));
        }
        assert!(matches!(
            handle_eisenhower_view(key(KeyCode::Enter)),
            Some(Message::Ui(UiMessage::EisenhowerViewSelected))
        ));
        assert!(handle_eisenhower_view(ch('x')).is_none());
    }

    #[test]
    fn weekly_planner_view_keys() {
        assert!(matches!(
            handle_weekly_planner_view(key(KeyCode::Esc)),
            Some(Message::Navigation(NavigationMessage::FocusSidebar))
        ));
        for k in [ch('h'), key(KeyCode::Left)] {
            assert!(matches!(
                handle_weekly_planner_view(k),
                Some(Message::Navigation(NavigationMessage::WeeklyPlannerLeft))
            ));
        }
        for k in [ch('l'), key(KeyCode::Right)] {
            assert!(matches!(
                handle_weekly_planner_view(k),
                Some(Message::Navigation(NavigationMessage::WeeklyPlannerRight))
            ));
        }
        for k in [ch('k'), key(KeyCode::Up)] {
            assert!(matches!(
                handle_weekly_planner_view(k),
                Some(Message::Navigation(NavigationMessage::WeeklyPlannerUp))
            ));
        }
        for k in [ch('j'), key(KeyCode::Down)] {
            assert!(matches!(
                handle_weekly_planner_view(k),
                Some(Message::Navigation(NavigationMessage::WeeklyPlannerDown))
            ));
        }
        assert!(matches!(
            handle_weekly_planner_view(key(KeyCode::Enter)),
            Some(Message::Ui(UiMessage::WeeklyPlannerViewSelected))
        ));
        assert!(handle_weekly_planner_view(ch('x')).is_none());
    }

    // ========================================================================
    // Reports / Network
    // ========================================================================

    #[test]
    fn reports_view_keys() {
        assert!(matches!(
            handle_reports_view(key(KeyCode::Esc)),
            Some(Message::Navigation(NavigationMessage::FocusSidebar))
        ));
        assert!(matches!(
            handle_reports_view(key(KeyCode::Tab)),
            Some(Message::Navigation(NavigationMessage::ReportsNextPanel))
        ));
        assert!(matches!(
            handle_reports_view(key(KeyCode::BackTab)),
            Some(Message::Navigation(NavigationMessage::ReportsPrevPanel))
        ));
        assert!(matches!(
            handle_reports_view(ch('w')),
            Some(Message::Ui(UiMessage::ReportsCycleTimeWindow))
        ));
        // j/k now drive the Tasks table; unbound keys still fall through.
        assert!(matches!(
            handle_reports_view(ch('j')),
            Some(Message::Ui(UiMessage::ReportsTasksMove(1)))
        ));
        assert!(handle_reports_view(ch('z')).is_none());
    }

    #[test]
    fn network_view_keys() {
        assert!(matches!(
            handle_network_view(key(KeyCode::Esc)),
            Some(Message::Navigation(NavigationMessage::FocusSidebar))
        ));
        for k in [ch('k'), key(KeyCode::Up), ch('h'), key(KeyCode::Left)] {
            assert!(matches!(
                handle_network_view(k),
                Some(Message::Navigation(NavigationMessage::NetworkUp))
            ));
        }
        for k in [ch('j'), key(KeyCode::Down), ch('l'), key(KeyCode::Right)] {
            assert!(matches!(
                handle_network_view(k),
                Some(Message::Navigation(NavigationMessage::NetworkDown))
            ));
        }
        assert!(matches!(
            handle_network_view(key(KeyCode::Enter)),
            Some(Message::Ui(UiMessage::NetworkViewSelected))
        ));
        assert!(handle_network_view(ch('x')).is_none());
    }

    #[test]
    fn reports_view_task_table_keys() {
        use taskflow::app::UiMessage;
        let cases: [(KeyEvent, i32); 8] = [
            (ch('j'), 1),
            (key(KeyCode::Down), 1),
            (ch('k'), -1),
            (key(KeyCode::Up), -1),
            (key(KeyCode::PageDown), 10),
            (key(KeyCode::PageUp), -10),
            (KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL), 10),
            (
                KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
                -10,
            ),
        ];
        for (k, delta) in cases {
            assert!(
                matches!(
                    handle_reports_view(k),
                    Some(Message::Ui(UiMessage::ReportsTasksMove(d))) if d == delta
                ),
                "{k:?} should move by {delta}"
            );
        }
        assert!(matches!(
            handle_reports_view(ch('g')),
            Some(Message::Ui(UiMessage::ReportsTasksMove(i32::MIN)))
        ));
        assert!(matches!(
            handle_reports_view(ch('G')),
            Some(Message::Ui(UiMessage::ReportsTasksMove(i32::MAX)))
        ));
        assert!(handle_reports_view(ch('x')).is_none());
    }
}
