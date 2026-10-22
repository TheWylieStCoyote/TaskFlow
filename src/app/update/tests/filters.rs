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

// ============================================================================
// Persistence: saving / deleting a filter in the TUI reaches the backend
// ============================================================================

mod persistence {
    use std::path::Path;

    use tempfile::TempDir;

    use crate::app::{update::update, Message, Model, UiMessage};
    use crate::domain::{Priority, SavedFilter};
    use crate::storage::BackendType;
    use crate::ui::{InputMode, InputTarget};

    fn stored_model(dir: &TempDir) -> Model {
        Model::new()
            .with_storage(BackendType::Json, dir.path().join("tasks.json"))
            .expect("storage")
    }

    fn reload(dir: &TempDir) -> Model {
        Model::new()
            .with_storage(BackendType::Json, dir.path().join("tasks.json"))
            .expect("reload")
    }

    fn save_filter_named(model: &mut Model, name: &str) {
        update(model, Message::Ui(UiMessage::SaveCurrentFilter));
        assert_eq!(model.input.mode, InputMode::Editing);
        assert_eq!(model.input.target, InputTarget::SavedFilterName);
        model.input.buffer = name.to_string();
        model.input.cursor = name.len();
        update(model, Message::Ui(UiMessage::SubmitInput));
    }

    #[test]
    fn saving_a_filter_in_the_tui_survives_reload() {
        let dir = TempDir::new().unwrap();
        let mut model = stored_model(&dir);
        model.filtering.filter.priority = Some(vec![Priority::High]);
        model.filtering.filter.include_completed = true;

        save_filter_named(&mut model, "Hot items");

        let saved: Vec<&SavedFilter> = model.saved_filters.values().collect();
        assert_eq!(saved.len(), 1);
        let saved_id = saved[0].id.clone();
        assert_eq!(model.active_saved_filter, Some(saved_id.clone()));
        assert!(model.storage.dirty);
        model.save().expect("save");

        let reloaded = reload(&dir);
        assert_eq!(reloaded.saved_filters.len(), 1);
        let restored = &reloaded.saved_filters[&saved_id];
        assert_eq!(restored.name, "Hot items");
        assert_eq!(restored.filter.priority, Some(vec![Priority::High]));
        assert!(restored.filter.include_completed);
    }

    #[test]
    fn deleting_a_filter_in_the_tui_survives_reload() {
        let dir = TempDir::new().unwrap();
        let mut model = stored_model(&dir);
        save_filter_named(&mut model, "Alpha");
        save_filter_named(&mut model, "Beta");
        model.save().unwrap();
        assert_eq!(reload(&dir).saved_filters.len(), 2);

        // The picker lists filters sorted by name, so index 0 is "Alpha".
        update(&mut model, Message::Ui(UiMessage::ShowSavedFilters));
        model.saved_filter_picker.selected = 0;
        update(&mut model, Message::Ui(UiMessage::DeleteSavedFilter));
        assert_eq!(model.saved_filters.len(), 1);
        model.save().unwrap();

        let reloaded = reload(&dir);
        assert_eq!(reloaded.saved_filters.len(), 1);
        assert_eq!(reloaded.saved_filters.values().next().unwrap().name, "Beta");
    }

    #[test]
    fn empty_filter_name_persists_nothing() {
        let dir = TempDir::new().unwrap();
        let mut model = stored_model(&dir);
        save_filter_named(&mut model, "");
        model.save().unwrap();
        assert!(reload(&dir).saved_filters.is_empty());
    }

    #[test]
    fn sample_data_mode_never_writes_filters() {
        let dir = TempDir::new().unwrap();
        let mut model = stored_model(&dir).with_sample_data();
        save_filter_named(&mut model, "Demo only");
        model.save().unwrap();
        assert!(
            !Path::new(&dir.path().join("tasks.json")).exists(),
            "sample data mode must not touch the disk"
        );
    }
}
