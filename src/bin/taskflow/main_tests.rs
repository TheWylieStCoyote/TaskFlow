//! Tests for `resolve_storage`: how CLI arguments and settings combine into
//! a storage backend and data path.

use std::path::PathBuf;

use taskflow::config::Settings;
use taskflow::storage::BackendType;

use crate::cli::Cli;
use crate::resolve_storage;

fn cli_with(backend: BackendType, data: Option<PathBuf>) -> Cli {
    Cli {
        data,
        backend,
        demo: false,
        debug: false,
        log_level: "info".to_string(),
        command: None,
    }
}

fn settings_with(backend: &str, data_path: Option<PathBuf>) -> Settings {
    Settings {
        backend: backend.to_string(),
        data_path,
        ..Settings::default()
    }
}

#[test]
fn default_json_cli_backend_defers_to_settings() {
    let cli = cli_with(BackendType::Json, None);
    let settings = settings_with("sqlite", Some(PathBuf::from("/data/tasks.db")));

    let (backend, _) = resolve_storage(&cli, &settings);

    assert_eq!(backend, BackendType::Sqlite);
}

#[test]
fn unknown_settings_backend_falls_back_to_default() {
    let cli = cli_with(BackendType::Json, None);
    let settings = settings_with("not-a-backend", None);

    let (backend, _) = resolve_storage(&cli, &settings);

    assert_eq!(backend, BackendType::default());
}

#[test]
fn explicit_non_json_cli_backend_wins() {
    let cli = cli_with(BackendType::Yaml, None);
    let settings = settings_with("sqlite", None);

    let (backend, _) = resolve_storage(&cli, &settings);

    assert_eq!(backend, BackendType::Yaml);
}

#[test]
fn data_flag_overrides_settings_path() {
    let cli = cli_with(BackendType::Json, Some(PathBuf::from("/cli/tasks.json")));
    let settings = settings_with("json", Some(PathBuf::from("/settings/tasks.json")));

    let (_, data_path) = resolve_storage(&cli, &settings);

    assert_eq!(data_path, PathBuf::from("/cli/tasks.json"));
}

#[test]
fn settings_data_path_used_without_data_flag() {
    let cli = cli_with(BackendType::Json, None);
    let settings = settings_with("json", Some(PathBuf::from("/settings/tasks.json")));

    let (_, data_path) = resolve_storage(&cli, &settings);

    assert_eq!(data_path, settings.get_data_path());
    assert_eq!(data_path, PathBuf::from("/settings/tasks.json"));
}
