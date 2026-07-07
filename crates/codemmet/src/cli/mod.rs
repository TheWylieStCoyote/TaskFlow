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
use codemmet_core::config::paths::{PathMapping, default_paths_config};
use codemmet_core::context::IndentUnit;
use codemmet_core::lang::registry::{Registry, find_project_snippets_dirs};
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

    /// Personal path-prefix → snippet-dir mapping file
    /// (default: `<config>/codemmet/paths.toml`).
    #[arg(long, global = true, value_name = "FILE")]
    paths: Option<PathBuf>,

    /// Do not auto-load project-local packs (`.codemmet/snippets`).
    #[arg(long, global = true)]
    no_project: bool,

    #[command(subcommand)]
    command: Command,
}

/// The resolved snippet override tiers for one invocation, in increasing
/// priority: `user < user-path-mapped < project` (project layered root-most →
/// nearest). The LSP resolves its own tiers per document.
#[derive(Debug, Clone, Default)]
pub struct Sources {
    /// The user override directory (`--config` or the platform default).
    user: Option<PathBuf>,
    /// Personal path-mapped directories matching the working directory,
    /// shallow prefix first.
    mapped: Vec<PathBuf>,
    /// Layered project directories (`.codemmet/snippets` up the tree),
    /// root-most first. Empty under `--no-project`.
    project: Vec<PathBuf>,
}

impl Sources {
    /// The override directories to load, lowest priority first.
    fn override_dirs(&self) -> Vec<&Path> {
        let mut dirs: Vec<&Path> = Vec::new();
        if let Some(user) = &self.user {
            dirs.push(user);
        }
        dirs.extend(self.mapped.iter().map(PathBuf::as_path));
        dirs.extend(self.project.iter().map(PathBuf::as_path));
        dirs
    }

    /// The user override directory, if any (for `capture --user`).
    fn user_dir(&self) -> Option<&Path> {
        self.user.as_deref()
    }

    /// Every override directory with a tier label, lowest priority first
    /// (for `check` reporting).
    fn labeled_dirs(&self) -> Vec<(&'static str, &Path)> {
        let mut out: Vec<(&'static str, &Path)> = Vec::new();
        if let Some(user) = &self.user {
            out.push(("user", user));
        }
        out.extend(self.mapped.iter().map(|d| ("path-mapped", d.as_path())));
        out.extend(self.project.iter().map(|d| ("project", d.as_path())));
        out
    }
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
    let paths_file = cli.paths.or_else(default_paths_config);

    // Resolve the snippet tiers relative to the working directory: the user
    // global dir, personal path-mapped dirs, then layered project packs
    // (`.codemmet/snippets` up the tree). The LSP resolves per document.
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mapping = match &paths_file {
        Some(path) => match PathMapping::load(path) {
            Ok(mapping) => mapping,
            Err(err) => return config_failure(&err),
        },
        None => PathMapping::empty(),
    };
    let project = if cli.no_project {
        Vec::new()
    } else {
        find_project_snippets_dirs(&cwd)
    };
    let sources = Sources {
        user: user_dir.clone(),
        mapped: mapping.dirs_for(&cwd),
        project,
    };

    match cli.command {
        Command::Expand(args) => expand::run(&sources, &args),
        Command::Wrap(args) => wrap::run(&sources, &args),
        Command::Capture(args) => capture::run(&sources, &args),
        Command::List(args) => list::run(&sources, &args),
        Command::Check => check::run(&sources),
        Command::Lsp(args) => crate::lsp::run(user_dir.as_deref(), paths_file.as_deref(), &args),
        #[cfg(feature = "train")]
        Command::Train(args) => crate::train::run(&sources, &args),
    }
}

/// Load builtin packs merged with the resolved override tiers.
pub fn load_registry(sources: &Sources) -> Result<Registry, ConfigError> {
    Registry::with_dirs_layered(&sources.override_dirs())
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
