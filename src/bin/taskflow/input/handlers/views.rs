//! View-specific input handlers.

use crossterm::event::{self, KeyCode};

use taskflow::app::{Message, Model, NavigationMessage, UiMessage};

/// Handle calendar view input
pub fn handle_calendar_view(key: event::KeyEvent, model: &mut Model) -> Option<Message> {
    // Esc exits to task list
    if key.code == KeyCode::Esc {
        return Some(Message::Navigation(NavigationMessage::GoToView(
            taskflow::app::ViewId::TaskList,
        )));
    }

    // Tab toggles focus between calendar grid and task list
    if key.code == KeyCode::Tab {
        return Some(if model.calendar_state.focus_task_list {
            Message::Navigation(NavigationMessage::CalendarFocusGrid)
        } else {
            Message::Navigation(NavigationMessage::CalendarFocusTaskList)
        });
    }

    if model.calendar_state.focus_task_list {
        // When focused on task list, h goes back to calendar grid
        match key.code {
            KeyCode::Char('h') | KeyCode::Left => {
                return Some(Message::Navigation(NavigationMessage::CalendarFocusGrid));
            }
            _ => {}
        }
    } else {
        // When focused on calendar grid, navigate days
        match key.code {
            KeyCode::Left => return Some(Message::Ui(UiMessage::CalendarPrevDay)),
            KeyCode::Right => return Some(Message::Ui(UiMessage::CalendarNextDay)),
            KeyCode::Char('h') => return Some(Message::Ui(UiMessage::CalendarPrevDay)),
            KeyCode::Char('l') => {
                // l moves to task list if there are tasks, otherwise next day
                if !model.tasks_for_selected_day().is_empty() {
                    return Some(Message::Navigation(
                        NavigationMessage::CalendarFocusTaskList,
                    ));
                }
                return Some(Message::Ui(UiMessage::CalendarNextDay));
            }
            _ => {}
        }
    }

    None
}

/// Handle habits view input
pub fn handle_habits_view(key: event::KeyEvent, model: &Model) -> Option<Message> {
    match key.code {
        // Exit to task list
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::GoToView(
            taskflow::app::ViewId::TaskList,
        ))),
        // Navigation
        KeyCode::Up | KeyCode::Char('k') => Some(Message::Ui(UiMessage::HabitUp)),
        KeyCode::Down | KeyCode::Char('j') => Some(Message::Ui(UiMessage::HabitDown)),
        // Create new habit
        KeyCode::Char('n') => Some(Message::Ui(UiMessage::StartCreateHabit)),
        // Edit selected habit
        KeyCode::Char('e') => {
            if let Some(&habit_id) = model.visible_habits.get(model.habit_selected) {
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

/// Handle timeline view input
pub fn handle_timeline_view(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        // Exit timeline view
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::GoToView(
            taskflow::app::ViewId::TaskList,
        ))),
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
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::GoToView(
            taskflow::app::ViewId::TaskList,
        ))),
        KeyCode::Char('h') | KeyCode::Left => {
            Some(Message::Navigation(NavigationMessage::KanbanLeft))
        }
        KeyCode::Char('l') | KeyCode::Right => {
            Some(Message::Navigation(NavigationMessage::KanbanRight))
        }
        _ => None,
    }
}

/// Handle eisenhower view input
pub fn handle_eisenhower_view(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::GoToView(
            taskflow::app::ViewId::TaskList,
        ))),
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
        _ => None,
    }
}

/// Handle weekly planner view input
pub fn handle_weekly_planner_view(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::GoToView(
            taskflow::app::ViewId::TaskList,
        ))),
        KeyCode::Char('h') | KeyCode::Left => {
            Some(Message::Navigation(NavigationMessage::WeeklyPlannerLeft))
        }
        KeyCode::Char('l') | KeyCode::Right => {
            Some(Message::Navigation(NavigationMessage::WeeklyPlannerRight))
        }
        _ => None,
    }
}

/// Handle reports view input
pub fn handle_reports_view(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Esc => Some(Message::Navigation(NavigationMessage::GoToView(
            taskflow::app::ViewId::TaskList,
        ))),
        KeyCode::Tab => Some(Message::Navigation(NavigationMessage::ReportsNextPanel)),
        KeyCode::BackTab => Some(Message::Navigation(NavigationMessage::ReportsPrevPanel)),
        _ => None,
    }
}
