//! Tests for the update arms that write config files (`keybindings.toml`,
//! `config.toml`) into the model's config directory.
//!
//! Every test points `Model::config_dir` at a temporary directory so nothing
//! touches the user's real configuration.

use std::path::Path;

use tempfile::tempdir;

use crate::app::{update::update, Message, Model, UiMessage};
use crate::config::{Action, Keybindings, Settings};

/// Creates a regular file to stand in for a config "directory", so that
/// `create_dir_all` on it fails.
fn file_as_config_dir(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("not-a-directory");
    std::fs::write(&path, "occupied").expect("write blocker file");
    path
}

#[test]
fn save_keybindings_writes_to_model_config_dir() {
    let tmp = tempdir().expect("tempdir");
    let mut model = Model::new().with_config_dir(tmp.path());
    model
        .keybindings
        .set_binding_checked("ctrl+shift+z".to_string(), Action::MoveLast);

    update(&mut model, Message::Ui(UiMessage::SaveKeybindings));

    let path = Keybindings::path_in(tmp.path());
    assert_eq!(path, tmp.path().join("keybindings.toml"));
    assert!(path.exists(), "keybindings.toml should be written");

    let reloaded = Keybindings::load_from_path(path);
    assert_eq!(reloaded.get_action("ctrl+shift+z"), Some(&Action::MoveLast));
    assert_eq!(
        model.alerts.status_message.as_deref(),
        Some("Keybindings saved")
    );
}

#[test]
fn save_keybindings_reports_write_failure() {
    let tmp = tempdir().expect("tempdir");
    let blocker = file_as_config_dir(tmp.path());
    let mut model = Model::new().with_config_dir(&blocker);

    update(&mut model, Message::Ui(UiMessage::SaveKeybindings));

    let status = model.alerts.status_message.expect("status message set");
    assert!(
        status.starts_with("Failed to save keybindings"),
        "unexpected status: {status}"
    );
    assert!(!blocker.join("keybindings.toml").exists());
}

#[test]
fn confirm_generate_config_writes_default_config() {
    let tmp = tempdir().expect("tempdir");
    let mut model = Model::new().with_config_dir(tmp.path());
    model.show_generate_config_prompt = true;

    update(&mut model, Message::Ui(UiMessage::ConfirmGenerateConfig));

    let path = Settings::path_in(tmp.path());
    assert_eq!(path, tmp.path().join("config.toml"));
    assert!(path.exists(), "config.toml should be written");

    let written = Settings::try_load_from_path(path.clone())
        .expect("config.toml parses")
        .expect("config.toml exists");
    let defaults = Settings::default();
    assert_eq!(written.backend, defaults.backend);
    assert_eq!(written.data_path, defaults.data_path);
    assert_eq!(written.default_priority, defaults.default_priority);

    assert!(!model.show_generate_config_prompt);
    assert!(model.alerts.error_message.is_none());
    let status = model.alerts.status_message.expect("status message set");
    assert!(
        status.contains(&path.display().to_string()),
        "status should name the written path: {status}"
    );
}

#[test]
fn confirm_generate_config_reports_failure() {
    let tmp = tempdir().expect("tempdir");
    let blocker = file_as_config_dir(tmp.path());
    let mut model = Model::new().with_config_dir(&blocker);
    model.show_generate_config_prompt = true;

    update(&mut model, Message::Ui(UiMessage::ConfirmGenerateConfig));

    let error = model.alerts.error_message.expect("error message set");
    assert!(
        error.starts_with("Failed to create config"),
        "unexpected error: {error}"
    );
    assert!(model.alerts.status_message.is_none());
    assert!(!model.show_generate_config_prompt);
    assert!(!blocker.join("config.toml").exists());
}

#[test]
fn cancel_generate_config_closes_prompt_without_writing() {
    let tmp = tempdir().expect("tempdir");
    let mut model = Model::new().with_config_dir(tmp.path());
    model.show_generate_config_prompt = true;

    update(&mut model, Message::Ui(UiMessage::CancelGenerateConfig));

    assert!(!model.show_generate_config_prompt);
    assert!(!Settings::path_in(tmp.path()).exists());
    assert!(!Keybindings::path_in(tmp.path()).exists());
    assert!(model.alerts.status_message.is_none());
    assert!(model.alerts.error_message.is_none());
}
