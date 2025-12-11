//! TaskFlow binary entry point.

mod cli;
mod commands;
mod input;
mod tui;

use std::fs::File;
use std::io;

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use tracing::{debug, info};

use taskflow::app::Model;
use taskflow::config::Settings;
use taskflow::storage::BackendType;

use cli::{parse_date, parse_priorities, parse_statuses, Cli, Commands, ListFilters};
use commands::{extract_git_todos, list_tasks, mark_task_done, quick_add_task};
use tui::run_tui;

/// Initialize the tracing/logging subsystem.
///
/// When `--debug` is passed, logs are written to `taskflow.log` in the current directory.
/// The log level can be controlled via `--log-level` (trace, debug, info, warn, error).
fn init_logging(debug: bool, log_level: &str) {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    if debug {
        // Parse log level, defaulting to debug when --debug is set
        let level = if log_level == "info" {
            "debug" // Default to debug when --debug flag is used
        } else {
            log_level
        };

        let filter = EnvFilter::try_new(format!("taskflow={level}"))
            .unwrap_or_else(|_| EnvFilter::new("taskflow=debug"));

        // Try to create log file, fall back to stderr if it fails
        if let Ok(file) = File::create("taskflow.log") {
            tracing_subscriber::registry()
                .with(filter)
                .with(
                    fmt::layer()
                        .with_writer(file)
                        .with_ansi(false)
                        .with_target(true)
                        .with_thread_ids(false)
                        .with_file(true)
                        .with_line_number(true),
                )
                .init();
        } else {
            eprintln!("Warning: Could not create taskflow.log, logging disabled");
        }
    }
    // When debug is false, no logging subscriber is installed (silent operation)
}

/// Load model with storage for CLI commands.
/// Returns an error with a descriptive message if storage cannot be loaded.
pub fn load_model_for_cli(cli: &Cli) -> anyhow::Result<Model> {
    let settings = Settings::load();
    let backend_type = if cli.backend == BackendType::Json {
        BackendType::parse(&settings.backend).unwrap_or_default()
    } else {
        cli.backend
    };
    let data_path = cli.data.clone().unwrap_or_else(|| settings.get_data_path());

    Model::new()
        .with_storage(backend_type, data_path.clone())
        .map_err(|e| anyhow::anyhow!("Could not load data from {}: {e}", data_path.display()))
}

/// Generate shell completion scripts and print to stdout
fn print_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut io::stdout());
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging if --debug flag is set
    init_logging(cli.debug, &cli.log_level);

    info!("TaskFlow starting");
    debug!(backend = ?cli.backend, data = ?cli.data, demo = cli.demo, "CLI arguments parsed");

    // Handle subcommands
    match &cli.command {
        Some(Commands::Completion { shell }) => {
            print_completions(*shell);
            return Ok(());
        }
        Some(Commands::Add { task }) => {
            return quick_add_task(&cli, task);
        }
        Some(Commands::List {
            view,
            completed,
            limit,
            project,
            tags,
            tags_any,
            priority,
            status,
            search,
            sort,
            reverse,
            due_before,
            due_after,
            estimate_min,
            estimate_max,
        }) => {
            let filters = ListFilters {
                project: project.clone(),
                tags: tags.clone(),
                tags_any: *tags_any,
                priority: priority.as_ref().map(|p| parse_priorities(p)),
                status: status.as_ref().map(|s| parse_statuses(s)),
                search: search.clone(),
                sort: sort.clone(),
                reverse: *reverse,
                due_before: due_before.as_ref().and_then(|s| parse_date(s)),
                due_after: due_after.as_ref().and_then(|s| parse_date(s)),
                estimate_min: *estimate_min,
                estimate_max: *estimate_max,
            };
            return list_tasks(&cli, view, *completed, *limit, &filters);
        }
        Some(Commands::Done {
            query,
            project,
            tags,
        }) => {
            return mark_task_done(&cli, query, project.as_deref(), tags.as_deref());
        }
        Some(Commands::GitTodos {
            repo,
            patterns,
            project,
            tags,
            priority,
            dry_run,
        }) => {
            return extract_git_todos(
                &cli,
                repo,
                patterns,
                project.as_deref(),
                tags.as_deref(),
                priority,
                *dry_run,
            );
        }
        None => {}
    }

    // No subcommand - run the TUI
    run_tui(cli)
}

#[cfg(test)]
mod cli_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn verify_cli() {
        // Validates the CLI configuration is correct
        Cli::command().debug_assert();
    }

    #[test]
    fn test_parse_no_subcommand() {
        let cli = Cli::try_parse_from(["taskflow"]).unwrap();
        assert!(cli.command.is_none());
        assert_eq!(cli.backend, BackendType::Json);
        assert!(!cli.demo);
    }

    #[test]
    fn test_parse_completion_bash() {
        let cli = Cli::try_parse_from(["taskflow", "completion", "bash"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Completion { shell: Shell::Bash })
        ));
    }

    #[test]
    fn test_parse_completion_zsh() {
        let cli = Cli::try_parse_from(["taskflow", "completion", "zsh"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Completion { shell: Shell::Zsh })
        ));
    }

    #[test]
    fn test_parse_completion_fish() {
        let cli = Cli::try_parse_from(["taskflow", "completion", "fish"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Completion { shell: Shell::Fish })
        ));
    }

    #[test]
    fn test_parse_with_backend() {
        let cli = Cli::try_parse_from(["taskflow", "--backend", "yaml"]).unwrap();
        assert_eq!(cli.backend, BackendType::Yaml);
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_parse_with_data_path() {
        let cli = Cli::try_parse_from(["taskflow", "--data", "/tmp/tasks.json"]).unwrap();
        assert_eq!(cli.data, Some(PathBuf::from("/tmp/tasks.json")));
    }

    #[test]
    fn test_parse_with_demo() {
        let cli = Cli::try_parse_from(["taskflow", "--demo"]).unwrap();
        assert!(cli.demo);
    }

    #[test]
    fn test_parse_all_backends() {
        for backend in ["json", "yaml", "sqlite", "markdown"] {
            let cli = Cli::try_parse_from(["taskflow", "--backend", backend]).unwrap();
            let expected = BackendType::parse(backend).unwrap();
            assert_eq!(cli.backend, expected);
        }
    }

    #[test]
    fn test_completion_output_bash() {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        generate(Shell::Bash, &mut cmd, "taskflow", &mut buf);
        let output = String::from_utf8(buf).unwrap();

        assert!(output.contains("_taskflow"));
        assert!(output.contains("--backend"));
        assert!(output.contains("--data"));
        assert!(output.contains("completion"));
    }

    #[test]
    fn test_completion_output_zsh() {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        generate(Shell::Zsh, &mut cmd, "taskflow", &mut buf);
        let output = String::from_utf8(buf).unwrap();

        assert!(output.contains("#compdef taskflow"));
        assert!(output.contains("--backend"));
    }

    #[test]
    fn test_completion_output_fish() {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        generate(Shell::Fish, &mut cmd, "taskflow", &mut buf);
        let output = String::from_utf8(buf).unwrap();

        assert!(output.contains("complete -c taskflow"));
    }

    #[test]
    fn test_invalid_backend_rejected() {
        let result = Cli::try_parse_from(["taskflow", "--backend", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_shell_rejected() {
        let result = Cli::try_parse_from(["taskflow", "completion", "invalid"]);
        assert!(result.is_err());
    }
}
