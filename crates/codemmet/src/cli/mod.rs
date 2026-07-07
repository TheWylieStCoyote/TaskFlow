//! Command-line interface: argument parsing and subcommand dispatch.
//!
//! Exit codes follow the spec in `PLAN.md`: 0 success, 1 abbreviation parse
//! error, 2 unknown trigger or argument-binding failure, 3 configuration or
//! environment error. Diagnostics go to stderr only.

mod capture;
mod check;
mod expand;
mod list;
mod wrap;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use codemmet_core::config::loader::{ConfigError, default_user_snippets_dir};
use codemmet_core::context::IndentUnit;
use codemmet_core::lang::registry::{Registry, find_project_snippets_dir};
use codemmet_core::snippet::render::TabstopMode;

/// Exit code for abbreviation parse errors.
pub const EXIT_PARSE: u8 = 1;
/// Exit code for unknown triggers and argument-binding failures.
pub const EXIT_RENDER: u8 = 2;
/// Exit code for configuration and environment errors.
pub const EXIT_CONFIG: u8 = 3;

/// Emmet-style abbreviation expansion for programming languages.
#[derive(Debug, Parser)]
#[command(
    name = "codemmet",
    version,
    long_version = env!("CODEMMET_LONG_VERSION"),
    about
)]
struct Cli {
    /// Snippet override directory (default: `<config>/codemmet/snippets`).
    #[arg(long, global = true, value_name = "DIR")]
    config: Option<PathBuf>,

    /// Do not auto-load project-local packs (`.codemmet/snippets`).
    #[arg(long, global = true)]
    no_project: bool,

    #[command(subcommand)]
    command: Command,
}

/// Snippet override directories, in increasing priority (project wins).
///
/// Accessible to the subcommand submodules; the LSP resolves its own sources
/// separately (project packs are opt-in there).
#[derive(Debug, Clone, Copy)]
pub struct Sources<'a> {
    /// The user override directory (`--config` or the platform default).
    user: Option<&'a Path>,
    /// The project override directory (`.codemmet/snippets`), when enabled.
    project: Option<&'a Path>,
}

/// The available subcommands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Expand an abbreviation to code on stdout.
    Expand(expand::ExpandArgs),
    /// Wrap a selection (stdin) in an expanded abbreviation.
    Wrap(wrap::WrapArgs),
    /// Capture example code into a reusable, parameterized snippet.
    Capture(capture::CaptureArgs),
    /// List every trigger registered for a language.
    List(list::ListArgs),
    /// Validate the user snippet override directory.
    Check,
    /// Run the LSP server over stdio.
    Lsp(crate::lsp::LspArgs),
    /// Practice the abbreviation syntax interactively.
    #[cfg(feature = "train")]
    Train(crate::train::TrainArgs),
}

/// Parse arguments and run the selected subcommand.
pub fn run() -> ExitCode {
    let cli = Cli::parse();
    let user_dir = cli.config.or_else(default_user_snippets_dir);
    // Project packs auto-load from the nearest `.codemmet/snippets` at or above
    // the working directory, unless `--no-project`. The LSP opts in separately,
    // so it keeps loading from `user_dir` only.
    let project_dir = if cli.no_project {
        None
    } else {
        std::env::current_dir()
            .ok()
            .and_then(|cwd| find_project_snippets_dir(&cwd))
    };
    let sources = Sources {
        user: user_dir.as_deref(),
        project: project_dir.as_deref(),
    };
    match cli.command {
        Command::Expand(args) => expand::run(sources, &args),
        Command::Wrap(args) => wrap::run(sources, &args),
        Command::Capture(args) => capture::run(sources, &args),
        Command::List(args) => list::run(sources, &args),
        Command::Check => check::run(sources),
        Command::Lsp(args) => crate::lsp::run(user_dir.as_deref(), &args),
        #[cfg(feature = "train")]
        Command::Train(args) => crate::train::run(sources, &args),
    }
}

/// Load builtin packs merged with the user and project override directories.
pub fn load_registry(sources: Sources) -> Result<Registry, ConfigError> {
    Registry::with_dirs(sources.user, sources.project)
}

/// Report a configuration/environment failure and yield exit code 3.
fn config_failure(err: &dyn std::fmt::Display) -> ExitCode {
    eprintln!("error: {err}");
    ExitCode::from(EXIT_CONFIG)
}

/// Tabstop handling for rendered output (mirrors [`TabstopMode`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Tabstops {
    /// Emit LSP snippet syntax (`${1:placeholder}`, `$0`).
    Keep,
    /// Emit plain text; placeholders become their text, markers vanish.
    Strip,
}

impl From<Tabstops> for TabstopMode {
    fn from(value: Tabstops) -> Self {
        match value {
            Tabstops::Keep => Self::Keep,
            Tabstops::Strip => Self::Strip,
        }
    }
}

/// Parse a `--indent` value: `"\t"` or a non-empty run of spaces.
fn parse_indent(text: &str) -> Result<IndentUnit, String> {
    if text == "\t" {
        return Ok(IndentUnit::Tab);
    }
    if !text.is_empty() && text.bytes().all(|b| b == b' ') {
        return u8::try_from(text.len())
            .map(IndentUnit::Spaces)
            .map_err(|_| format!("--indent is too wide: {} spaces (max 255)", text.len()));
    }
    Err(format!(
        "invalid --indent {text:?}: expected a single tab or a run of spaces"
    ))
}

/// Resolve the indent unit for `lang`: parse `indent_flag` if given, otherwise
/// fall back to the language profile's default. Errors on an unknown language
/// or an unparseable `--indent` value (both exit-code-3 conditions).
fn resolve_indent_unit(
    registry: &Registry,
    lang: &str,
    indent_flag: Option<&str>,
) -> Result<IndentUnit, String> {
    let Some(language) = registry.language(lang) else {
        return Err(format!("unknown language `{lang}`"));
    };
    indent_flag.map_or_else(|| Ok(language.profile.default_indent.clone()), parse_indent)
}
