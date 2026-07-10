//! Command-line interface: argument parsing and subcommand dispatch.
//!
//! Exit codes follow the spec in `PLAN.md`: 0 success, 1 abbreviation parse
//! error, 2 unknown trigger or argument-binding failure, 3 configuration or
//! environment error. Diagnostics go to stderr only.

mod capture;
mod check;
mod expand;
mod list;
mod render;
mod scaffold;
mod wrap;

use std::ffi::OsString;
use std::io::IsTerminal as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{CommandFactory as _, Parser, Subcommand, ValueEnum};
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
    // Version flags are hand-rolled: `-v`/`--version` prints the one-liner,
    // `-V`/`--version-full` the detailed build report (see `print_version`).
    disable_version_flag = true,
    about,
    // A token that names a subcommand is that subcommand, even after global
    // flags (`codemmet --config X lsp`); only non-subcommand tokens fall
    // through to the `.codemmet` FILE positional. The stricter
    // `args_conflicts_with_subcommands` broke every flag-before-subcommand
    // invocation by routing the subcommand name into the positional.
    subcommand_precedence_over_arg = true,
    arg_required_else_help = true
)]
struct Cli {
    /// Print the version (with the build's commit hash) and exit.
    #[arg(short = 'v', long = "version", action = clap::ArgAction::SetTrue)]
    version: bool,

    /// Print detailed version and build information (commit, branch, dirty
    /// state, profile, target, rustc, features) and exit.
    #[arg(short = 'V', long = "version-full", action = clap::ArgAction::SetTrue)]
    version_full: bool,

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

    /// A `.codemmet` source file to render (shorthand for
    /// `codemmet render <FILE>`); the inner extension picks the language.
    #[arg(value_name = "FILE.codemmet")]
    file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
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
    /// Render a `.codemmet` source file (stdin or FILE) to code on stdout.
    Render(render::RenderArgs),
    /// Wrap a selection (stdin) in an expanded abbreviation.
    Wrap(wrap::WrapArgs),
    /// Capture example code into a reusable, parameterized snippet.
    Capture(capture::CaptureArgs),
    /// Expand a multi-file snippet into a file manifest (optionally --write).
    Scaffold(scaffold::ScaffoldArgs),
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

/// Parse arguments, treating bare `codemmet` with piped stdin as the
/// file-mode filter (`codemmet render`). Interactive bare invocations keep
/// clap's help-and-exit-2 behavior.
fn parse_cli() -> Cli {
    let mut raw: Vec<OsString> = std::env::args_os().collect();
    if raw.len() == 1 && !std::io::stdin().is_terminal() {
        raw.push(OsString::from("render"));
    }
    Cli::parse_from(raw)
}

/// Parse arguments and run the selected subcommand.
pub fn run() -> ExitCode {
    let cli = parse_cli();
    if cli.version_full {
        print_version(true);
        return ExitCode::SUCCESS;
    }
    if cli.version {
        print_version(false);
        return ExitCode::SUCCESS;
    }
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
        Some(Command::Expand(args)) => expand::run(&sources, &args),
        Some(Command::Render(args)) => render::run(&sources, &args),
        Some(Command::Wrap(args)) => wrap::run(&sources, &args),
        Some(Command::Capture(args)) => capture::run(&sources, &args),
        Some(Command::Scaffold(args)) => scaffold::run(&sources, &args),
        Some(Command::List(args)) => list::run(&sources, &args),
        Some(Command::Check) => check::run(&sources),
        Some(Command::Lsp(args)) => {
            crate::lsp::run(user_dir.as_deref(), paths_file.as_deref(), &args)
        }
        #[cfg(feature = "train")]
        Some(Command::Train(args)) => crate::train::run(&sources, &args),
        None => run_positional(&sources, cli.file),
    }
}

/// Handle the subcommand-less invocation: a `.codemmet` positional renders;
/// anything else gets help (matching clap's bare-invocation behavior).
fn run_positional(sources: &Sources, file: Option<PathBuf>) -> ExitCode {
    /// clap's exit code for usage errors, which these are.
    const EXIT_USAGE: u8 = 2;
    let Some(file) = file else {
        // Flags but no subcommand and no file (e.g. `codemmet --no-project`).
        eprintln!("{}", Cli::command().render_help());
        return ExitCode::from(EXIT_USAGE);
    };
    if file.extension().is_some_and(|ext| ext == "codemmet") {
        return render::run(sources, &render::RenderArgs::for_file(file));
    }
    eprintln!(
        "error: `{}` is neither a subcommand nor a `.codemmet` file \
         (try `codemmet render <FILE>`, or --help)",
        file.display()
    );
    ExitCode::from(EXIT_USAGE)
}

/// Print the version: the one-liner (`-v`), or the detailed build report
/// (`-V`) — commit, branch, dirty state, and the toolchain that produced the
/// binary. Fields unavailable at build time (no git, e.g. a crates.io build)
/// read `unknown`.
fn print_version(full: bool) {
    println!("codemmet {}", env!("CODEMMET_LONG_VERSION"));
    if !full {
        return;
    }
    let dirty = env!("CODEMMET_GIT_DIRTY");
    let commit = env!("CODEMMET_GIT_COMMIT");
    match dirty {
        "dirty" => println!("commit:   {commit} (dirty: uncommitted changes)"),
        _ => println!("commit:   {commit}"),
    }
    println!("branch:   {}", env!("CODEMMET_GIT_BRANCH"));
    println!("profile:  {}", env!("CODEMMET_BUILD_PROFILE"));
    println!("target:   {}", env!("CODEMMET_BUILD_TARGET"));
    println!("rustc:    {}", env!("CODEMMET_RUSTC"));
    println!("features: {}", env!("CODEMMET_FEATURES"));
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
