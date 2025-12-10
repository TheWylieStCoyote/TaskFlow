//! Saved filter tests.

use crate::app::{update::update, Message, Model, UiMessage};
use crate::domain::{Filter, SavedFilter, SortSpec};
use crate::ui::InputMode;

fn create_model_with_filters() -> Model {
    let mut model = Model::new();

    let filter1 = SavedFilter::new(
        "High Priority".to_string(),
        Filter::default(),
        SortSpec::default(),
    );
    let filter2 = SavedFilter::new(
        "This Week".to_string(),
        Filter::default(),
        SortSpec::default(),
    );

    model.saved_filters.insert(filter1.id.clone(), filter1);
    model.saved_filters.insert(filter2.id.clone(), filter2);

    model
}

#[test]
fn test_show_saved_filters() {
    let mut model = create_model_with_filters();
    model.saved_filter_picker.visible = false;
    model.saved_filter_picker.selected = 5;

    update(&mut model, Message::Ui(UiMessage::ShowSavedFilters));

    assert!(model.saved_filter_picker.visible);
    assert_eq!(model.saved_filter_picker.selected, 0);
}

#[test]
fn test_hide_saved_filters() {
    let mut model = create_model_with_filters();
    model.saved_filter_picker.visible = true;

    update(&mut model, Message::Ui(UiMessage::HideSavedFilters));

    assert!(!model.saved_filter_picker.visible);
}

#[test]
fn test_saved_filter_up() {
    let mut model = create_model_with_filters();
    model.saved_filter_picker.selected = 1;

    update(&mut model, Message::Ui(UiMessage::SavedFilterUp));

    assert_eq!(model.saved_filter_picker.selected, 0);
}

#[test]
fn test_saved_filter_up_at_zero() {
    let mut model = create_model_with_filters();
    model.saved_filter_picker.selected = 0;

    update(&mut model, Message::Ui(UiMessage::SavedFilterUp));

    assert_eq!(model.saved_filter_picker.selected, 0);
}

#[test]
fn test_saved_filter_down() {
    let mut model = create_model_with_filters();
    model.saved_filter_picker.selected = 0;

    update(&mut model, Message::Ui(UiMessage::SavedFilterDown));

    assert_eq!(model.saved_filter_picker.selected, 1);
}

#[test]
fn test_saved_filter_down_at_end() {
    let mut model = create_model_with_filters();
    model.saved_filter_picker.selected = model.saved_filters.len() - 1;

    update(&mut model, Message::Ui(UiMessage::SavedFilterDown));

    // Should stay at end
    assert_eq!(
        model.saved_filter_picker.selected,
        model.saved_filters.len() - 1
    );
}

#[test]
fn test_apply_saved_filter() {
    let mut model = create_model_with_filters();
    model.saved_filter_picker.visible = true;
    model.saved_filter_picker.selected = 0;

    update(&mut model, Message::Ui(UiMessage::ApplySavedFilter));

    assert!(!model.saved_filter_picker.visible);
    assert!(model.active_saved_filter.is_some());
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_save_current_filter() {
    let mut model = create_model_with_filters();
    model.saved_filter_picker.visible = true;

    update(&mut model, Message::Ui(UiMessage::SaveCurrentFilter));

    assert!(!model.saved_filter_picker.visible);
    assert_eq!(model.input.mode, InputMode::Editing);
}

#[test]
fn test_delete_saved_filter() {
    let mut model = create_model_with_filters();
    let initial_count = model.saved_filters.len();
    model.saved_filter_picker.selected = 0;

    update(&mut model, Message::Ui(UiMessage::DeleteSavedFilter));

    assert_eq!(model.saved_filters.len(), initial_count - 1);
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_delete_active_saved_filter() {
    let mut model = create_model_with_filters();

    // Apply a filter first
    model.saved_filter_picker.selected = 0;
    update(&mut model, Message::Ui(UiMessage::ApplySavedFilter));
    assert!(model.active_saved_filter.is_some());

    // Reset selection and delete
    model.saved_filter_picker.selected = 0;
    update(&mut model, Message::Ui(UiMessage::DeleteSavedFilter));

    // Active filter should be cleared
    assert!(model.active_saved_filter.is_none());
}

#[test]
fn test_clear_saved_filter() {
    let mut model = create_model_with_filters();
    model.saved_filter_picker.selected = 0;
    update(&mut model, Message::Ui(UiMessage::ApplySavedFilter));
    assert!(model.active_saved_filter.is_some());

    update(&mut model, Message::Ui(UiMessage::ClearSavedFilter));

    assert!(model.active_saved_filter.is_none());
    assert!(model.alerts.status_message.is_some());
}
