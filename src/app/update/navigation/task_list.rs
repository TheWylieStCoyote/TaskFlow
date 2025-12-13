//! Basic task list navigation handlers.
//!
//! Handles Up, Down, First, Last, PageUp, PageDown, and Select for the main task list.

use crate::app::{FocusPane, Model, NavigationMessage, ViewId};

use super::calendar::{handle_calendar_down, handle_calendar_up};
use super::sidebar::{skip_sidebar_non_selectable_down, skip_sidebar_non_selectable_up};

/// Handle basic task list navigation messages (Up, Down, First, Last, etc.).
pub fn handle_task_list_navigation(model: &mut Model, msg: NavigationMessage) {
    match msg {
        NavigationMessage::Up => match model.focus_pane {
            FocusPane::TaskList => {
                if model.current_view == ViewId::Calendar {
                    if model.calendar_state.focus_task_list {
                        // Navigate tasks in calendar task list
                        if model.selected_index > 0 {
                            model.selected_index -= 1;
                        }
                    } else {
                        // In calendar grid, up moves to previous week (or wraps)
                        handle_calendar_up(model);
                    }
                } else if model.current_view == ViewId::Duplicates {
                    // Navigate duplicate pairs
                    if model.duplicates_view.selected > 0 {
                        model.duplicates_view.selected -= 1;
                        // Adjust scroll offset to keep selection visible
                        if model.duplicates_view.selected < model.duplicates_view.scroll_offset {
                            model.duplicates_view.scroll_offset = model.duplicates_view.selected;
                        }
                    }
                } else if model.selected_index > 0 {
                    model.selected_index -= 1;
                }
            }
            FocusPane::Sidebar => {
                if model.sidebar_selected > 0 {
                    model.sidebar_selected -= 1;
                    // Skip separators and headers
                    skip_sidebar_non_selectable_up(model);
                }
            }
        },
        NavigationMessage::Down => match model.focus_pane {
            FocusPane::TaskList => {
                if model.current_view == ViewId::Calendar {
                    if model.calendar_state.focus_task_list {
                        // Navigate tasks in calendar task list
                        let task_count = model.tasks_for_selected_day().len();
                        if model.selected_index < task_count.saturating_sub(1) {
                            model.selected_index += 1;
                        }
                    } else {
                        // In calendar grid, down moves to next week (or wraps)
                        handle_calendar_down(model);
                    }
                } else if model.current_view == ViewId::Duplicates {
                    // Navigate duplicate pairs
                    let max_index = model.duplicates_view.pairs.len().saturating_sub(1);
                    if model.duplicates_view.selected < max_index {
                        model.duplicates_view.selected += 1;
                        // Adjust scroll offset to keep selection visible (estimate viewport ~5 rows)
                        const ESTIMATED_VIEWPORT: usize = 5;
                        let scroll = model.duplicates_view.scroll_offset;
                        if model.duplicates_view.selected >= scroll + ESTIMATED_VIEWPORT {
                            model.duplicates_view.scroll_offset = model
                                .duplicates_view
                                .selected
                                .saturating_sub(ESTIMATED_VIEWPORT - 1);
                        }
                    }
                } else if model.selected_index < model.visible_tasks.len().saturating_sub(1) {
                    model.selected_index += 1;
                }
            }
            FocusPane::Sidebar => {
                let max_index = model.sidebar_item_count().saturating_sub(1);
                if model.sidebar_selected < max_index {
                    model.sidebar_selected += 1;
                    // Skip separators and headers
                    skip_sidebar_non_selectable_down(model, max_index);
                }
            }
        },
        NavigationMessage::First => match model.focus_pane {
            FocusPane::TaskList => model.selected_index = 0,
            FocusPane::Sidebar => model.sidebar_selected = 0,
        },
        NavigationMessage::Last => match model.focus_pane {
            FocusPane::TaskList => {
                if !model.visible_tasks.is_empty() {
                    model.selected_index = model.visible_tasks.len() - 1;
                }
            }
            FocusPane::Sidebar => {
                model.sidebar_selected = model.sidebar_item_count().saturating_sub(1);
            }
        },
        NavigationMessage::PageUp => match model.focus_pane {
            FocusPane::TaskList => {
                model.selected_index = model.selected_index.saturating_sub(10);
            }
            FocusPane::Sidebar => {
                model.sidebar_selected = model.sidebar_selected.saturating_sub(5);
            }
        },
        NavigationMessage::PageDown => match model.focus_pane {
            FocusPane::TaskList => {
                let max_index = model.visible_tasks.len().saturating_sub(1);
                model.selected_index = (model.selected_index + 10).min(max_index);
            }
            FocusPane::Sidebar => {
                let max_index = model.sidebar_item_count().saturating_sub(1);
                model.sidebar_selected = (model.sidebar_selected + 5).min(max_index);
            }
        },
        NavigationMessage::Select(index) => {
            if index < model.visible_tasks.len() {
                model.selected_index = index;
            }
        }
        _ => {}
    }
}
