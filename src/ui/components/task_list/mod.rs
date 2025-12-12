//! Task list widget and related components.
//!
//! This module provides the main task list display used throughout the application.

mod item;
#[cfg(test)]
mod tests;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{
        List, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
    },
};

use crate::app::{Model, ViewId};
use crate::config::Theme;
use crate::domain::{Task, TaskId};
use crate::ui::primitives::panel_block;

pub use item::{project_header_to_list_item, task_to_list_item, TaskItemContext};

/// Represents an item in the task list (either a task or a project header)
pub enum ListEntry<'a> {
    Task {
        task: &'a Task,
        index: usize, // Index in visible_tasks for selection tracking
        time_spent: u32,
        nesting_depth: usize, // 0 for root, 1+ for subtasks
        is_multi_selected: bool,
        has_dependencies: bool,
        is_recurring: bool,
        has_chain: bool, // Task is linked to another task (has next_task_id)
        subtask_progress: (usize, usize), // (completed, total) - only shown if total > 0
    },
    ProjectHeader {
        name: String,
        task_count: usize,
    },
}

/// Task list widget
pub struct TaskList<'a> {
    entries: Vec<ListEntry<'a>>,
    selected: usize,
    /// Maps display row to `visible_tasks` index (None for headers)
    row_to_task_index: Vec<Option<usize>>,
    active_tracking: Option<&'a TaskId>,
    theme: &'a Theme,
    is_grouped: bool,
}

impl<'a> TaskList<'a> {
    #[must_use]
    pub fn new(model: &'a Model, theme: &'a Theme) -> Self {
        let active_tracking = model.active_time_entry().map(|e| &e.task_id);
        let is_grouped = model.current_view == ViewId::Projects;

        if is_grouped {
            Self::new_grouped(model, theme, active_tracking)
        } else {
            Self::new_flat(model, theme, active_tracking)
        }
    }

    fn new_flat(model: &'a Model, theme: &'a Theme, active_tracking: Option<&'a TaskId>) -> Self {
        let mut entries = Vec::new();
        let mut row_to_task_index = Vec::new();

        for (idx, task_id) in model.visible_tasks.iter().enumerate() {
            if let Some(task) = model.tasks.get(task_id) {
                // Use cached values for performance
                let time_spent = model.task_cache.get_time_sum(*task_id);
                let nesting_depth = model.task_cache.get_depth(*task_id);
                let is_multi_selected = model.multi_select.selected.contains(task_id);
                let has_dependencies = !task.dependencies.is_empty();
                let is_recurring = task.recurrence.is_some();
                let has_chain = task.next_task_id.is_some();
                let subtask_progress = model.task_cache.get_subtask_progress(*task_id);
                entries.push(ListEntry::Task {
                    task,
                    index: idx,
                    time_spent,
                    nesting_depth,
                    is_multi_selected,
                    has_dependencies,
                    is_recurring,
                    has_chain,
                    subtask_progress,
                });
                row_to_task_index.push(Some(idx));
            }
        }

        Self {
            entries,
            selected: model.selected_index,
            row_to_task_index,
            active_tracking,
            theme,
            is_grouped: false,
        }
    }

    fn new_grouped(
        model: &'a Model,
        theme: &'a Theme,
        active_tracking: Option<&'a TaskId>,
    ) -> Self {
        let grouped = model.get_tasks_grouped_by_project();
        let mut entries = Vec::new();
        let mut row_to_task_index = Vec::new();

        for (_project_id, project_name, task_ids) in grouped {
            // Add project header
            entries.push(ListEntry::ProjectHeader {
                name: project_name,
                task_count: task_ids.len(),
            });
            row_to_task_index.push(None); // Headers are not selectable

            // Add tasks under this project
            for task_id in task_ids {
                if let Some(task) = model.tasks.get(&task_id) {
                    // Find the index in visible_tasks
                    let idx = model
                        .visible_tasks
                        .iter()
                        .position(|id| id == &task_id)
                        .unwrap_or(0);
                    // Use cached values for performance
                    let time_spent = model.task_cache.get_time_sum(task_id);
                    let nesting_depth = model.task_cache.get_depth(task_id);
                    let is_multi_selected = model.multi_select.selected.contains(&task_id);
                    let has_dependencies = !task.dependencies.is_empty();
                    let is_recurring = task.recurrence.is_some();
                    let has_chain = task.next_task_id.is_some();
                    let subtask_progress = model.task_cache.get_subtask_progress(task_id);
                    entries.push(ListEntry::Task {
                        task,
                        index: idx,
                        time_spent,
                        nesting_depth,
                        is_multi_selected,
                        has_dependencies,
                        is_recurring,
                        has_chain,
                        subtask_progress,
                    });
                    row_to_task_index.push(Some(idx));
                }
            }
        }

        Self {
            entries,
            selected: model.selected_index,
            row_to_task_index,
            active_tracking,
            theme,
            is_grouped: true,
        }
    }

    /// Find the display row for the currently selected task index
    fn selected_row(&self) -> Option<usize> {
        self.row_to_task_index
            .iter()
            .position(|idx| *idx == Some(self.selected))
    }
}

impl StatefulWidget for TaskList<'_> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let theme = self.theme;
        let selected_row = self.selected_row();

        let items: Vec<ratatui::widgets::ListItem<'_>> = self
            .entries
            .iter()
            .map(|entry| match entry {
                ListEntry::Task {
                    task,
                    index,
                    time_spent,
                    nesting_depth,
                    is_multi_selected,
                    has_dependencies,
                    is_recurring,
                    has_chain,
                    subtask_progress,
                } => {
                    let ctx = TaskItemContext {
                        task,
                        is_selected: *index == self.selected,
                        is_tracking: self.active_tracking == Some(&task.id),
                        time_spent: *time_spent,
                        nesting_depth: *nesting_depth,
                        is_multi_selected: *is_multi_selected,
                        has_dependencies: *has_dependencies,
                        is_recurring: *is_recurring,
                        has_chain: *has_chain,
                        subtask_progress: *subtask_progress,
                        theme,
                        git_branch: task.git_ref.as_ref().map(|r| r.branch.as_str()),
                    };
                    task_to_list_item(&ctx)
                }
                ListEntry::ProjectHeader { name, task_count } => {
                    project_header_to_list_item(name, *task_count, theme)
                }
            })
            .collect();

        let title = if self.is_grouped {
            "Tasks (by Project)"
        } else {
            "Tasks"
        };

        let list = List::new(items)
            .block(panel_block(title, theme))
            .highlight_style(
                Style::default()
                    .bg(theme.colors.accent_secondary.to_color())
                    .add_modifier(Modifier::BOLD),
            );

        // Update selection on the persisted state (keeps scroll offset intact)
        state.select(selected_row);

        StatefulWidget::render(list, area, buf, state);

        // Render scrollbar if content exceeds viewport
        let total_items = self.entries.len();
        let viewport_height = area.height.saturating_sub(2) as usize; // Account for borders

        if total_items > viewport_height {
            // Use selected_row for position to show where cursor is in the list
            let position = selected_row.unwrap_or(state.offset());

            let mut scrollbar_state =
                ScrollbarState::new(total_items.saturating_sub(1)).position(position);

            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .track_symbol(Some("│"))
                .thumb_symbol("█")
                .track_style(Style::default().fg(theme.colors.muted.to_color()))
                .thumb_style(Style::default().fg(theme.colors.accent.to_color()));

            // Render scrollbar in the inner area (inside the border)
            let scrollbar_area = Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: area.height,
            };
            StatefulWidget::render(scrollbar, scrollbar_area, buf, &mut scrollbar_state);
        }
    }
}

/// Widget implementation for backwards compatibility and testing.
/// Creates a default ListState (no scroll offset preserved).
impl Widget for TaskList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = ListState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}
