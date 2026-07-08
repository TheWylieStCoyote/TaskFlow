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
    /// The target exists: splice the content in at this byte offset (inside a
    /// class/struct body, via the tree-sitter injector). Only constructed in a
    /// tree-sitter build; the minimal build always falls back to `Append`.
    #[cfg_attr(not(feature = "tree-sitter"), allow(dead_code))]
    Inject { offset: usize },
}

impl Action {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Append => "append",
            Self::Inject { .. } => "inject",
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
    let indent = indent_unit.as_str();
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
        match plan(out, &args.lang, &indent) {
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
/// action, and prepare the content (fold in `on_create` on create; compute the
/// structural insertion on inject).
fn plan(out: &FileOutput, lang: &str, indent: &str) -> Result<Planned, String> {
    let (action, content) = if out.path.exists() {
        match out.on_exists {
            OnExists::Create => {
                return Err(format!(
                    "file section `{}` targets {} which already exists (its \
                     `on_exists` is `create`)",
                    out.label,
                    out.path.display()
                ));
            }
            OnExists::Append => (Action::Append, out.content.clone()),
            OnExists::Inject => plan_inject(out, lang, indent)?,
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

/// Plan an `inject` into an existing file: ask the tree-sitter injector for a
/// structural insertion point, falling back to an append when there's no
/// container (or in a tree-sitter-free build).
#[cfg(feature = "tree-sitter")]
fn plan_inject(out: &FileOutput, lang: &str, indent: &str) -> Result<(Action, String), String> {
    let existing = std::fs::read_to_string(&out.path)
        .map_err(|err| format!("cannot read {}: {err}", out.path.display()))?;
    if let Some(inj) = codemmet_tsctx::inject(lang, &existing, &out.content, indent) {
        Ok((Action::Inject { offset: inj.offset }, inj.text))
    } else {
        eprintln!(
            "note: no container found in {}; appending instead",
            out.path.display()
        );
        Ok((Action::Append, out.content.clone()))
    }
}

// The `Result` mirrors the tree-sitter build's fallible signature; this
// variant never fails but must share the shape for a single call site.
#[cfg(not(feature = "tree-sitter"))]
#[allow(clippy::unnecessary_wraps)]
fn plan_inject(out: &FileOutput, _lang: &str, _indent: &str) -> Result<(Action, String), String> {
    eprintln!(
        "note: `{}` exists; appending (`inject` needs the tree-sitter build)",
        out.path.display()
    );
    Ok((Action::Append, out.content.clone()))
}

/// Print the manifest as a JSON array.
fn emit_json(planned: &[Planned]) -> ExitCode {
    let entries: Vec<serde_json::Value> = planned
        .iter()
        .map(|e| {
            let mut obj = serde_json::json!({
                "path": e.path.display().to_string(),
                "action": e.action.as_str(),
                "content": e.content,
                "primary": e.primary,
            });
            // An `inject` entry carries the byte offset to splice at, so an
            // editor can apply it without re-parsing.
            if let Action::Inject { offset } = e.action {
                obj["offset"] = serde_json::json!(offset);
            }
            obj
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
        // Mark the primary file so an editor wrapper knows which to open.
        let mark = if entry.primary { " (primary)" } else { "" };
        println!("{} {}{}", entry.action.as_str(), entry.path.display(), mark);
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
        Action::Inject { offset } => {
            // `content` is the pre-indented block with its own surrounding
            // newlines; splice it in at the injector's byte offset.
            let mut existing =
                std::fs::read_to_string(&entry.path).map_err(|err| io_err("read", &err))?;
            existing.insert_str(offset.min(existing.len()), &entry.content);
            std::fs::write(&entry.path, existing).map_err(|err| io_err("write", &err))
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
