//! `codemmet scaffold`: expand a multi-file snippet into a file manifest,
//! optionally writing it to disk.

use std::io::Read as _;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Args;
use codemmet_core::Engine;
use codemmet_core::config::schema::OnExists;
use codemmet_core::context::ExpansionContext;
use codemmet_core::scaffold::{FileOutput, ScaffoldOptions, scaffold};

use super::{
    EXIT_CONFIG, EXIT_RENDER, Sources, Tabstops, config_failure, load_registry, resolve_indent_unit,
};

/// Arguments for `codemmet scaffold`.
#[derive(Debug, Args)]
pub struct ScaffoldArgs {
    /// Language id, file extension, or LSP language id.
    #[arg(long, value_name = "ID")]
    lang: String,

    /// Tabstop handling for the PRIMARY file (others are always plain text).
    #[arg(long, value_enum, default_value = "strip")]
    tabstops: Tabstops,

    /// Literal indent unit: a single tab or a run of spaces
    /// (default: the language profile's indent).
    #[arg(long, value_name = "STR")]
    indent: Option<String>,

    /// Base directory that `path` targets are resolved against.
    #[arg(long, value_name = "DIR", default_value = ".")]
    root: PathBuf,

    /// The current file, for `self`/sibling (`.cpp`) targets.
    #[arg(long, value_name = "FILE")]
    from: Option<PathBuf>,

    /// Apply the manifest to disk (create/append files) instead of printing it.
    #[arg(long)]
    write: bool,

    /// Print the manifest as one JSON array of `{path, action, content,
    /// primary}` objects.
    #[arg(long)]
    json: bool,

    /// The invocation (a single trigger with args, e.g. `djfeature:blog`);
    /// omitted or `-` reads it from stdin.
    #[arg(value_name = "ABBR")]
    abbr: Option<String>,
}

/// One resolved manifest entry: a [`FileOutput`] with its concrete action and
/// final content decided against the filesystem.
struct Planned {
    path: PathBuf,
    action: Action,
    content: String,
    primary: bool,
}

/// What applying an entry will do.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    /// The target does not exist: create it (with any `on_create` prefix).
    Create,
    /// The target exists: append to it.
    Append,
}

impl Action {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Append => "append",
        }
    }
}

/// Run `codemmet scaffold`.
pub fn run(sources: &Sources, args: &ScaffoldArgs) -> ExitCode {
    let registry = match load_registry(sources) {
        Ok(registry) => registry,
        Err(err) => return config_failure(&err),
    };
    let indent_unit = match resolve_indent_unit(&registry, &args.lang, args.indent.as_deref()) {
        Ok(unit) => unit,
        Err(msg) => return config_failure(&msg),
    };
    let abbr = match read_abbr(args.abbr.as_deref()) {
        Ok(abbr) => abbr,
        Err(msg) => return config_failure(&msg),
    };

    let engine = Engine::with_registry(registry);
    let ctx = ExpansionContext::plain(String::new(), indent_unit);
    let opts = ScaffoldOptions {
        root: &args.root,
        current_file: args.from.as_deref(),
    };
    let outputs = match scaffold(
        &engine,
        &args.lang,
        &abbr,
        &ctx,
        &opts,
        args.tabstops.into(),
    ) {
        Ok(outputs) => outputs,
        Err(err) => {
            eprintln!("error: cannot scaffold `{abbr}`: {err}");
            return ExitCode::from(EXIT_RENDER);
        }
    };

    // Decide each entry's concrete action against the filesystem.
    let mut planned = Vec::with_capacity(outputs.len());
    for out in &outputs {
        match plan(out) {
            Ok(entry) => planned.push(entry),
            Err(msg) => {
                eprintln!("error: {msg}");
                return ExitCode::from(EXIT_CONFIG);
            }
        }
    }

    if args.json {
        return emit_json(&planned);
    }
    if args.write {
        return apply(&planned);
    }
    emit_text(&planned)
}

/// Turn a [`FileOutput`] into a [`Planned`] entry: stat the target, pick the
/// action, and fold the `on_create` prefix into the content when creating.
fn plan(out: &FileOutput) -> Result<Planned, String> {
    let exists = out.path.exists();
    let (action, content) = if exists {
        match out.on_exists {
            OnExists::Create => {
                return Err(format!(
                    "file section `{}` targets {} which already exists (its \
                     `on_exists` is `create`)",
                    out.label,
                    out.path.display()
                ));
            }
            OnExists::Inject => {
                // Phase 1: no tree-sitter injector yet — append with a note.
                eprintln!(
                    "note: `{}` exists; appending (`inject` lands in a later phase)",
                    out.path.display()
                );
                (Action::Append, out.content.clone())
            }
            OnExists::Append => (Action::Append, out.content.clone()),
        }
    } else {
        let content = out.on_create.as_ref().map_or_else(
            || out.content.clone(),
            |prefix| format!("{}\n\n{}", prefix.trim_end_matches('\n'), out.content),
        );
        (Action::Create, content)
    };
    Ok(Planned {
        path: out.path.clone(),
        action,
        content,
        primary: out.primary,
    })
}

/// Print the manifest as a JSON array.
fn emit_json(planned: &[Planned]) -> ExitCode {
    let entries: Vec<serde_json::Value> = planned
        .iter()
        .map(|e| {
            serde_json::json!({
                "path": e.path.display().to_string(),
                "action": e.action.as_str(),
                "content": e.content,
                "primary": e.primary,
            })
        })
        .collect();
    println!("{}", serde_json::Value::Array(entries));
    ExitCode::SUCCESS
}

/// Print a human-readable manifest (a dry run).
fn emit_text(planned: &[Planned]) -> ExitCode {
    for entry in planned {
        let star = if entry.primary { " (primary)" } else { "" };
        println!(
            "# {} {}{}",
            entry.action.as_str(),
            entry.path.display(),
            star
        );
        println!("{}", entry.content.trim_end_matches('\n'));
        println!();
    }
    ExitCode::SUCCESS
}

/// Apply the manifest to disk: create new files and append to existing ones.
fn apply(planned: &[Planned]) -> ExitCode {
    for entry in planned {
        if let Err(msg) = apply_one(entry) {
            eprintln!("error: {msg}");
            return ExitCode::from(EXIT_CONFIG);
        }
        println!("{} {}", entry.action.as_str(), entry.path.display());
    }
    ExitCode::SUCCESS
}

/// Apply one entry, creating parent directories as needed.
fn apply_one(entry: &Planned) -> Result<(), String> {
    let io_err =
        |verb: &str, err: &std::io::Error| format!("cannot {verb} {}: {err}", entry.path.display());
    if let Some(parent) = entry.path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|err| io_err("create directory for", &err))?;
    }
    let ends_nl = entry.content.ends_with('\n');
    match entry.action {
        Action::Create => {
            let mut body = entry.content.clone();
            if !ends_nl {
                body.push('\n');
            }
            std::fs::write(&entry.path, body).map_err(|err| io_err("write", &err))
        }
        Action::Append => {
            use std::io::Write as _;
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&entry.path)
                .map_err(|err| io_err("open", &err))?;
            // Separate the appended block with a blank line, and terminate it.
            let block = format!("\n{}{}", entry.content, if ends_nl { "" } else { "\n" });
            file.write_all(block.as_bytes())
                .map_err(|err| io_err("append to", &err))
        }
    }
}

/// Resolve the invocation argument; `None` or `-` reads stdin.
fn read_abbr(arg: Option<&str>) -> Result<String, String> {
    match arg {
        Some(abbr) if abbr != "-" => Ok(abbr.to_owned()),
        _ => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|err| format!("cannot read invocation from stdin: {err}"))?;
            Ok(buf.trim_end_matches(['\r', '\n']).to_owned())
        }
    }
}
