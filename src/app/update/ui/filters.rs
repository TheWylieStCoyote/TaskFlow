//! Saved filter handlers

use crate::app::{Model, UiMessage};
use crate::ui::{InputMode, InputTarget};

/// Handle saved filter UI messages
pub fn handle_ui_saved_filters(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ShowSavedFilters => {
            model.saved_filter_picker.visible = true;
            model.saved_filter_picker.selected = 0;
        }
        UiMessage::HideSavedFilters => {
            model.saved_filter_picker.visible = false;
        }
        UiMessage::SavedFilterUp => {
            if model.saved_filter_picker.selected > 0 {
                model.saved_filter_picker.selected -= 1;
            }
        }
        UiMessage::SavedFilterDown => {
            let count = model.saved_filters.len();
            if count > 0 && model.saved_filter_picker.selected < count - 1 {
                model.saved_filter_picker.selected += 1;
            }
        }
        UiMessage::ApplySavedFilter => {
            // Get the sorted filter list and find selected filter
            let mut filter_list: Vec<_> = model.saved_filters.values().collect();
            filter_list.sort_by(|a, b| a.name.cmp(&b.name));

            if let Some(saved_filter) = filter_list.get(model.saved_filter_picker.selected) {
                // Clone data we need before modifying model
                let filter = saved_filter.filter.clone();
                let sort = saved_filter.sort.clone();
                let filter_id = saved_filter.id.clone();
                let filter_name = saved_filter.name.clone();

                // Apply the filter and sort
                model.filter = filter;
                model.sort = sort;
                model.active_saved_filter = Some(filter_id);
                model.saved_filter_picker.visible = false;
                model.refresh_visible_tasks();
                model.status_message = Some(format!("Applied filter: {filter_name}"));
            }
        }
        UiMessage::SaveCurrentFilter => {
            // Start input mode to name the filter
            model.input_mode = InputMode::Editing;
            model.input_target = InputTarget::SavedFilterName;
            model.input_buffer.clear();
            model.cursor_position = 0;
            model.saved_filter_picker.visible = false;
        }
        UiMessage::DeleteSavedFilter => {
            // Get the sorted filter list
            let mut filter_list: Vec<_> = model.saved_filters.values().collect();
            filter_list.sort_by(|a, b| a.name.cmp(&b.name));

            if let Some(saved_filter) = filter_list.get(model.saved_filter_picker.selected) {
                let id_to_remove = saved_filter.id.clone();
                let name = saved_filter.name.clone();

                // Clear active filter if it's being deleted
                if model.active_saved_filter.as_ref() == Some(&id_to_remove) {
                    model.active_saved_filter = None;
                }

                model.saved_filters.remove(&id_to_remove);
                model.dirty = true;

                // Adjust selection
                if model.saved_filter_picker.selected > 0
                    && model.saved_filter_picker.selected >= model.saved_filters.len()
                {
                    model.saved_filter_picker.selected =
                        model.saved_filters.len().saturating_sub(1);
                }

                model.status_message = Some(format!("Deleted filter: {name}"));
            }
        }
        UiMessage::ClearSavedFilter => {
            model.active_saved_filter = None;
            model.filter = crate::domain::Filter::default();
            model.sort = crate::domain::SortSpec::default();
            model.refresh_visible_tasks();
            model.status_message = Some("Filter cleared".to_string());
        }
        _ => {}
    }
}
