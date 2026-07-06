//! `codemmet expand`: abbreviation in, rendered code out.

use std::io::Read as _;
use std::process::ExitCode;

use clap::Args;
use codemmet_core::Engine;
use codemmet_core::context::ExpansionContext;
use codemmet_core::engine::ExpandError;
use codemmet_core::snippet::render::Expansion;

use super::{
    EXIT_PARSE, EXIT_RENDER, Sources, Tabstops, config_failure, load_registry, resolve_indent_unit,
};

/// Arguments for `codemmet expand`.
#[derive(Debug, Args)]
pub struct ExpandArgs {
    /// Language id, file extension, or LSP language id.
    #[arg(long, value_name = "ID")]
    lang: String,

    /// Tabstop handling.
    #[arg(long, value_enum, default_value = "strip")]
    tabstops: Tabstops,

    /// Literal indent unit: a single tab or a run of spaces
    /// (default: the language profile's indent).
    #[arg(long, value_name = "STR")]
    indent: Option<String>,

    /// Prefix prepended to every continuation line.
    #[arg(long, value_name = "STR", default_value = "")]
    base_indent: String,

    /// Print one JSON object (`{"text", "cursor", "is_snippet"}`) instead of
    /// plain text.
    #[arg(long)]
    json: bool,

    /// The abbreviation; omitted or `-` reads it from stdin.
    #[arg(value_name = "ABBR")]
    abbr: Option<String>,
}

/// Run `codemmet expand` with `user_dir` as the snippet override directory.
pub fn run(sources: Sources, args: &ExpandArgs) -> ExitCode {
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
    let ctx = ExpansionContext::plain(args.base_indent.clone(), indent_unit);
    match engine.expand(&abbr, &args.lang, &ctx, args.tabstops.into()) {
        Ok(expansion) => emit(&expansion, args.json),
        Err(err) => {
            eprintln!("error: cannot expand `{abbr}`: {err}");
            let code = match err {
                ExpandError::Parse(_) => EXIT_PARSE,
                ExpandError::Render(_) => EXIT_RENDER,
                ExpandError::UnknownLanguage(_) => super::EXIT_CONFIG,
            };
            ExitCode::from(code)
        }
    }
}

/// Resolve the abbreviation argument; `None` or `-` reads stdin, trimming
/// trailing newlines.
fn read_abbr(arg: Option<&str>) -> Result<String, String> {
    match arg {
        Some(abbr) if abbr != "-" => Ok(abbr.to_owned()),
        _ => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|err| format!("cannot read abbreviation from stdin: {err}"))?;
            Ok(buf.trim_end_matches(['\r', '\n']).to_owned())
        }
    }
}

/// Print the expansion to stdout, as plain text or a one-line JSON object.
fn emit(expansion: &Expansion, json: bool) -> ExitCode {
    if json {
        let cursor = expansion.cursor.map_or(
            serde_json::Value::Null,
            |pos| serde_json::json!({ "line": pos.line, "character": pos.character }),
        );
        let value = serde_json::json!({
            "text": expansion.body,
            "cursor": cursor,
            "is_snippet": expansion.is_snippet,
        });
        println!("{value}");
    } else {
        println!("{}", expansion.body);
    }
    ExitCode::SUCCESS
}
