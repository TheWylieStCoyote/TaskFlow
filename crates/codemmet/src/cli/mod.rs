//! Command-line interface: argument parsing and subcommand dispatch.
//!
//! Exit codes follow the spec in `PLAN.md`: 0 success, 1 abbreviation parse
//! error, 2 unknown trigger or argument-binding failure, 3 configuration or
//! environment error. Diagnostics go to stderr only.

mod check;
mod expand;
mod list;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use codemmet_core::config::loader::{ConfigError, default_user_snippets_dir};
use codemmet_core::lang::registry::Registry;

/// Exit code for abbreviation parse errors.
pub const EXIT_PARSE: u8 = 1;
/// Exit code for unknown triggers and argument-binding failures.
pub const EXIT_RENDER: u8 = 2;
/// Exit code for configuration and environment errors.
pub const EXIT_CONFIG: u8 = 3;

/// Emmet-style abbreviation expansion for programming languages.
#[derive(Debug, Parser)]
#[command(name = "codemmet", version, about)]
struct Cli {
    /// Snippet override directory (default: `<config>/codemmet/snippets`).
    #[arg(long, global = true, value_name = "DIR")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

/// The available subcommands (M1 scope; `wrap` and `lsp` land in M2).
#[derive(Debug, Subcommand)]
enum Command {
    /// Expand an abbreviation to code on stdout.
    Expand(expand::ExpandArgs),
    /// List every trigger registered for a language.
    List(list::ListArgs),
    /// Validate the user snippet override directory.
    Check,
}

/// Parse arguments and run the selected subcommand.
pub fn run() -> ExitCode {
    let cli = Cli::parse();
    let user_dir = cli.config.or_else(default_user_snippets_dir);
    match cli.command {
        Command::Expand(args) => expand::run(user_dir.as_deref(), &args),
        Command::List(args) => list::run(user_dir.as_deref(), &args),
        Command::Check => check::run(user_dir.as_deref()),
    }
}

/// Load builtin packs merged with the user override directory.
fn load_registry(user_dir: Option<&Path>) -> Result<Registry, ConfigError> {
    Registry::with_user_dir(user_dir)
}

/// Report a configuration/environment failure and yield exit code 3.
fn config_failure(err: &dyn std::fmt::Display) -> ExitCode {
    eprintln!("error: {err}");
    ExitCode::from(EXIT_CONFIG)
}
